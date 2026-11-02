//! The legacy Documents Dialogs' status-line legends, as `(key, action)` pairs. The page's own
//! legends are its Presenter's (`lib_documents::presenter`).

/// The link picker's legend.
pub(crate) fn picker_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_kind()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}

/// A form dialog's legend; Import confirms with `ctrl+enter`, since `enter` types a newline.
pub(crate) fn dialog_hints(import: bool) -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_next_field()),
        (
            if import { "ctrl+enter" } else { "enter" },
            crate::msg::desktop_hint_confirm(),
        ),
    ]
}
