//! The Locale in effect for this run: resolved once at startup, never changed at runtime.

use std::sync::OnceLock;

use lib_config::LocaleSource;
use lib_core::DateStyle;
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
/// chosen. Call once, after configuration is parsed and before the terminal enters raw mode.
pub fn init(requested: &str, source: LocaleSource) -> Locale {
    let effective = lib_locale::init_with_layers(requested, &[crate::msg::LAYER]);
    let _ = INFO.set(LocaleInfo {
        requested: requested.to_string(),
        effective,
        source,
    });
    effective
}

/// Loads this bin's Message layer for a unit test, since tests never run `main`. Safe to call
/// from any test: an explicit `init` replaces the lazily built default loader.
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

/// The Settings row for the effective Locale: its tag, and a note naming where it came from (or
/// that the request was not supported and fell back).
pub fn describe(info: &LocaleInfo) -> (String, String) {
    let tag = info.effective.tag();
    let note = if info.requested == tag {
        match info.source {
            LocaleSource::System => crate::msg::tui_settings_locale_from_system(),
            LocaleSource::Config => crate::msg::tui_settings_locale_from_config(),
            LocaleSource::Default => crate::msg::tui_settings_locale_from_default(),
        }
    } else {
        crate::msg::tui_settings_locale_unsupported(&info.requested)
    };
    (tag.to_string(), note)
}

/// The label for a date style choice (`None` is the Locale default), as a Message.
pub fn date_style_label(choice: Option<DateStyle>) -> String {
    match choice {
        None => crate::msg::tui_settings_date_style_default(),
        Some(DateStyle::Short) => crate::msg::tui_settings_date_style_short(),
        Some(DateStyle::Medium) => crate::msg::tui_settings_date_style_medium(),
        Some(DateStyle::Long) => crate::msg::tui_settings_date_style_long(),
        Some(DateStyle::Iso) => crate::msg::tui_settings_date_style_iso(),
    }
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
    fn the_source_is_named_in_the_note() {
        init_for_tests();
        assert_eq!(
            describe(&info("en-GB", LocaleSource::Config)),
            ("en-GB".to_string(), "from config".to_string())
        );
        assert_eq!(
            describe(&info("en-AU", LocaleSource::System)).1,
            "from your system"
        );
        assert_eq!(
            describe(&info("en-US", LocaleSource::Default)).1,
            "the default"
        );
    }

    #[test]
    fn an_unsupported_tag_says_it_fell_back() {
        init_for_tests();
        assert_eq!(
            describe(&info("fr-FR", LocaleSource::System)),
            ("en-US".to_string(), "fr-FR is not supported".to_string())
        );
    }

    #[test]
    fn date_style_labels_are_messages() {
        init_for_tests();
        let choices = [
            None,
            Some(DateStyle::Short),
            Some(DateStyle::Medium),
            Some(DateStyle::Long),
            Some(DateStyle::Iso),
        ];
        let labels: Vec<String> = choices.into_iter().map(date_style_label).collect();
        assert_eq!(labels, ["locale default", "short", "medium", "long", "iso"]);
    }
}
