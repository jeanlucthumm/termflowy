use std::collections::HashMap;

use crate::editor;
use crate::editor::Cursor::*;
use crate::editor::{CommandState, Cursor, InsertState};
use crate::raster::PixelState::*;
use crate::raster::{Browser, Direction, PixelState, Raster};
use crate::render::Point;
use crate::tree::Tree;

pub fn new_command_map() -> HashMap<String, editor::CommandHandler> {
    let mut map: HashMap<String, editor::CommandHandler> = HashMap::new();
    map.insert(String::from("i"), command_i);
    map.insert(String::from("h"), command_hl);
    map.insert(String::from("l"), command_hl);
    map.insert(String::from("j"), command_jk);
    map.insert(String::from("k"), command_jk);
    map.insert(String::from("b"), command_b);
    map
}

pub fn command_i(
    _key: &str,
    cursor: CommandState,
    tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, String> {
    if let Some(PixelState::Text { id, offset }) = raster.get(cursor.pos) {
        let _ = tree.activate(id);
        Ok(Insert(InsertState {
            pos: cursor.pos,
            offset: tree.get_active_content().len() - offset,
        }))
    } else {
        Err(format!("unknown position: {:?}", cursor.pos))
    }
}

pub fn command_hl(
    key: &str,
    cursor: CommandState,
    _tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, String> {
    let direction = match key {
        "h" => Direction::Left,
        _ => Direction::Right,
    };
    let pos = raster
        .browser(cursor.pos)
        .expect("")
        .go_while(direction, |state| !state.is_text())?
        .pos();
    Ok(Command(CommandState { pos, col: pos.1 }))
}

pub fn command_jk(
    key: &str,
    cursor: CommandState,
    _tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, String> {
    let direction = match key {
        "j" => Direction::Down,
        _ => Direction::Up,
    };
    let pos = raster
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
    Ok(Command(CommandState {
        pos,
        col: cursor.col,
    }))
}

pub fn command_b(
    _key: &str,
    cursor: CommandState,
    tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, String> {
    if let Text { id, offset } = raster.get(cursor.pos).unwrap() {
        tree.activate(id)?;
        let content_chars: Vec<char> = tree.get_active_content().chars().collect();
        let mut new_offset = offset;
        while new_offset > 0 {
            new_offset -= 1;
            if content_chars[new_offset] == ' ' && new_offset + 1 != offset {
                new_offset += 1;
                break;
            }
        }
        let pos = raster
            .browser(cursor.pos)
            .unwrap()
            .go_until_count(Direction::Left, (offset - new_offset) as u32, |state| {
                state.is_text()
            })?
            .pos();
        Ok(Command(CommandState { pos, col: pos.1 }))
    } else {
        Err(format!(
            "pixel state at position {:?} should have been text",
            cursor.pos
        ))
    }
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
