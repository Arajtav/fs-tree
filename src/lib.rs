use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, ReadDir},
    path::Path,
};

use rayon::prelude::*;

#[derive(Debug)]
pub enum Tree {
    Node {
        size: u64,
        children: HashMap<OsString, Tree>,
    },
    Leaf(u64),
}

impl Tree {
    pub fn get_size(&self) -> u64 {
        *match self {
            Tree::Node { size, .. } => size,
            Tree::Leaf(size) => size,
        }
    }
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
    let results: Vec<(OsString, Tree, u64)> = dir
        .par_bridge()
        .filter_map(|entry_res| {
            let entry = match entry_res {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!("Error: {err}");
                    return None;
                }
            };

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    eprintln!("Error reading metadata: {}", err);
                    return None;
                }
            };

            if metadata.is_symlink() {
                return None;
            }

            let file_name = entry.file_name();

            if metadata.is_file() {
                let len = metadata.len();
                return Some((file_name, Tree::Leaf(len), len));
            }

            let dir = match fs::read_dir(entry.path()) {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("Error reading directory {:?}: {}", entry, err);
                    return None;
                }
            };

            let (subtree_size, subtree_children) = recursive_scan_dir(dir);

            Some((
                file_name,
                Tree::Node {
                    size: subtree_size,
                    children: subtree_children,
                },
                subtree_size,
            ))
        })
        .collect();

    let size = results.iter().map(|(_, _, sz)| *sz).sum();
    let children = results
        .into_iter()
        .map(|(name, tree, _)| (name, tree))
        .collect();

    (size, children)
}
