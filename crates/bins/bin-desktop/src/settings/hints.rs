//! The status-line legends of the Settings pages and of the dialogs they open, as `(key, action)`
//! pairs. They live beside the Settings Views that own the keys, not in `Root`.

/// The keys of a Settings page with no row or field cursor of its own (the info pages and the
/// Institutions table, which is mouse-driven for now).
pub(crate) fn settings_plain_page_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scroll()),
        ("h", crate::msg::desktop_hint_index()),
    ]
}
