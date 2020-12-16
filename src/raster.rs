use crate::render::Point;
use Direction::*;
use PixelState::*;

pub struct Raster {
    pub map: Vec<Vec<PixelState>>,
    max: (i32, i32),
    current: (i32, i32),
}

impl Raster {
    // max is not inclusive
    pub fn new(max: (i32, i32)) -> Raster {
        Raster {
            map: vec![Vec::with_capacity((max.1 + 1) as usize); (max.0 + 1) as usize],
            max,
            current: (0, -1),
        }
    }

    pub fn push(&mut self, state: PixelState) {
        self.current = linear_move(self.current, self.max, 1).expect("cannot add to full raster");
        self.map[self.current.0 as usize].push(state);
    }

    pub fn push_multiple(&mut self, state: PixelState, count: u32) {
        for _ in 0..count {
            self.push(state);
        }
    }

    // TODO remove option and add an Unknown state. Do not use safe gets with Vec
    pub fn get(&self, pos: Point) -> Option<PixelState> {
        match self.map.get(pos.0 as usize) {
            Some(v) => match v.get(pos.1 as usize) {
                Some(state) => Some(*state),
                None => None,
            },
            None => None,
        }
    }

    pub fn browser(&self, pos: Point) -> Result<Browser, String> {
        if is_in_bounds(pos, self.max) {
            Ok(Browser { raster: self, pos })
        } else {
            Err(format!(
                "cannot get browser for pixel {:?} which is out of bounds {:?}",
                pos, self.max
            ))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PixelState {
    Empty,
    Filler(i32),
    Text {
        id: i32,
        offset: usize, // position in content
    },
    Bullet(i32),
}

impl PixelState {
    fn is_text(self) -> bool {
        match self {
            Text { .. } => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Direction {
    Left,
    Right,
}

pub struct Browser<'a> {
    raster: &'a Raster,
    pos: Point,
}

impl<'a> Browser<'a> {
    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn go_while_count<F>(
        &mut self,
        mut count: u32,
        dir: Direction,
        predicate: F,
    ) -> Result<(), &str>
    where
        F: Fn(PixelState) -> bool,
    {
        let offset = match dir {
            Left => -1,
            Right => 1,
        };
        while count > 0 {
            if let Some(pos) = linear_move(self.pos, self.raster.max, offset) {
                if let Some(state) = self.raster.get(pos) {
                    if predicate(state) {
                        count -= 1;
                    }
                }
                self.pos = pos;
            } else {
                return Err("could not browse past bounds")
            }
        }
        Ok(())
    }
}

// TODO this is old code for push(), benchmark it to see if faster
fn old_push() {
    // if self.current.0 >= self.max.0 {
    //     panic!("cannot add to full raster")
    // }
    // self.map[self.current.0 as usize].push(state);
    // self.current.1 = if self.current.1 < self.max.1 - 1 {
    //     self.current.1 + 1
    // } else {
    //     self.current.0 += 1;
    //     0
    // };
}

pub fn linear_move(mut pos: Point, max: Point, offset: i32) -> Option<Point> {
    let x = pos.1 + offset;
    pos.0 += x.div_euclid(max.1);
    pos.1 = x.rem_euclid(max.1);
    if 0 <= pos.0 && pos.0 < max.0 {
        Some(pos)
    } else {
        None
    }
}

pub fn is_in_bounds(pos: Point, max: Point) -> bool {
    0 <= pos.0 && pos.0 < max.0 && 0 <= pos.1 && pos.1 < max.1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster_from_vec(map: Vec<Vec<PixelState>>) -> Raster {
        let mut raster = Raster::new((map.len() as i32, map[0].len() as i32));
        for row in &map {
            for pixel in row {
                raster.push(*pixel);
            }
        }
        raster
    }

    #[test]
    fn raster_test() {
        let raster = raster_from_vec(vec![
            vec![Empty, Filler(2), Empty], //
            vec![Empty, Bullet(2), Empty], //
        ]);

        assert_eq!(raster.get((0, 0)).unwrap(), Empty);
        assert_eq!(raster.get((0, 1)).unwrap(), Filler(2));
        assert_eq!(raster.get((0, 2)).unwrap(), Empty);
        assert_eq!(raster.get((1, 0)).unwrap(), Empty);
        assert_eq!(raster.get((1, 1)).unwrap(), Bullet(2));
        assert_eq!(raster.get((1, 2)).unwrap(), Empty);
    }

    #[test]
    fn linear_move_test() {
        assert_eq!(linear_move((1, 1), (10, 10), 1), Some((1, 2))); // + on line
        assert_eq!(linear_move((1, 1), (10, 10), -1), Some((1, 0))); // - on line
        assert_eq!(linear_move((1, 1), (10, 10), 11), Some((2, 2))); // + overflow
        assert_eq!(linear_move((1, 1), (10, 10), -11), Some((0, 0))); // - overflow
        assert_eq!(linear_move((1, 1), (10, 10), 33), Some((4, 4))); // + multiple
        assert_eq!(linear_move((5, 5), (10, 10), -33), Some((2, 2))); // - multiple
    }

    #[test]
    fn browser_go_while_count_continuous() {
        let sample_text = Text { id: 0, offset: 0 };
        let raster = raster_from_vec(vec![
            vec![Empty, Filler(2), Empty],         //
            vec![Empty, sample_text, sample_text], //
            vec![sample_text, sample_text, Empty], //
        ]);

        let mut browser = raster.browser((1, 1)).unwrap();
        browser
            .go_while_count(2, Direction::Right, |state| state.is_text())
            .unwrap();
        assert_eq!(browser.pos(), (2, 0));
        browser
            .go_while_count(2, Direction::Left, |state| state.is_text())
            .unwrap();
        assert_eq!(browser.pos(), (1, 1));
    }

    #[test]
    fn browser_go_while_count_interrupted() {
        let sample_text = Text { id: 0, offset: 0 };
        let raster = raster_from_vec(vec![
            vec![Bullet(2), Filler(2), sample_text, sample_text, sample_text], //
            vec![Empty, Bullet(3), Filler(3), sample_text, sample_text],       //
            vec![Empty, Empty, Empty, sample_text, sample_text],               //
        ]);

        let mut browser = raster.browser((1, 3)).unwrap();
        browser
            .go_while_count(4, Direction::Right, |state| state.is_text())
            .unwrap();
        assert_eq!(browser.pos(), (2, 4));
        browser
            .go_while_count(2, Direction::Left, |state| state.is_text())
            .unwrap();
        assert_eq!(browser.pos(), (1, 3));

        // Browser won't reset position if bounds was hit
        assert!(browser
            .go_while_count(100, Direction::Right, |state| state.is_text())
            .is_err());
        assert_eq!(browser.pos(), (2, 4));
        assert!(browser
            .go_while_count(100, Direction::Left, |state| state.is_text())
            .is_err());
        assert_eq!(browser.pos(), (0, 0));
    }

    #[test]
    fn browser_bounds() {
        let raster = raster_from_vec(vec![
            vec![Empty, Empty], //
            vec![Empty, Empty], //
        ]);

        assert!(raster.browser((100, 100)).is_err());
    }
}
