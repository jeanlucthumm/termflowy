use ncurses as n;

pub fn setup_ncurses() {
    // Allows for wide characters
    n::setlocale(n::LcCategory::ctype, "");
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

pub fn pprint<T: std::fmt::Display>(msg: T) {
    n::addstr(&format!("{}\n", msg));
}

pub fn clear_remaining(win: n::WINDOW) {
    let (mut screen_y, mut screen_x, mut y, mut x): (i32, i32, i32, i32) = (0, 0, 0, 0);
    n::getmaxyx(win, &mut screen_y, &mut screen_x);
    n::getyx(win, &mut y, &mut x);

    let remaining = (screen_x - x) + (screen_y - y) * (screen_x);
    for _ in 0..remaining {
        n::addch(' ' as u32);
    }
}
