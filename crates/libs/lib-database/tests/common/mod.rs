//! What the workspace-wide convention tests share: one read of `cargo metadata`, which gives each
//! package's targets and dependencies as Cargo resolves them, renames included.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test crate: a failed setup should fail the test"
)]

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// `cargo metadata` for the workspace or package whose manifest sits in `dir`, its own packages
/// only and offline, so it reads manifests and never the network.
pub fn cargo_metadata(dir: &Path) -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(dir)
        .output()
        .expect("cargo metadata should run, since cargo is running this test");
    assert!(
        output.status.success(),
        "cargo metadata failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should print JSON")
}
