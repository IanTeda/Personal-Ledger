//! The process-wide loader, set once at startup, and the scoped test override.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

use fluent_bundle::FluentArgs;

use super::bundle::{self, Bundle};
use super::{Arg, Layer, pseudo};
use crate::locale::Locale;

static LOADER: OnceLock<Loader> = OnceLock::new();

thread_local! {
    static OVERRIDE: Cell<Option<Locale>> = const { Cell::new(None) };
}

struct Loader {
    active: Locale,
    bundles: HashMap<Locale, Bundle>,
}

impl Loader {
    fn build(active: Locale, extra_layers: &[Layer]) -> Loader {
        let mut layers: Vec<Layer> = vec![crate::msg::LAYER];
        layers.extend_from_slice(extra_layers);

        for (tag, id) in bundle::overlapping_ids(&layers) {
            tracing::warn!(
                locale = tag,
                id,
                "Message id defined in more than one layer"
            );
        }

        let mut bundles = HashMap::new();
        for locale in Locale::SUPPORTED {
            match bundle::build(locale, &layers) {
                Ok(bundle) => {
                    bundles.insert(locale, bundle);
                }
                Err(error) => {
                    tracing::warn!(%locale, %error, "could not build the Catalogue bundle")
                }
            }
        }
        Loader { active, bundles }
    }

    fn format(
        &self,
        locale: Locale,
        id: &str,
        attribute: Option<&str>,
        args: &[(&str, Arg)],
    ) -> String {
        let Some(bundle) = self.bundles.get(&locale) else {
            return miss(id, attribute);
        };
        let Some(message) = bundle.get_message(id) else {
            return miss(id, attribute);
        };
        let pattern = match attribute {
            Some(name) => message
                .get_attribute(name)
                .map(|attribute| attribute.value()),
            None => message.value(),
        };
        let Some(pattern) = pattern else {
            return miss(id, attribute);
        };

        let mut fluent_args = FluentArgs::new();
        for (name, arg) in args {
            match arg {
                Arg::Int(count) => fluent_args.set(*name, *count),
                Arg::Text(text) => fluent_args.set(*name, *text),
            }
        }

        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
        if !errors.is_empty() {
            tracing::warn!(id, ?attribute, ?errors, "Message did not format cleanly");
        }
        if locale.is_pseudo() {
            pseudo::wrap(&text)
        } else {
            text.into_owned()
        }
    }
}

/// An id missing everywhere (possible only for a dynamic id): `⟦id⟧` in debug builds so it
/// stands out, the plain id in release, with one warning.
fn miss(id: &str, attribute: Option<&str>) -> String {
    let key = match attribute {
        Some(attribute) => format!("{id}.{attribute}"),
        None => id.to_string(),
    };
    tracing::warn!(id = key, "Message missing from every Catalogue");
    if cfg!(debug_assertions) {
        format!("⟦{key}⟧")
    } else {
        key
    }
}

/// Sets the process-wide Locale from a requested BCP-47 tag and returns the Locale chosen.
///
/// The tag is negotiated against the supported set: an exact match wins, anything else becomes
/// `en-US`, and `en-XA` is chosen only by exact request. Set once: a later call keeps the first
/// Locale and returns it.
pub fn init(tag: &str) -> Locale {
    init_with_layers(tag, &[])
}

/// As [`init`], also composing a bin's own layer (its generated `msg::LAYER`) after the shared
/// one.
pub fn init_with_layers(tag: &str, layers: &[Layer]) -> Locale {
    let requested = Locale::negotiate(tag);
    if let Some(existing) = LOADER.get() {
        if existing.active != requested {
            tracing::warn!(active = %existing.active, %requested, "Locale is already set; ignoring init");
        }
        return existing.active;
    }
    LOADER
        .get_or_init(|| Loader::build(requested, layers))
        .active
}

/// Looks a Message up by id (and optional attribute). The generated accessors call this; use
/// them rather than calling it directly. A miss never panics.
pub fn format(id: &str, attribute: Option<&str>, args: &[(&str, Arg)]) -> String {
    let loader = LOADER.get_or_init(|| Loader::build(Locale::DEFAULT, &[]));
    let locale = OVERRIDE.with(Cell::get).unwrap_or(loader.active);
    loader.format(locale, id, attribute, args)
}

