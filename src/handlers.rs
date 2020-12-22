use std::collections::HashMap;

use crate::editor::Cursor::*;
use crate::editor::{CommandState, Cursor, InsertState};
use crate::raster::{Browser, Direction, PixelState, Raster};
use crate::render::Point;
use crate::tree::Tree;
use crate::editor;

pub fn new_command_map() -> HashMap<String, editor::CommandHandler> {
    let mut map: HashMap<String, editor::CommandHandler> = HashMap::new();
    map.insert(String::from("i"), command_i);
    map.insert(String::from("h"), command_hl);
    map.insert(String::from("l"), command_hl);
    map.insert(String::from("j"), command_jk);
    map.insert(String::from("k"), command_jk);
    map
}

pub fn command_i(
    _key: &str,
    cursor: CommandState,
    tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, &'static str> {
    if let Some(PixelState::Text { id, offset }) = raster.get(cursor.pos) {
        let _ = tree.activate(id);
        Ok(Insert(InsertState {
            pos: cursor.pos,
            offset: tree.get_active_content().len() - offset,
        }))
    } else {
        Err("")
    }
}

pub fn command_hl(
    key: &str,
    cursor: CommandState,
    _tree: &mut Tree,
    raster: &Raster,
) -> Result<Cursor, &'static str> {
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
) -> Result<Cursor, &'static str> {
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
    Ok(Command(CommandState { pos, col: cursor.col }))
}

fn find_left_text(b: Browser, col: u32) -> Result<Point, &str> {
    if b.state().is_text() {
        Ok(b.pos())
    } else {
        b.go_while_or_count(Direction::Left, col, |state| !state.is_text())?
            .map(|b| {
                if b.state().is_text() {
                    Ok(b.pos())
                } else {
                    Err("no text on target line")
                }
            })
    }
}
