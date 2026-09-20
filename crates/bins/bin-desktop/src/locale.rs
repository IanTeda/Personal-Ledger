//! The Locale in effect for this run: resolved once at startup, never changed at runtime.

use std::sync::OnceLock;

use lib_config::LocaleSource;
use lib_locale::Locale;

/// What startup resolved, kept so Settings can show the effective Locale and where it came from.
#[derive(Debug, Clone)]
pub struct LocaleInfo {
    /// The tag `lib-config` resolved, before negotiation (`fr-FR` may become `en-US`).
    pub requested: String,

    /// The supported Locale in effect.
    pub effective: Locale,

    /// Whether the value came from config, the system, or the built-in default.
    pub source: LocaleSource,
}

static INFO: OnceLock<LocaleInfo> = OnceLock::new();

/// Sets the process-wide Locale (with this bin's own Message layer) and remembers how it was
/// chosen. Call once, after configuration is parsed.
pub fn init(requested: &str, source: LocaleSource) -> Locale {
    let effective = lib_locale::init_with_layers(requested, &[crate::msg::LAYER]);
    let _ = INFO.set(LocaleInfo {
        requested: requested.to_string(),
        effective,
        source,
    });
    effective
}

/// Loads this bin's Message layer for a unit test, since tests never run `main`. Safe to call from
/// any test: an explicit `init` replaces the lazily built default loader.
#[cfg(test)]
pub fn init_for_tests() {
    lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
}

/// The startup record, or the Locale in effect from the default if nothing was recorded (as in
/// unit tests that never run `main`).
pub fn info() -> LocaleInfo {
    INFO.get().cloned().unwrap_or_else(|| {
        let effective = lib_locale::locale();
        LocaleInfo {
            requested: effective.tag().to_string(),
            effective,
            source: LocaleSource::Default,
        }
    })
}

/// The tag `gpui-component` understands (it ships `en`, `zh-CN`, `zh-HK` and `it`): every `en-*`
/// Locale maps to `en`.
pub fn gpui_component_tag(locale: Locale) -> &'static str {
    match locale {
        Locale::EnUs | Locale::EnGb | Locale::EnAu | Locale::EnXa => "en",
    }
}

/// The Settings line for the effective Locale and its source, and a note when the request was
/// not supported and fell back.
pub fn describe(info: &LocaleInfo) -> (String, Option<String>) {
    let tag = info.effective.tag();
    let line = match info.source {
        LocaleSource::System => crate::msg::desktop_display_locale_from_system(tag),
        LocaleSource::Config => crate::msg::desktop_display_locale_from_config(tag),
        LocaleSource::Default => crate::msg::desktop_display_locale_from_default(tag),
    };
    let note = (info.requested != tag)
        .then(|| crate::msg::desktop_display_locale_fallback(&info.requested, tag));
    (line, note)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(requested: &str, source: LocaleSource) -> LocaleInfo {
        LocaleInfo {
            requested: requested.to_string(),
            effective: Locale::negotiate(requested),
            source,
        }
    }

    #[test]
    fn every_english_locale_maps_to_gpui_components_en() {
        for locale in Locale::SUPPORTED {
            assert_eq!(gpui_component_tag(locale), "en");
        }
    }

    #[test]
    fn the_source_is_named_in_the_settings_line() {
        init_for_tests();
        assert_eq!(
            describe(&info("en-GB", LocaleSource::Config)),
            ("en-GB, from config".to_string(), None)
        );
        assert_eq!(
            describe(&info("en-AU", LocaleSource::System)).0,
            "en-AU, from your system"
        );
        assert_eq!(
            describe(&info("en-US", LocaleSource::Default)).0,
            "en-US, the default"
        );
    }

    #[test]
    fn an_unsupported_tag_says_it_fell_back() {
        init_for_tests();
        let (line, note) = describe(&info("fr-FR", LocaleSource::System));
        assert_eq!(line, "en-US, from your system");
        assert_eq!(
            note.as_deref(),
            Some("fr-FR is not supported, so en-US is used.")
        );
    }
}
