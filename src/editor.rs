use ncurses as n;

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
    window_store: render::WindowStore,
    cursor: CursorState,
}

impl Editor {
    pub fn new(window_store: render::WindowStore) -> Editor {
        Editor {
            bullet_tree: tree::Tree::new(Box::new(IdGen { current: 1 })),
            window_store,
            cursor: Command((0, 0)),
        }
    }

    pub fn init(&mut self) {
        self.cursor = match render::tree_render(self.bullet_tree.root_iter(), 0) {
            Some(pos) => Insert(pos),
            None => Command((0, 0)),
        };
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        let key = n::keyname(key).unwrap();
        if key == "^[" {
            return false;
        }
        match self.cursor {
            Command(pos) => {
                self.on_command_key_press(&key, pos);
                let _ = render::tree_render(self.bullet_tree.root_iter(), 0);
            },
            Insert(pos) => {
                self.on_insert_key_press(&key, pos);
                let cursor = match render::tree_render(self.bullet_tree.root_iter(), 0) {
                    Some(pos) => Insert(pos),
                    None => Command((0, 0)),
                };
                if let Insert(_) = self.cursor {
                    self.cursor = cursor;
                }
            },
        }
        let pos = match self.cursor {
            Command(pos) => pos,
            Insert(pos) => pos,
        };
        render::cursor_render(pos);
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
            Command(pos) => *pos,
            Insert(pos) => *pos,
        }
    }
}