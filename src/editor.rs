use ncurses as n;

use crate::raster::{Browser, Direction, PixelState, Raster};
use crate::tree;
use crate::{render, PanelUpdate};
use render::Point;
use CursorState::*;

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
    win: n::WINDOW,
    cursor: CursorState,
    raster: Raster,
}

impl Editor {
    pub fn new(win: n::WINDOW) -> Editor {
        let tree = tree::Tree::new(Box::new(IdGen { current: 1 }));
        let (raster, cursor) = render::tree_render(win, tree.root_iter(), 0, 0);
        let cursor = match cursor {
            Some(pos) => Insert(pos, 0),
            None => Command((0, 0)),
        };
        render::cursor_render(win, cursor.pos());
        Editor {
            bullet_tree: tree,
            win,
            cursor,
            raster,
        }
    }

    pub fn update(&mut self, key: &str) -> PanelUpdate {
        match self.cursor {
            Command(pos) => {
                self.on_command_key_press(&key, pos);
                let (raster, _) = render::tree_render(self.win, self.bullet_tree.root_iter(), 0, 0);
                self.raster = raster;
            }
            Insert(pos, offset) => {
                self.on_insert_key_press(&key, pos, offset);
                let result = render::tree_render(self.win, self.bullet_tree.root_iter(), 0, offset);
                let cursor = match result.1 {
                    Some(pos) => Insert(pos, offset),
                    None => Command((0, 0)),
                };
                self.raster = result.0;
                if let Insert(_, _) = self.cursor {
                    self.cursor = cursor;
                }
            }
        }
        render::cursor_render(self.win, self.cursor.pos());
        PanelUpdate {
            should_render: true,
            should_quit: false,
            status_msg: if let Command(pos) = self.cursor {
                format!("{:?}", self.raster.get(pos).unwrap())
            } else {
                String::new()
            },
        }
    }

    pub fn cursor(&self) -> CursorState {
        self.cursor
    }

    fn on_command_key_press(&mut self, key: &str, pos: Point) -> Result<(), &str> {
        match key {
            "h" => {
                self.cursor = Command(
                    self.raster
                        .browser(pos)
                        .expect(ERR_BOUNDS)
                        .go_while(Direction::Left, |state| !state.is_text())?
                        .pos(),
                );
            }
            "j" => {
                self.cursor = Command(
                    self.raster
                        .browser(pos)
                        .expect(ERR_BOUNDS)
                        .go_no_wrap(Direction::Down)?
                        .map(|b| find_left_text(b, pos.1 as u32))?,
                );
            }
            "k" => {
                self.cursor = Command(
                    self.raster
                        .browser(pos)
                        .expect(ERR_BOUNDS)
                        .go_no_wrap(Direction::Up)?
                        .map(|b| find_left_text(b, pos.1 as u32))?,
                );
            }
            "l" => {
                self.cursor = Command(
                    self.raster
                        .browser(pos)
                        .expect(ERR_BOUNDS)
                        .go_while(Direction::Right, |state| !state.is_text())?
                        .pos(),
                );
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
                self.cursor = Command(self.cursor.pos());
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
    Command(Point),
    // usize is how many chars away from last char in content
    Insert(Point, usize),
}

impl CursorState {
    pub fn pos(&self) -> Point {
        match self {
            Command(pos) | Insert(pos, _) => *pos,
        }
    }
}
