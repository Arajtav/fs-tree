use fs_tree_shared::ScanTree;
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
