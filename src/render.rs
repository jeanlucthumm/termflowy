use ncurses as n;

use crate::raster::PixelState;
use crate::raster::Raster;
use crate::tree;

const CHAR_BULLET: char = '•';
const CHAR_TRIANGLE_DOWN: char = '▼';
const CHAR_TRIANGLE_RIGHT: char = '▸';
const INDENTATION: &str = "  ";

pub type Point = (i32, i32);

#[derive(Clone, Copy)]
pub struct WindowStore {
    pub editor: n::WINDOW,
    pub status: n::WINDOW,
}

pub fn create_window(h: i32, w: i32, y: i32, x: i32) -> n::WINDOW {
    n::newwin(h, w, y, x)
}

pub fn setup_ncurses() {
    // Allows for wide characters
    n::setlocale(n::LcCategory::all, "");
    n::initscr();
    // Captures signal sequences and no buffer
    n::raw();
    // F keys and arrows
    n::keypad(n::stdscr(), true);
    // Doesn't echo typed keys
    n::noecho();
}

pub fn get_max_yx(win: n::WINDOW) -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getmaxyx(win, &mut y, &mut x);
    (y, x)
}

pub fn get_yx(win: n::WINDOW) -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getyx(win, &mut y, &mut x);
    (y, x)
}

pub fn clear_remaining(win: n::WINDOW) -> usize {
    let size = get_max_yx(win);
    let pos = get_yx(win);

    let remaining = (size.1 - pos.1) + (size.0 - pos.0 - 1) * (size.1);
    if remaining.is_negative() {
        panic!("tried to clear a negative amount on line");
    }
    for _ in 0..remaining {
        n::waddch(win, ' ' as u32);
    }
    remaining as usize
}

pub fn clear_remaining_line(win: n::WINDOW) -> usize {
    let size = get_max_yx(win);
    let pos = get_yx(win);

    let remaining_line = size.1 - pos.1;
    if remaining_line.is_negative() {
        panic!("tried to clear a negative amount on line");
    }
    for _ in 0..remaining_line {
        n::waddch(win, ' ' as u32);
    }
    remaining_line as usize
}

pub fn addstr_right_aligned(win: n::WINDOW, txt: &str) {
    let bounds = get_max_yx(win);
    n::mvwaddstr(win, 0, bounds.1 - txt.len() as i32, txt);
}

pub fn tree_render(
    win: n::WINDOW,
    node: tree::NodeIterator,
    indentation_lvl: usize,
    insert_offset: usize,
) -> (Raster, Option<(i32, i32)>) {
    n::wmove(win, 0, 0);
    let mut cursor_pos: Option<(i32, i32)> = None;
    let mut raster = Raster::new(get_max_yx(win));
    for child in node.children_iter() {
        let subtree_pos = subtree_render(win, child, indentation_lvl, insert_offset, &mut raster);
        cursor_pos = cursor_pos.or(subtree_pos);
    }
    raster.push_multiple(PixelState::Empty, clear_remaining(win) as u32);
    (raster, cursor_pos)
}

pub fn subtree_render(
    win: n::WINDOW,
    node: tree::NodeIterator,
    indentation_lvl: usize,
    insert_offset: usize,
    raster: &mut Raster,
) -> Option<(i32, i32)> {
    let mut cursor_pos = render_bullet(
        win,
        node.content(),
        indentation_lvl,
        node.id(),
        if node.is_active() {
            Some(insert_offset)
        } else {
            None
        },
        raster,
    );
    raster.push_multiple(PixelState::Empty, clear_remaining_line(win) as u32);

    for child in node.children_iter() {
        let subtree_pos = subtree_render(win, child, indentation_lvl + 1, insert_offset, raster);
        cursor_pos = cursor_pos.or(subtree_pos);
    }
    cursor_pos
}

