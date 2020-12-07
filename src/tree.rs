use std::rc::{Rc, Weak};
use std::cell::RefCell;

type BulletCell = RefCell<Bullet>;

#[derive(Debug)]
pub struct Bullet {
    parent: Weak<BulletCell>,
    sibling: Weak<BulletCell>,
    children: Vec<Rc<BulletCell>>,
    pub content: Content,
}

#[derive(Debug)]
pub struct Content {
    pub data: String,
}

impl Bullet {
    pub fn new_tree() -> (Rc<BulletCell>, Rc<BulletCell>) {
        let root = Rc::new(BulletCell::new(Self::new()));
        (root.clone(), Self::new_as_child_of(&root))
    }

    fn new() -> Bullet {
        Self::new_with_parent(Weak::new())
    }

    fn new_as_child_of(parent: &Rc<BulletCell>) -> Rc<BulletCell> {
        let bullet = Rc::new(RefCell::new(Self::new_with_parent(Rc::downgrade(parent))));
        parent.borrow_mut().children.push(bullet.clone());
        bullet
    }

    fn new_with_parent(parent: Weak<BulletCell>) -> Bullet {
        Bullet {
            parent,
            sibling: Weak::new(),
            children: vec![],
            content: Content {
                data: String::new(),
            },
        }
    }
}

fn create_sibling_of(active: Rc<BulletCell>) -> Rc<BulletCell> {
    let bullet = match active.borrow().parent.upgrade() {
        Some(parent) => Bullet::new_as_child_of(&parent),
        _ => Rc::new(BulletCell::new(Bullet::new())),
    };
    bullet.borrow_mut().sibling = Rc::downgrade(&active);
    bullet
}

fn indent(active: &Rc<BulletCell>) -> Result<(), &str>{
    let mut active = active.borrow_mut();
    if active.sibling.upgrade().is_some() {
        active.parent = active.sibling.clone();
        active.sibling = Weak::new();
        Ok(())
    } else {
        Err("could not indent: node has no sibling")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siblings_test() {
        let (_root, active) = Bullet::new_tree();
        active.borrow_mut().content.data.push_str("first");
        let sibling = create_sibling_of(active);
        assert_eq!(sibling.borrow().sibling.upgrade().unwrap().borrow().content.data,
                   "first");
    }

    #[test]
    fn indents_test() {
        let (_root, active) = Bullet::new_tree();
        active.borrow_mut().content.data = String::from("first");
        assert!(indent(&active).is_err());
        let second = create_sibling_of(active);
        assert!(indent(&second).is_ok());
        assert_eq!(second.borrow().parent.upgrade().unwrap()
                       .borrow().content.data, "first");
    }
}