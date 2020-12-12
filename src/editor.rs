use ncurses as n;

use crate::raster::{PixelState, Raster};
use crate::tree;
use crate::{render, PanelUpdate};
use render::Point;
use CursorState::*;

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
    raster: Option<Raster>,
}

impl Editor {
    pub fn new(win: n::WINDOW) -> Editor {
        Editor {
            bullet_tree: tree::Tree::new(Box::new(IdGen { current: 1 })),
            win,
            cursor: Command((0, 0)),
            raster: None,
        }
    }

    pub fn init(&mut self) {
        let result = render::tree_render(self.win, self.bullet_tree.root_iter(), 0, Some(0));
        self.cursor = match result.1 {
            Some(pos) => Insert(pos, 0),
            None => Command((0, 0)),
        };
        self.raster = Some(result.0);
        render::cursor_render(self.win, self.cursor.pos());
    }

    pub fn update(&mut self, key: &str) -> PanelUpdate {
        match self.cursor {
            Command(pos) => {
                self.on_command_key_press(&key, pos);
                let (raster, _) =
                    render::tree_render(self.win, self.bullet_tree.root_iter(), 0, None);
                self.raster = Some(raster);
            }
            Insert(pos, offset) => {
                self.on_insert_key_press(&key, pos, offset);
                let result =
                    render::tree_render(self.win, self.bullet_tree.root_iter(), 0, Some(offset));
                let cursor = match result.1 {
                    Some(pos) => Insert(pos, offset),
                    None => Command((0, 0)),
                };
                self.raster = Some(result.0);
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
                format!("{:?}", self.raster.as_ref().unwrap().get(pos).unwrap())
            } else {
                String::new()
            },
        }
    }

    pub fn cursor(&self) -> CursorState {
        self.cursor
    }

    fn on_command_key_press(&mut self, key: &str, pos: Point) {
        match key {
            "h" => {
                self.cursor = Command(render::check_bounds(self.win, pos, (0, -1)).unwrap_or(pos))
            }
            "j" => {
                self.cursor = Command(render::check_bounds(self.win, pos, (1, 0)).unwrap_or(pos))
            }
            "k" => {
                self.cursor = Command(render::check_bounds(self.win, pos, (-1, 0)).unwrap_or(pos))
            }
            "l" => {
                self.cursor = Command(render::check_bounds(self.win, pos, (0, 1)).unwrap_or(pos))
            }
            "i" => {
                if let Some(ref raster) = self.raster {
                    if let Some(PixelState::Text { id, offset }) = raster.get(self.cursor.pos()) {
                        let _ = self.bullet_tree.activate(id);
                        self.cursor = Insert(
                            self.cursor.pos(),
                            self.bullet_tree.get_active_content().len() - offset,
                        )
                    }
                }
            }
            _ => {}
        }
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
            "^?" => {
                self.bullet_tree.get_mut_active_content().pop();
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

#[derive(Copy, Clone)]
pub enum CursorState {
    Command(Point),
    // i32 is how many chars away from last char in content
    Insert(Point, usize),
}

impl CursorState {
    pub fn pos(&self) -> Point {
        match self {
            Command(pos) | Insert(pos, _) => *pos,
        }
    }
}
