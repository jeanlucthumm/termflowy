#![allow(dead_code)]
use ncurses as n;
use std::char;

mod editor;
mod render;

use editor::Editor;


fn setup_ncurses() {
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

fn get_max_yx(win: n::WINDOW) -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getmaxyx(win, &mut y, &mut x);
    (y, x)
}

// --------------------------------------------------------------------------------

fn test1() {
    n::addstr("Type any character to see it in bold\n");
    let ch = n::getch();

    if ch == n::KEY_F(1) {
        n::addstr("F1 key pressed");
    } else {
        n::addstr("The pressed key is ");
        n::attron(n::A_BOLD() | n::A_BLINK());
        n::addstr(&format!(
            "{}\n",
            char::from_u32(ch as u32).expect("could not convert character")
        ));
        n::attroff(n::A_BOLD() | n::A_BLINK());
    }
}

fn print_center(msg: &str) {
    let (y, x) = render::get_max_yx(n::stdscr());
    n::mvprintw(y / 2, (x - msg.chars().count() as i32) / 2, msg);
}

fn main_loop(e: &mut Editor) {
    loop {
        let key = n::getch();
        if key == editor::ctrl('c') {
            break;
        }
        if !e.on_key_press(key) {
            break;
        }
        n::refresh();
    }
}

fn main() {
    render::setup_ncurses();

    main_loop(&mut Editor::new());
    n::endwin();
}
