use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, VecDeque},
    ops::{Deref, DerefMut},
    rc::Rc,
};

use Dir::*;

type Link = Rc<RefCell<Node>>;

pub trait IdGenerator {
    fn gen(&self) -> i32;
}

pub enum Dir {
    Above,
    Below,
}

/// Invariants:
/// - There is an active node
/// - The active node is never the root node
/// - There is at least one root node and one child of the root node
/// - No two nodes have the same id
/// - All nodes but root nodes have a parent
pub struct Tree {
    active: Link,
    root: Link,
    generator: Box<dyn IdGenerator>,
    id_table: HashMap<i32, Link>,
}

impl Tree {
    pub fn new(generator: Box<dyn IdGenerator>) -> Tree {
        let mut id_table = HashMap::new();

        let root = Node::new_link(0, None);
        id_table.insert(root.borrow().id, root.clone());

        let first = Node::new_link(generator.gen(), Some(root.clone()));
        id_table.insert(first.borrow().id, first.clone());
        root.borrow_mut().children.push(first.clone());

        Tree {
            active: first,
            root,
            generator,
            id_table,
        }
    }

    pub fn create_sibling_above(&mut self) {
        let node = Node::new_link(self.generator.gen(), None);
        self.insert_node(node, Above);
    }

    pub fn create_sibling(&mut self) {
        let node = Node::new_link(self.generator.gen(), None);
        self.insert_node(node.clone(), Below);
        self.active = node;
    }

    pub fn insert_subtree(&mut self, subtree: Subtree, dir: Dir) {
        let subtree = subtree.make_unique(self.generator.as_ref());
        let root_id = subtree.root.borrow().id;
        self.insert_node(subtree.root, dir);
        self.activate(root_id)
            .expect("could not find subtree root right after insertion");
    }

    fn insert_node(&mut self, node: Link, dir: Dir) {
        let parent = self.active.borrow().parent.clone().unwrap();
        node.borrow_mut().parent = Some(parent.clone());
        parent
            .borrow_mut()
            .insert_child_relative(self.active.borrow().id, dir, node.clone())
            .expect("child not found in its own parent");
        self.register_in_table(&node);
    }

    fn register_in_table(&mut self, node: &Link) {
        self.id_table.insert(node.borrow().id, node.clone());
    }

    /// Indents the active node under its up sibling. Returns errors if there is no such sibling.
    /// If `first` then the active node will be placed as the first child of the sibling, otherwise
    /// last.
    pub fn indent(&mut self, first: bool) -> Result<(), String> {
        let sibling = match self.active.borrow().get_sibling(Above) {
            Some(x) => x,
            None => return Err(String::from("already at max indentation level")),
        };
        // Remove from previous parent
        let parent = self.active.borrow().parent.clone().unwrap();
        parent.borrow_mut().remove_child(self.active.borrow().id);

        // Establish parent-child relationship with former sibling
        match first {
            true => sibling.borrow_mut().insert_child_first(self.active.clone()),
            false => sibling.borrow_mut().insert_child_last(self.active.clone()),
        }
        self.active.borrow_mut().parent = Some(sibling);
        Ok(())
    }

    pub fn unindent(&mut self) -> Result<(), String> {
        // Break parent-child relationship
        let parent = self.active.borrow().parent.clone().unwrap();
        if parent.borrow().is_root() {
            return Err(String::from("cannot unindent further"));
        }
        parent.borrow_mut().remove_child(self.active.borrow().id);

        // Reinsert in grandparent
        let grandparent = parent.borrow().parent.clone().unwrap();
        grandparent
            .borrow_mut()
            .insert_child_relative(parent.borrow().id, Below, self.active.clone())
            .expect("could not find parent in grandparent while unindenting");
        self.active.borrow_mut().parent = Some(grandparent);
        Ok(())
    }

    pub fn activate(&mut self, id: i32) -> Result<(), String> {
        self.active = self
            .get_node(id)
            .cloned()
            .ok_or("could not find id to activate".to_string())?;
        Ok(())
    }

