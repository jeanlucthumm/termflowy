use std::rc::{Rc, Weak};
use std::cell::RefCell;

pub type BulletCell = RefCell<Bullet>;

#[derive(Debug)]
pub struct Bullet {
    id: i32,
    parent: Weak<BulletCell>,
    sibling: Weak<BulletCell>,
    pub children: Vec<Rc<BulletCell>>,
    pub content: Content,
}

#[derive(Debug)]
pub struct Content {
    pub data: String,
}

pub trait IdGenerator {
    fn gen(&mut self) -> i32;
}

impl Bullet {
    fn new(id: i32) -> Rc<BulletCell> {
        Self::new_with_parent(Weak::new(), id)
    }

    fn new_as_child_of(parent: &Rc<BulletCell>, id: i32) -> Rc<BulletCell> {
        let bullet = Self::new_with_parent(Rc::downgrade(parent), id);
        parent.borrow_mut().children.push(bullet.clone());
        bullet
    }

    fn new_with_parent(parent: Weak<BulletCell>, id: i32) -> Rc<BulletCell> {
        Rc::new(BulletCell::new(Bullet {
            id,
            parent,
            sibling: Weak::new(),
            children: vec![],
            content: Content {
                data: String::new(),
            },
        }))
    }

    fn remove_child(&mut self, id: i32) {
        self.children.retain(|x| x.borrow().id != id);
    }
}

pub fn new_tree(generator: &mut dyn IdGenerator) -> (Rc<BulletCell>, Rc<BulletCell>) {
    let root = Bullet::new(generator.gen());
    (root.clone(), Bullet::new_as_child_of(&root, generator.gen()))
}

pub fn create_sibling_of(active: &Rc<BulletCell>, generator: &mut dyn IdGenerator) -> Rc<BulletCell> {
    let bullet = match active.borrow().parent.upgrade() {
        Some(parent) => Bullet::new_as_child_of(&parent, generator.gen()),
        _ => Bullet::new(generator.gen()),
    };
    bullet.borrow_mut().sibling = Rc::downgrade(&active);
    bullet
}

pub fn indent(active: &Rc<BulletCell>) -> Result<(), &str> {
    if active.borrow().sibling.upgrade().is_some() {
        {
            let active_clone = active.clone();
            let active = active.borrow();
            if let Some(parent) = active.parent.upgrade() {
                parent.borrow_mut().remove_child(active.id);
            }
            let sibling = active.sibling.upgrade().unwrap();
            sibling.borrow_mut().children.push(active_clone);
        }
        {
            let mut active = active.borrow_mut();
            active.parent = active.sibling.clone();
            active.sibling = Weak::new();
        }
        Ok(())
    } else {
        Err("could not indent: node has no sibling")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestIdGen {
        current: i32,
    }

    impl TestIdGen {
        fn new() -> TestIdGen {
            TestIdGen {
                current: 0,
            }
        }
    }

    impl IdGenerator for TestIdGen {
        fn gen(&mut self) -> i32 {
            (self.current, self.current += 1).0
        }
    }

    #[test]
    fn siblings_test() {
        let mut gen = TestIdGen::new();
        let (_root, active) = new_tree(&mut gen);
        active.borrow_mut().content.data.push_str("first");
        let sibling = create_sibling_of(&active, &mut gen);
        assert_eq!(sibling.borrow().sibling.upgrade().unwrap().borrow().content.data,
                   "first");
    }

    #[test]
    fn indents_test() {
        let mut gen = TestIdGen::new();
        let (_root, active) = new_tree(&mut gen);
        active.borrow_mut().content.data = String::from("first");
        assert!(indent(&active).is_err());
        let second = create_sibling_of(&active, &mut gen);
        second.borrow_mut().content.data = String::from("second");
        assert!(indent(&second).is_ok());
        assert_eq!(second.borrow().parent.upgrade().unwrap()
                       .borrow().content.data, "first");
        assert_eq!(active.borrow().children.get(0).unwrap().borrow().content.data, "second");
    }

    #[test]
    fn bullet_remove_child_test() {
        let bullet = Bullet::new(0);
        let _ = Bullet::new_as_child_of(&bullet, 1);
        let _ = Bullet::new_as_child_of(&bullet, 2);
        bullet.borrow_mut().remove_child(1);
        assert_eq!(bullet.borrow().children.get(0).unwrap().borrow().id, 2);
    }
}