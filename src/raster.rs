use std::collections::HashMap;

pub struct Raster {
    pub map: HashMap<(i32, i32), PixelState>,
    max: (i32, i32),
    current: (i32, i32),
}

impl Raster {
    pub fn new(max: (i32, i32)) -> Raster {
        Raster {
            map: HashMap::new(),
            max,
            current: (0, 0),
        }
    }

    pub fn add(&mut self, state: PixelState) {
        if self.current.0 > self.max.0 {
            panic!("cannot add to full raster")
        }
        self.map.insert(self.current, state);
        self.current.1 = if self.current.1 < self.max.1 {
            self.current.1 + 1
        } else {
            self.current.0 += 1;
            0
        };
    }
}

#[derive(Debug, PartialEq)]
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
    use super::*;
    use super::PixelState::*;

    #[test]
    fn works() {
        let mut raster = Raster::new((1, 1));
        raster.add(Empty);
        raster.add(Filler(2));
        raster.add(Empty);
        raster.add(Bullet(2));

        assert_eq!(*raster.map.get(&(0, 0)).unwrap(), Empty);
        assert_eq!(*raster.map.get(&(0, 1)).unwrap(), Filler(2));
        assert_eq!(*raster.map.get(&(1, 0)).unwrap(), Empty);
        assert_eq!(*raster.map.get(&(1, 1)).unwrap(), Bullet(2));
    }
}