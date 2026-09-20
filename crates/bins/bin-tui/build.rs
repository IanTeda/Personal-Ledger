use std::env;
use std::path::PathBuf;

use lib_locale_build::{Options, run};

fn main() -> lib_locale_build::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    run(&Options {
        i18n_dir: manifest_dir.join("i18n"),
        out_file: out_dir.join("msg.rs"),
        runtime_path: "::lib_locale::runtime".to_string(),
        id_prefix: Some("tui-".to_string()),
        scan_dirs: vec![manifest_dir.join("src")],
    })
}
