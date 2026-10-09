//! The status-line legends of the Settings pages and of the dialogs they open, as `(key, action)`
//! pairs. They live beside the Settings Views that own the keys, not in `Shell`.

use crate::view::settings::inventory::InventoryRow;

/// The Settings Payees page's status-line legend: the Payees page's keys without the Transactions
/// hand-off, which stays on the old page.
pub(crate) fn settings_payees_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Settings Documents page's status-line legend (16p's keys), as `(key, action)`.
pub(crate) fn settings_documents_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("J/K", crate::msg::desktop_hint_reorder()),
        ("e", crate::msg::desktop_hint_edit()),
        ("x", crate::msg::desktop_hint_remove()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Settings Inventory page's status-line legend by the selected row (#491), as `(key, action)`.
pub(crate) fn settings_inventory_hints(
    selected: Option<InventoryRow>,
) -> Vec<(&'static str, String)> {
    match selected {
        Some(InventoryRow::Property(_)) => vec![
            ("j/k", crate::msg::desktop_inventory_hint_move()),
            ("\u{2192}/\u{2190}", crate::msg::desktop_hint_expand()),
            ("e", crate::msg::desktop_hint_edit()),
            ("x", crate::msg::desktop_hint_remove()),
            ("n", crate::msg::desktop_inventory_hint_new_property()),
            ("r", crate::msg::desktop_inventory_hint_new_room()),
        ],
        Some(InventoryRow::Room(_)) => vec![
            ("j/k", crate::msg::desktop_inventory_hint_move()),
            ("\u{2190}", crate::msg::desktop_inventory_hint_property()),
            ("J/K", crate::msg::desktop_hint_reorder()),
            ("e", crate::msg::desktop_hint_edit()),
            ("x", crate::msg::desktop_hint_remove()),
            ("r", crate::msg::desktop_inventory_hint_new_room()),
            ("n", crate::msg::desktop_inventory_hint_new_property()),
        ],
        None => vec![("n", crate::msg::desktop_inventory_hint_new_property())],
    }
}

/// The Settings Tags page's status-line legend (2j's keys), as `(key, action)`.
pub(crate) fn settings_tags_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("x", crate::msg::desktop_hint_remove()),
        ("m", crate::msg::desktop_hint_merge()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Settings index rail's keys (the keyboard model's index scope): `j`/`k` swap the page live,
/// `l`/`enter` step into it.
pub(crate) fn settings_index_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_page()),
        ("l/enter", crate::msg::desktop_hint_open()),
    ]
}

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

/// The keys of a Settings page with no row or field cursor of its own (the info pages, General,
/// and the Units and Institutions tables, which are mouse-driven for now).
pub(crate) fn settings_plain_page_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scroll()),
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

/// The status-line legend while the 7e Merge dialog is open.
pub(crate) fn merge_tags_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2191}/\u{2193}", crate::msg::desktop_hint_choose()),
    ]
}

/// The status-line legend while the Add or Edit tag dialog is open (the handoff's 7b/7c), with
/// `←/→ colour` added: the swatch row has no other key. Edit adds `space` for its Active checkbox.
pub(crate) fn tag_dialog_hints(editing: bool) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_colour()),
    ];
    if editing {
        hints.push(("space", crate::msg::desktop_hint_toggle_active()));
    }
    hints
}

/// The status-line legend while an Add or Edit payee dialog is open (the handoff's 6b).
pub(crate) fn payee_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
    ]
}

/// The status-line legend while a plain confirm dialog is open (Delete payee, Remove tag, Clear
/// logs): each has at most the one confirm field, so no `tab`.
pub(crate) fn confirm_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}

/// The Settings Categories page's status-line legend (2i's keys), as `(key, action)`.
pub(crate) fn settings_categories_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("\u{2192}/\u{2190}", crate::msg::desktop_hint_expand()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
        ("N", crate::msg::desktop_hint_sub()),
    ]
}
