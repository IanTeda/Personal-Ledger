//! The Transactions page's status-line legends, as `(key, action)` pairs: the page itself and
//! the filter popover that opens over it.

/// The status-line legend while the filter popover is open (`docs/ux/desktop-mockups/03-transactions/
/// README.md`'s 4b), with `^r reset` added: the bundle's `reset` is a button, and this shell is
/// keyboard-first.
pub(crate) fn filter_hints() -> Vec<(&'static str, String)> {
    vec![
        ("tab", crate::msg::desktop_hint_next_field()),
        ("enter", crate::msg::desktop_hint_apply()),
        ("esc", crate::msg::desktop_hint_cancel()),
        ("^r", crate::msg::desktop_hint_reset()),
    ]
}

/// The Transactions page's status-line legend (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a),
/// without the mockup's `R reconcile` (an Accounts action) and with `/` reading `search` beside a
/// separate `f filter`, since here `/` searches and `f` opens the filter popover.
pub(crate) fn transactions_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit()),
        ("n", crate::msg::desktop_hint_add()),
        ("/", crate::msg::desktop_hint_search()),
        ("f", crate::msg::desktop_hint_filter()),
    ]
}
