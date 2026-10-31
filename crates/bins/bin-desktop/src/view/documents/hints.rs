//! The Documents page's status-line legends, as `(key, action)` pairs: the index rail, the Library
//! list, the Inbox and the dialogs that open over the page.

/// The Library list's legend: the handoff's own strings.
pub(crate) fn library_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("/", lib_locale::msg::hint_search()),
        ("l", crate::msg::desktop_hint_link()),
        ("L", crate::msg::desktop_hint_follow()),
        ("o", crate::msg::desktop_hint_show_in_folder()),
        ("i", crate::msg::desktop_hint_inbox()),
    ]
}

/// The Inbox's legend, also drawn as the key strip under its detail.
pub(crate) fn inbox_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("y", crate::msg::desktop_hint_accept()),
        ("e", crate::msg::desktop_hint_edit()),
        ("x", crate::msg::desktop_hint_skip()),
        ("l", crate::msg::desktop_hint_link_elsewhere()),
        ("L", crate::msg::desktop_hint_follow()),
    ]
}

/// The index rail's legend.
pub(crate) fn index_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scope()),
        ("l/enter", crate::msg::desktop_hint_list()),
        ("i", crate::msg::desktop_hint_inbox()),
    ]
}

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
