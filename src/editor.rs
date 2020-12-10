use ncurses as n;

use crate::render;
use crate::tree;

const KEY_BACKSPACE: i32 = 127;
const KEY_ENTER: i32 = 10;
const KEY_TAB: i32 = 9;

// TODO First goal
// - Can edit text as expected
// - Bullets new on every enter
// - Indentation levels with tab and s-tab

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
    cursor_pos: (i32, i32),
}

impl Editor {
    pub fn new(window_store: render::WindowStore) -> Editor {
        Editor {
            bullet_tree: tree::Tree::new(Box::new(IdGen { current: 1 })),
            window_store,
            cursor_pos: (0, 0),
        }
    }

    pub fn init(&mut self) {
        self.cursor_pos = match render::tree_render(self.bullet_tree.root_iter(), 0) {
            Some(coord) => coord,
            None => (0, 0),
        };
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        let key = n::keyname(key).unwrap();
        if key == "^C" {
            return false;
        }
        match key.as_str() {
            // Indent
            "^I" => {
                let _ = self.bullet_tree.indent();
            }
            // Enter
            "^J" => {
                self.bullet_tree.create_sibling();
            }
            // Backspace
            "^?" => {
                self.bullet_tree.get_mut_active_content().pop();
            }
            _ => {
                self.bullet_tree.get_mut_active_content().push_str(&key);
            }
        };
        self.cursor_pos = match render::tree_render(self.bullet_tree.root_iter(), 0) {
            Some(coord) => coord,
            None => (0, 0),
        };
        render::cursor_render(self.cursor_pos);
        true
    }
}
