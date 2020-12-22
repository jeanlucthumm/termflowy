use crate::editor::Cursor;
use crate::render;
use crate::render::Window;

pub fn render_status(win: &mut dyn Window, cursor: Cursor, msg: &str) {
    let bounds = win.get_max_yx();
    win.move_addstr((0, 0), &" ".repeat(bounds.1 as usize));
    win.move_addstr(
        (0, 0),
        match cursor {
            Cursor::Command(_) => "COMMAND",
            Cursor::Insert(_) => "INSERT",
        },
    );
    render::addstr_right_aligned(&mut *win, msg);
    win.refresh();
}
