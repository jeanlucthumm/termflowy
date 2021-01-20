/// Invariants:
/// - Command handlers are always passed cursors which are [browsable](PixelState::is_browsable),
///   ecept the handler for <C-c>
use std::collections::HashMap;

use crate::editor::{self, Clipboard, Cursor};
use crate::editor::{CommandState, HandlerInput, HandlerOutput, InsertState};
use crate::editor::{Cursor::*, HistoryItem};
use crate::raster::PixelState::*;
use crate::raster::{Browser, Direction};
use crate::render;
use crate::render::{Point, Window};
use crate::tree::Tree;

const SEPARATORS: [char; 1] = [' '];

pub fn new_command_map() -> HashMap<String, editor::Handler> {
    let mut map: HashMap<String, editor::Handler> = HashMap::new();
    map.insert(String::from("i"), command_i);
    map.insert(String::from("h"), command_hl);
    map.insert(String::from("l"), command_hl);
    map.insert(String::from("j"), command_jk);
    map.insert(String::from("k"), command_jk);
    map.insert(String::from("b"), command_bwe);
    map.insert(String::from("w"), command_bwe);
    map.insert(String::from("e"), command_bwe);
    map.insert(String::from("A"), command_shift_a);
    map.insert(String::from("o"), command_o);
    map.insert(String::from("O"), command_shift_o);
    map.insert(String::from("d"), command_d);
    map.insert(String::from("y"), command_y);
    map.insert(String::from("p"), command_p_shift_p);
    map.insert(String::from("P"), command_p_shift_p);
    map.insert(String::from("u"), command_u);
    map
}

pub fn new_insert_map() -> HashMap<String, editor::Handler> {
    let mut map: HashMap<String, editor::Handler> = HashMap::new();
    map.insert(String::from("^I"), insert_tab);
    map.insert(String::from("KEY_BTAB"), insert_shift_tab);
    map.insert(String::from("^J"), insert_enter);
    map.insert(String::from("KEY_BACKSPACE"), insert_backspace);
    map.insert(String::from("^?"), insert_backspace);
    map.insert(String::from("^C"), insert_control_c);
    map.insert(String::from("KEY_LEFT"), insert_arrow_keys);
    map.insert(String::from("KEY_RIGHT"), insert_arrow_keys);
    map.insert(String::from("KEY_UP"), insert_arrow_keys);
    map.insert(String::from("KEY_DOWN"), insert_arrow_keys);
    map
}

pub fn command_i(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    let (id, offset) = match p.raster.get(cursor.pos).unwrap() {
        Text { id, offset } => (id, offset),
        Placeholder(id) => (id, 0),
        err => panic!(
            "handler should only be passed browsable pixel states but got: {:?}",
            err
        ),
    };
    p.tree.activate(id)?;
    Ok(HandlerOutput::new().set_cursor(Insert(InsertState {
        pos: cursor.pos,
        offset: p.tree.get_active_content().len() - offset,
    })))
}

pub fn command_hl(p: HandlerInput) -> Result<HandlerOutput, String> {
    let direction = match p.key {
        "h" => Direction::Left,
        _ => Direction::Right,
    };
    let pos = p
        .raster
        .browser(p.cursor.command_state().pos)
        .expect("")
        .go_while(direction, |state| !state.is_browsable())?
        .pos();
    Ok(HandlerOutput::new().set_cursor(Cursor::new_command(pos)))
}

pub fn command_jk(p: HandlerInput) -> Result<HandlerOutput, String> {
    let direction = match p.key {
        "j" => Direction::Down,
        _ => Direction::Up,
    };
    let cursor = p.cursor.command_state();
    let pos = p
        .raster
        .browser(cursor.pos)
        .expect("")
        .go_no_wrap(direction, 1)?
        .go_no_wrap(
            Direction::Right,
            (cursor.col as u32)
                .checked_sub(cursor.pos.1 as u32)
                .expect("y pos should never be bigger than col"),
        )?
        .map(|b| find_left_text(b, cursor.pos.1 as u32))?;
    Ok(HandlerOutput::new().set_cursor(Cursor::new_command(pos)))
}

