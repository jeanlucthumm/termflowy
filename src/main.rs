#![allow(dead_code)]
use ncurses as n;
use std::char;

mod editor;
mod render;
mod tree;
mod raster;

use editor::Editor;
use render::debug;

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
