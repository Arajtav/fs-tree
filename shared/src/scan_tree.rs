use rayon::prelude::*;
use std::{
    ffi::OsString,
    fs::{self},
    path::Path,
};

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;

#[cfg(feature = "metadata_timestamps")]
#[derive(Default)]
pub struct MetaTimestamps {
    pub access: i64,
    pub creation: i64,
    pub modification: i64,
}

#[cfg(feature = "metadata_ownership")]
#[derive(Default)]
pub struct MetaOwnership {
    pub uid: u32,
    pub gid: u32,
}

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
        timestamps: MetaTimestamps,
        #[cfg(feature = "metadata_ownership")]
        ownership: MetaOwnership,
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

pub fn scan_dir(entry: &Path, #[cfg(target_family = "unix")] dev_id: Option<u64>) -> ScanTree {
    let dir = match fs::read_dir(entry) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error reading directory {entry:?}: {err}");
            return ScanTree::File {
                size: 0,
                name: "".into(),
                #[cfg(feature = "metadata_timestamps")]
                timestamps: MetaTimestamps::default(),
                #[cfg(feature = "metadata_ownership")]
                ownership: MetaOwnership::default(),
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

            #[cfg(target_family = "unix")]
            if let Some(id) = dev_id {
                if id != metadata.dev() {
                    return None;
                }
            }

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
                let timestamps = {
                    #[cfg(target_family = "unix")]
                    {
                        MetaTimestamps {
                            access: metadata.atime(),
                            creation: metadata.ctime(),
                            modification: metadata.mtime(),
                        }
                    }

                    #[cfg(target_family = "windows")]
                    {
                        fn to_unix_time(filetime: u64) -> i64 {
                            const EPOCH_DIFFERENCE: i64 = 11644473600;
                            (filetime as i64 / 10000000).saturating_sub(EPOCH_DIFFERENCE)
                        }

                        MetaTimestamps {
                            access: to_unix_time(metadata.last_access_time()),
                            creation: to_unix_time(metadata.creation_time()),
                            modification: to_unix_time(metadata.last_write_time()),
                        }
                    }
                };

                #[cfg(feature = "metadata_ownership")]
                let ownership = {
                    #[cfg(target_family = "unix")]
                    {
                        MetaOwnership {
                            uid: metadata.uid(),
                            gid: metadata.gid(),
                        }
                    }

                    #[cfg(not(target_family = "unix"))]
                    {
                        compile_error!(
                            "The `metadata_ownership` feature is only available on Unix like systems"
                        );

                        MetaOwnership {
                            uid: 0,
                            gid: 0,
                        }
                    }
                };

                return Some(ScanTree::File {
                    size: len,
                    name: entry.file_name(),
                    #[cfg(feature = "metadata_timestamps")]
                    timestamps,
                    #[cfg(feature = "metadata_ownership")]
                    ownership
                });
            }

            let child = scan_dir(
                &entry.path(),
                #[cfg(target_family = "unix")]
                dev_id,
            );

            #[cfg(feature = "ignore_zero")]
            if child.get_size() == 0 {
                return None;
            }

            Some(child)
        })
        .collect();

    #[cfg(feature = "sort")]
    results.sort_unstable_by_key(|e| std::cmp::Reverse(e.get_size()));

    ScanTree::Dir {
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
    }
}
