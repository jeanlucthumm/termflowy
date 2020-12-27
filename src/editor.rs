use std::collections::HashMap;

use render::Point;
use Cursor::*;

use crate::raster::Raster;
use crate::render::{tree_render, Window};
use crate::{handlers, tree};
use crate::{render, PanelUpdate};

const ERR_BOUNDS: &str = "cursor position was out of bounds";

struct IdGen {
    current: i32,
}

impl tree::IdGenerator for IdGen {
    fn gen(&mut self) -> i32 {
        (self.current, self.current += 1).0
    }
}

pub struct Editor {
    bullet_tree: tree::Tree,
    win: Box<dyn Window>,
    cursor: Cursor,
    raster: Raster,
    command_map: HashMap<String, Handler>,
    insert_map: HashMap<String, Handler>,
}

impl Editor {
    pub fn new(mut win: Box<dyn Window>) -> Editor {
        let tree = tree::Tree::new(Box::new(IdGen { current: 1 }));
        let (raster, cursor) = render::tree_render(&mut *win, tree.root_iter(), 0, 0);
        let cursor = match cursor {
            Some(pos) => Insert(InsertState { pos, offset: 0 }),
            None => Command(CommandState {
                pos: (0, 0),
                col: 0,
            }),
        };
        win.move_cursor(cursor.pos());
        Editor {
            bullet_tree: tree,
            win,
            cursor,
            raster,
            command_map: handlers::new_command_map(),
            insert_map: handlers::new_insert_map(),
        }
    }

    pub fn update(&mut self, key: &str) -> PanelUpdate {
        let mut status_msg = String::new();
        match self.cursor {
            Command(_) => {
                if let Err(msg) = self.on_command_key_press(&key) {
                    status_msg = msg;
                }
            }
            Insert(_) => {
                if let Err(msg) = self.on_insert_key_press(&key) {
                    status_msg = msg;
                }
            }
        }
        self.win.move_cursor(self.cursor.pos());
        PanelUpdate {
            should_quit: false,
            status_msg,
        }
    }

    pub fn focus(&mut self) {
        self.win.move_cursor(self.cursor.pos());
        self.win.refresh();
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn on_command_key_press(&mut self, key: &str) -> Result<(), String> {
        if let Some(handler) = self.command_map.get(key) {
            let output = (*handler)(HandlerInput {
                key,
                cursor: self.cursor,
                tree: &mut self.bullet_tree,
                raster: &self.raster,
                win: &mut *self.win,
            })?;
            if let Some(cursor) = output.cursor {
                self.cursor = cursor;
            }
            if let Some(raster) = output.raster {
                self.raster = raster;
            }
            Ok(())
        } else {
            Err(format!("unknown command key: {}", key))
        }
    }

    fn on_insert_key_press(&mut self, key: &str) -> Result<(), String> {
        if let Some(handler) = self.insert_map.get(key) {
            let output = (*handler)(HandlerInput {
                key,
                cursor: self.cursor,
                tree: &mut self.bullet_tree,
                raster: &self.raster,
                win: &mut *self.win,
            })?;
            if let Some(cursor) = output.cursor {
                self.cursor = cursor;
            }
            if let Some(raster) = output.raster {
                self.raster = raster;
            }
            Ok(())
        } else {
            let content = self.bullet_tree.get_mut_active_content();
            let cursor = self.cursor.insert_state();
            content.insert_str(content.len() - cursor.offset, &key);
            let (raster, pos) = tree_render(
                self.win.as_mut(),
                self.bullet_tree.root_iter(),
                0,
                cursor.offset,
            );
            self.raster = raster;
            self.cursor = Insert(InsertState {
                pos: pos.unwrap(),
                offset: cursor.offset,
            });
            Ok(())
        }
    }
}

#[derive(Copy, Clone)]
pub struct CommandState {
    pub pos: Point,
    pub col: i32,
}

#[derive(Copy, Clone)]
pub struct InsertState {
    pub pos: Point,
    pub offset: usize,
}

#[derive(Copy, Clone)]
pub enum Cursor {
    Command(CommandState),
    Insert(InsertState),
}

impl Cursor {
    pub fn pos(&self) -> Point {
        match self {
            Command(CommandState { pos, .. }) | Insert(InsertState { pos, .. }) => *pos,
        }
    }

    pub fn command_state(self) -> CommandState {
        match self {
            Command(state) => state,
            _ => panic!("assumed cursor was command but it was not"),
        }
    }

    pub fn insert_state(self) -> InsertState {
        match self {
            Insert(state) => state,
            _ => panic!("assumed cursor was insert but it was not"),
        }
    }
}

pub type Handler = fn(HandlerInput) -> Result<HandlerOutput, String>;

pub struct HandlerInput<'a> {
    pub key: &'a str,
    pub cursor: Cursor,
    pub tree: &'a mut tree::Tree,
    pub raster: &'a Raster,
    pub win: &'a mut dyn Window,
}

pub struct HandlerOutput {
    pub cursor: Option<Cursor>,
    pub raster: Option<Raster>,
}
