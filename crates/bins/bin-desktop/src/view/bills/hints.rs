//! The status-line legends of the Bills tabs and of the Pay and Skip dialogs, as `(key, action)`
//! pairs, beside the Bills Views that own the keys.

/// The Bills Schedule tab's status-line legend (`docs/ux/desktop-mockups/12-bills/README.md`'s 8a), with
/// `[/]` and `0` added for the period nav and its All toggle, and 8f's `1–5` and `f` for the
/// filter row it absorbed (#381).
pub(crate) fn bills_schedule_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("p", crate::msg::desktop_hint_pay()),
        ("s", crate::msg::desktop_hint_skip()),
        ("[/]", crate::msg::desktop_hint_period()),
        ("0", crate::msg::desktop_hint_all()),
        ("1\u{2013}5", crate::msg::desktop_hint_status_chips()),
        ("f", crate::msg::desktop_hint_filters()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Bills Planner tab's status-line legend (`docs/ux/desktop-mockups/12-bills/README.md`'s 8b).
pub(crate) fn bills_planner_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("n", crate::msg::desktop_hint_new()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The status-line legend while the Pay dialog is open.
pub(crate) fn pay_bill_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("j/k", crate::msg::desktop_hint_choose()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_switch_panel()),
    ]
}

/// The status-line legend while the Skip dialog is open (the handoff's 8e).
pub(crate) fn skip_bill_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}
