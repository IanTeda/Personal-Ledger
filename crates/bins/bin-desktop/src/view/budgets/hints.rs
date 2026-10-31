//! The Budgets status-line legends: each tab's own (`BudgetsView::page_status`) and each Budgets
//! Dialog's, which replace them while it is open (`BudgetsDialog::hints`, ADR-0037).

/// The Budgets Progress tab's status-line legend (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s
/// 9a).
pub(crate) fn budgets_progress_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("s", crate::msg::desktop_hint_stop_budgeting()),
        ("B", crate::msg::desktop_hint_switch_budget()),
        ("[/]", crate::msg::desktop_hint_period()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Budgets Plan tab's status-line legend in Normal mode (9b).
pub(crate) fn budgets_plan_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/j/k/l", crate::msg::desktop_hint_cell()),
        ("enter", crate::msg::desktop_hint_edit()),
        ("r", crate::msg::desktop_hint_rollover()),
        ("x", crate::msg::desktop_hint_clear()),
        ("f", crate::msg::desktop_hint_fill()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Plan grid's legend while a cell is being typed into.
pub(crate) fn budgets_plan_insert_hints() -> Vec<(&'static str, String)> {
    vec![
        ("enter", crate::msg::desktop_hint_save_cell()),
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_next_month()),
        ("shift+enter", crate::msg::desktop_hint_this_month_only()),
    ]
}

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

/// The Budgets History tab's status-line legend (9c).
pub(crate) fn budgets_history_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("h/l", crate::msg::desktop_hint_month()),
        ("enter", crate::msg::desktop_hint_open()),
        ("x", crate::msg::desktop_hint_export()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The status-line legend while the Switcher (11b) is open.
pub(crate) fn budgets_switcher_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("/", lib_locale::msg::hint_search()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("esc", crate::msg::desktop_hint_close()),
    ]
}

/// The status-line legend while Manage budgets (11f) is open.
pub(crate) fn budgets_manage_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_duplicate()),
        ("*", crate::msg::desktop_hint_set_default()),
        ("x", crate::msg::desktop_hint_archive()),
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
