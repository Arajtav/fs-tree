use rayon::prelude::*;
use std::{
    ffi::OsString,
    fs::{self},
    path::Path,
};

#[cfg(feature = "metadata_ownership")]
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

#[cfg(any(feature = "metadata_timestamps", feature = "metadata_ownership"))]
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;

pub enum ScanTree {
    Dir {
        name: OsString,
        size: u64,
        children: Vec<ScanTree>,
        #[cfg(feature = "count_files")]
        files: usize,
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

pub fn scan_dir(entry: &Path) -> std::io::Result<ScanTree> {
    let dir = fs::read_dir(entry)?;

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

                #[cfg(feature = "metadata_timestamps")]
                #[cfg(target_family = "unix")]
                let (access, creation, modification) =
                    (metadata.atime(), metadata.ctime(), metadata.mtime());

                #[cfg(feature = "metadata_timestamps")]
                #[cfg(target_family = "windows")]
                let (access, creation, modification) = {
                    fn to_unix_time(filetime: u64) -> i64 {
                        const EPOCH_DIFFERENCE: i64 = 11644473600;
                        (filetime as i64 / 10000000).saturating_sub(EPOCH_DIFFERENCE)
                    }
                    (
                        to_unix_time(metadata.last_access_time()),
                        to_unix_time(metadata.creation_time()),
                        to_unix_time(metadata.last_write_time()),
                    )
                };

                #[cfg(all(feature = "metadata_ownership", not(target_family = "unix")))]
                compile_error!(
                    "The `metadata_ownership` feature is only available on Unix like systems"
                );
                #[cfg(all(feature = "metadata_ownership", not(target_family = "unix")))]
                let (uid, gid) = (0, 0);

                #[cfg(all(feature = "metadata_ownership", target_family = "unix"))]
                let (uid, gid) = (metadata.uid(), metadata.gid());

                return Some(ScanTree::File {
                    size: len,
                    name: entry.file_name(),
                    #[cfg(feature = "metadata_timestamps")]
                    access,
                    #[cfg(feature = "metadata_timestamps")]
                    creation,
                    #[cfg(feature = "metadata_timestamps")]
                    modification,
                    #[cfg(feature = "metadata_ownership")]
                    uid,
                    #[cfg(feature = "metadata_ownership")]
                    gid,
                });
            }

            let child = scan_dir(&entry.path()).ok();

            #[cfg(feature = "ignore_zero")]
            if let Some(child) = &child
                && child.get_size() == 0
            {
                return None;
            }

            child
        })
        .collect();
    results.sort_unstable_by_key(|e| std::cmp::Reverse(e.get_size()));
    Ok(ScanTree::Dir {
        name: entry.components().next_back().unwrap().as_os_str().into(),
        size: results.iter().map(|e| e.get_size()).sum(),
        #[cfg(feature = "count_files")]
        files: results
            .iter()
            .map(|e| match e {
                ScanTree::Dir { files, .. } => *files,
                ScanTree::File { .. } => 1usize,
            })
            .sum::<usize>(),
        children: results,
    })
}
