use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
struct Language {
    color: Option<String>,
    extensions: Option<Vec<String>>,
}

fn hex_to_rgb(hex: &str) -> [f32; 3] {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

fn main() {
    let yaml = fs::read_to_string("languages.yml").expect("languages.yml not found.");
    let languages: HashMap<String, Language> =
        serde_yaml::from_str(&yaml).expect("failed to parse languages.yml");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("extensions_map.rs");
    let mut file = fs::File::create(&dest_path).unwrap();

    let mut emmited = HashSet::new();

    let mut out = "static MAP: phf::Map<&'static [u8], [f32; 3]> = phf::phf_map! {\n".to_owned();

    for lang in languages.values() {
        let color = match &lang.color {
            Some(color) => color,
            None => continue,
        };

        let extensions = match &lang.extensions {
            Some(extensions) => extensions,
            None => continue,
        };

        let rgb = hex_to_rgb(color);

        for ext in extensions {
            let ext = &ext[1..];
            if !emmited.insert(ext) {
                continue;
            };

            out += &format!(
                "    b\"{}\" => [{:.3}, {:.3}, {:.3}],\n",
                ext, rgb[0], rgb[1], rgb[2]
            );
        }
    }

    out += "};\n";
    file.write(&out.as_bytes()).unwrap();
    println!("{out}");

    println!("cargo:rerun-if-changed=languages.yml");
}