fn render_bullet(
    win: n::WINDOW,
    content: &str,
    indentation_lvl: usize,
    node_id: i32,
    insert_offset: Option<usize>,
    raster: &mut Raster,
) -> Option<(i32, i32)> {
    let mut indentation_str = INDENTATION.repeat(indentation_lvl as usize);
    n::waddstr(win, &format!("{}{} ", indentation_str, CHAR_BULLET));
    raster.push_multiple(PixelState::Empty, indentation_str.len() as u32);
    raster.push(PixelState::Bullet(node_id));
    raster.push(PixelState::Filler(node_id));

    indentation_str.push_str("  "); // for filler and bullet
    let limit = (get_max_yx(win).1 - indentation_str.len() as i32) as usize;
    if let Some(insert_offset) = insert_offset {
        let insert_index = content
            .len()
            .checked_sub(insert_offset)
            .expect("offset should not be larger than len, raster generation is probably wrong");
        Some(render_content_slices_active(
            win,
            split_every_n(content, limit),
            limit,
            &indentation_str,
            node_id,
            insert_index,
            raster,
        ))
    } else {
        render_content_slices(
            win,
            split_every_n(content, limit),
            limit,
            &indentation_str,
            node_id,
            raster,
        );
        None
    }
}

fn render_content_slices(
    win: n::WINDOW,
    slices: Vec<&str>,
    limit: usize,
    indentation_str: &str,
    node_id: i32,
    raster: &mut Raster,
) {
    let mut offset = 0;
    for slice in slices {
        n::waddstr(win, slice);
        for _ in 0..slice.len() {
            raster.push(PixelState::Text {
                id: node_id,
                offset,
            });
            offset += 1;
        }
        if slice.len() == limit {
            n::waddstr(win, &indentation_str);
            raster.push_multiple(PixelState::Filler(node_id), indentation_str.len() as u32);
        }
    }
}

fn render_content_slices_active(
    win: n::WINDOW,
    slices: Vec<&str>,
    limit: usize,
    indentation_str: &str,
    node_id: i32,
    insert_index: usize,
    raster: &mut Raster,
) -> (i32, i32) {
    let mut insert_cursor = None;
    let mut offset = 0;
    for slice in slices {
        // n::waddstr(win,&format!("offset: {}, len: {}, index: {}", offset, slice.len(), insert_index)); // DEBUG
        if offset + slice.len() >= insert_index {
            let before = &slice[0..insert_index - offset];
            n::waddstr(win, before);
            insert_cursor = Some(get_yx(win));
            n::waddstr(win, &slice[insert_index - offset..slice.len()]);
        } else {
            n::waddstr(win, slice);
        }
        for _ in 0..slice.len() {
            raster.push(PixelState::Text {
                id: node_id,
                offset,
            });
            offset += 1;
        }
        if slice.len() == limit {
            n::waddstr(win, &indentation_str);
            raster.push_multiple(PixelState::Filler(node_id), indentation_str.len() as u32);
        }
    }
    if insert_index == 0 {
        get_yx(win)
    } else {
        insert_cursor.expect("could not find cursor position in active node")
        // (0, 0)
    }
}

fn split_every_n(string: &str, n: usize) -> Vec<&str> {
    let mut start = 0;
    let mut end = n;
    let mut slices = vec![];
    while end < string.len() {
        slices.push(&string[start..end]);
        start = end;
        end += n;
    }
    slices.push(&string[start..string.len()]);
    slices
}

pub fn cursor_render(win: n::WINDOW, pos: (i32, i32)) {
    n::wmove(win, pos.0, pos.1);
}

pub fn check_bounds(win: n::WINDOW, mut pos: Point, offset: Point) -> Option<Point> {
    let max = get_max_yx(win);
    pos.0 += offset.0;
    pos.1 += offset.1;
    if pos.0 >= max.0 || pos.0 < 0 || pos.1 >= max.1 || pos.1 < 0 {
        None
    } else {
        Some(pos)
    }
}

pub mod debug {
    use super::*;

    pub fn pprint<T: std::fmt::Display>(win: n::WINDOW, msg: T) {
        n::waddstr(win, &format!("{} ", msg));
        n::wrefresh(win);
    }

    pub fn create_window(h: i32, w: i32, y: i32, x: i32) -> n::WINDOW {
        let win = n::newwin(h, w, y, x);
        n::box_(win, 0, 0);
        n::wrefresh(win);
        win
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_indentation_test() {
        assert_eq!(split_every_n("12345", 3), ["123", "45"]);
        assert_eq!(split_every_n("123456", 2), ["12", "34", "56"]);
        assert_eq!(split_every_n("123456", 10), ["123456"]);
    }
}
