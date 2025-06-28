use clap::Parser;
use fs_tree::scan_dir;
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

    let scanned = scan_dir(&args.entrypoint);

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
