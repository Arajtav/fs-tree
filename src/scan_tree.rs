use rayon::prelude::*;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, ReadDir},
    os::linux::fs::MetadataExt,
    path::Path,
};

pub enum ScanTree {
    Dir {
        size: u64,
        children: HashMap<OsString, ScanTree>,
    },
    File {
        size: u64,
        access: i64,
        creation: i64,
        modification: i64,
    },
}

pub fn scan_dir(entry: &Path) -> ScanTree {
    let dir = match fs::read_dir(entry) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error reading directory {:?}: {}", entry, err);
            return ScanTree::File {
                size: 0,
                access: 0,
                creation: 0,
                modification: 0,
            };
        }
    };

    let (size, children) = recursive_scan_dir(dir);
    ScanTree::Dir { size, children }
}

fn recursive_scan_dir(dir: ReadDir) -> (u64, HashMap<OsString, ScanTree>) {
    let results: Vec<(OsString, ScanTree, u64)> = dir
        .par_bridge()
        .filter_map(|entry_res| {
            let entry = match entry_res {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!("Error: {err}");
                    return None;
                }
            };

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    eprintln!("Error reading metadata: {}", err);
                    return None;
                }
            };

            if metadata.is_symlink() {
                return None;
            }

            let file_name = entry.file_name();

            if metadata.is_file() {
                let len = metadata.len();
                return Some((
                    file_name,
                    ScanTree::File {
                        size: len,
                        access: metadata.st_atime(),
                        creation: metadata.st_ctime(), // change but whatever
                        modification: metadata.st_mtime(),
                    },
                    len,
                ));
            }

            let dir = match fs::read_dir(entry.path()) {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("Error reading directory {:?}: {}", entry, err);
                    return None;
                }
            };

            let (subtree_size, subtree_children) = recursive_scan_dir(dir);

            Some((
                file_name,
                ScanTree::Dir {
                    size: subtree_size,
                    children: subtree_children,
                },
                subtree_size,
            ))
        })
        .collect();

    let size = results.iter().map(|(_, _, sz)| *sz).sum();
    let children = results
        .into_iter()
        .map(|(name, tree, _)| (name, tree))
        .collect();

    (size, children)
}
