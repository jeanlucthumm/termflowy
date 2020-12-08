#![allow(dead_code)]
use ncurses as n;
use std::char;

mod editor;
mod render;
mod tree;

use editor::Editor;
use render::debug;

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

fn main_loop(e: &mut Editor, windows: render::WindowStore) {
    e.init();
    let mut is_debug = false;
    loop {
        let key = n::getch();
        let key_str = n::keyname(key).unwrap();
        if key_str == "^C" {
            break;
        }
        if key_str == "^D" {
            is_debug = !is_debug;
        }
        if is_debug {
            debug::pprint(windows.debug, key_str);
        } else {
            if !e.on_key_press(key) {
                break;
            }
        }

        n::refresh();
    }
}

fn main() {
    render::setup_ncurses();

    let window_store = render::WindowStore {
        debug: render::debug::create_window(10, 50, 10, 10),
    };

    main_loop(&mut Editor::new(window_store), window_store);

    n::getch();
    n::endwin();
}