pub fn command_bwe(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    match p.raster.get(cursor.pos) {
        Some(Text { id, .. }) => p.tree.activate(id)?,
        err => return Err(format!("invalid pixel state: {:?}", err)),
    };
    if let Some(Text { id, .. }) = p.raster.get(cursor.pos) {
        p.tree.activate(id)?;
    } else {
        return Err(String::from("invalid pixel state"));
    }
    let content = p.tree.get_active_content();
    let (dir, final_offset, skip_index) = match p.key {
        "b" => (Direction::Left, 1, 0),
        "w" => (Direction::Right, 1, content.len() - 1),
        "e" => (Direction::Right, -1, content.len() - 1),
        _ => panic!("check key handler mappings"),
    };
    // Go to another bullet if we are on extremities
    let browser = match p.raster.get(cursor.pos).unwrap() {
        Text { offset, .. } if offset == skip_index => p
            .raster
            .browser(cursor.pos)
            .unwrap()
            .go_while(dir, |state| !state.is_browsable())?,
        Text { .. } => p.raster.browser(cursor.pos).unwrap(),
        state => return Err(format!("invalid command pixel state: {:?}", state)),
    };
    if let Text { id, offset } = browser.state() {
        p.tree.activate(id)?;
        let pos = jump_to_next_separator(
            p.tree.get_active_content(),
            offset,
            dir,
            final_offset,
            &SEPARATORS,
            browser,
        )?
        .pos();
        Ok(HandlerOutput::new().set_cursor(Cursor::new_command(pos)))
    } else {
        panic!();
    }
}

pub fn command_shift_a(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    p.tree.activate(p.raster.get(cursor.pos).unwrap().id())?;
    let pos = p
        .raster
        .browser(cursor.pos)
        .unwrap()
        .map(|b| match b.state() {
            Placeholder(_) => b.pos(),
            _ => b
                .go_while(Direction::Right, |state| state.is_browsable())
                .unwrap()
                .pos(),
        });
    Ok(HandlerOutput::new().set_cursor(Cursor::new_insert(pos)))
}

pub fn command_o(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree
        .activate(p.raster.get(p.cursor.pos()).unwrap().id())?;
    p.tree.create_sibling();
    render_and_make_insert_output(p.tree, p.win, 0)
}

pub fn command_shift_o(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree
        .activate(p.raster.get(p.cursor.pos()).unwrap().id())?;
    p.tree.create_sibling_above();
    render_and_make_insert_output(p.tree, p.win, 0)
}

pub fn command_d(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    match p.sticky_key {
        Some("d") => {
            let pixel_state = p.raster.get(cursor.pos).unwrap();
            p.tree.activate(pixel_state.id())?;
            let (subtree, parent, sibling) = p.tree.get_subtree();
            p.tree.delete()?; // default active selection matches 'dd'
            let (raster, pos) = render::tree_render(p.win, p.tree.root_iter(), 0, 0);
            let pos = find_left_text(
                raster.browser((pos.unwrap().0, cursor.col))?,
                cursor.col as u32,
            )?;
            Ok(HandlerOutput::new()
                .set_cursor(Cursor::new_command(pos))
                .set_clipboard(Clipboard::Tree(subtree.clone()))
                .set_history_item(HistoryItem::Tree {
                    parent,
                    sibling,
                    tree: subtree,
                    cursor: p.cursor,
                })
                .set_raster(raster))
        }
        Some(_) => Ok(HandlerOutput::new().set_cursor(p.cursor)),
        None => Ok(HandlerOutput::new()
            .set_cursor(p.cursor)
            .set_sticky_key(String::from("d"))),
    }
}

