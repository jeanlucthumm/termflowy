use ncurses as n;

use crate::editor::CursorState;
use crate::render;



pub fn render_status(win: n::WINDOW, cursor: CursorState, msg: &str) {
    let bounds = render::get_max_yx(win);
    n::mvwaddstr(win, 0, 0, &" ".repeat(bounds.1 as usize));
    n::mvwaddstr(win, 0, 0, match cursor {
        CursorState::Command(_) => "COMMAND",
        CursorState::Insert(_, _) => "INSERT",
    });
    render::addstr_right_aligned(win, msg);
}