use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use ncurses as n;

use crate::raster::PixelState;
use crate::raster::Raster;
use crate::raster::{is_in_bounds, linear_move};
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
    fn getch(&self) -> String;
}

pub struct NCurses(pub n::WINDOW);

impl NCurses {
    pub fn new(win: n::WINDOW) -> NCurses {
        n::keypad(win, true);
        NCurses(win)
    }
}

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

    fn getch(&self) -> String {
        n::keyname(n::wgetch(self.0)).expect("wgetch returned unexpected value for keyname")
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
    active_id: i32,
    insert_offset: usize,
) -> (Raster, (i32, i32)) {
    win.move_cursor((0, 0));
    let mut cursor_pos: Option<(i32, i32)> = None;
    let mut raster = Raster::new(win.get_max_yx());
    for child in node.children_iter() {
        let subtree_pos = subtree_render(win, child, 0, insert_offset, active_id, &mut raster);
        cursor_pos = cursor_pos.or(subtree_pos);
    }
    raster.push_multiple(PixelState::Empty, clear_remaining(win) as u32);
    (raster, cursor_pos.expect("could not find active node during tree_render"))
}

pub fn subtree_render(
    win: &mut dyn Window,
    node: tree::NodeIterator,
    indentation_lvl: usize,
    insert_offset: usize,
    active_id: i32,
    raster: &mut Raster,
) -> Option<(i32, i32)> {
    let is_active = node.id() == active_id;
    let mut cursor_pos = render_bullet(
        win,
        &node.content(),
        indentation_lvl,
        node.id(),
        match is_active {
            true => Some(insert_offset),
            false => None,
        },
        raster,
    );
    raster.push_multiple(PixelState::Empty, clear_remaining_line(win) as u32);

    for child in node.children_iter() {
        let subtree_pos = subtree_render(win, child, indentation_lvl + 1, insert_offset, active_id, raster);
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
    if slices.is_empty() {
        win.addch(' ');
        raster.push(PixelState::Placeholder(node_id));
        return;
    }
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
    if slices.is_empty() {
        let active_pos = win.get_yx();
        win.addch(' ');
        raster.push(PixelState::Placeholder(node_id));
        return active_pos;
    }
    let mut insert_cursor = None;
    let mut offset = 0;
    for slice in slices {
        // If the insertion index is in the current slice, we have to record the cursor position
        if offset <= insert_index && insert_index < offset + slice.len() {
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
    // Allows an index == len of content which means we are inserting at the end of content
    if insert_index == offset {
        win.get_yx()
    } else {
        insert_cursor.expect("could not find cursor position in active node")
    }
}

fn split_every_n(string: &str, n: usize) -> Vec<&str> {
    if string.is_empty() {
        return Vec::new();
    }
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

pub struct TestWindow {
    pub max: Point,
    pub pos: Point,
    pub screen: Vec<Vec<char>>,
    pub print_on_refresh: bool,
}

impl TestWindow {
    // TODO figure out how to use unsafe code so we don't have to use |print_on_refresh|
    pub fn new(max: Point, print_on_refresh: bool) -> TestWindow {
        TestWindow {
            max,
            pos: (0, 0),
            screen: vec![vec![' '; max.1 as usize]; max.0 as usize],
            print_on_refresh,
        }
    }

    pub fn print(&self) {
        println!("{}", self);
    }

    fn is_cursor_at_end(&self) -> bool {
        self.pos.0 == self.max.0 - 1 && self.pos.1 == self.max.1 - 1
    }
}

impl Display for TestWindow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        let horizontal = "─".repeat(self.screen[0].len());
        buffer.push('┌');
        buffer.push_str(&horizontal);
        buffer.push_str("┐\n");
        for row in &self.screen {
            buffer.push('│');
            for c in row {
                buffer.push(*c);
            }
            buffer.push_str("│\n");
        }
        buffer.push('└');
        buffer.push_str(&horizontal);
        buffer.push_str("┘\n");
        write!(
            f,
            "TestWindow max: {:?} pos: {:?}\n{}",
            self.max, self.pos, buffer,
        )
    }
}

impl Debug for TestWindow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // write!(f, "{}", &self.to_string())
        write!(f, "{}", self)
    }
}

impl Window for TestWindow {
    fn get_max_yx(&self) -> (i32, i32) {
        self.max
    }

    fn get_yx(&self) -> (i32, i32) {
        self.pos
    }

    fn move_cursor(&mut self, pos: (i32, i32)) {
        self.pos = pos;
    }

    fn addstr(&mut self, s: &str) {
        for c in s.chars() {
            self.addch(c);
        }
    }

    fn addch(&mut self, c: char) {
        self.screen[self.pos.0 as usize][self.pos.1 as usize] = c;
        if !self.is_cursor_at_end() {
            self.pos = linear_move(self.pos, self.max, 1)
                .unwrap_or_else(|| panic!("For character: {}\n{}", c, &self));
        }
    }

    fn move_addstr(&mut self, pos: (i32, i32), s: &str) {
        if !is_in_bounds(pos, self.max) {
            panic!("For pos: {:?}\n{}", pos, &self);
        }
        self.pos = pos;
        self.addstr(s);
    }

