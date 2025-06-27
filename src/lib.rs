use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, ReadDir},
    path::Path,
};

#[derive(Debug)]
pub enum Tree {
    Node {
        size: u64,
        children: HashMap<OsString, Tree>,
    },
    Leaf(u64),
}

pub fn scan_dir(entry: &Path) -> Tree {
    let dir = match fs::read_dir(entry) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error reading directory {:?}: {}", entry, err);
            return Tree::Node {
                size: 0,
                children: HashMap::new(),
            };
        }
    };

    let (size, children) = recursive_scan_dir(dir);
    Tree::Node { size, children }
}

fn recursive_scan_dir(dir: ReadDir) -> (u64, HashMap<OsString, Tree>) {
    let mut size = 0u64;
    let mut children = HashMap::new();
    for entry in dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("Error: {err}");
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!("Error reading metadata: {}", err);
                continue;
            }
        };

        if metadata.is_symlink() {
            continue;
        }

        let file_name = entry.file_name();

        if metadata.is_file() {
            let len = metadata.len();
            size += len;
            children.insert(file_name, Tree::Leaf(len));
            continue;
        }

        let dir = match fs::read_dir(entry.path()) {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("Error reading directory {:?}: {}", entry, err);
                continue;
            }
        };

        let (subtree_size, subtree_children) = recursive_scan_dir(dir);
        size += subtree_size;
        children.insert(
            file_name,
            Tree::Node {
                size: subtree_size,
                children: subtree_children,
            },
        );
    }

    (size, children)
}
