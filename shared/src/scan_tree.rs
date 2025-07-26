use rayon::prelude::*;
use std::{
    ffi::OsString,
    fs::{self},
    path::Path,
};

#[cfg(feature = "full_metadata")]
use std::os::linux::fs::MetadataExt;

pub enum ScanTree {
    Dir {
        name: OsString,
        size: u64,
        children: Vec<ScanTree>,
    },
    File {
        name: OsString,
        size: u64,
        #[cfg(feature = "full_metadata")]
        access: i64,
        #[cfg(feature = "full_metadata")]
        creation: i64,
        #[cfg(feature = "full_metadata")]
        modification: i64,
    },
}

impl ScanTree {
    fn get_size(&self) -> u64 {
        match *self {
            ScanTree::Dir { size, .. } => size,
            ScanTree::File { size, .. } => size,
        }
    }
}

pub fn scan_dir(entry: &Path) -> ScanTree {
    let dir = match fs::read_dir(entry) {
        Ok(dir) => dir,
        Err(err) => {
            eprint!("Error reading directory {entry:?}: {err}");
            #[cfg(feature = "full_metadata")]
            return ScanTree::File {
                size: 0,
                name: "".into(),
                access: 0,
                creation: 0,
                modification: 0,
            };
            #[cfg(not(feature = "full_metadata"))]
            return ScanTree::File {
                size: 0,
                name: "".into(),
            };
        }
    };

    let mut results: Vec<ScanTree> = dir
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
                    eprintln!("Error reading metadata: {err}");
                    return None;
                }
            };

            if metadata.is_symlink() {
                return None;
            }

            if metadata.is_file() {
                let len = metadata.len();
                return Some(
                    #[cfg(feature = "full_metadata")]
                    ScanTree::File {
                        size: len,
                        name: entry.file_name(),
                        access: metadata.st_atime(),
                        creation: metadata.st_ctime(), // change but whatever
                        modification: metadata.st_mtime(),
                    },
                    #[cfg(not(feature = "full_metadata"))]
                    ScanTree::File {
                        size: len,
                        name: entry.file_name(),
                    },
                );
            }

            Some(scan_dir(&entry.path()))
        })
        .collect();
    results.sort_unstable_by_key(|e| std::cmp::Reverse(e.get_size()));
    ScanTree::Dir {
        name: entry.components().next_back().unwrap().as_os_str().into(),
        size: results.iter().map(|e| e.get_size()).sum(),
        children: results,
    }
}
