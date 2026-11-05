//! The status-line legends of the Settings pages and of the dialogs they open, as `(key, action)`
//! pairs. They live beside the Settings Views that own the keys, not in `Root`.

/// The Display page's keys while a control above the Colour Theme grid has focus.
pub(crate) fn settings_display_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_field()),
        ("h/l", crate::msg::desktop_hint_change()),
        ("enter", crate::msg::desktop_hint_toggle()),
        ("esc", crate::msg::desktop_hint_index()),
    ]
}

/// The Display page's keys while the Colour Theme grid has focus.
pub(crate) fn settings_colour_grid_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/j/k/l", crate::msg::desktop_hint_colour()),
        ("enter", crate::msg::desktop_hint_choose()),
        ("esc", crate::msg::desktop_hint_field()),
    ]
}

/// The keys of a Settings page with no row or field cursor of its own (the info pages and the
/// Institutions table, which is mouse-driven for now).
pub(crate) fn settings_plain_page_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scroll()),
        ("h", crate::msg::desktop_hint_index()),
    ]
}

/// The General page's keys: `j`/`k` walk On start and Toasts, `h`/`l` change the one focused.
pub(crate) fn settings_general_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_field()),
        ("h/l", crate::msg::desktop_hint_change()),
        ("esc", crate::msg::desktop_hint_index()),
    ]
}

/// The Ledger page's keys: `j`/`k` change the Base unit select.
pub(crate) fn settings_ledger_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_change()),
        ("h", crate::msg::desktop_hint_index()),
    ]
}

/// The Tracing page's keys while it has focus: the level radios, scrolling the log box, and
/// Clear logs.
pub(crate) fn settings_tracing_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/l", crate::msg::desktop_hint_level()),
        ("j/k", crate::msg::desktop_hint_scroll()),
        ("c", crate::msg::desktop_hint_clear_logs()),
        ("esc", crate::msg::desktop_hint_index()),
    ]
}

/// The status-line legend while a plain confirm dialog is open (Remove tag, Clear logs): each has
/// at most the one confirm field, so no `tab`.
pub(crate) fn confirm_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}
