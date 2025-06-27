use clap::Parser;
use fs_tree::scan_dir;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,
}

fn main() {
    let args = Args::parse();

    let scanned = scan_dir(&args.entrypoint);

    // todo: print as json
    println!("{scanned:?}");
}