    fn refresh(&self) {
        if self.print_on_refresh {
            self.print();
        }
    }

    fn getch(&self) -> String {
        panic!("test window has no function getch since it does not receive input")
    }
}

impl PartialEq for TestWindow {
    fn eq(&self, other: &Self) -> bool {
        if self.max != other.max || self.pos != other.pos {
            return false;
        }
        for i in 0..self.max.0 {
            for j in 0..self.max.1 {
                let i = i as usize;
                let j = j as usize;
                if self.screen[i][j] != other.screen[i][j] {
                    return false;
                }
            }
        }
        true
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
    use std::cell::Cell;

    use super::*;

    struct TestIdGen {
        counter: Cell<i32>,
    }

    impl TestIdGen {
        fn new() -> TestIdGen {
            TestIdGen { counter: Cell::new(1) }
        }
    }

    impl tree::IdGenerator for TestIdGen {
        fn gen(&self) -> i32 {
            (self.counter.get(), self.counter.set(self.counter.get() + 1)).0
        }
    }

    fn make_windows(max: Point) -> (TestWindow, TestWindow, Raster) {
        (
            TestWindow::new(max, false),
            TestWindow::new(max, false),
            Raster::new(max),
        )
    }

    #[test]
    fn add_indentation_test() {
        assert_eq!(split_every_n("12345", 3), ["123", "45"]);
        assert_eq!(split_every_n("123456", 2), ["12", "34", "56"]);
        assert_eq!(split_every_n("123456", 10), ["123456"]);
    }

    #[test]
    fn split_every_n_empty() {
        let empty: Vec<&str> = Vec::new();
        assert_eq!(split_every_n("", 2), empty);
        assert_eq!(split_every_n("", 0), empty);
    }

    #[test]
    fn render_content_slices_works() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("hello");
        render_content_slices(&mut win, vec!["hello"], 10, "  ", 0, &mut raster);
        assert_eq!(win, exp);

        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("  ");
        exp.addstr("12345678  9123");
        win.addstr("  ");
        render_content_slices(&mut win, vec!["12345678", "9123"], 8, "  ", 0, &mut raster);
        assert_eq!(win, exp);

        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("  ");
        exp.addstr("12345678  ");
        win.addstr("  ");
        render_content_slices(&mut win, vec!["12345678"], 8, "  ", 0, &mut raster);
        assert_eq!(win, exp);
    }

    #[test]
    fn zero_index_simple_render_active() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("hello");
        assert_eq!(
            render_content_slices_active(&mut win, vec!["hello"], 10, "  ", 0, 0, &mut raster),
            (0, 0)
        );
        assert_eq!(win, exp);
    }

    #[test]
    fn edge_index_simple_render_active() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("hello");
        // |insert_index| equal to len is allowed because during normal insertion, cursor is one
        // past the length of the string
        assert_eq!(
            render_content_slices_active(&mut win, vec!["hello"], 10, "  ", 0, 5, &mut raster),
            (0, 5)
        );
        assert_eq!(win, exp);
    }

    #[test]
    fn nonzero_index_simple_render_active() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("hello");
        assert_eq!(
            render_content_slices_active(&mut win, vec!["hello"], 10, "  ", 0, 2, &mut raster),
            (0, 2)
        );
        assert_eq!(win, exp);
    }

    #[test]
    fn zero_index_multiple_lines_render_active() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("  12345678  1234");
        win.addstr("  ");
        assert_eq!(
            render_content_slices_active(
                &mut win,
                vec!["12345678", "1234"],
                8,
                "  ",
                0,
                0,
                &mut raster
            ),
            (0, 2)
        );
        assert_eq!(win, exp);
    }

    #[test]
    fn edge_index_multiple_lines_render_active() {
        let (mut exp, mut win, mut raster) = make_windows((10, 10));
        exp.addstr("  12345678  1234");
        win.addstr("  ");
        assert_eq!(
            render_content_slices_active(
                &mut win,
                vec!["12345678", "1234"],
                8,
                "  ",
                0,
                12,
                &mut raster
            ),
            (1, 6)
        );
        assert_eq!(win, exp);
    }

    #[test]
    fn render_empty_tree() {
        let (mut exp, mut win, _raster) = make_windows((10, 10));
        exp.addch(CHAR_BULLET);
        clear_remaining(&mut exp);
        let tree = tree::Tree::new(Box::new(TestIdGen::new()));
        tree_render(&mut win, tree.root_iter(), tree.get_active_id(), 0);
        assert_eq!(win, exp);
    }

    #[test]
    fn clear_remaining_line_test() {
        let mut win = TestWindow::new((10, 10), false);
        win.addstr("xx");
        clear_remaining_line(&mut win);
        assert_eq!(win.pos, (1, 0));
    }

    #[test]
    fn clear_remaining_test() {
        let mut win = TestWindow::new((10, 10), false);
        win.addstr("xx");
        clear_remaining(&mut win);
        assert_eq!(win.pos, (9, 9));
    }
}
