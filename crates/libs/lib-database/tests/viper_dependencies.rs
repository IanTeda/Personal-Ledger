//! Enforces the VIPER Dependencies rule over `cargo metadata`: a feature crate never
//! depends on another feature crate, a UI toolkit, `lib_config`, `lib_tracing` or `lib_database`, and a cross-domain module
//! depends on the feature crates it writes and nothing else does on it, and no crate but the bins
//! and `lib_database` depends on `lib_shell`, which hosts the modules, and no feature or shared
//! crate depends on an adapter crate (docs/development/viper-architecture.md, Rules > Dependencies;
//! ADR-0062).
//!
//! `deny.toml` already bans the toolkits and `lib_database` outside the bins, and each
//! cross-domain module outside the bins and its adapter, but cargo-deny can't express "no feature
//! crate depends on another", so this test carries that half. It lives here because
//! `lib_database` is the one crate that already sees every feature crate (moved from `lib-ledger`
//! with that crate, #1056).
//!
//! Every crate in `FEATURE_CRATES` and `SHARED_CRATES` is checked, so a new feature crate is added
//! to `FEATURE_CRATES` and keeps the rule from its first commit.

mod common;

use common::{CROSS_DOMAIN, FEATURE_CRATES, SHARED_CRATES};

use serde_json::Value;

/// The Shell module, which hosts every other module: only the bins and `lib_database`, which
/// answers its rail-count port, may depend on it (as in `deny.toml`), so no feature crate,
/// cross-domain module, shared crate or any other crate may.
const SHELL: &str = "lib_shell";

/// The only crates that may depend on the Shell: the two Clients and `lib_database`, whose
/// `StoredShellReads` answers its rail-count port over the stores (#1026), as `deny.toml`'s
/// `lib_shell` wrappers list.
const SHELL_DEPENDANTS: &[&str] = &["bin_desktop", "bin_tui", "lib_database"];

/// The Settings module, which only the two Clients and `lib_database`, whose `StoredSettingsReads`
/// and `LedgerPreferencesAdapter` answer its Ledger ports, may depend on (as in `deny.toml`). The
/// TUI reads Settings › Ledger through those ports while it keeps its own pages (#1359).
const SETTINGS: &str = "lib_settings";

/// The only crates that may depend on [`SETTINGS`], as `deny.toml`'s `lib_settings` wrappers list.
const SETTINGS_DEPENDANTS: &[&str] = &["bin_desktop", "bin_tui", "lib_database"];

/// The adapter crates, which do a feature crate's I/O behind its ports (ADR-0062): no feature
/// crate, shared crate or cross-domain module may depend on one.
const ADAPTER_CRATES: &[&str] = &["lib_database", "lib_price_feed"];

/// The price feed adapter, which answers `lib_units`' transport port and so may depend on that
/// feature crate and no other.
const PRICE_FEED: &str = "lib_price_feed";

/// The only feature crates [`PRICE_FEED`] may depend on.
const PRICE_FEED_FEATURES: &[&str] = &["lib_units"];

/// Crates no feature crate may depend on besides the other feature crates: the toolkits, and
/// `lib_config` and `lib_tracing`, which only the bins and `lib_database` reach (a feature crate
/// takes `KeyBindingConfig` from `lib_core`, #1456).
const FORBIDDEN: &[&str] = &[
    "gpui",
    "gpui-component",
    "ratatui",
    "crossterm",
    "lib_config",
    "lib_tracing",
];

