use std::env;
use std::path::PathBuf;

use lib_locale_build::{Options, run};

fn main() -> lib_locale_build::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    // The Remove tag and Merge tags Messages moved here from `lib-tags` with their keys, so the
    // layer keeps the `tags-` prefix: no Client composes both copies, since the ticket that hosts
    // these Dialogs drops `lib-tags`' (#1003). The Presenters are the only readers, so `src` is
    // scanned for unused Messages.
    run(&Options {
        i18n_dir: manifest_dir.join("i18n"),
        out_file: out_dir.join("msg.rs"),
        runtime_path: "::lib_locale::runtime".to_string(),
        id_prefix: Some("tags-".to_string()),
        scan_dirs: vec![manifest_dir.join("src")],
    })
}
