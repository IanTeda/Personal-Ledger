//! Keeps `#[tracing::instrument]` out of the feature crates (ADR-0047). It lives in `lib-database`
//! beside `error_convention.rs` because this is the one crate that already looks across the
//! others, not because the rule is about persistence.
//!
//! Checked: every crate in `FEATURE_CRATES`, every module in `CROSS_DOMAIN` and the `SHARED_CRATES`.
//! Each crate's source is parsed with `syn` by following the module tree from `lib.rs`, so
//! `#[cfg(test)]` items and `tests/` are not read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test crate: a failed setup should fail the test"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{CROSS_DOMAIN, FEATURE_CRATES, SHARED_CRATES};
use syn::visit::Visit;

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
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

fn is_instrument(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "instrument")
}

/// Reports each fn in `source` carrying an `instrument` attribute, skipping `#[cfg(test)]` items.
fn violations_in(source: &str, file: &str) -> Vec<String> {
    struct Finder<'a> {
        file: &'a str,
        found: Vec<String>,
    }
    impl Finder<'_> {
        fn check(&mut self, attrs: &[syn::Attribute], name: &syn::Ident) {
            if attrs.iter().any(is_instrument) {
                self.found.push(format!(
                    "{}: fn {name} carries #[instrument]; ADR-0047 keeps spans out of feature \
                     crates, so use explicit tracing::info! events instead",
                    self.file
                ));
            }
        }
    }
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            if !is_cfg_test(&item.attrs) {
                self.check(&item.attrs, &item.sig.ident);
                syn::visit::visit_item_fn(self, item);
            }
        }
        fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
            if !is_cfg_test(&item.attrs) {
                self.check(&item.attrs, &item.sig.ident);
                syn::visit::visit_impl_item_fn(self, item);
            }
        }
        fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
            if !is_cfg_test(&item.attrs) {
                self.check(&item.attrs, &item.sig.ident);
                syn::visit::visit_trait_item_fn(self, item);
            }
        }
        fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
            if !is_cfg_test(&item.attrs) {
                syn::visit::visit_item_mod(self, item);
            }
        }
        fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
            if !is_cfg_test(&item.attrs) {
                syn::visit::visit_item_impl(self, item);
            }
        }
    }
    let parsed =
        syn::parse_file(source).unwrap_or_else(|error| panic!("{file} should parse: {error}"));
    let mut finder = Finder {
        file,
        found: Vec::new(),
    };
    finder.visit_file(&parsed);
    finder.found
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

/// Every source file the module tree under `file` loads, `#[cfg(test)]` modules skipped.
fn load(file: &Path, is_root: bool, out: &mut Vec<PathBuf>) {
    if out.iter().any(|seen| seen == file) {
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
    out.push(file.to_path_buf());
    collect_modules(&parsed.items, &module_dir, out);
}

fn collect_modules(items: &[syn::Item], module_dir: &Path, out: &mut Vec<PathBuf>) {
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

#[test]
fn no_feature_cross_domain_or_shared_crate_uses_instrument() {
    let libs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("lib-database sits in crates/libs");
    let mut found = Vec::new();
    for name in FEATURE_CRATES
        .iter()
        .chain(SHARED_CRATES)
        .chain(CROSS_DOMAIN.iter().map(|(module, _)| module))
    {
        let root = libs.join(name.replace('_', "-")).join("src/lib.rs");
        assert!(
            root.is_file(),
            "{name} is checked here but {} is missing; was it renamed?",
            root.display()
        );
        let mut files = Vec::new();
        load(&root, true, &mut files);
        for file in files {
            let source = fs::read_to_string(&file).unwrap_or_default();
            found.extend(violations_in(&source, &file.display().to_string()));
        }
    }
    assert!(
        found.is_empty(),
        "ADR-0047: #[instrument] stays out of feature crates; use explicit tracing::info! events \
         instead.\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn an_instrument_attribute_on_a_fn_is_reported() {
    let source = "#[tracing::instrument(skip_all)]\nfn run() {}\nstruct S;\nimpl S { #[instrument] fn go(&self) {} }";
    let found = violations_in(source, "x.rs");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("ADR-0047") && found[0].contains("fn run") && found[0].contains("x.rs")
    );
}

#[test]
fn a_fn_without_instrument_or_in_a_test_module_passes() {
    let source = "fn run() { tracing::info!(\"x\"); }\n#[cfg(test)]\nmod tests { #[tracing::instrument] fn t() {} }";
    assert!(violations_in(source, "x.rs").is_empty());
}
