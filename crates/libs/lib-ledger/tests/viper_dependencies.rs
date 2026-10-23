//! Enforces the VIPER Dependencies rule over `cargo metadata`: a migrated feature crate never
//! depends on another feature crate, a UI toolkit or `lib_database`, and a cross-domain module
//! depends on the feature crates it writes and nothing else does on it, and no module depends on
//! `lib_shell`, which hosts them (docs/development/viper-architecture.md, Rules > Dependencies).
//!
//! `deny.toml` already bans the toolkits and `lib_database` outside the bins, and each
//! cross-domain module outside the bins and its adapter, but cargo-deny can't express "no feature
//! crate depends on another", so this test carries that half. It lives here because `lib_ledger`
//! is the one crate that already sees every feature crate.
//!
//! Only crates in `MIGRATED` are checked for feature crates. Today's domain crates still lean on
//! each other, so each VIPER transition slice adds its crate here once it complies. Every feature
//! crate, migrated or not, is checked for a cross-domain module and for `lib_shell`.

use std::process::Command;

use serde_json::Value;

/// Crates that must keep the rule: the feature crates that have moved to the VIPER layout, and the
/// shared crates every module may depend on, which no feature crate may reach another through:
/// `lib_viper`, the generic types they share, and `lib_toast`, the Notices they send the Shell.
/// `lib_shell`, the Shell module, keeps the rule too.
const MIGRATED: &[&str] = &[
    "lib_bills",
    "lib_shell",
    "lib_toast",
    "lib_transactions",
    "lib_units",
    "lib_viper",
];

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

/// Each cross-domain module and the feature crates it writes, the only feature crates it may
/// depend on. No feature crate may depend on a cross-domain module.
const CROSS_DOMAIN: &[(&str, &[&str])] =
    &[("lib_bill_payments", &["lib_bills", "lib_transactions"])];

/// The Shell module, which hosts every other module: only the bins may depend on it, so no
/// module, cross-domain module or shared crate may.
const SHELL: &str = "lib_shell";

/// Crates no feature crate may depend on besides the other feature crates.
const FORBIDDEN: &[&str] = &[
    "gpui",
    "gpui-component",
    "ratatui",
    "crossterm",
    "lib_database",
];

/// Whether `dependency` is one `name` may not have: a feature crate (unless `name` is a
/// cross-domain module that writes it), a toolkit, `lib_database`, a cross-domain module, or the
/// Shell.
fn forbidden(name: &str, dependency: &str, migrated: &[&str]) -> bool {
    if dependency == name {
        return false;
    }
    if dependency == SHELL {
        return FEATURE_CRATES.contains(&name)
            || CROSS_DOMAIN.iter().any(|(m, _)| *m == name)
            || migrated.contains(&name);
    }
    if CROSS_DOMAIN.iter().any(|(module, _)| *module == dependency) {
        return FEATURE_CRATES.contains(&name) || CROSS_DOMAIN.iter().any(|(m, _)| *m == name);
    }
    if let Some((_, writes)) = CROSS_DOMAIN.iter().find(|(module, _)| *module == name) {
        return (FEATURE_CRATES.contains(&dependency) && !writes.contains(&dependency))
            || FORBIDDEN.contains(&dependency);
    }
    migrated.contains(&name)
        && (FEATURE_CRATES.contains(&dependency) || FORBIDDEN.contains(&dependency))
}

/// Each forbidden direct dependency (normal, dev or build) of a migrated crate, a cross-domain
/// module or any feature crate, as `"<crate> -> <dependency>"`.
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
        .flat_map(|(name, dependencies)| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency["name"].as_str())
                .filter(move |dependency| forbidden(name, dependency, migrated))
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
    for crate_name in MIGRATED
        .iter()
        .chain(CROSS_DOMAIN.iter().map(|(module, _)| module))
    {
        assert!(
            names.contains(crate_name),
            "{crate_name} is checked here but not in the workspace; was it renamed?"
        );
    }

    let found = violations(&metadata, MIGRATED);
    assert!(
        found.is_empty(),
        "VIPER Dependencies rule broken: a feature crate never depends on another feature crate, \
         a UI toolkit (gpui, gpui-component, ratatui, crossterm), lib_database or a cross-domain \
         module, and a cross-domain module depends only on the feature crates it writes. See \
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

#[test]
fn violations_flags_a_feature_crate_toolkit_or_database_reaching_lib_toast() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_toast",
                "dependencies": [
                    { "name": "lib_units" },
                    { "name": "gpui" },
                    { "name": "ratatui" },
                    { "name": "lib_database" },
                ]
            }
        ]
    });

    assert_eq!(
        violations(&metadata, MIGRATED),
        vec![
            "lib_toast -> lib_units",
            "lib_toast -> gpui",
            "lib_toast -> ratatui",
            "lib_toast -> lib_database"
        ]
    );
}

#[test]
fn violations_flags_a_feature_crate_on_a_cross_domain_module_and_what_the_module_doesnt_write() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_bills",
                "dependencies": [{ "name": "lib_payees" }, { "name": "lib_bill_payments" }]
            },
            {
                "name": "lib_bill_payments",
                "dependencies": [
                    { "name": "lib_bills" },
                    { "name": "lib_transactions" },
                    { "name": "lib_core" },
                    { "name": "lib_units" },
                    { "name": "lib_database" },
                ]
            },
            {
                "name": "bin_desktop",
                "dependencies": [{ "name": "lib_bill_payments" }]
            }
        ]
    });

    assert_eq!(
        violations(&metadata, &[]),
        vec![
            "lib_bills -> lib_bill_payments",
            "lib_bill_payments -> lib_units",
            "lib_bill_payments -> lib_database"
        ]
    );
}

#[test]
fn violations_flags_any_module_depending_on_lib_shell_and_lib_shell_reaching_a_module() {
    let metadata = serde_json::json!({
        "packages": [
            { "name": "lib_payees", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_units", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_bill_payments", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_toast", "dependencies": [{ "name": "lib_shell" }] },
            {
                "name": "lib_shell",
                "dependencies": [
                    { "name": "lib_core" },
                    { "name": "lib_toast" },
                    { "name": "lib_bills" },
                    { "name": "gpui" },
                    { "name": "lib_database" },
                ]
            },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_shell" }] }
        ]
    });

    assert_eq!(
        violations(&metadata, MIGRATED),
        vec![
            "lib_payees -> lib_shell",
            "lib_units -> lib_shell",
            "lib_bill_payments -> lib_shell",
            "lib_toast -> lib_shell",
            "lib_shell -> lib_bills",
            "lib_shell -> gpui",
            "lib_shell -> lib_database"
        ]
    );
}
