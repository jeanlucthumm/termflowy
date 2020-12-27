use std::collections::HashMap;

type NodeMap = HashMap<i32, Node>;

pub trait IdGenerator {
    fn gen(&mut self) -> i32;
}

/// Invariants:
/// - There is an active node
/// - The active node is never the root node
/// - There is at least one root node and one child of the root node
/// - No two nodes have the same id
/// - All nodes but root have a parent
pub struct Tree {
    active: i32,
    nodes: NodeMap,
    generator: Box<dyn IdGenerator>,
}

impl Tree {
    pub fn new(mut generator: Box<dyn IdGenerator>) -> Tree {
        let mut nodes = NodeMap::new();
        let id = generator.gen();
        let mut root = Node::new(0, None);
        root.children.push(id);
        nodes.insert(0, root);
        let first = Node::new(id, Some(0));
        nodes.insert(id, first);
        Tree {
            active: id,
            nodes,
            generator,
        }
    }

    /// Create another bullet at the same level, i.e. a sibling of the active node
    pub fn create_sibling(&mut self) {
        let active = self.nodes.get(&self.active).unwrap();
        let mut node = Node::new(self.generator.gen(), active.parent);
        node.sibling = Some(active.id);

        let parent = self.nodes.get_mut(&node.parent.unwrap()).unwrap();
        parent.children.push(node.id);

        self.active = node.id;
        self.nodes.insert(node.id, node);
    }

    pub fn indent(&mut self) -> Result<(), String> {
        let active = self.nodes.get(&self.active).unwrap();
        let id = active.id;
        let parent_id = active.parent.unwrap();
        let sibling_id = if let Some(x) = active.sibling {
            x
        } else {
            return Err(String::from("could not indent: node has no siblings"));
        };

        let parent = self.nodes.get_mut(&parent_id).unwrap();
        parent.children.retain(|i| *i != id);

        let sibling = self.nodes.get_mut(&sibling_id).unwrap();
        let new_sibling = match sibling.children.last() {
            Some(id) => Some(*id),
            None => None,
        };
        sibling.children.push(id);

        let active = self.nodes.get_mut(&id).unwrap();
        active.parent = Some(sibling_id);
        active.sibling = new_sibling;
        Ok(())
    }

    pub fn unindent(&mut self) -> Result<(), String> {
        let active = self.nodes.get(&self.active).unwrap();
        let id = active.id;
        let parent_id = active.parent.unwrap();
        if parent_id == 0 {
            return Err(String::from("could not unindent: already at top level"));
        }
        let grandparent_id = self.nodes.get(&parent_id).unwrap().parent.unwrap();

        let parent = self.nodes.get_mut(&parent_id).unwrap();
        parent.children.retain(|i| *i != id);
        let grandparent = self.nodes.get_mut(&grandparent_id).unwrap();
        if let Some(index) = grandparent.children.iter().position(|i| *i == parent_id) {
            grandparent.children.insert(index + 1, id);
        } else {
            panic!("bad tree invariant: parent not found in children of its own parent");
        }

        let active = self.nodes.get_mut(&id).unwrap();
        active.parent = Some(grandparent_id);
        active.sibling = Some(parent_id);

        Ok(())
    }

    pub fn activate(&mut self, id: i32) -> Result<(), String> {
        if !self.nodes.contains_key(&id) {
            Err(format!(
                "could not activate node with id {}: does not exist",
                id
            ))
        } else if id == 0 {
            Err(String::from("cannot active root node"))
        } else {
            self.active = id;
            Ok(())
        }
    }

    pub fn delete(&mut self) -> Result<(), String> {
        let active = self.nodes.get(&self.active).unwrap();
        let parent = self.nodes.get(&active.parent.unwrap()).unwrap();
        if parent.id == 0 && active.sibling.is_none() {
            return Err(String::from("cannot delete the last bullet"));
        }
        let index_in_parent = parent
            .children
            .iter()
            .position(|id| *id == active.id)
            .unwrap();
        // Node that has active as sibling
        let sibling_id = parent.children.get(index_in_parent + 1).copied();
        // Sibling of active
        let active_sibling_id = active.sibling;
        let active_id = active.id;

        let parent_id = parent.id;
        self.nodes
            .get_mut(&parent_id)
            .unwrap()
            .children
            .remove(index_in_parent);

        if let Some(sibling_id) = sibling_id {
            self.nodes.get_mut(&sibling_id).unwrap().sibling = active_sibling_id;
            self.active = sibling_id;
        } else if let Some(active_sibling_id) = active_sibling_id {
            self.active = active_sibling_id;
        } else {
            if parent_id == 0 {
                panic!("delete should not lead to root being active");
            }
            self.active = parent_id;
        }

        self.delete_subtree_ids(active_id);

        Ok(())
    }

