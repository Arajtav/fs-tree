use rayon::prelude::*;
use std::{
    ffi::OsString,
    fs::{self},
    path::Path,
};

#[cfg(any(feature = "metadata_timestamps", feature = "metadata_ownership"))]
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
        #[cfg(feature = "metadata_timestamps")]
        access: i64,
        #[cfg(feature = "metadata_timestamps")]
        creation: i64,
        #[cfg(feature = "metadata_timestamps")]
        modification: i64,
        #[cfg(feature = "metadata_ownership")]
        uid: u32,
        #[cfg(feature = "metadata_ownership")]
        gid: u32,
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
            return ScanTree::File {
                size: 0,
                name: "".into(),
                #[cfg(feature = "metadata_timestamps")]
                access: 0,
                #[cfg(feature = "metadata_timestamps")]
                creation: 0,
                #[cfg(feature = "metadata_timestamps")]
                modification: 0,
                #[cfg(feature = "metadata_ownership")]
                uid: 0,
                #[cfg(feature = "metadata_ownership")]
                gid: 0,
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

                #[cfg(feature = "ignore_zero")]
                if len == 0 {
                    return None;
                }

                return Some(ScanTree::File {
                    size: len,
                    name: entry.file_name(),
                    #[cfg(feature = "metadata_timestamps")]
                    access: metadata.st_atime(),
                    #[cfg(feature = "metadata_timestamps")]
                    creation: metadata.st_ctime(), // change but whatever
                    #[cfg(feature = "metadata_timestamps")]
                    modification: metadata.st_mtime(),
                    #[cfg(feature = "metadata_ownership")]
                    uid: metadata.st_uid(),
                    #[cfg(feature = "metadata_ownership")]
                    gid: metadata.st_gid(),
                });
            }

            let child = scan_dir(&entry.path());

            #[cfg(feature = "ignore_zero")]
            if child.get_size() == 0 {
                return None;
            }

            Some(child)
        })
        .collect();
    results.sort_unstable_by_key(|e| std::cmp::Reverse(e.get_size()));
    ScanTree::Dir {
        name: entry.components().next_back().unwrap().as_os_str().into(),
        size: results.iter().map(|e| e.get_size()).sum(),
        children: results,
    }
}
