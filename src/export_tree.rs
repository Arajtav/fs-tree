use crate::scan_tree::ScanTree;
use serde::Serialize;

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
}

impl From<ScanTree> for ExportTree {
    fn from(tree: ScanTree) -> Self {
        match tree {
            ScanTree::File { size, name, .. } => ExportTree::File {
                size,
                name: name.to_string_lossy().into(),
            },
            ScanTree::Dir {
                size,
                name,
                children,
            } => ExportTree::Dir {
                size,
                children: children.into_iter().map(ExportTree::from).collect(),
                name: name.to_string_lossy().into(),
            },
        }
    }
}
