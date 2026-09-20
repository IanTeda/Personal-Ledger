//! Cross-Locale checks over the shared layer, run as ordinary tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fluent_syntax::ast::Entry;
use fluent_syntax::parser;
use lib_locale::{Locale, msg, runtime, with_locale};
use lib_locale_build::{Options, generate};

fn keys(files: &[(&str, &str)]) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (_, source) in files {
        let resource = match parser::parse(*source) {
            Ok(resource) | Err((resource, _)) => resource,
        };
        for entry in &resource.body {
            if let Entry::Message(message) = entry {
                let id = message.id.name;
                if message.value.is_some() {
                    keys.insert(id.to_string());
                }
                for attribute in &message.attributes {
                    keys.insert(format!("{id}.{}", attribute.id.name));
                }
            }
        }
    }
    keys
}

fn layer_files(tag: &str) -> Vec<(&'static str, &'static str)> {
    msg::LAYER
        .iter()
        .find(|(locale, _)| *locale == tag)
        .map(|(_, files)| files.to_vec())
        .unwrap_or_default()
}

#[test]
fn every_locale_checks_clean_against_en_us() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = generate(&Options {
        i18n_dir: manifest.join("i18n"),
        out_file: manifest.join("unused.rs"),
        runtime_path: "crate::runtime".to_string(),
        id_prefix: None,
        scan_dirs: Vec::new(),
    });
    assert!(generated.is_ok(), "{:?}", generated.err());
}

#[test]
fn every_id_in_every_locale_exists_in_en_us() {
    let source = keys(&layer_files("en-US"));
    assert!(!source.is_empty());
    for (tag, files) in msg::LAYER {
        for key in keys(files) {
            assert!(
                source.contains(&key),
                "{tag} defines `{key}`, which en-US lacks"
            );
        }
    }
}

#[test]
fn sparse_locales_fall_back_correctly() {
    assert_eq!(with_locale(Locale::EnUs, msg::column_colour), "Color");
    assert_eq!(with_locale(Locale::EnGb, msg::column_colour), "Colour");
    assert_eq!(with_locale(Locale::EnAu, msg::column_colour), "Colour");
    assert_eq!(with_locale(Locale::EnAu, msg::nav_accounts), "Accounts");
}

#[test]
fn en_xa_renders_every_message_without_a_literal_id() {
    with_locale(Locale::EnXa, || {
        for key in keys(&layer_files("en-US")) {
            let (id, attribute) = match key.split_once('.') {
                Some((id, attribute)) => (id, Some(attribute)),
                None => (key.as_str(), None),
            };
            let text = runtime::format(id, attribute, &[]);
            assert!(
                text.starts_with('[') && text.ends_with(']'),
                "{key}: {text}"
            );
            assert!(!text.contains(id), "{key} rendered its own id: {text}");
            assert!(!text.contains('⟦'), "{key} missed: {text}");
        }
    });
}

#[test]
fn init_then_accessor() {
    let locale = lib_locale::init("en-US");
    assert_eq!(locale, Locale::EnUs);
    assert_eq!(msg::nav_settings(), "Settings");
    assert_eq!(lib_locale::init("fr-FR"), Locale::EnUs);
}

#[test]
fn generated_rich_accessor_returns_segments_with_tokens() {
    let segments = with_locale(Locale::EnUs, || msg::hint_press_key("Enter", "save"));
    let shown: Vec<(&str, Option<usize>)> = segments
        .iter()
        .map(|s| (s.text.as_str(), s.token))
        .collect();
    // Parameters follow the text's order (`key`, `action`), and so do the token indices.
    assert_eq!(
        shown,
        vec![
            ("Press ", None),
            ("Enter", Some(0)),
            (" to ", None),
            ("save", Some(1))
        ]
    );
}

#[test]
fn generated_rich_accessor_survives_en_xa() {
    let segments = with_locale(Locale::EnXa, || msg::hint_press_key("Enter", "save"));
    assert!(
        segments
            .iter()
            .any(|s| s.token == Some(0) && s.text == "Enter")
    );
    assert!(segments.first().is_some_and(|s| s.text.starts_with('[')));
    assert!(segments.last().is_some_and(|s| s.text.ends_with(']')));
}
