use clap::Parser;
use fs_tree_shared::{SavedTree, scan_dir};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    #[arg(default_value = ".")]
    entrypoint: PathBuf,

    /// Whether the generated json should be formatted.
    #[arg(long, short)]
    pretty: bool,
}

fn main() {
    let args = Args::parse();

    let scan = match scan_dir(&args.entrypoint) {
        Ok(scan) => scan,
        Err(err) => {
            eprintln!("Error reading directory {:?}: {err}", args.entrypoint);
            return;
        }
    };

    let scanned = SavedTree::from(scan);

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
