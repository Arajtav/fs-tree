use clap::ValueEnum;
use std::ffi::OsString;

use crate::scan_tree::ScanTree;

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ColorMode {
    Access,
    Modification,
    Creation,
}

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

fn grayscale_from_age(now: i64, then: i64) -> u32 {
    let now = now as i64;

    if then > now {
        // some green
        return 0x3eeca9;
    }

    const MAX_AGE: f32 = 5.0 * 365.0 * 24.0 * 60.0 * 60.0;

    let normalized_age = ((now - then) as f32 / MAX_AGE).min(1.0);

    let fade = 1.0 - (normalized_age * 9.0 + 1.0).log10();
    let gray = (fade.clamp(0.0, 1.0) * 255.0).round() as u32;

    (gray << 16) | (gray << 8) | gray
}

impl RenderTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            RenderTree::Dir { size, .. } => size,
            RenderTree::File { size, .. } => size,
        }
    }

    pub fn from_scan_tree(
        tree: ScanTree,
        name: OsString,
        color_mode: &ColorMode,
        now: i64,
    ) -> Self {
        match tree {
            ScanTree::File {
                size,
                access,
                creation,
                modification,
            } => {
                let color = grayscale_from_age(
                    now,
                    match color_mode {
                        ColorMode::Access => access,
                        ColorMode::Creation => creation,
                        ColorMode::Modification => modification,
                    },
                );

                RenderTree::File { size, color, name }
            }
            ScanTree::Dir { size, children } => {
                let mut children: Vec<RenderTree> = children
                    .into_iter()
                    .map(|(child_name, child_tree)| {
                        RenderTree::from_scan_tree(child_tree, child_name, &color_mode, now)
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
