use ncurses as n;

use crate::editor::CursorState;
use crate::render;

pub fn render_status(win: n::WINDOW, cursor: CursorState) {
    n::mvwaddstr(win, 0, 0, match cursor {
        CursorState::Command(_) => "COMMAND",
        CursorState::Insert(_) => "INSERT",
    });
    render::addstr_right_aligned(win, "testing");
}