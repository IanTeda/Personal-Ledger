//! The status-line legends of the Budgets Dialogs still on the legacy host (#955), which replace
//! the page's while one is open (`BudgetsDialog::hints`, ADR-0037).

/// The status-line legend while the Category detail dialog (9d) is open.
pub(crate) fn budgets_detail_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_close()),
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("t", crate::msg::desktop_hint_transactions()),
    ]
}

/// The status-line legend while Edit budget (9e) is open.
pub(crate) fn budgets_limit_hints() -> Vec<(&'static str, String)> {
    vec![
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_save()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The status-line legend while Fill (9f) is open.
pub(crate) fn budgets_fill_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_source()),
        ("enter", crate::msg::desktop_hint_fill()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The status-line legend while Stop budgeting (9g) is open.
pub(crate) fn budgets_stop_hints() -> Vec<(&'static str, String)> {
    vec![
        ("\u{2191}/\u{2193}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}
