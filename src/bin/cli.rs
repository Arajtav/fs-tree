use clap::Parser;
use fs_tree::{export_tree::ExportTree, scan_tree::scan_dir};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,

    /// Whether the generated json should be formatted.
    #[arg(long, short)]
    pretty: bool,
}

fn main() {
    let args = Args::parse();

    let scanned = ExportTree::from_scan_tree(
        scan_dir(&args.entrypoint),
        args.entrypoint.to_string_lossy().into(),
    );

    let result = if args.pretty {
        serde_json::to_string_pretty(&scanned)
    } else {
        serde_json::to_string(&scanned)
    };

    match result {
        Ok(json) => println!("{json}"),
        Err(err) => eprintln!("Error serializing to JSON: {err}"),
    }
}
