use std::collections::HashMap;

type NodeMap = HashMap<i32, Node>;

pub trait IdGenerator {
    fn gen(&mut self) -> i32;
}

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

    pub fn indent(&mut self) -> Result<(), &str> {
        let active = self.nodes.get(&self.active).unwrap();
        let id = active.id;
        let parent_id = active.parent.unwrap();
        let sibling_id = if let Some(x) = active.sibling {
            x
        } else {
            return Err("could not indent: node has no siblings");
        };

        let parent = self.nodes.get_mut(&parent_id).unwrap();
        parent.children.retain(|i| *i != id);
        let sibling = self.nodes.get_mut(&sibling_id).unwrap();
        sibling.children.push(id);
        let active = self.nodes.get_mut(&id).unwrap();
        active.parent = Some(sibling_id);
        active.sibling = None;
        Ok(())
    }

    pub fn get_mut_active_content(&mut self) -> &mut String {
        &mut self.nodes.get_mut(&self.active).unwrap().content
    }

    pub fn root_iter(&self) -> NodeIterator {
        NodeIterator::new(self.nodes.get(&0).unwrap(), &self.nodes, self.active)
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

    pub fn children_iter(&self) -> impl Iterator<Item = NodeIterator> {
        self.current
            .children
            .iter()
            .map(move |i| self.nodes.get(i).unwrap())
            .map(move |n| Self::new(n, self.nodes, self.active_id))
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
}
