use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

include!(concat!(env!("OUT_DIR"), "/extensions_map.rs"));

pub fn get_color_from_extension(extension: &OsStr) -> [f32; 3] {
    match MAP.get(extension.as_bytes()) {
        Some(v) => *v,
        None => [0.25, 0.25, 0.25],
    }
}
