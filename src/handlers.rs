use std::collections::HashMap;

use crate::editor;
use crate::editor::Cursor::*;
use crate::editor::{CommandState, HandlerInput, HandlerOutput, InsertState};
use crate::raster::PixelState::*;
use crate::raster::{Browser, Direction, PixelState};
use crate::render;
use crate::render::Point;

const SEPARATORS: [char; 1] = [' '];

pub fn new_command_map() -> HashMap<String, editor::Handler> {
    let mut map: HashMap<String, editor::Handler> = HashMap::new();
    map.insert(String::from("i"), command_i);
    map.insert(String::from("h"), command_hl);
    map.insert(String::from("l"), command_hl);
    map.insert(String::from("j"), command_jk);
    map.insert(String::from("k"), command_jk);
    map.insert(String::from("b"), command_b);
    map.insert(String::from("e"), command_e);
    map.insert(String::from("A"), command_shift_a);
    map.insert(String::from("o"), command_o);
    map
}

pub fn command_i(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    if let Some(PixelState::Text { id, offset }) = p.raster.get(cursor.pos) {
        let _ = p.tree.activate(id);
        Ok(HandlerOutput {
            cursor: Some(Insert(InsertState {
                pos: cursor.pos,
                offset: p.tree.get_active_content().len() - offset,
            })),
            raster: None,
        })
    } else {
        Err(format!("unknown position: {:?}", cursor.pos))
    }
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
        .go_while(direction, |state| !state.is_text())?
        .pos();
    Ok(HandlerOutput {
        cursor: Some(Command(CommandState { pos, col: pos.1 })),
        raster: None,
    })
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
    Ok(HandlerOutput {
        cursor: Some(Command(CommandState {
            pos,
            col: cursor.col,
        })),
        raster: None,
    })
}

pub fn command_b(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    // Get initial browser position
    let browser = match p.raster.get(cursor.pos).unwrap() {
        Text { offset: 0, .. } => {
            // Go to previous bullet
            p.raster
                .browser(cursor.pos)
                .unwrap()
                .go_while(Direction::Left, |state| !state.is_text())?
        }
        Empty => {
            p.raster
                .browser(cursor.pos)
                .unwrap()
                .go_until_count(Direction::Left, 1, |_| true)?
        }
        _ => p.raster.browser(cursor.pos).unwrap(),
    };
    if let Text { id, offset } = browser.state() {
        p.tree.activate(id)?;
        let pos = jump_to_next_separator(
            p.tree.get_active_content(),
            offset,
            Direction::Left,
            1,
            &SEPARATORS,
            browser,
        )?
        .pos();
        Ok(HandlerOutput {
            cursor: Some(Command(CommandState { pos, col: pos.1 })),
            raster: None,
        })
    } else {
        Err(format!("unexpected pixel state: {:?}", browser.state()))
    }
}

pub fn command_e(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    if let Text { id, offset } = p.raster.get(cursor.pos).unwrap() {
        p.tree.activate(id)?;
        let content_chars: Vec<char> = p.tree.get_active_content().chars().collect();
        let mut new_offset = offset;
        while new_offset < content_chars.len() - 1 {
            new_offset += 1;
            if content_chars[new_offset] == ' ' && new_offset - 1 != offset {
                new_offset -= 1;
                break;
            }
        }
        let pos = p
            .raster
            .browser(cursor.pos)
            .unwrap()
            .go_until_count(Direction::Right, (new_offset - offset) as u32, |state| {
                state.is_text()
            })?
            .pos();
        Ok(HandlerOutput {
            cursor: Some(Command(CommandState { pos, col: pos.1 })),
            raster: None,
        })
    } else {
        Err(format!(
            "pixel state at position {:?} should have been text",
            cursor.pos
        ))
    }
}

pub fn command_shift_a(p: HandlerInput) -> Result<HandlerOutput, String> {
    let cursor = p.cursor.command_state();
    if let Text { .. } = p.raster.get(cursor.pos).unwrap() {
        let pos = p
            .raster
            .browser(cursor.pos)
            .unwrap()
            .go_while(Direction::Right, |state| state != PixelState::Empty)?
            .pos();
        Ok(HandlerOutput {
            cursor: Some(Insert(InsertState { pos, offset: 0 })),
            raster: None,
        })
    } else {
        Err(format!(
            "pixel state at position {:?} should have been text",
            cursor.pos
        ))
    }
}

pub fn command_o(p: HandlerInput) -> Result<HandlerOutput, String> {
    p.tree.create_sibling();
    let (raster, pos) = render::tree_render(p.win, p.tree.root_iter(), 0, 0);
    Ok(HandlerOutput {
        cursor: Some(Insert(InsertState {
            pos: pos.unwrap(),
            offset: 0,
        })),
        raster: Some(raster),
    })
}

fn find_left_text(b: Browser, col: u32) -> Result<Point, String> {
    if b.state().is_text() {
        Ok(b.pos())
    } else {
        b.go_while_or_count(Direction::Left, col, |state| !state.is_text())?
            .map(|b| {
                if b.state().is_text() {
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
    let delta = match find_separator(string, index, reverse, sep) {
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
        Some(i) => i as i32 - index as i32 + final_offset,
        None => {
            // Go to extremities if no sep
            match dir {
                Direction::Left => -(index as i32),
                Direction::Right => (string.len() - index - 1) as i32,
                _ => panic!(),
            }
        }
    };
    Ok(browser.go_wrap(dir, delta.abs() as u32)?)
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
