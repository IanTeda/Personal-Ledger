//! What the workspace-wide convention tests share: one read of `cargo metadata`, which gives each
//! package's targets and dependencies as Cargo resolves them, renames included.

#![allow(
    dead_code,
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

/// The shared crates every module may depend on, which no feature crate may reach another through:
/// `lib_viper`, the generic types they share, and `lib_toast`, the Notices they send the Shell.
/// `lib_shell`, the Shell module, keeps the rule too.
pub const SHARED_CRATES: &[&str] = &["lib_shell", "lib_toast", "lib_viper"];

/// Every feature crate: one per domain, the Dashboard, which reads others through its ports, and
/// Settings, whose pages reach Configuration and the Ledger through theirs.
pub const FEATURE_CRATES: &[&str] = &[
    "lib_accounts",
    "lib_bills",
    "lib_budgets",
    "lib_categories",
    "lib_dashboard",
    "lib_documents",
    "lib_institutions",
    "lib_inventory",
    "lib_payees",
    "lib_settings",
    "lib_tags",
    "lib_transactions",
    "lib_units",
];

/// Each cross-domain module and the feature crates it writes, the only feature crates it may
/// depend on. No feature crate may depend on a cross-domain module.
pub const CROSS_DOMAIN: &[(&str, &[&str])] = &[
    ("lib_bill_payments", &["lib_bills", "lib_transactions"]),
    (
        "lib_account_deletion",
        &["lib_accounts", "lib_transactions", "lib_bills"],
    ),
    ("lib_unit_deletion", &["lib_units", "lib_institutions"]),
    ("lib_tag_removal", &["lib_tags", "lib_transactions"]),
    (
        "lib_category_deletion",
        &[
            "lib_categories",
            "lib_transactions",
            "lib_bills",
            "lib_payees",
            "lib_budgets",
        ],
    ),
    (
        "lib_transaction_import",
        &["lib_payees", "lib_transactions"],
    ),
    (
        "lib_ledger_creation",
        &[
            "lib_units",
            "lib_institutions",
            "lib_budgets",
            "lib_documents",
        ],
    ),
];
