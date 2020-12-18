use ncurses as n;

use crate::raster::PixelState;
use crate::raster::Raster;
use crate::tree;

const CHAR_BULLET: char = '•';
const CHAR_TRIANGLE_DOWN: char = '▼';
const CHAR_TRIANGLE_RIGHT: char = '▸';
const INDENTATION: &str = "  ";

pub type Point = (i32, i32);

pub struct WindowStore {
    pub editor: Box<dyn Window>,
    pub status: Box<dyn Window>,
}

pub trait Window {
    fn get_max_yx(&self) -> Point;
    fn get_yx(&self) -> Point;
    fn move_cursor(&mut self, pos: Point);
    fn addstr(&mut self, s: &str);
    fn addch(&mut self, c: char);
    fn move_addstr(&mut self, pos: Point, s: &str);
    fn refresh(&self);
}

pub struct NCurses(pub n::WINDOW);

impl Window for NCurses {
    fn get_max_yx(&self) -> (i32, i32) {
        let mut y: i32 = 0;
        let mut x: i32 = 0;
        n::getmaxyx(self.0, &mut y, &mut x);
        (y, x)
    }

    fn get_yx(&self) -> (i32, i32) {
        let mut y: i32 = 0;
        let mut x: i32 = 0;
        n::getyx(self.0, &mut y, &mut x);
        (y, x)
    }

    fn move_cursor(&mut self, pos: (i32, i32)) {
        n::wmove(self.0, pos.0, pos.1);
    }

    fn addstr(&mut self, s: &str) {
        n::waddstr(self.0, s);
    }

    fn addch(&mut self, c: char) {
        n::waddch(self.0, c as u32);
    }

    fn move_addstr(&mut self, pos: (i32, i32), s: &str) {
        n::mvwaddstr(self.0, pos.0, pos.1, s);
    }

    fn refresh(&self) {
        n::wrefresh(self.0);
    }
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

pub fn get_screen_bounds() -> (i32, i32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    n::getmaxyx(n::stdscr(), &mut y, &mut x);
    (y, x)
}

pub fn create_window(h: i32, w: i32, y: i32, x: i32) -> n::WINDOW {
    n::newwin(h, w, y, x)
}

pub fn clear_remaining(win: &mut dyn Window) -> usize {
    let size = win.get_max_yx();
    let pos = win.get_yx();

    let remaining = (size.1 - pos.1) + (size.0 - pos.0 - 1) * (size.1);
    if remaining.is_negative() {
        panic!("tried to clear a negative amount on line");
    }
    for _ in 0..remaining {
        win.addch(' ');
    }
    remaining as usize
}

pub fn clear_remaining_line(win: &mut dyn Window) -> usize {
    let size = win.get_max_yx();
    let pos = win.get_yx();

    let remaining_line = size.1 - pos.1;
    if remaining_line.is_negative() {
        panic!("tried to clear a negative amount on line");
    }
    for _ in 0..remaining_line {
        win.addch(' ');
    }
    remaining_line as usize
}

pub fn addstr_right_aligned(win: &mut dyn Window, txt: &str) {
    let bounds = win.get_max_yx();
    win.move_addstr((0, bounds.1 - txt.len() as i32), txt);
}

pub fn tree_render(
    win: &mut dyn Window,
    node: tree::NodeIterator,
    indentation_lvl: usize,
    insert_offset: usize,
) -> (Raster, Option<(i32, i32)>) {
    win.move_cursor((0, 0));
    let mut cursor_pos: Option<(i32, i32)> = None;
    let mut raster = Raster::new(win.get_max_yx());
    for child in node.children_iter() {
        let subtree_pos = subtree_render(win, child, indentation_lvl, insert_offset, &mut raster);
        cursor_pos = cursor_pos.or(subtree_pos);
    }
    raster.push_multiple(PixelState::Empty, clear_remaining(win) as u32);
    (raster, cursor_pos)
}

pub fn subtree_render(
    win: &mut dyn Window,
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
    win: &mut dyn Window,
    content: &str,
    indentation_lvl: usize,
    node_id: i32,
    insert_offset: Option<usize>,
    raster: &mut Raster,
) -> Option<(i32, i32)> {
    let mut indentation_str = INDENTATION.repeat(indentation_lvl as usize);
    win.addstr(&format!("{}{} ", indentation_str, CHAR_BULLET));
    raster.push_multiple(PixelState::Empty, indentation_str.len() as u32);
    raster.push(PixelState::Bullet(node_id));
    raster.push(PixelState::Filler(node_id));

    indentation_str.push_str("  "); // for filler and bullet
    let limit = (win.get_max_yx().1 - indentation_str.len() as i32) as usize;
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
    win: &mut dyn Window,
    slices: Vec<&str>,
    limit: usize,
    indentation_str: &str,
    node_id: i32,
    raster: &mut Raster,
) {
    let mut offset = 0;
    for slice in slices {
        win.addstr(slice);
        for _ in 0..slice.len() {
            raster.push(PixelState::Text {
                id: node_id,
                offset,
            });
            offset += 1;
        }
        if slice.len() == limit {
            win.addstr(&indentation_str);
            raster.push_multiple(PixelState::Filler(node_id), indentation_str.len() as u32);
        }
    }
}

fn render_content_slices_active(
    win: &mut dyn Window,
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
            win.addstr(before);
            insert_cursor = Some(win.get_yx());
            win.addstr(&slice[insert_index - offset..slice.len()]);
        } else {
            win.addstr(slice);
        }
        for _ in 0..slice.len() {
            raster.push(PixelState::Text {
                id: node_id,
                offset,
            });
            offset += 1;
        }
        if slice.len() == limit {
            win.addstr(&indentation_str);
            raster.push_multiple(PixelState::Filler(node_id), indentation_str.len() as u32);
        }
    }
    if insert_index == 0 {
        win.get_yx()
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
