//! The `en-XA` pseudo-Locale transform, applied to `en-US` text at load time.

use std::borrow::Cow;

/// Accents and elongates each text fragment, keeping `<tag>` names intact. Markers stay off
/// because they bracket every fragment around placeables, not the whole Message.
pub(super) fn transform(text: &str) -> Cow<'_, str> {
    fluent_pseudo::transform_dom(text, false, true, false)
}

/// Adds the outer brackets that show where a Message starts and ends.
pub(super) fn wrap(text: &str) -> String {
    format!("[{text}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_tag_names_and_sentinels() {
        let out = transform("Press <open>open</open> to see \u{e000}0\u{e001}");
        assert!(out.contains("<open>"));
        assert!(out.contains("</open>"));
        assert!(out.contains("\u{e000}0\u{e001}"));
        assert_ne!(out, "Press <open>open</open> to see \u{e000}0\u{e001}");
    }

    #[test]
    fn wraps_the_whole_message() {
        assert_eq!(wrap("abc"), "[abc]");
    }
}
