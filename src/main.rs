use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut to_scan = vec![args.entrypoint];
    let mut scanned = vec![];
    while !to_scan.is_empty() {
        let entry = to_scan.pop().unwrap();
        let metadata = fs::metadata(&entry)?;

        if !metadata.is_dir() {
            scanned.push((entry, metadata.len()));
            continue;
        }
        let dir = fs::read_dir(entry)?;
        to_scan.append(
            &mut dir
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }

    println!("{scanned:?}");

    Ok(())
}
