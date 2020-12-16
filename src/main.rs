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
    loop_times: Vec<Duration>
}

pub struct PanelUpdate {
    pub should_render: bool,
    pub should_quit: bool,
    pub status_msg: String,
}

fn average(times: &Vec<Duration>) -> f32 {
    times.iter().map(|d| d.as_millis()).sum::<u128>() as f32 / times.len() as f32
}

fn main_loop(e: &mut Editor, w: render::WindowStore) -> RenderStats {
    let mut stats = RenderStats {
        key_render_times: vec![],
        loop_times: vec![],
    };
    n::wrefresh(w.editor);
    render_status(w.status, e.cursor(), "");
    n::wrefresh(w.status);
    loop {
        let key = n::getch();
        let loop_now = Instant::now();
        let key = n::keyname(key).unwrap();
        if key == "^[" {
            break;
        }

        let now = Instant::now();
        let e_update = e.update(&key);
        stats.key_render_times.push(now.elapsed());
        if e_update.should_quit {
            break;
        }
        let cursor = e.cursor();

        render_status(w.status, cursor, &e_update.status_msg);

        n::wrefresh(w.status);
        n::wmove(w.editor, cursor.pos().0, cursor.pos().1);
        n::wrefresh(w.editor);
        stats.loop_times.push(loop_now.elapsed());
    }
    stats
}

fn main() {
    render::setup_ncurses();

    let bounds = render::get_max_yx(n::stdscr());

    let window_store = render::WindowStore {
        editor: render::create_window(bounds.0 - 2, bounds.1, 0, 0),
        status: render::create_window(1, bounds.1, bounds.0 - 1, 0),
    };

    let stats = main_loop(&mut Editor::new(window_store.editor), window_store);
    n::endwin();

    // 5 ms
    println!("average editor latency: {:.2}", average(&stats.key_render_times));
    println!("average loop latency: {:.2}", average(&stats.loop_times));
}
