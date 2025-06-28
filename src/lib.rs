use gpui::hash;
use rayon::prelude::*;
use serde::{Serialize, Serializer};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, ReadDir},
    path::Path,
};

enum Tree {
    Node {
        size: u64,
        children: HashMap<OsString, Tree>,
    },
    Leaf(u64),
}

impl Tree {
    pub fn to_fs_tree(&self, name: OsString) -> FsTree {
        match self {
            Tree::Leaf(size) => FsTree::Leaf {
                size: *size,
                color: (hash(&name) & 0xffffff) as u32,
                name,
            },
            Tree::Node { size, children } => {
                let mut children: Vec<FsTree> = children
                    .iter()
                    .map(|(child_name, child_tree)| child_tree.to_fs_tree(child_name.clone()))
                    .collect();

                children.sort_by(|a, b| b.get_size().cmp(&a.get_size()));

                FsTree::Node {
                    size: *size,
                    children,
                    name,
                }
            }
        }
    }
}

pub fn scan_dir(entry: &Path) -> FsTree {
    let dir = match fs::read_dir(entry) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error reading directory {:?}: {}", entry, err);
            return FsTree::Node {
                name: entry.into(),
                size: 0,
                children: Vec::new(),
            };
        }
    };

    let (size, children) = recursive_scan_dir(dir);
    Tree::Node { size, children }.to_fs_tree(entry.into())
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

fn serialize_os_string<S>(s: &OsString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&s.to_string_lossy())
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum FsTree {
    #[serde(rename = "dir")]
    Node {
        #[serde(serialize_with = "serialize_os_string")]
        name: OsString,
        size: u64,
        children: Vec<FsTree>,
    },
    #[serde(rename = "file")]
    Leaf {
        #[serde(serialize_with = "serialize_os_string")]
        name: OsString,
        size: u64,
        #[serde(skip)]
        color: u32,
    },
}

impl FsTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            FsTree::Node { size, .. } => size,
            FsTree::Leaf { size, .. } => size,
        }
    }
}