    fn delete_subtree_ids(&mut self, id: i32) {
        let children = self.nodes.get(&id).unwrap().children.clone();
        for i in children {
            self.delete_subtree_ids(i);
        }
        self.nodes.remove(&id);
    }

    pub fn get_mut_active_content(&mut self) -> &mut String {
        &mut self.nodes.get_mut(&self.active).unwrap().content
    }

    pub fn get_active_content(&self) -> &String {
        &self.nodes.get(&self.active).unwrap().content
    }

    pub fn root_iter(&self) -> NodeIterator {
        NodeIterator::new(self.nodes.get(&0).unwrap(), &self.nodes, self.active)
    }

    pub fn active_iter(&self) -> NodeIterator {
        NodeIterator::new(self.nodes.get(&self.active).unwrap(), &self.nodes, self.active)
    }
}

#[derive(Debug)]
struct Node {
    id: i32,
    parent: Option<i32>,
    sibling: Option<i32>,
    children: Vec<i32>,
    content: String,
}

impl Node {
    fn new(id: i32, parent: Option<i32>) -> Node {
        Node {
            id,
            parent,
            sibling: None,
            children: vec![],
            content: String::new(),
        }
    }
}

pub struct NodeIterator<'a> {
    nodes: &'a NodeMap,
    current: &'a Node,
    active_id: i32,
}

