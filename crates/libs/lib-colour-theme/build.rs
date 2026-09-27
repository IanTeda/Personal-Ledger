//! Compiles each `themes/<id>.ini` into a built-in Colour Theme `const`, so a missing role, a
//! missing Colour Variant or a bad hex value fails the build rather than a test.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::{env, fs};

#[path = "src/parse.rs"]
mod parse;

/// The default Colour Theme, generated first so it sits at index 0.
const DEFAULT_ID: &str = "modernist";

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let themes_dir = manifest_dir.join("themes");
    println!("cargo::rerun-if-changed={}", themes_dir.display());
    println!("cargo::rerun-if-changed=src/parse.rs");

    let mut files = Vec::new();
    for entry in fs::read_dir(&themes_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "ini") {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("non-UTF-8 theme file name: {}", path.display()))?
                .to_string();
            if !id.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                return Err(format!("theme id `{id}` must be lowercase snake_case").into());
            }
            files.push((id, path));
        }
    }
    files.sort_by(|a, b| (a.0 != DEFAULT_ID, &a.0).cmp(&(b.0 != DEFAULT_ID, &b.0)));
    if files.first().is_none_or(|(id, _)| id != DEFAULT_ID) {
        return Err(format!("themes/{DEFAULT_ID}.ini is missing").into());
    }

    let mut out = String::new();
    writeln!(
        out,
        "pub(crate) const BUILT_IN: [ColourTheme; {}] = [",
        files.len()
    )?;
    for (id, path) in &files {
        let source = fs::read_to_string(path)?;
        let theme = parse::parse_theme(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        writeln!(out, "    ColourTheme {{")?;
        writeln!(out, "        id: {id:?},")?;
        writeln!(out, "        light: Palette::from_rgb({:?}),", theme.light)?;
        writeln!(out, "        dark: Palette::from_rgb({:?}),", theme.dark)?;
        writeln!(out, "    }},")?;
    }
    writeln!(out, "];")?;
    fs::write(out_dir.join("built_in.rs"), out)?;
    Ok(())
}
