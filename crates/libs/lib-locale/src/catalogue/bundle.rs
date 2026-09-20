//! Builds one Fluent bundle per Locale from the embedded layers.

use std::collections::HashMap;

use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use fluent_syntax::ast::Entry;
use fluent_syntax::parser;

use super::{Layer, pseudo};
use crate::error::Result;
use crate::locale::Locale;

pub(super) type Bundle = FluentBundle<FluentResource>;

/// Composes every layer's Catalogue for `locale` into one bundle.
///
/// The fallback chain is added lowest priority first, so later resources override earlier ones.
/// Unicode isolating marks are off, because Messages are rendered in terminal cells and gpui
/// text where the marks would show. `en-XA` uses the `en-US` plural rules and the pseudo
/// transform.
pub(super) fn build(locale: Locale, layers: &[Layer]) -> Result<Bundle> {
    let mut bundle = FluentBundle::new_concurrent(vec![locale.language_identifier()?]);
    bundle.set_use_isolating(false);
    if locale.is_pseudo() {
        bundle.set_transform(Some(pseudo::transform));
    }

    for chain_locale in locale.fallback_chain().iter().rev() {
        for layer in layers {
            for (tag, files) in layer.iter() {
                if *tag != chain_locale.tag() {
                    continue;
                }
                for (name, source) in files.iter() {
                    let resource = match FluentResource::try_new((*source).to_string()) {
                        Ok(resource) => resource,
                        Err((resource, errors)) => {
                            tracing::warn!(
                                file = name,
                                ?errors,
                                "Catalogue file has syntax errors"
                            );
                            resource
                        }
                    };
                    bundle.add_resource_overriding(resource);
                }
            }
        }
    }
    Ok(bundle)
}

/// Message ids that appear in more than one layer for the same Locale. A bin's prefixed ids
/// should never overlap the shared layer, so any hit is reported once at `init`.
pub(super) fn overlapping_ids(layers: &[Layer]) -> Vec<(String, String)> {
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    let mut overlaps = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        for (tag, files) in layer.iter() {
            for (_, source) in files.iter() {
                let resource = match parser::parse(*source) {
                    Ok(resource) | Err((resource, _)) => resource,
                };
                for entry in &resource.body {
                    let Entry::Message(message) = entry else {
                        continue;
                    };
                    let key = ((*tag).to_string(), message.id.name.to_string());
                    match seen.insert(key.clone(), index) {
                        Some(previous) if previous != index => overlaps.push(key),
                        _ => {}
                    }
                }
            }
        }
    }
    overlaps
}
