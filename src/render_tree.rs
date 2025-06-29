use gpui::hash;
use std::ffi::OsString;

use crate::scan_tree::ScanTree;

pub enum RenderTree {
    Dir {
        name: OsString,
        size: u64,
        children: Vec<RenderTree>,
    },
    File {
        name: OsString,
        size: u64,
        color: u32,
    },
}

impl RenderTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            RenderTree::Dir { size, .. } => size,
            RenderTree::File { size, .. } => size,
        }
    }

    pub fn from_scan_tree(tree: ScanTree, name: OsString) -> Self {
        match tree {
            ScanTree::File(size) => RenderTree::File {
                size,
                color: (hash(&name) & 0xffffff) as u32,
                name,
            },
            ScanTree::Dir { size, children } => {
                let mut children: Vec<RenderTree> = children
                    .into_iter()
                    .map(|(child_name, child_tree)| {
                        RenderTree::from_scan_tree(child_tree, child_name)
                    })
                    .collect();

                children.sort_by(|a, b| b.get_size().cmp(&a.get_size()));

                RenderTree::Dir {
                    size,
                    children,
                    name,
                }
            }
        }
    }
}
