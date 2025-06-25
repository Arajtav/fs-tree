use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
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

pub fn scan_dir(entrypoint: &Path) -> Result<Tree, std::io::Error> {
    let mut to_scan: Vec<PathBuf> = vec![entrypoint.to_owned()];
    let mut scanned = Tree::new();
    while !to_scan.is_empty() {
        let entry = to_scan.pop().unwrap();
        let metadata = match fs::metadata(&entry) {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!("Error reading metadata for {:?}: {}", entry, err);
                continue;
            }
        };

        if metadata.is_symlink() {
            continue;
        }

        if metadata.is_file() {
            scanned.insert(&entry.canonicalize().unwrap(), metadata.len());
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        let dir = match fs::read_dir(entry.clone()) {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("Error reading directory {:?}: {}", entry, err);
                continue;
            }
        };
        to_scan.append(
            &mut dir
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok(scanned)
}
