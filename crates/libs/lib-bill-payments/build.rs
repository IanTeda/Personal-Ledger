use std::env;
use std::path::PathBuf;

use lib_locale_build::{Options, run};

fn main() -> lib_locale_build::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    // A feature crate's layer is composed like a bin's, so it carries its own id prefix; the
    // Presenters are the only readers, so `src` is scanned for unused Messages.
    run(&Options {
        i18n_dir: manifest_dir.join("i18n"),
        out_file: out_dir.join("msg.rs"),
        runtime_path: "::lib_locale::runtime".to_string(),
        id_prefix: Some("bill-payments-".to_string()),
        scan_dirs: vec![manifest_dir.join("src")],
    })
}
