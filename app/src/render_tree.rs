use clap::ValueEnum;
use fs_tree_shared::ScanTree;
use std::{
    ffi::{OsStr, OsString},
    path::Path,
};

#[cfg(target_family = "unix")]
use crate::colors::get_color_from_id;
use crate::{colors::get_color_from_age, extensions::get_color_from_extension};

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ColorMode {
    Access,
    Modification,
    Creation,
    Extension,
    #[cfg(target_family = "unix")]
    User,
    #[cfg(target_family = "unix")]
    Group,
}

pub enum RenderTree {
    Dir {
        size: u64,
        name: OsString,
        files: usize,
        children: Vec<RenderTree>,
    },
    File {
        size: u64,
        name: OsString,
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

    pub fn get_name(&self) -> &OsStr {
        match self {
            RenderTree::Dir { name, .. } => name,
            RenderTree::File { name, .. } => name,
        }
    }

    pub fn from_scan_tree(
        tree: ScanTree,
        color_mode: &ColorMode,
        now: i64,
        #[cfg(target_family = "unix")] cuid: u32,
        #[cfg(target_family = "unix")] cgid: u32,
    ) -> Self {
        match tree {
            ScanTree::File {
                size,
                name,
                timestamps,
                #[cfg(target_family = "unix")]
                ownership,
            } => {
                let color = match color_mode {
                    ColorMode::Access => get_color_from_age(now, timestamps.access),
                    ColorMode::Creation => get_color_from_age(now, timestamps.creation),
                    ColorMode::Modification => get_color_from_age(now, timestamps.modification),
                    ColorMode::Extension => {
                        get_color_from_extension(Path::new(&name).extension().unwrap_or_default())
                    }
                    #[cfg(target_family = "unix")]
                    ColorMode::User => get_color_from_id(ownership.uid, cuid),
                    #[cfg(target_family = "unix")]
                    ColorMode::Group => get_color_from_id(ownership.gid, cgid),
                };

                RenderTree::File { size, name, color }
            }
            ScanTree::Dir {
                size,
                name,
                children,
                files,
            } => {
                #[cfg(target_family = "unix")]
                let children = children
                    .into_iter()
                    .map(|e| RenderTree::from_scan_tree(e, color_mode, now, cuid, cgid))
                    .collect();

                #[cfg(target_family = "windows")]
                let children = children
                    .into_iter()
                    .map(|e| RenderTree::from_scan_tree(e, color_mode, now))
                    .collect();

                RenderTree::Dir {
                    size,
                    name,
                    files,
                    children,
                }
            }
        }
    }
}
