//! The Accounts page's status-line legend, as `(key, action)` pairs.

/// The Accounts page's status-line legend (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3a), as
/// `(key, action)`.
pub(crate) fn accounts_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open_ledger()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}
