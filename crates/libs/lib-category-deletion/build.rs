use std::env;
use std::path::PathBuf;

use lib_locale_build::{Options, run};

fn main() -> lib_locale_build::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    // The Delete category Messages moved here from `lib-categories` with their keys, so the layer
    // keeps the `categories-` prefix: no Client composes both copies, since the ticket that hosts
    // this Dialog drops `lib-categories`' (#1005). The Presenter is the only reader, so `src` is
    // scanned for unused Messages.
    run(&Options {
        i18n_dir: manifest_dir.join("i18n"),
        out_file: out_dir.join("msg.rs"),
        runtime_path: "::lib_locale::runtime".to_string(),
        id_prefix: Some("categories-".to_string()),
        scan_dirs: vec![manifest_dir.join("src")],
    })
}