impl<'a> NodeIterator<'a> {
    fn new(node: &'a Node, nodes: &'a NodeMap, active_id: i32) -> NodeIterator<'a> {
        NodeIterator {
            nodes,
            current: node,
            active_id,
        }
    }

    pub fn content(&self) -> &String {
        &self.current.content
    }

    pub fn id(&self) -> i32 {
        self.current.id
    }

    pub fn children_iter(&self) -> impl Iterator<Item = NodeIterator> {
        self.current
            .children
            .iter()
            .map(move |i| self.nodes.get(i).unwrap())
            .map(move |n| Self::new(n, self.nodes, self.active_id))
    }

    pub fn next_parent(&mut self) -> Option<i32> {
        match self.nodes.get(&self.active_id).unwrap().parent {
            Some(0) => None,
            Some(id) => {
                self.active_id = id;
                Some(id)
            }
            None => panic!("all nonroot nodes should have parents"),
        }
    }

    pub fn next_sibling(&mut self) -> Option<i32> {
        match self.nodes.get(&self.active_id).unwrap().sibling {
            Some(id) => {
                self.active_id = id;
                Some(id)
            }
            None => None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.current.id == self.active_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGen {
        current: i32,
    }

    impl TestGen {
        fn new() -> TestGen {
            TestGen { current: 1 }
        }
    }

    impl IdGenerator for TestGen {
        fn gen(&mut self) -> i32 {
            (self.current, self.current += 1).0
        }
    }

    fn new_test_tree() -> Tree {
        Tree::new(Box::new(TestGen::new()))
    }

    #[test]
    fn siblings_test() {
        let mut tree = new_test_tree();

        assert_eq!(tree.active, 1);
        tree.create_sibling();
        assert_eq!(tree.active, 2);

        let active_node = tree.nodes.get(&tree.active).unwrap();
        assert_eq!(active_node.parent.unwrap(), 0);
        assert_eq!(active_node.sibling.unwrap(), 1);

        let root_node = tree.nodes.get(&0).unwrap();
        assert!(root_node.children.iter().any(|i| *i == 1));
        assert!(root_node.children.iter().any(|i| *i == 2));
    }

    #[test]
    fn indents_test() {
        let mut tree = new_test_tree();

        assert!(tree.indent().is_err());
        tree.create_sibling();
        assert!(tree.indent().is_ok());

        let active_node = tree.nodes.get(&tree.active).unwrap();
        assert_eq!(active_node.parent, Some(1));
        assert_eq!(active_node.sibling, None);
        assert_eq!(active_node.id, 2);

        let parent_node = tree.nodes.get(&1).unwrap();
        assert!(parent_node.children.iter().any(|i| *i == 2));
    }

    #[test]
    fn unindents_test() {
        let mut tree = new_test_tree();

        assert!(tree.unindent().is_err()); // 1 is already top
        tree.create_sibling(); // id = 2
        assert!(tree.indent().is_ok()); // (2 under 1)
        assert!(tree.unindent().is_ok()); // (2 under root)
        let two = tree.nodes.get(&2).unwrap();
        assert_eq!(two.parent, Some(0));
        assert_eq!(two.sibling, Some(1));
        println!("{:?}", two);

        assert!(tree.indent().is_ok());
        tree.create_sibling(); // id = 3 (under 1)
        tree.create_sibling(); // id = 4 (under 1)
        tree.create_sibling(); // id = 5 (under 1)
        assert!(tree.unindent().is_ok()); // (5 under root)
        assert!(tree.indent().is_ok()); // (5 under 1)
        let five = tree.nodes.get(&5).unwrap();
        assert_eq!(five.parent, Some(1));
        assert_eq!(five.sibling, Some(4));
    }

    #[test]
    fn node_iterator() {
        let mut tree = new_test_tree();

        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.create_sibling(); // id = 4
        assert!(tree.indent().is_ok()); // (4 under 3)
        tree.create_sibling(); // id = 5 (under 3)

        let root_exp_children = vec![1, 2, 3];
        let root_itr = tree.root_iter();
        let root_children: Vec<NodeIterator> = root_itr.children_iter().collect();
        let mut three_itr = None;

        assert_eq!(root_exp_children.len(), root_children.len());
        for child in &root_children {
            assert!(root_exp_children.iter().any(|&x| x == child.current.id));
            if child.current.id == 3 {
                three_itr = Some(child);
            }
        }

        let three_exp_children = vec![4, 5];
        let three_children: Vec<NodeIterator> = three_itr.unwrap().children_iter().collect();
        assert_eq!(three_children.len(), three_exp_children.len());
        for child in three_children {
            assert!(three_exp_children.iter().any(|&x| x == child.current.id));
        }
    }

    #[test]
    fn delete_simple() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.delete().unwrap(); // id 3 deleted
        assert!(tree.nodes.get(&3).is_none());
        assert_eq!(tree.nodes.get(&0).unwrap().children.iter().any(|id| *id == 3), false);
    }

    #[test]
    fn activate_and_delete() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.activate(2).unwrap();
        tree.delete().unwrap();
        assert!(tree.nodes.get(&2).is_none());
        assert_eq!(tree.nodes.get(&0).unwrap().children.iter().any(|id| *id == 2), false);
        assert_eq!(tree.nodes.get(&3).unwrap().sibling, Some(1));
    }

    #[test]
    fn delete_deletes_children() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.indent().unwrap(); // 3 under 2
        tree.create_sibling(); // id = 4, under 2
        tree.create_sibling(); // id = 5, under 2
        tree.create_sibling(); // id = 6
        tree.indent().unwrap(); // 6 under 5
        tree.create_sibling(); // id = 7
        tree.indent().unwrap(); // 7 under 6

        tree.activate(2).unwrap();
        tree.delete().unwrap();
        assert!(tree.nodes.get(&2).is_none());
        assert!(tree.nodes.get(&3).is_none());
        assert!(tree.nodes.get(&4).is_none());
        assert!(tree.nodes.get(&5).is_none());
        assert!(tree.nodes.get(&6).is_none());
        assert!(tree.nodes.get(&7).is_none());
        assert_eq!(tree.nodes.get(&0).unwrap().children.iter().any(|id| *id == 2), false);
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
        tree.create_sibling(); // id = 2
        tree.indent().unwrap(); // 2 under 1
        tree.create_sibling(); // id = 3
        tree.delete().unwrap(); // delete 3
        assert_eq!(tree.active, 2);

        // With no sibling
        tree.delete().unwrap(); // delete 2
        assert_eq!(tree.active, 1);

        // Tree is just root and 1 at this point.
        // With self as sibling and first in list
        tree.create_sibling(); // id = 4
        tree.create_sibling(); // id = 5
        tree.activate(4).unwrap();
        tree.delete().unwrap();
        assert_eq!(tree.active, 5);

        // With self as sibling and middle in list
    }
}
