#![feature(iter_advance_by)]
#![allow(dead_code)]
#![allow(clippy::eval_order_dependence)]

use crate::{render::NCurses, status::render_status};
use editor::Editor;
use ncurses as n;
use std::{panic, time::{Duration, Instant}};

mod editor;
mod handlers;
mod raster;
mod render;
mod status;
mod tree;

struct RenderStats {
    key_render_times: Vec<Duration>,
    loop_times: Vec<Duration>,
}

pub struct PanelUpdate {
    pub should_quit: bool,
    pub status_msg: String,
}

fn average(times: &[Duration]) -> f32 {
    times.iter().map(|d| d.as_millis()).sum::<u128>() as f32 / times.len() as f32
}

fn main_loop(wins: &mut render::WindowStore, mut e: Editor) -> RenderStats {
    let mut stats = RenderStats {
        key_render_times: vec![],
        loop_times: vec![],
    };
    render_status(wins.status.as_mut(), e.cursor(), "");
    loop {
        let key = wins.editor.getch();
        let loop_now = Instant::now();
        if key == "^[" {
            break;
        }

        let now = Instant::now();
        let e_update = e.update(&key, wins.editor.as_mut());
        stats.key_render_times.push(now.elapsed());
        if e_update.should_quit {
            break;
        }
        let cursor = e.cursor();

        render_status(wins.status.as_mut(), cursor, &e_update.status_msg);
        stats.loop_times.push(loop_now.elapsed());
    }
    stats
}

fn main() {
    render::setup_ncurses();
    let default_hook = panic::take_hook(); 
    panic::set_hook(Box::new(move |info| {
        n::endwin();
        n::delscreen(n::stdscr());
        default_hook(info);
    }));

    let bounds = render::get_screen_bounds();

    let mut window_store = render::WindowStore {
        editor: Box::new(NCurses::new(render::create_window(bounds.0 - 2, bounds.1, 0, 0))),
        status: Box::new(NCurses::new(render::create_window(1, bounds.1, bounds.0 - 1, 0))),
    };
    let editor = Editor::new(window_store.editor.as_mut());
    let stats = main_loop(&mut window_store, editor);
    n::endwin();
    n::delscreen(n::stdscr());

    // 5 ms
    println!(
        "average editor latency: {:.2}",
        average(&stats.key_render_times)
    );
    println!("average loop latency: {:.2}", average(&stats.loop_times));
}
