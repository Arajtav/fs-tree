mod tree;

use clap::Parser;
use std::{fs, path::PathBuf};

use crate::tree::Tree;

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut to_scan = vec![args.entrypoint];
    let mut scanned = Tree::new();
    while !to_scan.is_empty() {
        let entry = to_scan.pop().unwrap();
        let metadata = match fs::metadata(&entry) {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!("Error reading metadata for {:?}: {}", entry, err);
                continue;
            }
        };

        if metadata.is_symlink() {
            continue;
        }

        if metadata.is_file() {
            scanned.insert(&entry.canonicalize().unwrap(), metadata.len());
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        let dir = match fs::read_dir(entry.clone()) {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("Error reading directory {:?}: {}", entry, err);
                continue;
            }
        };
        to_scan.append(
            &mut dir
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }

    println!("{scanned:?}");

    Ok(())
}
