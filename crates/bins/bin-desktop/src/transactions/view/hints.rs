//! The Transactions page's status-line legend, as `(key, action)` pairs.

/// The Transactions page's status-line legend (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a),
/// without the mockup's `R reconcile` (an Accounts action) and with `/` reading `search` beside a
/// separate `f filter`, since here `/` searches and `f` opens the filter popover.
pub(crate) fn transactions_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit()),
        ("n", crate::msg::desktop_hint_add()),
        ("/", lib_locale::msg::hint_search()),
        ("f", crate::msg::desktop_hint_filter()),
    ]
}
