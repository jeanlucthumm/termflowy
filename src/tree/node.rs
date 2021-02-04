use super::Dir::{self, *};
use std::{cell::RefCell, rc::Rc};

pub type Link = Rc<RefCell<Node>>;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: i32,
    pub parent: Option<Link>,
    pub children: Vec<Link>,
    pub content: String,
}

impl Node {
    pub fn new(id: i32, parent: Option<Link>) -> Node {
        Node {
            id,
            parent,
            children: vec![],
            content: String::new(),
        }
    }

    pub fn new_link(id: i32, parent: Option<Link>) -> Link {
        Link::new(RefCell::new(Self::new(id, parent)))
    }

    pub fn new_link_from_other(link: &Link) -> Link {
        Link::new(RefCell::new(link.borrow().clone()))
    }

    /// Inserts a `child` above or below an existing child with an id of `relative_id` (if it exists).
    pub fn insert_child_relative(
        &mut self,
        relative_id: i32,
        dir: Dir,
        child: Link,
    ) -> Result<(), ()> {
        let index = match (
            self.children
                .iter()
                .position(|l| l.borrow().id == relative_id),
            dir,
        ) {
            (Some(index), Below) => index + 1,
            (Some(index), Above) => index,
            (None, _) => return Err(()),
        };
        self.children.insert(index, child);
        Ok(())
    }

    /// Inserts a child node but does not update the parent field of the child
    pub fn insert_child_last(&mut self, child: Link) {
        self.children.push(child);
    }

    pub fn insert_child_first(&mut self, child: Link) {
        self.children.insert(0, child);
    }

    /// Removes the child with the given id. Will borrow every child Link.
    pub fn remove_child(&mut self, child_id: i32) {
        self.children.retain(|l| l.borrow().id != child_id);
    }

    /// Gets the sibling above or below the current node. This will borrow the parent to access
    /// its children and will borrow a Link to itself. Siblings are nodes on the same layer as
    /// the current node.
    pub fn get_sibling(&self, dir: Dir) -> Option<Link> {
        let parent = match self.parent {
            Some(ref parent) => parent.borrow(),
            None => return None,
        };
        if let Some(index) = parent
            .children
            .iter()
            .position(|l| l.borrow().id == self.id)
        {
            let index = match dir {
                Below => index + 1,
                Above => match index.checked_sub(1) {
                    Some(index) => index,
                    None => return None,
                },
            };
            parent.children.get(index).cloned()
        } else {
            None
        }
    }

    pub fn is_root(&self) -> bool {
        self.id == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_node_setup() -> Link {
        let node = Node::new_link(0, None);
        let first = Node::new_link(1, Some(node.clone()));
        node.borrow_mut().insert_child_last(first.clone());
        node
    }

    fn get_id(link: &Link) -> i32 {
        link.borrow().id
    }

    fn get_children_ids(link: &Link) -> Vec<i32> {
        link.borrow().children.iter().map(get_id).collect()
    }

    #[test]
    fn get_sibling_test() {
        let node = Node::new_link(0, None);
        assert!(node.borrow().get_sibling(Above).is_none());
        assert!(node.borrow().get_sibling(Below).is_none());

        let first = Node::new_link(1, Some(node.clone()));
        node.borrow_mut().insert_child_last(first.clone());

        let second = Node::new_link(2, Some(node.clone()));
        node.borrow_mut().insert_child_last(second.clone());

        assert_eq!(
            first.borrow().get_sibling(Below).map(|s| s.borrow().id),
            Some(2)
        );
        assert_eq!(
            second.borrow().get_sibling(Above).map(|s| s.borrow().id),
            Some(1)
        );
    }

    #[test]
    fn insert_child_relative_test() {
        let node = two_node_setup();
        let child = Node::new_link(2, Some(node.clone()));
        node.borrow_mut().insert_child_relative(1, Below, child).unwrap();
        assert_eq!(get_children_ids(&node), [1, 2]);

        let child = Node::new_link(3, Some(node.clone()));
        node.borrow_mut().insert_child_relative(2, Above, child).unwrap();
        assert_eq!(get_children_ids(&node), [1, 3, 2]);

        let child = Node::new_link(4, Some(node.clone()));
        assert!(node.borrow_mut().insert_child_relative(123123123, Above, child).is_err()); 
    }

    #[test]
    fn remove_child_test() {
        let node = two_node_setup();
        node.borrow_mut().remove_child(1);
        assert_eq!(get_children_ids(&node), []);
    }
}
