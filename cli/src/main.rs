mod export_tree;

use clap::Parser;
use export_tree::ExportTree;
use fs_tree_shared::scan_dir;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    #[arg(default_value = ".")]
    entrypoint: PathBuf,

    /// Whether the generated json should be formatted.
    #[arg(long, short)]
    pretty: bool,

    /// Whether to go through filesystem boundaries
    #[cfg(target_family = "unix")]
    #[arg(long, short)]
    cross_fs: bool,
}

fn main() {
    let args = Args::parse();

    #[cfg(target_family = "unix")]
    let dev_id = if args.cross_fs {
        None
    } else {
        use std::os::unix::fs::MetadataExt;

        Some(match args.entrypoint.metadata() {
            Ok(m) => m.dev(),
            Err(err) => {
                eprintln!("Error getting dev_id: {err}");
                return;
            }
        })
    };

    let scanned = ExportTree::from(scan_dir(
        &args.entrypoint,
        #[cfg(target_family = "unix")]
        dev_id,
    ));

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