/// Whether `dependency` is one `name` may not have: a feature crate (unless `name` is a
/// cross-domain module that writes it), a toolkit, an adapter crate, a cross-domain module, or the
/// Shell (for any crate but [`SHELL_DEPENDANTS`]). The price feed may depend on no feature crate
/// but `lib_units`.
fn forbidden(name: &str, dependency: &str) -> bool {
    if dependency == name {
        return false;
    }
    if name == PRICE_FEED && FEATURE_CRATES.contains(&dependency) {
        return !PRICE_FEED_FEATURES.contains(&dependency);
    }
    if ADAPTER_CRATES.contains(&dependency) {
        return FEATURE_CRATES.contains(&name)
            || SHARED_CRATES.contains(&name)
            || CROSS_DOMAIN.iter().any(|(module, _)| *module == name);
    }
    if dependency == SHELL {
        return !SHELL_DEPENDANTS.contains(&name);
    }
    if dependency == SETTINGS && !SETTINGS_DEPENDANTS.contains(&name) {
        return true;
    }
    if CROSS_DOMAIN.iter().any(|(module, _)| *module == dependency) {
        return FEATURE_CRATES.contains(&name) || CROSS_DOMAIN.iter().any(|(m, _)| *m == name);
    }
    if let Some((_, writes)) = CROSS_DOMAIN.iter().find(|(module, _)| *module == name) {
        return (FEATURE_CRATES.contains(&dependency) && !writes.contains(&dependency))
            || FORBIDDEN.contains(&dependency);
    }
    (FEATURE_CRATES.contains(&name) || SHARED_CRATES.contains(&name))
        && (FEATURE_CRATES.contains(&dependency) || FORBIDDEN.contains(&dependency))
}

/// Each forbidden direct dependency (normal, dev or build) of a feature crate, a shared crate or a
/// cross-domain module, and any crate's on the Shell, as `"<crate> -> <dependency>"`.
fn violations(metadata: &Value) -> Vec<String> {
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
                .filter(move |dependency| forbidden(name, dependency))
                .map(move |dependency| format!("{name} -> {dependency}"))
        })
        .collect()
}

#[test]
fn feature_crates_depend_on_no_feature_crate_toolkit_or_database() {
    let metadata = common::cargo_metadata(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));

    let names: Vec<&str> = metadata["packages"]
        .as_array()
        .expect("cargo metadata should list packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    for crate_name in FEATURE_CRATES
        .iter()
        .chain(SHARED_CRATES)
        .chain(CROSS_DOMAIN.iter().map(|(module, _)| module))
        .chain(SHELL_DEPENDANTS)
        .chain(SETTINGS_DEPENDANTS)
        .chain(ADAPTER_CRATES)
    {
        assert!(
            names.contains(crate_name),
            "{crate_name} is checked here but not in the workspace; was it renamed?"
        );
    }

    let found = violations(&metadata);
    assert!(
        found.is_empty(),
        "VIPER Dependencies rule broken: a feature crate never depends on another feature crate, \
         a UI toolkit (gpui, gpui-component, ratatui, crossterm), an adapter crate (lib_database, \
         lib_price_feed) or a cross-domain module, lib_price_feed depends on no feature crate but \
         lib_units, a cross-domain module depends only on the feature crates it writes, and only the \
         bins and lib_database depend on lib_shell or lib_settings. See \
         docs/development/viper-architecture.md, Rules > Dependencies. Move shared value types to \
         lib_core, declare a read port in protocols.rs, or put a multi-domain write in a \
         cross-domain module.\nOffending dependencies:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn violations_flags_feature_toolkit_and_database_dependencies_of_every_feature_crate() {
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
        violations(&metadata),
        vec![
            "lib_units -> lib_bills",
            "lib_units -> gpui",
            "lib_units -> lib_database",
            "lib_bills -> lib_payees"
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
        violations(&metadata),
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
                "dependencies": [{ "name": "lib_bill_payments" }]
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
        violations(&metadata),
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
        violations(&metadata),
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

#[test]
fn violations_flags_every_crate_but_the_bins_and_lib_database_depending_on_lib_shell() {
    let metadata = serde_json::json!({
        "packages": [
            { "name": "lib_core", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_config", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_rpc", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "bin_sync_server", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "lib_database", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_shell" }] },
            { "name": "bin_tui", "dependencies": [{ "name": "lib_shell" }] }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_core -> lib_shell",
            "lib_config -> lib_shell",
            "lib_rpc -> lib_shell",
            "bin_sync_server -> lib_shell"
        ]
    );
}

#[test]
fn violations_flags_every_crate_but_the_bins_and_lib_database_depending_on_lib_settings() {
    let metadata = serde_json::json!({
        "packages": [
            { "name": "lib_config", "dependencies": [{ "name": "lib_settings" }] },
            { "name": "bin_sync_server", "dependencies": [{ "name": "lib_settings" }] },
            { "name": "lib_ledger_creation", "dependencies": [{ "name": "lib_settings" }] },
            { "name": "lib_database", "dependencies": [{ "name": "lib_settings" }] },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_settings" }] },
            { "name": "bin_tui", "dependencies": [{ "name": "lib_settings" }] }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_config -> lib_settings",
            "bin_sync_server -> lib_settings",
            "lib_ledger_creation -> lib_settings"
        ]
    );
}

#[test]
fn violations_keeps_lib_units_off_lib_institutions_and_both_off_lib_unit_deletion() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_units",
                "dependencies": [{ "name": "lib_institutions" }, { "name": "lib_unit_deletion" }]
            },
            {
                "name": "lib_institutions",
                "dependencies": [{ "name": "lib_unit_deletion" }]
            },
            {
                "name": "lib_unit_deletion",
                "dependencies": [
                    { "name": "lib_units" },
                    { "name": "lib_institutions" },
                    { "name": "lib_accounts" },
                ]
            }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_units -> lib_institutions",
            "lib_units -> lib_unit_deletion",
            "lib_institutions -> lib_unit_deletion",
            "lib_unit_deletion -> lib_accounts",
        ]
    );
}

#[test]
fn violations_keeps_lib_transaction_import_on_payees_and_transactions_and_off_every_feature_crate()
{
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_payees",
                "dependencies": [{ "name": "lib_transaction_import" }]
            },
            {
                "name": "lib_transactions",
                "dependencies": [{ "name": "lib_transaction_import" }]
            },
            {
                "name": "lib_transaction_import",
                "dependencies": [
                    { "name": "lib_payees" },
                    { "name": "lib_transactions" },
                    { "name": "lib_core" },
                    { "name": "lib_viper" },
                    { "name": "lib_accounts" },
                    { "name": "lib_categories" },
                ]
            },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_transaction_import" }] }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_payees -> lib_transaction_import",
            "lib_transactions -> lib_transaction_import",
            "lib_transaction_import -> lib_accounts",
            "lib_transaction_import -> lib_categories"
        ]
    );
}

