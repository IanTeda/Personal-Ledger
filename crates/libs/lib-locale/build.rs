use std::env;
use std::path::PathBuf;

use lib_locale_build::{Options, run};

fn main() -> lib_locale_build::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    // The shared layer's Messages are called from the bins, so no unused-Message scan here.
    run(&Options {
        i18n_dir: manifest_dir.join("i18n"),
        out_file: out_dir.join("msg.rs"),
        runtime_path: "crate::runtime".to_string(),
        id_prefix: None,
        scan_dirs: Vec::new(),
    })
}
