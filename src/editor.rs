use ncurses as n;

const CHAR_BULLET: char = '•';
const CHAR_TRIANGLE_DOWN: char = '▼';
const CHAR_TRIANGLE_RIGHT: char = '▸';

const KEY_BACKSPACE: i32 = 127;
const KEY_ENTER: i32 = 10;

// TODO First goal
// - Can edit text as expected
// - Bullets new on every enter
// - Indentation levels with tab and s-tab

pub struct Editor {
    active_bullet: usize,
    bullets: Vec<Bullet>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            active_bullet: 0,
            bullets: vec![Bullet::new()],
        }
    }

    pub fn on_key_press(&mut self, key: i32) -> bool {
        if key == ctrl('c') {
            return false;
        }
        match key {
            KEY_ENTER => {
                self.bullets.push(Bullet::new());
                self.active_bullet += 1;
            }
            KEY_BACKSPACE => self.get_active_bullet().remove_char(),
            _ => self.get_active_bullet().add_char(key as u8 as char),
        };
        self.render();
        true
    }

    fn render(&self) {
        n::wmove(n::stdscr(), 0, 0);
        for bullet in &self.bullets {
            bullet.print();
        }
    }

    fn get_active_bullet(&mut self) -> &mut Bullet {
        &mut self.bullets[self.active_bullet]
    }
}

pub struct Bullet {
    content: String,
    children: Vec<Box<Bullet>>,
}

impl Bullet {
    fn new() -> Bullet {
        Bullet {
            content: String::new(),
            children: Vec::new(),
        }
    }

    fn add_char(&mut self, c: char) {
        self.content.push(c);
    }

    fn remove_char(&mut self) {
        self.content.pop();
    }

    fn print(&self) {
        n::addstr(&format!(" {} {}\n", &CHAR_BULLET, &self.content));
    }
}

pub fn ctrl(c: char) -> i32 {
    (c as i32) & 0x1f
}
