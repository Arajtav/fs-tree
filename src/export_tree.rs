use serde::Serialize;

use crate::scan_tree::ScanTree;

#[derive(Serialize)]
#[serde(untagged)]
pub enum ExportTree {
    Dir {
        name: String,
        size: u64,
        children: Vec<ExportTree>,
    },
    File {
        name: String,
        size: u64,
    },
}

impl ExportTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            ExportTree::Dir { size, .. } => size,
            ExportTree::File { size, .. } => size,
        }
    }

    pub fn from_scan_tree(tree: ScanTree, name: String) -> Self {
        match tree {
            ScanTree::File(size) => ExportTree::File { size, name },
            ScanTree::Dir { size, children } => {
                let mut children: Vec<ExportTree> = children
                    .into_iter()
                    .map(|(child_name, child_tree)| {
                        ExportTree::from_scan_tree(child_tree, child_name.to_string_lossy().into())
                    })
                    .collect();

                children.sort_by(|a, b| b.get_size().cmp(&a.get_size()));

                ExportTree::Dir {
                    size,
                    children,
                    name,
                }
            }
        }
    }
}
