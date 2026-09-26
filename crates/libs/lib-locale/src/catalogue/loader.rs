//! The process-wide loader, set once at startup, and the scoped test override.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use fluent_bundle::FluentArgs;

use super::bundle::{self, Bundle};
use super::{Arg, Layer, pseudo};
use crate::locale::Locale;
use crate::rich::{self, Segment};

/// The process-wide loader. A lookup before `init` builds a default one (so tests and tools work
/// without setup); an explicit `init` replaces that default, but never another explicit one.
static LOADER: RwLock<Option<Arc<Loader>>> = RwLock::new(None);

thread_local! {
    static OVERRIDE: Cell<Option<Locale>> = const { Cell::new(None) };
}

struct Loader {
    active: Locale,
    bundles: HashMap<Locale, Bundle>,
    explicit: bool,
}

impl Loader {
    fn build(active: Locale, extra_layers: &[Layer], explicit: bool) -> Loader {
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
        Loader {
            active,
            bundles,
            explicit,
        }
    }

    fn format(
        &self,
        locale: Locale,
        id: &str,
        attribute: Option<&str>,
        args: &[(&str, Arg<'_>)],
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

impl Loader {
    fn format_rich(
        &self,
        locale: Locale,
        id: &str,
        attribute: Option<&str>,
        args: &[(&str, Arg<'_>)],
    ) -> Vec<Segment> {
        let tokens: Vec<&str> = args
            .iter()
            .filter_map(|(_, arg)| match arg {
                Arg::Text(text) => Some(*text),
                Arg::Int(_) => None,
            })
            .collect();
        let sentinels: Vec<String> = (0..tokens.len()).map(rich::sentinel).collect();

        let mut next = 0;
        let swapped: Vec<(&str, Arg<'_>)> = args
            .iter()
            .map(|(name, arg)| match arg {
                Arg::Int(_) => (*name, *arg),
                Arg::Text(_) => {
                    let sentinel = &sentinels[next];
                    next += 1;
                    (*name, Arg::Text(sentinel.as_str()))
                }
            })
            .collect();

        rich::split(&self.format(locale, id, attribute, &swapped), &tokens)
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
/// `en-US`, and `en-XA` is chosen only by exact request. Set once: a later explicit call keeps the first
/// Locale and returns it.
pub fn init(tag: &str) -> Locale {
    init_with_layers(tag, &[])
}

/// As [`init`], also composing a bin's own layer (its generated `msg::LAYER`) after the shared
/// one.
pub fn init_with_layers(tag: &str, layers: &[Layer]) -> Locale {
    let requested = Locale::negotiate(tag);
    let mut slot = LOADER.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = slot.as_ref().filter(|loader| loader.explicit) {
        if existing.active != requested {
            tracing::warn!(active = %existing.active, %requested, "Locale is already set; ignoring init");
        }
        return existing.active;
    }
    let loader = Arc::new(Loader::build(requested, layers, true));
    let active = loader.active;
    *slot = Some(loader);
    active
}

/// The loader in use, building the default one on first need.
fn current() -> Arc<Loader> {
    if let Some(loader) = LOADER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        return Arc::clone(loader);
    }
    let mut slot = LOADER.write().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(slot.get_or_insert_with(|| Arc::new(Loader::build(Locale::DEFAULT, &[], false))))
}

/// Looks a Message up by id (and optional attribute). The generated accessors call this; use
/// them rather than calling it directly. A miss never panics.
pub fn format(id: &str, attribute: Option<&str>, args: &[(&str, Arg<'_>)]) -> String {
    current().format(locale(), id, attribute, args)
}

/// As [`format`], for a rich Message: every text argument travels as a sentinel and the result
/// is split into [`Segment`]s (see the `rich` module). Counts stay plain. The generated
/// accessors for a Message with tags, or marked `# @rich`, call this.
pub fn format_rich(id: &str, attribute: Option<&str>, args: &[(&str, Arg<'_>)]) -> Vec<Segment> {
    current().format_rich(locale(), id, attribute, args)
}

/// The Locale in effect on this thread: the scoped override if any, else the process-wide one.
pub fn locale() -> Locale {
    OVERRIDE.with(Cell::get).unwrap_or_else(|| current().active)
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
        Loader::build(Locale::EnAu, &[LAYER_A], false)
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

    const RICH: Layer = &[(
        "en-US",
        &[(
            "r.ftl",
            "tui-open = Press <open>open</open> to see { $key } details\ntui-count = { $count ->\n    [one] <b>{ $count }</b> match for { $name }\n   *[other] <b>{ $count }</b> matches for { $name }\n}\n",
        )],
    )];

    fn segment_texts(segments: &[Segment]) -> Vec<(String, Option<String>, Option<usize>)> {
        segments
            .iter()
            .map(|s| (s.text.clone(), s.tag.clone(), s.token))
            .collect()
    }

    #[test]
    fn rich_messages_split_tags_and_tokens() {
        let loader = Loader::build(Locale::EnUs, &[RICH], false);
        let segments = loader.format_rich(
            Locale::EnUs,
            "tui-open",
            None,
            &[("key", Arg::Text("Ctrl+K"))],
        );
        assert_eq!(
            segment_texts(&segments),
            vec![
                ("Press ".to_string(), None, None),
                ("open".to_string(), Some("open".to_string()), None),
                (" to see ".to_string(), None, None),
                ("Ctrl+K".to_string(), None, Some(0)),
                (" details".to_string(), None, None),
            ]
        );
    }

    #[test]
    fn a_forged_tag_in_an_argument_stays_literal() {
        let loader = Loader::build(Locale::EnUs, &[RICH], false);
        let segments = loader.format_rich(
            Locale::EnUs,
            "tui-open",
            None,
            &[("key", Arg::Text("<open>x</open>"))],
        );
        let token = segments.iter().find(|s| s.token == Some(0));
        assert_eq!(
            token.map(|s| (s.text.as_str(), s.tag.as_deref())),
            Some(("<open>x</open>", None))
        );
        assert_eq!(
            segments
                .iter()
                .filter(|s| s.tag.as_deref() == Some("open"))
                .count(),
            1
        );
    }

    #[test]
    fn counts_stay_plain_beside_tokens() {
        let loader = Loader::build(Locale::EnUs, &[RICH], false);
        let args = [("count", Arg::Int(2)), ("name", Arg::Text("Food"))];
        let segments = loader.format_rich(Locale::EnUs, "tui-count", None, &args);
        assert_eq!(
            segment_texts(&segments),
            vec![
                ("2".to_string(), Some("b".to_string()), None),
                (" matches for ".to_string(), None, None),
                ("Food".to_string(), None, Some(0)),
            ]
        );
    }

    #[test]
    fn en_xa_keeps_tags_and_sentinels_intact() {
        let loader = Loader::build(Locale::EnXa, &[RICH], false);
        let segments = loader.format_rich(
            Locale::EnXa,
            "tui-open",
            None,
            &[("key", Arg::Text("Ctrl+K"))],
        );
        let texts = segment_texts(&segments);
        assert!(
            texts
                .iter()
                .any(|(text, tag, _)| tag.as_deref() == Some("open") && text != "open")
        );
        assert!(texts.contains(&("Ctrl+K".to_string(), None, Some(0))));
        assert!(segments.first().is_some_and(|s| s.text.starts_with('[')));
        assert!(segments.last().is_some_and(|s| s.text.ends_with(']')));
        assert!(
            segments
                .iter()
                .all(|s| !s.text.contains('<') && !s.text.contains('\u{e000}'))
        );
    }
}