pub fn command_y(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    match p.sticky_key {
        Some("y") => {
            let pixel_state = p.raster.get(cursor.pos).unwrap();
            p.tree.activate(pixel_state.id())?;
            let (subtree, _, _) = p.tree.get_subtree();
            Ok(HandlerOutput::new().set_clipboard(Clipboard::Tree(subtree)))
        }
        Some(_) => Ok(HandlerOutput::new().set_cursor(p.cursor)),
        None => Ok(HandlerOutput::new()
            .set_cursor(p.cursor)
            .set_sticky_key(String::from("y"))),
    }
}

pub fn command_p_shift_p(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    p.tree.activate(p.raster.get(cursor.pos).unwrap().id())?;
    let below = match p.key {
        "p" => true,
        "P" => false,
        _ => panic!("wrong key passed to handler, check table"),
    };
    match p.clipboard {
        Some(Clipboard::Tree(subtree)) => {
            p.tree.insert_subtree(subtree.clone(), below);
        }
        None => {
            return Err(String::from("nothing to paste"));
        }
    };
    let (raster, insert_pos) = render::tree_render(p.win, p.tree.root_iter(), 0, 0);
    let pos = (insert_pos.unwrap().0, cursor.pos.1);
    let pos = find_left_text(raster.browser(pos).unwrap(), pos.1 as u32)?;
    Ok(HandlerOutput::new()
        .set_cursor(Cursor::new_command(pos))
        .set_raster(raster))
}

pub fn command_u(p: HandlerInput) -> Result<HandlerOutput, String> {
    match p.history.pop_back() {
        Some(HistoryItem::Tree {
            parent,
            sibling,
            tree,
            cursor: history_cursor,
        }) => {
            match (parent, sibling) {
                (_, Some(sibling)) => {
                    p.tree.activate(sibling)?;
                    p.tree.insert_subtree(tree, true);
                }
                (parent, None) => {
                    p.tree.activate(parent)?;
                    p.tree.insert_subtree(tree, true);
                    p.tree.indent_as_first()?;
                }
            }
            let (raster, _) = render::tree_render(p.win, p.tree.root_iter(), 0, 0);
            Ok(HandlerOutput::new()
                .set_raster(raster)
                .set_cursor(history_cursor))
        }
        Some(HistoryItem::Text { .. }) => todo!(),
        None => return Ok(HandlerOutput::new()),
    }
}

fn find_left_text(b: Browser, col: u32) -> Result<Point, String> {
    if b.state().is_browsable() {
        Ok(b.pos())
    } else {
        b.go_while_or_count(Direction::Left, col, |state| !state.is_browsable())?
            .map(|b| {
                if b.state().is_browsable() {
                    Ok(b.pos())
                } else {
                    Err(String::from("no text on target line"))
                }
            })
    }
}

fn find_separator(string: &str, mut index: usize, reverse: bool, sep: &[char]) -> Option<usize> {
    if index >= string.len() {
        return None;
    }
    let string: Vec<char> = string.chars().collect();
    if reverse {
        while index > 1 {
            index -= 1;
            if sep.iter().any(|c| *c == string[index]) {
                return Some(index);
            }
        }
    } else {
        while index < string.len() - 1 {
            index += 1;
            if sep.iter().any(|c| *c == string[index]) {
                return Some(index);
            }
        }
    }
    None
}

