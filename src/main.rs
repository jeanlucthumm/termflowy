#![allow(dead_code)]
use editor::Editor;
use ncurses as n;
use std::time::{Duration, Instant};
use crate::status::render_status;

mod editor;
mod raster;
mod render;
mod tree;
mod status;

struct RenderStats {
    key_render_times: Vec<Duration>,
}

fn average(times: &Vec<Duration>) -> f32 {
    times.iter().map(|d| d.as_millis()).sum::<u128>() as f32 / times.len() as f32
}

fn main_loop(e: &mut Editor, w: render::WindowStore) -> RenderStats {
    let mut stats = RenderStats {
        key_render_times: vec![],
    };
    e.init();
    n::wrefresh(w.editor);
    render_status(w.status, e.cursor());
    n::wrefresh(w.status);
    loop {
        let key = n::getch();
        let key_str = n::keyname(key).unwrap();
        if key_str == "^[" {
            break;
        }
        let now = Instant::now();
        if !e.on_key_press(key) {
            break;
        }
        stats.key_render_times.push(now.elapsed());
        let cursor = e.cursor();

        render_status(w.status, cursor);

        n::wrefresh(w.status);
        n::wmove(w.editor, cursor.pos().0, cursor.pos().1);
        n::wrefresh(w.editor);
    }
    stats
}

fn main() {
    render::setup_ncurses();

    let bounds = render::get_max_yx(n::stdscr());

    let window_store = render::WindowStore {
        editor: render::create_window(bounds.0 - 2, bounds.1 - 1, 0, 0),
        status: render::create_window(1, bounds.1 - 1, bounds.0 - 1, 0),
    };

    let stats = main_loop(&mut Editor::new(window_store.editor), window_store);

    n::getch();
    n::endwin();

    // 5 ms
    println!("average latency: {:.2}", average(&stats.key_render_times));
}
