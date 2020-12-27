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
    map.insert(String::from("b"), command_bwe);
    map.insert(String::from("w"), command_bwe);
    map.insert(String::from("e"), command_bwe);
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
    Ok(make_pos_command_output(
        p.raster
            .browser(p.cursor.command_state().pos)
            .expect("")
            .go_while(direction, |state| !state.is_text())?
            .pos(),
    ))
}

pub fn command_jk(p: HandlerInput) -> Result<HandlerOutput, String> {
    let direction = match p.key {
        "j" => Direction::Down,
        _ => Direction::Up,
    };
    let cursor = p.cursor.command_state();
    Ok(make_pos_command_output(
        p.raster
            .browser(cursor.pos)
            .expect("")
            .go_no_wrap(direction, 1)?
            .go_no_wrap(
                Direction::Right,
                (cursor.col as u32)
                    .checked_sub(cursor.pos.1 as u32)
                    .expect("y pos should never be bigger than col"),
            )?
            .map(|b| find_left_text(b, cursor.pos.1 as u32))?,
    ))
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
            .go_while(dir, |state| !state.is_text())?,
        Text { .. } => p.raster.browser(cursor.pos).unwrap(),
        state => return Err(format!("invalid command pixel state: {:?}", state)),
    };
    if let Text { id, offset } = browser.state() {
        p.tree.activate(id)?;
        Ok(make_pos_command_output(
            jump_to_next_separator(
                p.tree.get_active_content(),
                offset,
                dir,
                final_offset,
                &SEPARATORS,
                browser,
            )?
            .pos(),
        ))
    } else {
        panic!();
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
        x if x >= string.len() as i32 => string.len().checked_sub(1).unwrap_or(0) as i32,
        _ => final_index,
    };
    Ok(browser.go_wrap(dir, (final_index - index as i32).abs() as u32)?)
}

fn make_pos_command_output(pos: Point) -> HandlerOutput {
    HandlerOutput {
        cursor: Some(Command(CommandState { pos, col: pos.1 })),
        raster: None,
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
