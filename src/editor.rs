use CursorState::*;
use render::Point;

use crate::{PanelUpdate, render};
use crate::raster::{Browser, Direction, PixelState, Raster};
use crate::render::Window;
use crate::tree;

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
    cursor: CursorState,
    raster: Raster,
}

impl Editor {
    pub fn new(mut win: Box<dyn Window>) -> Editor {
        let tree = tree::Tree::new(Box::new(IdGen { current: 1 }));
        let (raster, cursor) = render::tree_render(&mut *win, tree.root_iter(), 0, 0);
        let cursor = match cursor {
            Some(pos) => Insert(pos, 0),
            None => Command((0, 0), 0),
        };
        win.move_cursor(cursor.pos());
        Editor {
            bullet_tree: tree,
            win,
            cursor,
            raster,
        }
    }

    pub fn update(&mut self, key: &str) -> PanelUpdate {
        match self.cursor {
            Command(pos, col) => {
                let _ = self.on_command_key_press(&key, pos, col);
                let (raster, _) = render::tree_render(&mut *self.win, self.bullet_tree.root_iter(), 0, 0);
                self.raster = raster;
            }
            Insert(pos, offset) => {
                self.on_insert_key_press(&key, pos, offset);
                let result = render::tree_render(&mut *self.win, self.bullet_tree.root_iter(), 0, offset);
                let cursor = match result.1 {
                    Some(pos) => Insert(pos, offset),
                    None => Command((0, 0), 0),
                };
                self.raster = result.0;
                if let Insert(_, _) = self.cursor {
                    self.cursor = cursor;
                }
            }
        }
        self.win.move_cursor(self.cursor.pos());
        PanelUpdate {
            should_render: true,
            should_quit: false,
            status_msg: if let Command(pos, _) = self.cursor {
                format!("{:?}", self.raster.get(pos).unwrap())
            } else {
                String::new()
            },
        }
    }

    pub fn focus(&mut self) {
        self.win.move_cursor(self.cursor.pos());
        self.win.refresh();
    }

    pub fn cursor(&self) -> CursorState {
        self.cursor
    }

    fn on_command_key_press(&mut self, key: &str, pos: Point, col: i32) -> Result<(), &str> {
        match key {
            "h" => {
                let pos = self
                    .raster
                    .browser(pos)
                    .expect(ERR_BOUNDS)
                    .go_while(Direction::Left, |state| !state.is_text())?
                    .pos();
                self.cursor = Command(pos, pos.1);
            }
            "j" | "k" => {
                let initial_dir = if key == "j" {
                    Direction::Down
                } else {
                    Direction::Up
                };
                let pos = self
                    .raster
                    .browser(pos)
                    .expect(ERR_BOUNDS)
                    .go_no_wrap(initial_dir, 1)?
                    .go_no_wrap(
                        Direction::Right,
                        (col as u32)
                            .checked_sub(pos.1 as u32)
                            .expect("y pos should never be bigger than col"),
                    )?
                    .map(|b| find_left_text(b, pos.1 as u32))?;
                self.cursor = Command(pos, col);
            }
            "l" => {
                let pos = self
                    .raster
                    .browser(pos)
                    .expect(ERR_BOUNDS)
                    .go_while(Direction::Right, |state| !state.is_text())?
                    .pos();
                self.cursor = Command(pos, pos.1);
            }
            "i" => {
                if let Some(PixelState::Text { id, offset }) = self.raster.get(self.cursor.pos()) {
                    let _ = self.bullet_tree.activate(id);
                    self.cursor = Insert(
                        self.cursor.pos(),
                        self.bullet_tree.get_active_content().len() - offset,
                    )
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_insert_key_press(&mut self, key: &str, _pos: Point, offset: usize) {
        match key {
            // Indent
            "^I" => {
                let _ = self.bullet_tree.indent();
            }
            // Unindent
            "KEY_BTAB" => {
                let _ = self.bullet_tree.unindent();
            }
            // Enter
            "^J" => {
                self.bullet_tree.create_sibling();
            }
            // Backspace
            "KEY_BACKSPACE" => {
                let content = self.bullet_tree.get_mut_active_content();
                if let Some(remove_index) = content
                    .len()
                    .checked_sub(offset)
                    .expect("offset should not be larger than length of content")
                    .checked_sub(1)
                {
                    content.remove(remove_index);
                }
            }
            "^C" => {
                self.cursor = Command(self.cursor.pos(), self.cursor().pos().1);
            }
            _ => {
                let content = self.bullet_tree.get_mut_active_content();
                content.insert_str(content.len() - offset, &key);
            }
        };
    }
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

#[derive(Copy, Clone)]
pub enum CursorState {
    // i32 is the 'x' of the last horizontal move
    Command(Point, i32),
    // usize is how many chars away from last char in content
    Insert(Point, usize),
}

impl CursorState {
    pub fn pos(&self) -> Point {
        match self {
            Command(pos, _) | Insert(pos, _) => *pos,
        }
    }
}
