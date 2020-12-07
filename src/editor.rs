use ncurses as n;

use crate::render;
use crate::tree;
use std::rc::Rc;

const CHAR_BULLET: char = '•';
const CHAR_TRIANGLE_DOWN: char = '▼';
const CHAR_TRIANGLE_RIGHT: char = '▸';

const KEY_BACKSPACE: i32 = 127;
const KEY_ENTER: i32 = 10;
const KEY_TAB: i32 = 9;

const INDENTATION: &'static str = "  ";

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
        render_tree(&self.root_bullet);
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        if key == ctrl('c') {
            return false;
        }
        match key {
            KEY_TAB => {
                tree::indent(&self.active_bullet);
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
        render_tree(&self.root_bullet);
        true
    }
}

pub fn ctrl(c: char) -> i32 {
    (c as i32) & 0x1f
}

fn render_tree(root: &Rc<tree::BulletCell>) {
    let (y, x) = render::get_yx(n::stdscr());
    n::wmove(n::stdscr(), 0, 0);

    for child in &root.borrow().children {
        render_subtree(child, 0);
    }
    n::wmove(n::stdscr(), y, x);
}

fn render_subtree(bullet: &Rc<tree::BulletCell>, indentation_lvl: usize) {
    let content = &bullet.borrow().content;
    n::addstr(&format!("{}{} {}", INDENTATION.repeat(indentation_lvl), CHAR_BULLET, content.data));
    render::clear_remaining_line(n::stdscr());

    for child in &bullet.borrow().children {
        render_subtree(child, indentation_lvl + 1);
    }
}
