use std::cell::RefCell;
use std::collections::HashMap;

type NodeCell = RefCell<Node>;
type NodeMap = HashMap<i32, NodeCell>;

pub trait IdGenerator {
    fn gen(&mut self) -> i32;
}

pub struct Tree<'a> {
    active: i32,
    nodes: NodeMap,
    generator: &'a mut dyn IdGenerator,
}

impl<'a> Tree<'a> {
    pub fn new(generator: &'a mut dyn IdGenerator) -> Tree<'a> {
        let mut nodes = NodeMap::new();
        let id = generator.gen();
        let mut root = Node::new(0, None);
        root.children.push(id);
        nodes.insert(0, NodeCell::new(root));
        let first = Node::new(id, Some(0));
        nodes.insert(id, NodeCell::new(first));
        Tree {
            active: id,
            nodes,
            generator,
        }
    }

    /// Create another bullet at the same level, i.e. a sibling of the active node
    pub fn create_sibling(&mut self) {
        let active = self.nodes.get(&self.active).unwrap().borrow();
        let mut node = Node::new(self.generator.gen(), active.parent);
        node.sibling = Some(active.id);
        drop(active);
        self.active = node.id;
        self.nodes.insert(node.id, NodeCell::new(node));
    }

    pub fn indent(&mut self) -> Result<(), &str> {
        let mut active = self.nodes.get(&self.active).unwrap().borrow_mut();
        let mut parent = self
            .nodes
            .get(&active.parent.unwrap())
            .unwrap()
            .borrow_mut();
        if let Some(sibling) = active.sibling {
            let mut sibling = self.nodes.get(&sibling).unwrap().borrow_mut();
            parent.children.retain(|i| *i != active.id);
            active.parent = active.sibling;
            active.sibling = None;
            sibling.children.push(active.id);
            Ok(())
        } else {
            Err("could not indent: node has no siblings")
        }
    }
}

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

    #[test]
    fn siblings_test() {
        let mut gen = TestGen::new();
        let mut tree = Tree::new(&mut gen);

        assert_eq!(tree.active, 1);
        tree.create_sibling();
        assert_eq!(tree.active, 2);

        let active_node = tree.nodes.get(&tree.active).unwrap().borrow();
        assert_eq!(active_node.parent.unwrap(), 0);
        assert_eq!(active_node.sibling.unwrap(), 1);
    }

    #[test]
    fn indents_test() {
        let mut gen = TestGen::new();
        let mut tree = Tree::new(&mut gen);

        assert!(tree.indent().is_err());
        tree.create_sibling();
        assert!(tree.indent().is_ok());

        let active_node = tree.nodes.get(&tree.active).unwrap().borrow();
        assert_eq!(active_node.parent, Some(1));
        assert_eq!(active_node.sibling, None);
        assert_eq!(active_node.id, 2);
    }
}
