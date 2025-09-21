# fs-tree

[![License](https://img.shields.io/github/license/arajtav/fs-tree)](https://raw.githubusercontent.com/Arajtav/fs-tree/meow/LICENSE)

Simple app to visualize your disk usage.

## Installation

1. Clone the repository `git clone https://github.com/Arajtav/fs-tree.git`.
2. Go into the cloned repository and build with `cargo build --release`.
3. The built binaries will be in `./target/release/fs_tree_{app,cli}` (with `.exe` on windows).

## CLI usage

CLI version is a simple program that scans your filesystem from given directory, and prints the result as json.

Usage: `fs_tree_cli [ENTRYPOINT]`. The entry point defaults to `.`. You can use the CLI with `-p` to format generated json. For more flags see `fs_tree_cli --help`.

## App usage

App works the same as CLI, except instead of outputting the scan results to stdout, it renders them as a treemap. As you hover over the files and directories the information about them will show up.

### Colors

After the entry point you can specify how should the files be colored.
The options are:
- `extension` (default) - colors the files based on file type, images are green, audio cyan and so on. Note that since every file is processed separately, and the only information available is the extension, the color may be assigned incorrectly.
- `access` - colors are based on when was the last time the file was opened. Brighter colors are files opened more recently, darker colors are older.
- `modification` - same as `access` but for modification time.
- `creation` - same as `access` but for creation time. On unix like systems it actually is `change` time, not creation (updates when permissions change, etc.).
- `user` (unix only) - colors are based on file uid. Current user is white, root is black, other users are bright colors, and system users (uid below 1000) are darker colors.
- `group` (unix only) - same as `user` but for gid.

### Navigation

Currently the navigation is mouse only, right click on the directory to go into it, use back and forward buttons to go back and forward in history, and use middle mouse button to open the file or directory in system chosen app.

### Performance

### Scan
On system, the scan of `~` (almost 1 million files, Btrfs, Linux) takes below a second. On my Windows install (NTFS, Windows 10) the scan takes significantly longer.

### Rendering

The app despite rendering few millions rectangles, due to instancing and no reallocations, renders in realtime (by that I mean reacting to cursor movement). The slowest part so far is recomputing the treemap when the window changes in size, or when a subdirectory is entered/left.
