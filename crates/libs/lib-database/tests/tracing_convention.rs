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

use std::path::Path;

use common::{CROSS_DOMAIN, FEATURE_CRATES, SHARED_CRATES, crate_sources, is_cfg_test, item_attrs};
use syn::visit::{self, Visit};

fn is_instrument(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "instrument")
}

/// Reports each fn in `parsed` carrying an `instrument` attribute, skipping `#[cfg(test)]` items.
fn violations_in(parsed: &syn::File, file: &str) -> Vec<String> {
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
    // `#[cfg(test)]` is skipped once per kind of item, so the fn visitors below only look for
    // `instrument`.
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_item(&mut self, item: &'ast syn::Item) {
            if !is_cfg_test(item_attrs(item)) {
                visit::visit_item(self, item);
            }
        }
        fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
            if !matches!(item, syn::ImplItem::Fn(f) if is_cfg_test(&f.attrs)) {
                visit::visit_impl_item(self, item);
            }
        }
        fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
            if !matches!(item, syn::TraitItem::Fn(f) if is_cfg_test(&f.attrs)) {
                visit::visit_trait_item(self, item);
            }
        }
        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            self.check(&item.attrs, &item.sig.ident);
            visit::visit_item_fn(self, item);
        }
        fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
            self.check(&item.attrs, &item.sig.ident);
            visit::visit_impl_item_fn(self, item);
        }
        fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
            self.check(&item.attrs, &item.sig.ident);
            visit::visit_trait_item_fn(self, item);
        }
    }
    let mut finder = Finder {
        file,
        found: Vec::new(),
    };
    finder.visit_file(parsed);
    finder.found
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
        let dir = libs.join(name.replace('_', "-"));
        assert!(
            dir.join("src/lib.rs").is_file(),
            "{name} is checked here but {} has no src/lib.rs; was it renamed?",
            dir.display()
        );
        for (file, parsed) in crate_sources(&dir) {
            found.extend(violations_in(&parsed, &file.display().to_string()));
        }
    }
    assert!(
        found.is_empty(),
        "ADR-0047: #[instrument] stays out of feature crates; use explicit tracing::info! events \
         instead.\n  {}",
        found.join("\n  ")
    );
}

fn flags(source: &str) -> Vec<String> {
    let parsed = syn::parse_file(source).expect("the fixture should parse");
    violations_in(&parsed, "x.rs")
}

#[test]
fn an_instrument_attribute_on_a_fn_is_reported() {
    let source = "#[tracing::instrument(skip_all)]\nfn run() {}\nstruct S;\nimpl S { #[instrument] fn go(&self) {} }";
    let found = flags(source);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("ADR-0047") && found[0].contains("fn run") && found[0].contains("x.rs")
    );
}

#[test]
fn a_fn_without_instrument_or_in_a_test_module_passes() {
    let source = "fn run() { tracing::info!(\"x\"); }\n#[cfg(test)]\nmod tests { #[tracing::instrument] fn t() {} }";
    assert!(flags(source).is_empty());
}
