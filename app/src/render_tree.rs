use clap::ValueEnum;
use fs_tree_shared::ScanTree;
use std::path::Path;

use crate::extensions::get_color_from_extension;

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ColorMode {
    Access,
    Modification,
    Creation,
    Extension,
}

pub enum RenderTree {
    Dir {
        size: u64,
        children: Vec<RenderTree>,
    },
    File {
        size: u64,
        color: [f32; 3],
    },
}

fn grayscale_from_age(now: i64, then: i64) -> [f32; 3] {
    if then > now {
        // some green
        return [0.243, 0.925, 0.663];
    }

    const MAX_AGE: f32 = 5.0 * 365.0 * 24.0 * 60.0 * 60.0;

    let normalized_age = ((now - then) as f32 / MAX_AGE).min(1.0);

    let fade = 1.0 - (normalized_age * 9.0 + 1.0).log10();
    let gray = fade.clamp(0.0, 1.0);

    [gray, gray, gray]
}

impl RenderTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            RenderTree::Dir { size, .. } => size,
            RenderTree::File { size, .. } => size,
        }
    }

    pub fn from_scan_tree(tree: ScanTree, color_mode: &ColorMode, now: i64) -> Self {
        match tree {
            ScanTree::File {
                size,
                name,
                access,
                creation,
                modification,
            } => {
                let color = match color_mode {
                    ColorMode::Access => grayscale_from_age(now, access),
                    ColorMode::Creation => grayscale_from_age(now, creation),
                    ColorMode::Modification => grayscale_from_age(now, modification),
                    ColorMode::Extension => {
                        get_color_from_extension(Path::new(&name).extension().unwrap_or_default())
                    }
                };

                RenderTree::File { size, color }
            }
            ScanTree::Dir { size, children, .. } => {
                let children: Vec<RenderTree> = children
                    .into_iter()
                    .map(|e| RenderTree::from_scan_tree(e, color_mode, now))
                    .collect();

                RenderTree::Dir { size, children }
            }
        }
    }
}
