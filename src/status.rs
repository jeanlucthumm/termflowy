use crate::editor::CursorState;
use crate::render;
use crate::render::Window;

pub fn render_status(win: &mut dyn Window, cursor: CursorState, msg: &str) {
    let bounds = win.get_max_yx();
    win.move_addstr((0, 0), &" ".repeat(bounds.1 as usize));
    win.move_addstr(
        (0, 0),
        match cursor {
            CursorState::Command(_, _) => "COMMAND",
            CursorState::Insert(_, _) => "INSERT",
        },
    );
    render::addstr_right_aligned(&mut *win, msg);
    win.refresh();
}