fn jump_to_next_separator<'a>(
    string: &str,
    index: usize,
    dir: Direction,
    final_offset: i32,
    sep: &[char],
    browser: Browser<'a>,
) -> Result<Browser<'a>, String> {
    let reverse = match dir {
        Direction::Left => true,
        Direction::Right => false,
        _ => panic!(),
    };
    let final_index = match find_separator(string, index, reverse, sep) {
        // Ignore separator if it is right next to current index
        Some(i) if (i as i32 - index as i32).abs() == 1 => {
            return jump_to_next_separator(
                string,
                i,
                dir,
                final_offset,
                sep,
                browser.go_wrap(dir, 1)?,
            );
        }
        Some(i) => i as i32 + final_offset,
        None => {
            // Go to extremities if no sep
            match dir {
                Direction::Left => 0,
                Direction::Right => string.len() as i32 - 1,
                _ => panic!(),
            }
        }
    };
    let final_index = match final_index {
        x if x < 0 => 0,
        x if x >= string.len() as i32 => string.len().saturating_sub(1) as i32,
        _ => final_index,
    };
    Ok(browser.go_wrap(dir, (final_index - index as i32).abs() as u32)?)
}

pub fn insert_tab(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree.indent()?;
    render_and_make_insert_output(p.tree, p.win, 0)
}

pub fn insert_shift_tab(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree.unindent()?;
    render_and_make_insert_output(p.tree, p.win, 0)
}

pub fn insert_enter(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree.create_sibling();
    render_and_make_insert_output(p.tree, p.win, 0)
}

pub fn insert_backspace(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.insert_state();
    let content = p.tree.get_mut_active_content();
    if let Some(remove_index) = content
        .len()
        .checked_sub(cursor.offset)
        .expect("offset should not be larger than length of content")
        .checked_sub(1)
    {
        content.remove(remove_index);
        render_and_make_insert_output(p.tree, p.win, 0)
    } else {
        let mut itr = p.tree.active_iter();
        let new_active = match itr.next_sibling() {
            Some(id) => id,
            None => match itr.next_parent() {
                Some(id) => id,
                None => return Err(String::from("cannot backspace over first bullet")),
            },
        };
        p.tree.delete()?;
        p.tree.activate(new_active)?;
        render_and_make_insert_output(p.tree, p.win, 0)
    }
}

pub fn insert_control_c(p: HandlerInput) -> Result<HandlerOutput, String> {
    let pos = p.cursor.pos();
    Ok(
        HandlerOutput::new().set_cursor(Command(match p.raster.get(pos).unwrap() {
            state if state.is_browsable() => CommandState { pos, col: pos.1 },
            Empty => {
                // We are inserting at end so cursor is one past text
                let pos = p
                    .raster
                    .browser(pos)
                    .unwrap()
                    .go_wrap(Direction::Left, 1)?
                    .map(|b| match b.state() {
                        Text { .. } => b.pos(),
                        err => panic!("insert cursor was out of bounds on ctrl-c: {:?}", err),
                    });
                CommandState { pos, col: pos.1 }
            }
            _ => panic!("insert cursor was out of bounds on ctrl-c"),
        })),
    )
}

fn insert_arrow_keys(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree
        .get_mut_active_content()
        .push_str(" USE VIM KEYBINDINGS YOU PLEB ");
    render_and_make_insert_output(p.tree, p.win, p.cursor.insert_state().offset)
}

fn render_and_make_insert_output(
    tree: &mut Tree,
    win: &mut dyn Window,
    offset: usize,
) -> Result<HandlerOutput, String> {
    let (raster, pos) = render::tree_render(win, tree.root_iter(), 0, 0);
    if let Some(pos) = pos {
        Ok(HandlerOutput::new()
            .set_cursor(Insert(InsertState { offset, pos }))
            .set_raster(raster))
    } else {
        Err(String::from("tree could not find active bullet position"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_separator_test() {
        assert_eq!(find_separator("asdf", 1, false, &[' ']), None);
        assert_eq!(find_separator("asdf", 1, true, &[' ']), None);
        assert_eq!(find_separator("as df", 3, true, &[' ']), Some(2));
        assert_eq!(find_separator("as df", 1, false, &[' ']), Some(2));
        // On the sep
        assert_eq!(find_separator("as df fff", 5, false, &[' ']), None);
        assert_eq!(find_separator("as df fff", 5, true, &[' ']), Some(2));
    }
}
