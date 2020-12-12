#![allow(dead_code)]
use editor::Editor;
use ncurses as n;
use render::debug;
use std::time::{Duration, Instant};

mod editor;
mod raster;
mod render;
mod tree;

struct RenderStats {
    key_render_times: Vec<Duration>,
}

fn average(times: &Vec<Duration>) -> f32 {
    times.iter().map(|d| d.as_millis()).sum::<u128>() as f32 / times.len() as f32
}

fn main_loop(e: &mut Editor, windows: render::WindowStore) -> RenderStats {
    e.init();
    let mut is_debug = false;
    let mut stats = RenderStats {
        key_render_times: vec![],
    };
    loop {
        let key = n::getch();
        let key_str = n::keyname(key).unwrap();
        if key_str == "^[" {
            break;
        }
        if key_str == "^D" {
            is_debug = !is_debug;
        }
        if is_debug {
            debug::pprint(windows.debug, key_str);
        } else {
            let now = Instant::now();
            if !e.on_key_press(key) {
                break;
            }
            stats.key_render_times.push(now.elapsed());
        }

        n::refresh();
    }
    stats
}

fn main() {
    render::setup_ncurses();

    let window_store = render::WindowStore {
        debug: render::debug::create_window(10, 50, 10, 10),
    };

    let stats = main_loop(&mut Editor::new(n::stdscr()), window_store);

    n::getch();
    n::endwin();

    // currently around 38 ms mostly due to the raster (without, it's 2 ms)
    println!("average latency: {:.2}", average(&stats.key_render_times));
}
