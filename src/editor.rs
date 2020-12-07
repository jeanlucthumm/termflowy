use crate::render;
use crate::tree;
use std::rc::Rc;

const KEY_BACKSPACE: i32 = 127;
const KEY_ENTER: i32 = 10;
const KEY_TAB: i32 = 9;

// TODO First goal
// - Can edit text as expected
// - Bullets new on every enter
// - Indentation levels with tab and s-tab

pub struct Editor {
    root_bullet: Rc<tree::BulletCell>,
    active_bullet: Rc<tree::BulletCell>,
    window_store: render::WindowStore,
    id_gen: IdGen,
}

struct IdGen {
    current: i32,
}

impl tree::IdGenerator for IdGen {
    fn gen(&mut self) -> i32 {
        (self.current, self.current += 1).0
    }
}

impl Editor {
    pub fn new(window_store: render::WindowStore) -> Editor {
        let mut id_gen = IdGen {
            current: 0,
        };
        let (root, active) = tree::new_tree(&mut id_gen);
        Editor {
            root_bullet: root,
            active_bullet: active,
            window_store,
            id_gen,
        }
    }

    pub fn init(&self) {
        render::tree_render(&self.root_bullet, 0);
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        if key == ctrl('c') {
            return false;
        }
        match key {
            KEY_TAB => {
                let _ = tree::indent(&self.active_bullet);
            }
            KEY_ENTER => {
                self.active_bullet = tree::create_sibling_of(&self.active_bullet, &mut self.id_gen);
            }
            KEY_BACKSPACE => {
                self.active_bullet.borrow_mut().content.data.pop();
            }
            _ => {
                self.active_bullet.borrow_mut().content.data.push(key as u8 as char);
            }
        };
        render::tree_render(&self.root_bullet, 0);
        true
    }
}

pub fn ctrl(c: char) -> i32 {
    (c as i32) & 0x1f
}
