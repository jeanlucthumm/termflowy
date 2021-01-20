use std::collections::{HashMap, VecDeque};

type NodeMap = HashMap<i32, Node>;

pub trait IdGenerator {
    fn gen(&self) -> i32;
}

/// Invariants:
/// - There is an active node
/// - The active node is never the root node
/// - There is at least one root node and one child of the root node
/// - No two nodes have the same id
/// - All nodes but root nodes have a parent
pub struct Tree {
    active: i32,
    nodes: NodeMap,
    generator: Box<dyn IdGenerator>,
}

impl Tree {
    pub fn new(generator: Box<dyn IdGenerator>) -> Tree {
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

    pub fn create_sibling_above(&mut self) {
        let node = Node::new(self.generator.gen(), None);
        self.insert_node(node, false);
    }

    pub fn create_sibling(&mut self) {
        let node = Node::new(self.generator.gen(), None);
        self.insert_node(node, true);
    }

    pub fn insert_subtree(&mut self, subtree: Subtree, below: bool) {
        let mut subtree = subtree.make_unique(self.generator.as_ref());
        let root = subtree.nodes.remove(&subtree.root).unwrap();
        let root_id = root.id;
        self.insert_node(root, below);
        for (id, node) in subtree.nodes {
            // No need to use |insert_node| since subtree is isolated
            self.nodes.insert(id, node);
        }
        self.activate(root_id)
            .expect("could not activate subtree root after it was just inserted");
    }

    fn insert_node(&mut self, mut node: Node, below: bool) {
        let active = self.nodes.get(&self.active).unwrap();
        let active_id = active.id;
        node.parent = Some(active.parent.unwrap());
        node.sibling = match below {
            true => Some(active.id),
            false => active.sibling,
        };
        if !below {
            let active = self.nodes.get_mut(&self.active).unwrap();
            active.sibling = Some(node.id);
        }

        let parent = self.nodes.get_mut(&node.parent.unwrap()).unwrap();
        let down_sibling_id; // sibling beneath active
        if let Some(index) = parent.children.iter().position(|id| *id == active_id) {
            down_sibling_id = match below {
                true => parent.children.get(index + 1).cloned(),
                false => None, // sibling ids only point up
            };
            let insert_index = match below {
                true => index + 1,
                false => index,
            };
            parent.children.insert(insert_index, node.id);
        } else {
            panic!("child not found in its own parent")
        }

        if let Some(down_sibling_id) = down_sibling_id {
            let down_sibling = self.nodes.get_mut(&down_sibling_id).unwrap();
            down_sibling.sibling = Some(node.id)
        }

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
        let down_sibling_id = parent.children.get(index_in_parent + 1).copied();
        let active_sibling_id = active.sibling;
        let active_id = active.id;

        let parent_id = parent.id;
        self.nodes
            .get_mut(&parent_id)
            .unwrap()
            .children
            .remove(index_in_parent);

        if let Some(sibling_id) = down_sibling_id {
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

        self.delete_ids_recursive(active_id);

        Ok(())
    }

    // TODO delete this shit
    fn delete_ids_recursive(&mut self, id: i32) {
        for i in self.nodes.get(&id).unwrap().children.clone() {
            self.delete_ids_recursive(i);
        }
        self.nodes.remove(&id);
    }

    fn get_ids_recursive(&self, id: i32) -> Vec<i32> {
        let mut ids: Vec<i32> = Vec::new();
        ids.push(id);
        for i in self.nodes.get(&id).unwrap().children.clone() {
            ids.append(&mut self.get_ids_recursive(i));
        }
        ids
    }

    fn get_id_gen(&self) -> &dyn IdGenerator {
        self.generator.as_ref()
    }

    pub fn get_subtree(&self) -> (Subtree, i32, Option<i32>) {
        let mut nodes: HashMap<i32, Node> = self
            .active_iter()
            .traverse(TraversalType::PostOrder)
            .map(|n| (n.current.id, n.current.clone()))
            .collect();
        let active_node = nodes
            .get_mut(&self.active)
            .expect("did not find active node when generating subtree");
        let parent = active_node.parent.unwrap();
        let sibling = active_node.sibling;
        active_node.parent = None;
        active_node.sibling = None;
        (
            Subtree {
                root: self.active,
                nodes,
            },
            parent,
            sibling,
        )
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
        NodeIterator::new(
            self.nodes.get(&self.active).unwrap(),
            &self.nodes,
            self.active,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Subtree {
    root: i32,
    nodes: NodeMap,
}

impl Subtree {
    pub fn get_root_id(&self) -> i32 {
        self.root
    }

    pub fn root_itr(&self) -> NodeIterator {
        NodeIterator::new(self.nodes.get(&self.root).unwrap(), &self.nodes, self.root)
    }

    pub fn ids(&self) -> Vec<i32> {
        self.root_itr()
            .traverse(TraversalType::Level)
            .map(|n| n.id())
            .collect()
    }

    fn make_unique(mut self, id_gen: &dyn IdGenerator) -> Subtree {
        let id_map: HashMap<i32, i32> = self
            .nodes
            .iter()
            .map(|(id, _)| (*id, id_gen.gen()))
            .collect();
        let convert = |i| id_map.get(&i).copied().unwrap();
        self.nodes = self
            .nodes
            .into_iter()
            .map(|(id, mut n)| {
                n.id = convert(id);
                (n.id, n)
            })
            .collect();
        self.root = convert(self.root);
        for (_, n) in &mut self.nodes {
            n.parent = n.parent.map(convert);
            n.sibling = n.sibling.map(convert);
            n.children = n.children.iter().copied().map(convert).collect();
        }
        self
    }
}

#[derive(Debug, Clone)]
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

    pub fn children_iter<'b>(&'b self) -> impl 'b + Iterator<Item = NodeIterator<'a>> {
        self.current
            .children
            .iter()
            .map(move |i| self.nodes.get(i).unwrap())
            .map(move |n| Self::new(n, self.nodes, self.active_id))
    }

    pub fn traverse(self, traversal: TraversalType) -> impl Iterator<Item = NodeIterator<'a>> {
        TreeTraversalIterator::new(self, traversal)
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

struct TreeTraversalIterator<'a> {
    deque: VecDeque<(NodeIterator<'a>, bool)>,
    traversal: TraversalType,
}

pub enum TraversalType {
    PostOrder,
    Level,
}

impl<'a> TreeTraversalIterator<'a> {
    fn new(itr: NodeIterator, traversal: TraversalType) -> TreeTraversalIterator {
        TreeTraversalIterator {
            deque: vec![(itr, false)].into_iter().collect(),
            traversal,
        }
    }

    fn post_order(&mut self) -> Option<NodeIterator<'a>> {
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

    fn level(&mut self) -> Option<NodeIterator<'a>> {
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

impl<'a> Iterator for TreeTraversalIterator<'a> {
    type Item = NodeIterator<'a>;

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
    fn create_sibling_in_middle_of_list() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling();
        tree.indent().unwrap(); // id 3 under 2
        tree.create_sibling(); // id 4 under 2
        tree.create_sibling(); // id 5 under 2
        tree.activate(4).unwrap();
        tree.create_sibling(); // id 6 under 2 (after 4, before 5)

        let children = &tree.nodes.get(&2).unwrap().children;
        assert_eq!(children.get(2), Some(&6));
        assert_eq!(children.get(3), Some(&5));

        let five = tree.nodes.get(&5).unwrap();
        assert_eq!(five.sibling, Some(6))
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
        assert_eq!(
            tree.nodes
                .get(&0)
                .unwrap()
                .children
                .iter()
                .any(|id| *id == 3),
            false
        );
    }

    #[test]
    fn activate_and_delete() {
        let mut tree = new_test_tree();
        tree.create_sibling(); // id = 2
        tree.create_sibling(); // id = 3
        tree.activate(2).unwrap();
        tree.delete().unwrap();
        assert!(tree.nodes.get(&2).is_none());
        assert_eq!(
            tree.nodes
                .get(&0)
                .unwrap()
                .children
                .iter()
                .any(|id| *id == 2),
            false
        );
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
        assert_eq!(
            tree.nodes
                .get(&0)
                .unwrap()
                .children
                .iter()
                .any(|id| *id == 2),
            false
        );
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
    }

    #[test]
    fn create_sibling_above_test() {
        let mut tree = new_test_tree();

        tree.create_sibling_above(); // id = 2
        tree.create_sibling_above(); // id = 3
        tree.create_sibling_above(); // id = 4
        tree.activate(1).unwrap();
        tree.indent().unwrap(); // 1 under 2
        tree.create_sibling_above(); // id = 5
        tree.create_sibling(); // id = 6

        // 4. --
        // 3. --
        // 2. --
        //      5. --
        //      6. --
        //      1. --

        let root = tree.nodes.get(&0).unwrap();
        assert_eq!(root.children, [4, 3, 2]);
        let two = tree.nodes.get(&2).unwrap();
        assert_eq!(two.children, [5, 6, 1]);
    }

    #[test]
    fn get_subtree_test() {
        let mut tree = new_test_tree();

        tree.create_sibling(); // id = 2
        tree.indent().unwrap(); // 2 under 1
        tree.create_sibling(); // id = 3 under 1
        tree.create_sibling(); // id = 4 under 1
        tree.create_sibling(); // id = 5 under 1

        tree.activate(1).unwrap();
        let (subtree, parent, sibling) = tree.get_subtree();

        assert_eq!(subtree.root, 1);
        let root = subtree.nodes.get(&subtree.root).unwrap();
        assert_eq!(root.children, [2, 3, 4, 5]);
        assert_eq!(root.sibling, None);
        assert_eq!(root.parent, None);
        
        assert_eq!(parent, 0);
        assert_eq!(sibling, None);
    }

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
        tree.indent().unwrap();
        tree.create_sibling(); // id = 4
        tree.create_sibling(); // id = 5
        tree.indent().unwrap();
        tree.create_sibling(); // id = 6
        tree.unindent().unwrap();
        tree.create_sibling(); // id = 7
        tree.unindent().unwrap();
        tree.create_sibling(); // id = 8
        tree.create_sibling(); // id = 9
        tree.indent().unwrap();
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
}
