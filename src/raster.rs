use crate::render::Point;

pub struct Raster {
    pub map: Vec<Vec<PixelState>>,
    max: (i32, i32),
    current: (i32, i32),
}

impl Raster {
    pub fn new(max: (i32, i32)) -> Raster {
        Raster {
            map: vec![Vec::with_capacity((max.1 + 1) as usize); (max.0 + 1) as usize],
            max,
            current: (0, 0),
        }
    }

    pub fn push(&mut self, state: PixelState) {
        if self.current.0 > self.max.0 {
            panic!("cannot add to full raster")
        }
        self.map[self.current.0 as usize].push(state);
        self.current.1 = if self.current.1 < self.max.1 {
            self.current.1 + 1
        } else {
            self.current.0 += 1;
            0
        };
    }

    pub fn push_multiple(&mut self, state: PixelState, count: u32) {
        for _ in 0..count {
            self.push(state);
        }
    }

    pub fn get(&self, pos: Point) -> Option<PixelState> {
        match self.map.get(pos.0 as usize) {
            Some(v) => match v.get(pos.1 as usize) {
                Some(state) => Some(*state),
                None => None,
            }
            None => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PixelState {
    Empty,
    Filler(i32),
    Text {
        id: i32,
        pos: usize, // position in content
    },
    Bullet(i32),
}

#[cfg(test)]
mod tests {
    use super::PixelState::*;
    use super::*;

    #[test]
    fn works() {
        let mut raster = Raster::new((1, 1));
        raster.push(Empty);
        raster.push(Filler(2));
        raster.push(Empty);
        raster.push(Bullet(2));

        assert_eq!(raster.get((0, 0)).unwrap(), Empty);
        assert_eq!(raster.get((0, 1)).unwrap(), Filler(2));
        assert_eq!(raster.get((1, 0)).unwrap(), Empty);
        assert_eq!(raster.get((1, 1)).unwrap(), Bullet(2));
    }
}
