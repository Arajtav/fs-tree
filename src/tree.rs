use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Component, Path},
};

#[derive(Debug)]
pub enum Tree {
    Node {
        size: u64,
        children: HashMap<OsString, Tree>,
    },
    #[allow(dead_code)]
    Leaf(u64),
}

impl Tree {
    pub fn new() -> Self {
        Tree::Node {
            children: HashMap::new(),
            size: 0,
        }
    }

    // Will break if:
    // - leaf is replaced with a node, e.g. (/a, 10), (/a/b, 5)
    // - leaf is modified e.g. (/a, 10), (/a, 5)
    pub fn insert(&mut self, path: &Path, value: u64) {
        let mut current = self;
        for comp in path.components() {
            match comp {
                Component::RootDir => continue,
                Component::Normal(name) => match current {
                    Tree::Node { children, size } => {
                        *size += value;
                        current = children.entry(name.to_os_string()).or_insert(Tree::new());
                    }
                    Tree::Leaf(_) => *current = Tree::new(),
                },
                _ => panic!("Unsupported path component: {:?}", comp),
            }
        }
        *current = Tree::Leaf(value);
    }

    #[cfg(test)]
    pub fn get(&self, path: &Path) -> u64 {
        let mut current = self;
        for comp in path.components() {
            match comp {
                Component::RootDir => continue,
                Component::Normal(name) => match current {
                    Tree::Node { children, .. } => {
                        current = match children.get(name) {
                            Some(child) => child,
                            None => return 0,
                        };
                    }
                    Tree::Leaf(_) => return 0,
                },
                _ => panic!("Unsupported path component: {:?}", comp),
            }
        }
        match current {
            Tree::Node { size, .. } => *size,
            Tree::Leaf(size) => *size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insertion() {
        let mut tree = Tree::new();
        tree.insert(&Path::new("/a/b/c"), 5);
        tree.insert(&Path::new("/a/b/d"), 6);
        tree.insert(&Path::new("/a/e"), 7);
        tree.insert(&Path::new("/f"), 8);
        assert_eq!(tree.get(&Path::new("/")), 26);
        assert_eq!(tree.get(&Path::new("/a/b/d")), 6);
        assert_eq!(tree.get(&Path::new("/a/b")), 11);
    }
}
