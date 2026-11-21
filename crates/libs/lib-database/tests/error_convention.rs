//! Enforces the `error.rs` convention over `crates/libs/*` (#1300, parent #1274): every library
//! crate has a private `src/error.rs` (`mod error`, never `pub mod error`, #1303), re-exports
//! `Error` and `Result` from `lib.rs`, and has no
//! public function whose error is a `String`, so a caller matches on a typed variant, never on
//! text (`/rust-style`). It lives beside `viper_dependencies.rs` for the same reason: this is the
//! one crate that already looks across the library crates.

use std::fs;
use std::path::{Path, PathBuf};

/// The library crates' directories, `crates/libs/*`, or the reason they cannot be listed.
fn library_crates() -> Result<Vec<PathBuf>, String> {
    let libs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("lib-database sits inside crates/libs")?;
    let entries = fs::read_dir(libs).map_err(|error| format!("crates/libs unreadable: {error}"))?;
    let mut crates: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    crates.sort();
    Ok(crates)
}

fn crate_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Whether `lib_rs` re-exports both `Error` and `Result` from its `error` module, in any of the
/// spellings `pub use error::{Error, Result}`, `pub use crate::error::{Error, Result}` or a
/// multi-line brace list.
fn reexports_error(lib_rs: &str) -> bool {
    let flat: String = lib_rs.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.split(';').any(|statement| {
        let statement = statement.trim();
        let Some(rest) = statement
            .find("pub use ")
            .map(|at| &statement[at + "pub use ".len()..])
        else {
            return false;
        };
        let Some(list) = rest
            .strip_prefix("crate::error::")
            .or_else(|| rest.strip_prefix("error::"))
        else {
            return false;
        };
        let names: Vec<&str> = list
            .trim_matches(|c| c == '{' || c == '}' || c == ' ')
            .split(',')
            .map(str::trim)
            .collect();
        names.contains(&"Error") && names.contains(&"Result")
    })
}

/// Whether `lib_rs` declares `error` with any visibility wider than the crate: bare `pub`,
/// `pub(super)`, `pub(self)` or `pub(in …)`, any of which would give a path to `Error` beyond the
/// crate. Only `pub(crate)` and private `mod error` are allowed, so the rule is "private, or
/// `pub(crate)`" and nothing else is excepted.
fn declares_public_error_module(lib_rs: &str) -> bool {
    lib_rs.lines().any(|line| {
        let line = line.trim();
        let declaration = if let Some(restricted) = line.strip_prefix("pub(") {
            let Some(close) = restricted.find(')') else {
                return false;
            };
            if restricted[..close].trim() == "crate" {
                return false;
            }
            restricted[close + 1..].trim_start()
        } else if let Some(rest) = line.strip_prefix("pub ") {
            rest
        } else {
            return false;
        };
        declaration
            .strip_prefix("mod error")
            .is_some_and(|after| after.starts_with([';', ' ', '{']))
    })
}

/// Each public function signature in `source` whose return type is `Result<_, String>`.
fn string_error_signatures(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("pub ") {
        let after = &rest[at..];
        rest = &after["pub ".len()..];
        let head = after.trim_start_matches("pub ");
        if !(head.starts_with("fn ")
            || head.starts_with("async fn ")
            || head.starts_with("const fn ")
            || head.starts_with("unsafe fn "))
        {
            continue;
        }
        let end = after.find(['{', ';']).unwrap_or(after.len());
        let signature: String = after[..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let Some(arrow) = signature.find("->") else {
            continue;
        };
        let returns = signature[arrow..].replace(' ', "");
        if returns.contains("Result<") && returns.contains(",String>") {
            found.push(signature);
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Everything the convention flags in `dir`, as `"<crate>: <what>"`.
fn violations_in(dir: &Path) -> Vec<String> {
    let name = crate_name(dir);
    let mut found = Vec::new();
    if !dir.join("src/error.rs").is_file() {
        found.push(format!("{name}: has no src/error.rs"));
    }
    let lib_rs = fs::read_to_string(dir.join("src/lib.rs")).unwrap_or_default();
    if !reexports_error(&lib_rs) {
        found.push(format!(
            "{name}: lib.rs lacks `pub use error::{{Error, Result}}`"
        ));
    }
    if declares_public_error_module(&lib_rs) {
        found.push(format!("{name}: lib.rs declares error as a public module"));
    }
    let mut files = Vec::new();
    rust_files(&dir.join("src"), &mut files);
    files.sort();
    for file in files {
        // Generated code is build output, never hand-edited.
        if file
            .components()
            .any(|part| part.as_os_str() == "generated")
        {
            continue;
        }
        let source = fs::read_to_string(&file).unwrap_or_default();
        for signature in string_error_signatures(&source) {
            found.push(format!(
                "{name}: {} returns a String error: {signature}",
                file.strip_prefix(dir).unwrap_or(&file).display()
            ));
        }
    }
    found
}

#[test]
fn every_library_crate_follows_the_error_rs_convention() {
    let crates = library_crates().expect("crates/libs/* should be readable");
    assert!(
        crates.len() > 20,
        "crates/libs/* should list the library crates"
    );
    let found: Vec<String> = crates.iter().flat_map(|dir| violations_in(dir)).collect();
    assert!(
        found.is_empty(),
        "error.rs convention broken: every library crate has a private src/error.rs, re-exports \
         `Error` and `Result` from lib.rs and returns no `Result<_, String>` from a public \
         function. See /rust-style.\nOffending crates:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_convention_flags_a_missing_error_file_re_export_and_string_error() {
    let dir = std::env::temp_dir().join(format!("error_convention_{}", std::process::id()));
    let src = dir.join("src");
    fs::create_dir_all(&src).expect("temp crate dir");
    fs::write(dir.join("Cargo.toml"), "").expect("manifest");
    fs::write(
        src.join("lib.rs"),
        "pub fn f() -> Result<u8, String> { Ok(1) }\n",
    )
    .expect("lib");
    let found = violations_in(&dir);
    fs::remove_dir_all(&dir).ok();
    assert_eq!(found.len(), 3, "{found:?}");

    assert!(reexports_error(
        "mod error;\npub use error::{Error, Result};"
    ));
    assert!(reexports_error(
        "pub use crate::error::{\n    Error,\n    Result,\n};"
    ));
    assert!(!reexports_error("pub use error::Error;"));
}

#[test]
fn the_convention_flags_a_public_error_module() {
    let dir = std::env::temp_dir().join(format!("error_convention_public_{}", std::process::id()));
    let src = dir.join("src");
    fs::create_dir_all(&src).expect("temp crate dir");
    fs::write(dir.join("Cargo.toml"), "").expect("manifest");
    fs::write(src.join("error.rs"), "").expect("error module");
    fs::write(
        src.join("lib.rs"),
        "pub mod error;\npub use error::{Error, Result};\n",
    )
    .expect("lib");
    let found = violations_in(&dir);
    fs::remove_dir_all(&dir).ok();
    assert_eq!(
        found,
        vec![format!(
            "{}: lib.rs declares error as a public module",
            crate_name(&dir)
        )]
    );

    assert!(declares_public_error_module("pub mod error;"));
    assert!(declares_public_error_module("    pub mod error {}"));
    assert!(declares_public_error_module("pub(super) mod error;"));
    assert!(declares_public_error_module(
        "pub(in crate::foo) mod error;"
    ));
    assert!(declares_public_error_module("pub(self) mod error;"));
    assert!(!declares_public_error_module("mod error;"));
    assert!(!declares_public_error_module("pub(crate) mod error;"));
}
