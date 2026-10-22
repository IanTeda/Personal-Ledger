//! Enforces the VIPER Dependencies rule over `cargo metadata`: a migrated feature crate never
//! depends on another feature crate, a UI toolkit or `lib_database`
//! (docs/development/viper-architecture.md, Rules > Dependencies).
//!
//! `deny.toml` already bans the toolkits and `lib_database` outside the bins, but cargo-deny can't
//! express "no feature crate depends on another", so this test carries that half. It lives here
//! because `lib_ledger` is the one crate that already sees every feature crate.
//!
//! Only crates in `MIGRATED` are checked. Today's domain crates still lean on each other, so each
//! VIPER transition slice adds its crate here once it complies.

use std::process::Command;

use serde_json::Value;

/// Crates that must keep the rule: the feature crates that have moved to the VIPER layout, and
/// `lib_viper`, the generic types they share, which no feature crate may reach another through.
const MIGRATED: &[&str] = &["lib_transactions", "lib_units", "lib_viper"];

/// Every feature crate, migrated or not: one per domain.
const FEATURE_CRATES: &[&str] = &[
    "lib_accounts",
    "lib_bills",
    "lib_budgets",
    "lib_categories",
    "lib_documents",
    "lib_institutions",
    "lib_inventory",
    "lib_payees",
    "lib_tags",
    "lib_transactions",
    "lib_units",
];

/// Crates no feature crate may depend on besides the other feature crates.
const FORBIDDEN: &[&str] = &[
    "gpui",
    "gpui-component",
    "ratatui",
    "crossterm",
    "lib_database",
];

/// Each forbidden direct dependency (normal, dev or build) of a migrated crate, as
/// `"<crate> -> <dependency>"`.
fn violations(metadata: &Value, migrated: &[&str]) -> Vec<String> {
    let packages = metadata["packages"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    packages
        .iter()
        .filter_map(|package| {
            Some((
                package["name"].as_str()?,
                package["dependencies"].as_array()?,
            ))
        })
        .filter(|(name, _)| migrated.contains(name))
        .flat_map(|(name, dependencies)| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency["name"].as_str())
                .filter(move |dependency| {
                    *dependency != name
                        && (FEATURE_CRATES.contains(dependency) || FORBIDDEN.contains(dependency))
                })
                .map(move |dependency| format!("{name} -> {dependency}"))
        })
        .collect()
}

#[test]
fn migrated_feature_crates_depend_on_no_feature_crate_toolkit_or_database() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata should run, since cargo is running this test");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata should print JSON");

    let names: Vec<&str> = metadata["packages"]
        .as_array()
        .expect("cargo metadata should list packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    for crate_name in MIGRATED {
        assert!(
            names.contains(crate_name),
            "{crate_name} is in MIGRATED but not in the workspace; was it renamed?"
        );
    }

    let found = violations(&metadata, MIGRATED);
    assert!(
        found.is_empty(),
        "VIPER Dependencies rule broken: a feature crate never depends on another feature crate, \
         a UI toolkit (gpui, gpui-component, ratatui, crossterm) or lib_database. See \
         docs/development/viper-architecture.md, Rules > Dependencies. Move shared value types to \
         lib_core, declare a read port in protocols.rs, or put a multi-domain write in a \
         cross-domain module.\nOffending dependencies:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn violations_flags_feature_toolkit_and_database_dependencies_of_migrated_crates_only() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_units",
                "dependencies": [
                    { "name": "lib_core" },
                    { "name": "lib_bills" },
                    { "name": "gpui" },
                    { "name": "lib_database" },
                ]
            },
            {
                "name": "lib_bills",
                "dependencies": [{ "name": "lib_payees" }]
            }
        ]
    });

    assert_eq!(
        violations(&metadata, &["lib_units"]),
        vec![
            "lib_units -> lib_bills",
            "lib_units -> gpui",
            "lib_units -> lib_database"
        ]
    );
}