    pub fn delete(&mut self) -> Result<(), String> {
        let active_link = self.active.clone();
        let active = active_link.borrow();
        let parent = active.parent.as_ref().unwrap();

        match (
            parent.borrow(),
            active.get_sibling(Above),
            active.get_sibling(Below),
        ) {
            (p, _, _) if p.is_root() && p.children.len() == 1 => {
                return Err(String::from("cannot delete last node"))
            }
            (p, None, None) if !p.is_root() => self.active = parent.clone(),
            (_, _, Some(below)) => self.active = below,
            (_, Some(above), None) => self.active = above,
            _ => panic!(),
        }

        // Get rid of old node and children
        parent.borrow_mut().remove_child(active.id);
        let ids: Vec<i32> = NodeIterator::new(active_link.clone())
            .traverse(TraversalType::PostOrder)
            .map(|n| n.id())
            .collect();
        for id in ids {
            self.id_table
                .remove(&id)
                .expect(&format!("could not find node to remove: {}", id));
        }

        Ok(())
    }

    fn get_id_gen(&self) -> &dyn IdGenerator {
        self.generator.as_ref()
    }

    pub fn get_subtree(&self) -> Subtree {
        todo!()
    }

    pub fn get_mut_active_content(&mut self) -> impl DerefMut<Target = String> + '_ {
        RefMut::map(self.active.borrow_mut(), |n| &mut n.content)
    }

    pub fn get_active_content(&self) -> impl Deref<Target = String> + '_ {
        Ref::map(self.active.borrow(), |n| &n.content)
    }

    pub fn get_active_id(&self) -> i32 {
        self.active.borrow().id
    }

    fn get_node(&self, id: i32) -> Option<&Link> {
        self.id_table.get(&id)
    }

    pub fn root_iter(&self) -> NodeIterator {
        NodeIterator::new(self.root.clone())
    }

    pub fn active_iter(&self) -> NodeIterator {
        NodeIterator::new(self.active.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Subtree {
    root: Link,
    parent: Link,
    above_sibling: Link,
}

impl Subtree {
    pub fn root_itr(&self) -> NodeIterator {
        NodeIterator::new(self.root.clone())
    }

    pub fn ids(&self) -> Vec<i32> {
        self.root_itr()
            .traverse(TraversalType::Level)
            .map(|n| n.id())
            .collect()
    }

    fn make_unique(self, id_gen: &dyn IdGenerator) -> Subtree {
        for node_itr in self.root_itr().traverse(TraversalType::PostOrder) {
            node_itr.node.borrow_mut().id = id_gen.gen();
        }
        self
    }
}

#[derive(Debug, Clone)]
struct Node {
    id: i32,
    parent: Option<Link>,
    children: Vec<Link>,
    content: String,
}

impl Node {
    fn new(id: i32, parent: Option<Link>) -> Node {
        Node {
            id,
            parent,
            children: vec![],
            content: String::new(),
        }
    }

    fn new_link(id: i32, parent: Option<Link>) -> Link {
        Link::new(RefCell::new(Self::new(id, parent)))
    }

    fn new_link_from_other(link: &Link) -> Link {
        Link::new(RefCell::new(link.borrow().clone()))
    }

    /// Inserts a `child` above or below an existing child with an id of `relative_id` (if it exists).
    fn insert_child_relative(&mut self, relative_id: i32, dir: Dir, child: Link) -> Result<(), ()> {
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
    fn insert_child_last(&mut self, child: Link) {
        self.children.push(child);
    }

    fn insert_child_first(&mut self, child: Link) {
        self.children.insert(0, child);
    }

    /// Removes the child with the given id. Will borrow every child Link.
    fn remove_child(&mut self, child_id: i32) {
        self.children.retain(|l| l.borrow().id != child_id);
    }

    /// Gets the sibling above or below the current node. This will borrow the parent to access
    /// its children and will borrow a Link to itself. Siblings are nodes on the same layer as
    /// the current node.
    fn get_sibling(&self, dir: Dir) -> Option<Link> {
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

    fn is_root(&self) -> bool {
        self.id == 0
    }
}

fn make_unique_subtree(node: Link) -> Link {
    let mut new_node = node.borrow().clone();
    new_node.children = new_node
        .children
        .into_iter()
        .map(|n| make_unique_subtree(n))
        .collect();
    Link::new(RefCell::new(new_node))
}

pub struct NodeIterator {
    node: Link,
}

impl NodeIterator {
    fn new(node: Link) -> NodeIterator {
        NodeIterator { node }
    }

    pub fn content(&self) -> impl Deref<Target = String> + '_ {
        Ref::map(self.node.borrow(), |n| &n.content)
    }

    pub fn id(&self) -> i32 {
        self.node.borrow().id
    }

    pub fn children_iter(&self) -> impl Iterator<Item = NodeIterator> {
        self.node
            .borrow()
            .children
            .clone()
            .into_iter()
            .map(|n| Self::new(n))
    }

    pub fn traverse(self, traversal: TraversalType) -> impl Iterator<Item = NodeIterator> {
        TreeTraversalIterator::new(self, traversal)
    }

    pub fn next_parent(&mut self) -> Option<NodeIterator> {
        self.node
            .borrow()
            .parent
            .clone()
            .map(|n| NodeIterator::new(n))
    }

    pub fn next_sibling(&mut self, dir: Dir) -> Option<NodeIterator> {
        self.node
            .borrow()
            .get_sibling(dir)
            .map(|n| NodeIterator::new(n.clone()))
    }
}

struct TreeTraversalIterator {
    deque: VecDeque<(NodeIterator, bool)>,
    traversal: TraversalType,
}

pub enum TraversalType {
    PostOrder,
    Level,
}

impl TreeTraversalIterator {
    fn new(itr: NodeIterator, traversal: TraversalType) -> TreeTraversalIterator {
        TreeTraversalIterator {
            deque: vec![(itr, false)].into_iter().collect(),
            traversal,
        }
    }

    fn post_order(&mut self) -> Option<NodeIterator> {
        let node = match self.deque.pop_back() {
            None => return None,
            Some((itr, true)) => return Some(itr),
            Some((itr, false)) => itr,
        };
        let children: Vec<(NodeIterator, bool)> =
            node.children_iter().map(|n| (n, false)).collect();
        let mut children = children.into_iter().rev().collect();
        self.deque.push_back((node, true));
        self.deque.append(&mut children);
        self.post_order()
    }

    fn level(&mut self) -> Option<NodeIterator> {
        let node = match self.deque.pop_front() {
            None => return None,
            Some((itr, true)) => return Some(itr),
            Some((itr, false)) => itr,
        };
        let children: Vec<(NodeIterator, bool)> =
            node.children_iter().map(|n| (n, false)).collect();
        self.deque.push_back((node, true));
        // TODO use VecDeque::prepend once it's implemented
        for child in children {
            self.deque.push_back(child);
        }
        self.level()
    }
}

impl Iterator for TreeTraversalIterator {
    type Item = NodeIterator;

    fn next(&mut self) -> Option<Self::Item> {
        match self.traversal {
            TraversalType::PostOrder => self.post_order(),
            TraversalType::Level => self.level(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct TestGen {
        current: Cell<i32>,
    }

    impl TestGen {
        fn new() -> TestGen {
            TestGen {
                current: Cell::new(1),
            }
        }
    }

    impl IdGenerator for TestGen {
        fn gen(&self) -> i32 {
            (self.current.get(), self.current.set(self.current.get() + 1)).0
        }
    }

    fn new_test_tree() -> Tree {
        Tree::new(Box::new(TestGen::new()))
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
    fn make_unique_subtree_test() {
        let node = Node::new_link(0, None);
        let first = Node::new_link(1, Some(node.clone()));
        node.borrow_mut().insert_child_last(first.clone());

        let subtree = make_unique_subtree(node.clone());
        first.borrow_mut().id = 5;

        assert_eq!(subtree.borrow().children[0].borrow().id, 1);
        assert_eq!(node.borrow().children[0].borrow().id, 5);
    }

    #[test]
    fn siblings_test() {
        let mut tree = new_test_tree();

        assert_eq!(tree.get_active_id(), 1);
        tree.create_sibling();
        assert_eq!(tree.get_active_id(), 2);

        assert_eq!(tree.active.borrow().parent.as_ref().unwrap().borrow().id, 0);
        assert_eq!(
            tree.active.borrow().get_sibling(Above).unwrap().borrow().id,
            1
        );

        let root_node = tree.get_node(0).unwrap();
        assert!(root_node
            .borrow()
            .children
            .iter()
            .any(|n| n.borrow().id == 1));
        assert!(root_node
            .borrow()
            .children
            .iter()
            .any(|n| n.borrow().id == 2));
    }

    #[test]
    fn create_sibling_in_middle_of_list() {
        // 2.
        //   3.
        //   4.
        //   6.
        //   5.
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling();
        tree.indent(false).unwrap(); // id 3 under 2
        tree.create_sibling(); // id 4 under 2
        tree.create_sibling(); // id 5 under 2
        tree.activate(4).unwrap();
        tree.create_sibling(); // id 6 under 2 (after 4, before 5)

        let children = &tree.get_node(2).unwrap().borrow().children;
        assert_eq!(children.get(2).unwrap().borrow().id, 6);
        assert_eq!(children.get(3).unwrap().borrow().id, 5);

        let six = tree.get_node(6).unwrap();
        assert_eq!(
            six.borrow().get_sibling(Below).map(|s| s.borrow().id),
            Some(5)
        );
    }

    #[test]
    fn indents_test() {
        let mut tree = new_test_tree();

        assert!(tree.indent(false).is_err());
        tree.create_sibling();
        assert!(tree.indent(false).is_ok());

        let active_node = tree.active.borrow();
        assert_eq!(active_node.parent.as_ref().map(get_id), Some(1));
        assert_eq!(active_node.id, 2);

        let parent_node = tree.get_node(1).unwrap();
        assert!(parent_node
            .borrow()
            .children
            .iter()
            .any(|n| n.borrow().id == 2));
    }

    #[test]
    fn unindents_test() {
        // 1.
        let mut tree = new_test_tree();
        assert!(tree.unindent().is_err()); // 1 is already top
        tree.create_sibling(); // id = 2
        assert!(tree.indent(false).is_ok()); // (2 under 1)
        assert!(tree.unindent().is_ok()); // (2 under root)
        let two = tree.get_node(2).unwrap();
        assert_eq!(two.borrow().parent.as_ref().map(get_id), Some(0));
        // TODO figure out why printing a Link causes stack overflow

        assert!(tree.indent(false).is_ok());
        tree.create_sibling(); // id = 3 (under 1)
        tree.create_sibling(); // id = 4 (under 1)
        tree.create_sibling(); // id = 5 (under 1)
        assert!(tree.unindent().is_ok()); // (5 under root)
        assert!(tree.indent(false).is_ok()); // (5 under 1)
        let five = tree.get_node(5).unwrap();
        assert_eq!(five.borrow().parent.as_ref().map(get_id), Some(1));
    }

    #[test]
    fn node_iterator() {
        let mut tree = new_test_tree();

        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.create_sibling(); // id = 4
        assert!(tree.indent(false).is_ok()); // (4 under 3)
        tree.create_sibling(); // id = 5 (under 3)

        let root_exp_children = vec![1, 2, 3];
        let root_itr = tree.root_iter();
        let root_children: Vec<NodeIterator> = root_itr.children_iter().collect();
        let mut three_itr = None;

        assert_eq!(root_exp_children.len(), root_children.len());
        for child in &root_children {
            assert!(root_exp_children.iter().any(|&x| x == child.id()));
            if child.id() == 3 {
                three_itr = Some(child);
            }
        }

        let three_exp_children = vec![4, 5];
        let three_children: Vec<NodeIterator> = three_itr.unwrap().children_iter().collect();
        assert_eq!(three_children.len(), three_exp_children.len());
        for child in three_children {
            assert!(three_exp_children.iter().any(|&x| x == child.id()));
        }
    }

    #[test]
    fn delete_simple() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.delete().unwrap(); // id 3 deleted
        assert!(tree.get_node(3).is_none());
        assert!(tree
            .get_node(0)
            .unwrap()
            .borrow()
            .children
            .iter()
            .all(|n| n.borrow().id != 3));
    }

    #[test]
    fn activate_and_delete() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.activate(2).unwrap();
        tree.delete().unwrap();
        assert!(tree.get_node(2).is_none());
        assert!(tree
            .get_node(0)
            .unwrap()
            .borrow()
            .children
            .iter()
            .all(|n| n.borrow().id != 2));
    }

    #[test]
    fn delete_deletes_children() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.indent(false).unwrap(); // 3 under 2
        tree.create_sibling(); // id = 4, under 2
        tree.create_sibling(); // id = 5, under 2
        tree.create_sibling(); // id = 6
        tree.indent(false).unwrap(); // 6 under 5
        tree.create_sibling(); // id = 7
        tree.indent(false).unwrap(); // 7 under 6

        tree.activate(2).unwrap();
        tree.delete().unwrap();
        assert!(tree.get_node(2).is_none());
        assert!(tree.get_node(3).is_none());
        assert!(tree.get_node(4).is_none());
        assert!(tree.get_node(5).is_none());
        assert!(tree.get_node(6).is_none());
        assert!(tree.get_node(7).is_none());
        assert!(tree
            .get_node(0)
            .unwrap()
            .borrow()
            .children
            .iter()
            .all(|n| n.borrow().id != 2));
    }

    #[test]
    fn cannot_delete_last_node() {
        let mut tree = new_test_tree();
        assert!(tree.delete().is_err())
    }

    #[test]
    fn delete_updates_active() {
        let mut tree = new_test_tree();

        // With own sibling
        // 1.
        //   2.
        //   3. <-- deleted
        tree.create_sibling(); // id = 2
        tree.indent(false).unwrap(); // 2 under 1
        tree.create_sibling(); // id = 3
        tree.delete().unwrap(); // delete 3
        assert_eq!(tree.get_active_id(), 2);

        // With no sibling
        tree.delete().unwrap(); // delete 2
        assert_eq!(tree.get_active_id(), 1);

        // 1.
        // 4. <-- deleted
        // 5.
        tree.create_sibling(); // id = 4
        tree.create_sibling(); // id = 5
        tree.activate(4).unwrap();
        tree.delete().unwrap();
        assert_eq!(tree.get_active_id(), 5);
    }

    #[test]
    fn create_sibling_above_test() {
        let mut tree = new_test_tree();

        tree.create_sibling_above(); // id = 2
        tree.create_sibling_above(); // id = 3
        tree.create_sibling_above(); // id = 4
        tree.activate(1).unwrap();
        tree.indent(false).unwrap(); // 1 under 2
        tree.create_sibling_above(); // id = 5
        tree.create_sibling(); // id = 6

        // 4. --
        // 3. --
        // 2. --
        //      5. --
        //      6. --
        //      1. --

        let root = tree.get_node(0).unwrap();
        assert_eq!(get_children_ids(&root), [4, 3, 2]);
        let two = tree.get_node(2).unwrap();
        assert_eq!(get_children_ids(&two), [5, 6, 1]);
    }

    #[test]
    fn get_subtree_test() {
        let mut tree = new_test_tree();

        // 1.
        //   2.
        //     3.
        //   4.
        //   5.
        tree.create_sibling(); // id = 2
        tree.indent(false).unwrap(); // 2 under 1
        tree.create_sibling(); // id = 3 under 1
        tree.create_sibling(); // id = 4 under 1
        tree.create_sibling(); // id = 5 under 1
        tree.activate(3).unwrap();
        tree.indent(false).unwrap();

        tree.activate(1).unwrap();
        let subtree = tree.get_subtree();

        let level_ids: Vec<i32> = subtree
            .root_itr()
            .traverse(TraversalType::Level)
            .map(|n| n.id())
            .collect();
        assert_eq!(level_ids, [1, 2, 4, 5, 3]);
    }

    /*


    fn new_deep_tree() -> Tree {
        let mut tree = new_test_tree();
        // 1.
        // 2.
        //      3.
        //      4.
        //          5.
        //      6.
        // 7.
        // 8.
        //      9.
        //      10.
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.indent(false).unwrap();
        tree.create_sibling(); // id = 4
        tree.create_sibling(); // id = 5
        tree.indent(false).unwrap();
        tree.create_sibling(); // id = 6
        tree.unindent().unwrap();
        tree.create_sibling(); // id = 7
        tree.unindent().unwrap();
        tree.create_sibling(); // id = 8
        tree.create_sibling(); // id = 9
        tree.indent(false).unwrap();
        tree.create_sibling(); // id = 10
        tree
    }

    #[test]
    fn post_order_traversal() {
        let tree = new_deep_tree();
        let post_order_ids: Vec<i32> = tree
            .root_iter()
            .traverse(TraversalType::PostOrder)
            .map(|n| n.id())
            .collect();
        assert_eq!(post_order_ids, [1, 3, 5, 4, 6, 2, 7, 9, 10, 8, 0]);
    }

    #[test]
    fn level_traversal() {
        let tree = new_deep_tree();
        let in_order_ids: Vec<i32> = tree
            .root_iter()
            .traverse(TraversalType::Level)
            .map(|n| n.id())
            .collect();
        assert_eq!(in_order_ids, [0, 1, 2, 7, 8, 3, 4, 6, 9, 10, 5]);
    }

    #[test]
    fn make_unique_subtree() {
        let mut tree = new_deep_tree();
        tree.activate(2).unwrap();
        let (subtree, _, _) = tree.get_subtree();
        let initial_ids = subtree.ids();
        let final_ids: Vec<i32> = subtree.make_unique(tree.get_id_gen()).ids();
        assert!(!final_ids.iter().any(|i| initial_ids.contains(i)));
    }

    #[test]
    fn indent_as_first_test() {
        let mut tree = new_deep_tree();
        tree.activate(7).unwrap();
        tree.indent_as_first().unwrap();

        let seven = tree.get_node(7).unwrap();
        assert_eq!(seven.parent, Some(2));
        assert_eq!(seven.sibling, None);
    }
    */
}
