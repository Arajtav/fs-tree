use std::ffi::OsStr;

use ab_glyph::FontArc;
use fontdb::{Database, Source};

const UNITS: [&str; 5] = ["", "Ki", "Mi", "Gi", "Ti"];

fn format_size(size: u64) -> String {
    if size == 0 {
        return "0 B".to_string();
    }

    let exp = (size as f64).log2() / 10.0;
    let unit_index = exp.floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);

    let value = size as f64 / 1024f64.powi(unit_index as i32);

    format!("{value:.2} {}B", UNITS[unit_index])
}

pub fn entry_description(name: &OsStr, is_file: bool, size: u64, count: usize) -> String {
    let name = name.to_string_lossy();
    let size = format_size(size);
    if is_file {
        format!("{name} {size}")
    } else {
        format!("{name} ({count} files) {size}")
    }
}

pub fn get_font() -> Option<FontArc> {
    let mut db = Database::new();
    db.load_system_fonts();

    let query = fontdb::Query {
        families: &[
            fontdb::Family::Name("Noto Sans Mono"),
            fontdb::Family::Name("SF Mono"),
            fontdb::Family::Name("Menlo"),
            fontdb::Family::Name("Monaco"),
            fontdb::Family::Name("Consolas"),
            fontdb::Family::Name("Lucida Console"),
            fontdb::Family::Name("Liberation Mono"),
            fontdb::Family::Name("Ubuntu Mono"),
            fontdb::Family::Monospace,
            fontdb::Family::SansSerif,
        ],
        ..Default::default()
    };

    if let Some(id) = db.query(&query) {
        if let Some((font, _)) = db.face_source(id) {
            match font {
                Source::File(path) => {
                    if let Ok(bytes) = std::fs::read(path) {
                        return FontArc::try_from_vec(bytes).ok();
                    }
                }
                Source::Binary(data) => {
                    return FontArc::try_from_vec(data.as_ref().as_ref().to_owned()).ok();
                }
                _ => {}
            }
        }
    }

    None
}