/// Runs `f` with this thread's Messages rendered in `locale`, leaving the process-wide loader
/// alone, so parallel tests can use different Locales. It does not reach spawned threads.
pub fn with_locale<T>(locale: Locale, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<Locale>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|cell| cell.set(self.0));
        }
    }

    let _restore = Restore(OVERRIDE.with(|cell| cell.replace(Some(locale))));
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYER_A: Layer = &[
        (
            "en-US",
            &[(
                "a.ftl",
                "tui-hits = { $count ->\n    [one] { $count } match\n   *[other] { $count } matches\n}\ntui-colour = Pick a color for { $name }\ntui-only-us = Only in en-US\n",
            )],
        ),
        (
            "en-GB",
            &[("a.ftl", "tui-colour = Pick a colour for { $name }\n")],
        ),
        (
            "en-AU",
            &[(
                "a.ftl",
                "tui-hits = { $count ->\n    [one] { $count } hit\n   *[other] { $count } hits\n}\n",
            )],
        ),
    ];

    fn loader() -> Loader {
        Loader::build(Locale::EnAu, &[LAYER_A])
    }

    fn hits(loader: &Loader, locale: Locale, count: i64) -> String {
        loader.format(locale, "tui-hits", None, &[("count", Arg::Int(count))])
    }

    #[test]
    fn sparse_locales_fall_back_through_the_chain() {
        let loader = loader();
        let name = [("name", Arg::Text("Food"))];
        assert_eq!(
            loader.format(Locale::EnAu, "tui-colour", None, &name),
            "Pick a colour for Food"
        );
        assert_eq!(
            loader.format(Locale::EnUs, "tui-colour", None, &name),
            "Pick a color for Food"
        );
        assert_eq!(
            loader.format(Locale::EnAu, "tui-only-us", None, &[]),
            "Only in en-US"
        );
        assert_eq!(
            loader.format(Locale::EnGb, "tui-only-us", None, &[]),
            "Only in en-US"
        );
    }

    #[test]
    fn locale_specific_override_beats_the_source() {
        let loader = loader();
        assert_eq!(hits(&loader, Locale::EnAu, 1), "1 hit");
        assert_eq!(hits(&loader, Locale::EnAu, 2), "2 hits");
        assert_eq!(hits(&loader, Locale::EnGb, 2), "2 matches");
    }

    #[test]
    fn pseudo_locale_wraps_and_transforms() {
        let loader = loader();
        let text = loader.format(Locale::EnXa, "tui-only-us", None, &[]);
        assert!(text.starts_with('[') && text.ends_with(']'));
        assert_ne!(text, "[Only in en-US]");
    }

    #[test]
    fn misses_return_the_id() {
        let loader = loader();
        let text = loader.format(Locale::EnUs, "no-such-id", None, &[]);
        assert!(text.contains("no-such-id"));
        let attribute = loader.format(Locale::EnUs, "tui-only-us", Some("nope"), &[]);
        assert!(attribute.contains("tui-only-us.nope"));
        if cfg!(debug_assertions) {
            assert_eq!(text, "⟦no-such-id⟧");
        } else {
            assert_eq!(text, "no-such-id");
        }
    }

    #[test]
    fn overlapping_layers_are_reported() {
        const CLASH: Layer = &[("en-US", &[("b.ftl", "tui-only-us = Clash\n")])];
        assert_eq!(bundle::overlapping_ids(&[LAYER_A, CLASH]).len(), 1);
        assert!(bundle::overlapping_ids(&[LAYER_A]).is_empty());
    }

    #[test]
    fn scoped_override_restores_the_previous_locale() {
        assert_eq!(OVERRIDE.with(Cell::get), None);
        let inner = with_locale(Locale::EnGb, || {
            with_locale(Locale::EnAu, || OVERRIDE.with(Cell::get));
            OVERRIDE.with(Cell::get)
        });
        assert_eq!(inner, Some(Locale::EnGb));
        assert_eq!(OVERRIDE.with(Cell::get), None);
    }

    #[test]
    fn scoped_override_selects_the_shared_catalogue_locale() {
        assert_eq!(
            with_locale(Locale::EnGb, crate::msg::column_colour),
            "Colour"
        );
        assert_eq!(
            with_locale(Locale::EnAu, crate::msg::column_colour),
            "Colour"
        );
        assert_eq!(
            with_locale(Locale::EnUs, crate::msg::column_colour),
            "Color"
        );
    }
}
