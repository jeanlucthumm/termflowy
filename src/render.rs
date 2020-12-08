use crate::tree;
use ncurses as n;
use std::rc::Rc;

const CHAR_BULLET: char = '•';
const CHAR_TRIANGLE_DOWN: char = '▼';
const CHAR_TRIANGLE_RIGHT: char = '▸';
const INDENTATION: &'static str = "  ";

#[derive(Clone, Copy)]
pub struct WindowStore {
    pub debug: n::WINDOW,
}

pub fn setup_ncurses() {
    // Allows for wide characters
    n::setlocale(n::LcCategory::ctype, "");
    n::initscr();
    // Captures signal sequences and no buffer
    n::raw();
    // F keys and arrows
    n::keypad(n::stdscr(), true);
    // Doesn't echo typed keys
    n::noecho();
}

pub fn get_max_yx(win: n::WINDOW) -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getmaxyx(win, &mut y, &mut x);
    (y, x)
}

pub fn get_yx(win: n::WINDOW) -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getyx(win, &mut y, &mut x);
    (y, x)
}

pub fn pprint<T: std::fmt::Display>(msg: T) {
    n::addstr(&format!("{}\n", msg));
}

pub fn clear_remaining(win: n::WINDOW) {
    let (mut screen_y, mut screen_x, mut y, mut x): (i32, i32, i32, i32) = (0, 0, 0, 0);
    n::getmaxyx(win, &mut screen_y, &mut screen_x);
    n::getyx(win, &mut y, &mut x);

    let remaining = (screen_x - x) + (screen_y - y) * (screen_x);
    for _ in 0..remaining {
        n::addch(' ' as u32);
    }
}

pub fn clear_remaining_line(win: n::WINDOW) {
    let (mut _screen_y, mut screen_x, mut _y, mut x): (i32, i32, i32, i32) = (0, 0, 0, 0);
    n::getmaxyx(win, &mut _screen_y, &mut screen_x);
    n::getyx(win, &mut _y, &mut x);

    let remaining_line = screen_x - x;
    for _ in 0..remaining_line {
        n::addch(' ' as u32);
    }
}

pub fn tree_render(
    root: &Rc<tree::BulletCell>,
    indentation_lvl: usize,
    active_id: i32,
) -> Option<(i32, i32)> {
    n::wmove(n::stdscr(), 0, 0);
    let mut active_pos: Option<(i32, i32)> = None;
    for child in &root.borrow().children {
        active_pos = active_pos.or(subtree_render(child, indentation_lvl, active_id));
    }
    clear_remaining(n::stdscr());
    active_pos
}

pub fn subtree_render(
    bullet: &Rc<tree::BulletCell>,
    indentation_lvl: usize,
    active_id: i32,
) -> Option<(i32, i32)> {
    let content = &bullet.borrow().content;
    n::addstr(&format!(
        "{}{} {}",
        INDENTATION.repeat(indentation_lvl),
        CHAR_BULLET,
        content.data
    ));
    let mut active_pos: Option<(i32, i32)> = None;
    if bullet.borrow().id == active_id {
        active_pos = Some(get_yx(n::stdscr()));
    }
    clear_remaining_line(n::stdscr());

    for child in &bullet.borrow().children {
        active_pos = active_pos.or(subtree_render(child, indentation_lvl + 1, active_id));
    }
    active_pos
}

pub fn cursor_render(pos: (i32, i32)) {
    n::wmove(n::stdscr(), pos.0, pos.1);
}

pub mod debug {
    use super::*;

    pub fn is_quit_key(key: i32) -> bool {
        ('c' as i32) & 0x1f == key
    }

    pub fn pprint<T: std::fmt::Display>(win: n::WINDOW, msg: T) {
        n::waddstr(win, &format!("{} ", msg));
        n::wrefresh(win);
    }

    pub fn create_window(h: i32, w: i32, y: i32, x: i32) -> n::WINDOW {
        let win = n::newwin(h, w, y, x);
        n::box_(win, 0, 0);
        n::wrefresh(win);
        win
    }
}
