use serde::{Deserialize, Serialize};

use crate::ScanTree;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SavedTree {
    Dir {
        name: String,
        size: u64,
        children: Vec<SavedTree>,
    },
    File {
        name: String,
        size: u64,
    },
}

impl From<ScanTree> for SavedTree {
    fn from(tree: ScanTree) -> Self {
        match tree {
            ScanTree::File { size, name, .. } => SavedTree::File {
                size,
                name: name.to_string_lossy().into(),
            },
            ScanTree::Dir {
                size,
                name,
                children,
                ..
            } => SavedTree::Dir {
                size,
                children: children.into_iter().map(SavedTree::from).collect(),
                name: name.to_string_lossy().into(),
            },
        }
    }
}
