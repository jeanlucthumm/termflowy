use ncurses::*;
use std::char;

struct Bullet {
    content: String,
    children: Vec<Box<Bullet>>,
}

impl Bullet {
    fn new(content: String) -> Bullet {
        Bullet {
            content,
            children: Vec::new(),
        }
    }
}

fn main() {
    setlocale(LcCategory::ctype, "");
    initscr();
    raw();
    keypad(stdscr(), true);
    noecho();

    addstr("GFW 不喜欢 VPN 协议。\n");
    addstr("Type any character to see it in bold\n");
    let ch = getch();

    if ch == KEY_F(1) {
        addstr("F1 key pressed");
    } else {
        addstr("The pressed key is ");
        attron(A_BOLD());
        addstr(&format!(
            "{}\n",
            char::from_u32(ch as u32).expect("could not convert character")
        ));
        attroff(A_BOLD());
    }

    refresh();
    getch();
    endwin();
}