#[test]
fn violations_flags_a_feature_shared_or_cross_domain_crate_on_an_adapter_crate() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_units",
                "dependencies": [{ "name": "lib_price_feed" }, { "name": "lib_database" }]
            },
            { "name": "lib_viper", "dependencies": [{ "name": "lib_price_feed" }] },
            { "name": "lib_toast", "dependencies": [{ "name": "lib_database" }] },
            { "name": "lib_unit_deletion", "dependencies": [{ "name": "lib_price_feed" }] },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_price_feed" }] },
            { "name": "bin_tui", "dependencies": [{ "name": "lib_price_feed" }] }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_units -> lib_price_feed",
            "lib_units -> lib_database",
            "lib_viper -> lib_price_feed",
            "lib_toast -> lib_database",
            "lib_unit_deletion -> lib_price_feed",
        ]
    );
}

#[test]
fn violations_keeps_lib_price_feed_off_every_feature_crate_but_lib_units() {
    let metadata = serde_json::json!({
        "packages": [
            {
                "name": "lib_price_feed",
                "dependencies": [
                    { "name": "lib_units" },
                    { "name": "lib_core" },
                    { "name": "reqwest" },
                    { "name": "lib_accounts" },
                    { "name": "lib_settings" },
                ]
            }
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_price_feed -> lib_accounts",
            "lib_price_feed -> lib_settings"
        ]
    );
}

#[test]
fn violations_flags_a_feature_shared_or_cross_domain_crate_depending_on_lib_config_or_lib_tracing()
{
    let metadata = serde_json::json!({
        "packages": [
            { "name": "lib_units", "dependencies": [{ "name": "lib_core" }, { "name": "lib_config" }] },
            { "name": "lib_toast", "dependencies": [{ "name": "lib_tracing" }] },
            { "name": "lib_bill_payments", "dependencies": [{ "name": "lib_config" }] },
            { "name": "bin_desktop", "dependencies": [{ "name": "lib_config" }, { "name": "lib_tracing" }] },
        ]
    });

    assert_eq!(
        violations(&metadata),
        vec![
            "lib_units -> lib_config",
            "lib_toast -> lib_tracing",
            "lib_bill_payments -> lib_config",
        ]
    );
}
