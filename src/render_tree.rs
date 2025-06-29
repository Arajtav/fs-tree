use gpui::hash;
use std::ffi::OsString;

use crate::scan_tree::ScanTree;

pub enum RenderTree {
    Dir {
        name: OsString,
        size: u64,
        children: Vec<RenderTree>,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    File {
        name: OsString,
        size: u64,
        color: u32,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
}

impl RenderTree {
    pub fn from_scan_tree(
        tree: ScanTree,
        name: OsString,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        horizontal: bool,
    ) -> Self {
        match tree {
            ScanTree::File(size) => RenderTree::File {
                size,
                color: (hash(&name) & 0xffffff) as u32,
                name,
                x,
                y,
                dx,
                dy,
            },
            ScanTree::Dir { size, children } => {
                let mut children: Vec<(OsString, ScanTree)> = children.into_iter().collect();
                children.sort_by(|a, b| b.1.get_size().cmp(&a.1.get_size()));

                let mut offset = 0.0;

                let mapped_children: Vec<RenderTree> = children
                    .into_iter()
                    .map(|(child_name, child_tree)| {
                        if horizontal {
                            let child_dx = dx * child_tree.get_size() as f32 / size as f32;
                            let child_tree = RenderTree::from_scan_tree(
                                child_tree,
                                child_name,
                                x + offset,
                                y,
                                child_dx,
                                dy,
                                false,
                            );
                            offset += child_dx;
                            child_tree
                        } else {
                            let child_dy = dy * child_tree.get_size() as f32 / size as f32;
                            let child_tree = RenderTree::from_scan_tree(
                                child_tree,
                                child_name,
                                x,
                                y + offset,
                                dx,
                                child_dy,
                                true,
                            );
                            offset += child_dy;
                            child_tree
                        }
                    })
                    .collect();

                RenderTree::Dir {
                    name,
                    size,
                    children: mapped_children,
                    x,
                    y,
                    dx,
                    dy,
                }
            }
        }
    }
}
