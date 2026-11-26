//! What the workspace-wide convention tests share: one read of `cargo metadata`, which gives each
//! package's targets and dependencies as Cargo resolves them, renames included.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test crate: a failed setup should fail the test"
)]
#![allow(
    dead_code,
    reason = "each test binary that declares `mod common` uses only part of it"
)]

use std::fs;
use std::path::{Path, PathBuf};
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

/// Whether `attrs` hold `#[cfg(test)]` or `#[cfg(all(test, ..))]`.
pub fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr.parse_args::<syn::Meta>().is_ok_and(|meta| {
                match meta {
                syn::Meta::Path(path) => path.is_ident("test"),
                syn::Meta::List(list) => {
                    list.path.is_ident("all")
                        && list.tokens.clone().into_iter().any(|token| {
                            matches!(token, proc_macro2::TokenTree::Ident(ident) if ident == "test")
                        })
                }
                syn::Meta::NameValue(_) => false,
            }
            })
    })
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(pair) if pair.path.is_ident("path") => match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) => Some(text.value()),
            _ => None,
        },
        _ => None,
    })
}

/// The file a `mod name;` declared with `attrs` inside `module_dir` loads.
fn module_file(module_dir: &Path, name: &str, attrs: &[syn::Attribute]) -> Option<PathBuf> {
    if let Some(path) = path_attr(attrs) {
        return Some(module_dir.join(path));
    }
    [
        module_dir.join(format!("{name}.rs")),
        module_dir.join(name).join("mod.rs"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Adds to `out` the files the module tree under `items` loads, skipping `#[cfg(test)]` and
/// `generated` modules.
fn collect_modules(items: &[syn::Item], module_dir: &Path, out: &mut Vec<(PathBuf, syn::File)>) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        if is_cfg_test(&module.attrs) || name == "generated" {
            continue;
        }
        match &module.content {
            Some((_, inner)) => collect_modules(inner, &module_dir.join(&name), out),
            None => {
                if let Some(file) = module_file(module_dir, &name, &module.attrs) {
                    load(&file, false, out);
                }
            }
        }
    }
}

/// Parses `file` and the modules it loads into `out`. A crate root's modules sit beside it; any
/// other file's sit in the directory named after it.
fn load(file: &Path, is_root: bool, out: &mut Vec<(PathBuf, syn::File)>) {
    if out.iter().any(|(seen, _)| seen == file) {
        return;
    }
    let source = fs::read_to_string(file).unwrap_or_default();
    let parsed = syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("{} should parse: {error}", file.display()));
    let module_dir = match file.file_name().and_then(|name| name.to_str()) {
        _ if is_root => file.parent().map(Path::to_path_buf),
        Some("mod.rs") => file.parent().map(Path::to_path_buf),
        _ => file
            .parent()
            .zip(file.file_stem())
            .map(|(dir, stem)| dir.join(stem)),
    }
    .unwrap_or_default();
    // Pushed before its modules load, so the seen check above stops a module loading twice; its
    // items are lent out while they do, rather than the whole file cloned.
    let index = out.len();
    out.push((file.to_path_buf(), parsed));
    let items = std::mem::take(&mut out[index].1.items);
    collect_modules(&items, &module_dir, out);
    out[index].1.items = items;
}

/// Every source file the crate's lib and bin targets compile, parsed.
pub fn crate_sources(dir: &Path) -> Vec<(PathBuf, syn::File)> {
    let mut out = Vec::new();
    let mut roots = vec![dir.join("src/lib.rs"), dir.join("src/main.rs")];
    let mut bins: Vec<PathBuf> = fs::read_dir(dir.join("src/bin"))
        .into_iter()
        .flatten()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    bins.sort();
    roots.extend(bins);
    for root in roots.into_iter().filter(|root| root.is_file()) {
        load(&root, true, &mut out);
    }
    out
}

/// The attributes of `item`, for the kinds a `#[cfg(test)]` can sit on; none for the rest.
pub fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// The attributes of an impl item, for the kinds a `#[cfg(test)]` can sit on; none for the rest.
pub fn impl_item_attrs(item: &syn::ImplItem) -> &[syn::Attribute] {
    match item {
        syn::ImplItem::Const(item) => &item.attrs,
        syn::ImplItem::Fn(item) => &item.attrs,
        syn::ImplItem::Type(item) => &item.attrs,
        syn::ImplItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

/// The attributes of a trait item, for the kinds a `#[cfg(test)]` can sit on; none for the rest.
pub fn trait_item_attrs(item: &syn::TraitItem) -> &[syn::Attribute] {
    match item {
        syn::TraitItem::Const(item) => &item.attrs,
        syn::TraitItem::Fn(item) => &item.attrs,
        syn::TraitItem::Type(item) => &item.attrs,
        syn::TraitItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impl_and_trait_item_attrs_return_the_cfg_test_attributes() {
        let parsed: syn::File = syn::parse_str(
            "impl A { #[cfg(test)] fn f() {} #[cfg(test)] const C: u8 = 1; fn g() {} }
             trait T { #[cfg(test)] fn f(); #[cfg(test)] const C: u8; fn g(); }",
        )
        .unwrap();
        let (syn::Item::Impl(imp), syn::Item::Trait(tr)) = (&parsed.items[0], &parsed.items[1])
        else {
            panic!("an impl then a trait");
        };
        let impl_flags: Vec<bool> = imp
            .items
            .iter()
            .map(|item| is_cfg_test(impl_item_attrs(item)))
            .collect();
        assert_eq!(impl_flags, [true, true, false]);
        let trait_flags: Vec<bool> = tr
            .items
            .iter()
            .map(|item| is_cfg_test(trait_item_attrs(item)))
            .collect();
        assert_eq!(trait_flags, [true, true, false]);
    }
}
