#![allow(dead_code)]
#![allow(clippy::eval_order_dependence)]

use crate::render::{NCurses, Window};
use crate::status::render_status;
use editor::Editor;
use ncurses as n;
use std::time::{Duration, Instant};

mod editor;
mod raster;
mod render;
mod status;
mod tree;
mod handlers;

struct RenderStats {
    key_render_times: Vec<Duration>,
    loop_times: Vec<Duration>,
}

pub struct PanelUpdate {
    pub should_render: bool,
    pub should_quit: bool,
    pub status_msg: String,
}

fn average(times: &[Duration]) -> f32 {
    times.iter().map(|d| d.as_millis()).sum::<u128>() as f32 / times.len() as f32
}

fn main_loop(e: &mut Editor, mut status: Box<dyn Window>) -> RenderStats {
    let mut stats = RenderStats {
        key_render_times: vec![],
        loop_times: vec![],
    };
    render_status(&mut *status, e.cursor(), "");
    // TODO this is shitty hack, find the true reason the cursor is fucked
    n::mv(e.cursor().pos().0, e.cursor().pos().1);
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

        render_status(&mut *status, cursor, &e_update.status_msg);
        e.focus();
        stats.loop_times.push(loop_now.elapsed());
    }
    stats
}

fn main() {
    render::setup_ncurses();

    let bounds = render::get_screen_bounds();

    let window_store = render::WindowStore {
        editor: Box::new(NCurses(render::create_window(bounds.0 - 2, bounds.1, 0, 0))),
        status: Box::new(NCurses(render::create_window(1, bounds.1, bounds.0 - 1, 0))),
    };

    let stats = main_loop(&mut Editor::new(window_store.editor), window_store.status);
    n::endwin();

    // 5 ms
    println!(
        "average editor latency: {:.2}",
        average(&stats.key_render_times)
    );
    println!("average loop latency: {:.2}", average(&stats.loop_times));
}
