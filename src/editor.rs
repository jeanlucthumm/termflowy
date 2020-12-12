use ncurses as n;

use crate::raster::Raster;
use crate::render;
use crate::tree;
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
        let result = render::tree_render(self.win, self.bullet_tree.root_iter(), 0);
        self.cursor = match result.1 {
            Some(pos) => Insert(pos),
            None => Command((0, 0)),
        };
        self.raster = Some(result.0);
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        let key = n::keyname(key).unwrap();
        if key == "^[" {
            return false;
        }
        match self.cursor {
            Command(pos) => {
                self.on_command_key_press(&key, pos);
                let (raster, _) = render::tree_render(self.win, self.bullet_tree.root_iter(), 0);
                self.raster = Some(raster);
            }
            Insert(pos) => {
                self.on_insert_key_press(&key, pos);
                let result = render::tree_render(self.win, self.bullet_tree.root_iter(), 0);
                let cursor = match result.1 {
                    Some(pos) => Insert(pos),
                    None => Command((0, 0)),
                };
                self.raster = Some(result.0);
                if let Insert(_) = self.cursor {
                    self.cursor = cursor;
                }
            }
        }
        let pos = match self.cursor {
            Command(pos) => pos,
            Insert(pos) => pos,
        };
        render::cursor_render(self.win, pos);
        true
    }

    fn on_command_key_press(&mut self, key: &str, pos: Point) {
        match key {
            "h" => self.cursor = Command(render::check_bounds(pos, (0, -1)).unwrap_or(pos)),
            "j" => self.cursor = Command(render::check_bounds(pos, (1, 0)).unwrap_or(pos)),
            "k" => self.cursor = Command(render::check_bounds(pos, (-1, 0)).unwrap_or(pos)),
            "l" => self.cursor = Command(render::check_bounds(pos, (0, 1)).unwrap_or(pos)),
            _ => {}
        }
    }

    fn on_insert_key_press(&mut self, key: &str, _pos: Point) {
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
                self.bullet_tree.get_mut_active_content().push_str(&key);
            }
        };
    }
}

enum CursorState {
    Command(Point),
    Insert(Point),
}

impl CursorState {
    fn pos(&self) -> Point {
        match self {
            Command(pos) | Insert(pos) => *pos,
        }
    }
}
