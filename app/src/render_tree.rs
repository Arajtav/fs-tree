use clap::ValueEnum;
use fs_tree_shared::ScanTree;
use std::path::Path;

use crate::{
    colors::{get_color_from_age, get_color_from_id},
    extensions::get_color_from_extension,
};

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ColorMode {
    Access,
    Modification,
    Creation,
    Extension,
    User,
    Group,
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

impl RenderTree {
    pub fn get_size(&self) -> u64 {
        *match self {
            RenderTree::Dir { size, .. } => size,
            RenderTree::File { size, .. } => size,
        }
    }

    pub fn from_scan_tree(
        tree: ScanTree,
        color_mode: &ColorMode,
        now: i64,
        cuid: u32,
        cgid: u32,
    ) -> Self {
        match tree {
            ScanTree::File {
                size,
                name,
                access,
                creation,
                modification,
                uid,
                gid,
            } => {
                let color = match color_mode {
                    ColorMode::Access => get_color_from_age(now, access),
                    ColorMode::Creation => get_color_from_age(now, creation),
                    ColorMode::Modification => get_color_from_age(now, modification),
                    ColorMode::Extension => {
                        get_color_from_extension(Path::new(&name).extension().unwrap_or_default())
                    }
                    ColorMode::User => get_color_from_id(uid, cuid),
                    ColorMode::Group => get_color_from_id(gid, cgid),
                };

                RenderTree::File { size, color }
            }
            ScanTree::Dir { size, children, .. } => {
                let children: Vec<RenderTree> = children
                    .into_iter()
                    .map(|e| RenderTree::from_scan_tree(e, color_mode, now, cuid, cgid))
                    .collect();

                RenderTree::Dir { size, children }
            }
        }
    }
}
