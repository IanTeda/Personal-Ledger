//! The **Data & backup** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): a
//! 300px column of label/value rows (Store location, Last backup), then two buttons -- **Backup
//! now** and **Export ledger (CSV)** -- same plain, unbordered row shape as
//! `view::settings::sync_server` (a distinct component from `general::summary_panel`'s bordered
//! rows, per that module's own doc).
//!
//! Both buttons are permanently out of scope for this map (the map's own Out-of-scope: stand-in
//! handlers with no real effect) rather than "not yet built" behind a future ticket -- so, like
//! `sync_server::sync_now_button`, their stub status-line messages name no issue.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use crate::theme::color;

pub type OnBackupNowClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnExportLedgerClick = Rc<dyn Fn(&mut Window, &mut App)>;

const COLUMN_WIDTH: gpui::Pixels = px(300.0);

pub fn render(
    on_backup_now_click: OnBackupNowClick,
    on_export_ledger_click: OnExportLedgerClick,
) -> AnyElement {
    div()
        .w(COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(row(
            crate::msg::desktop_settings_backup_location(),
            "~/.ledger/personal",
            px(11.0),
        ))
        .child(row(
            crate::msg::desktop_settings_backup_last(),
            "12 sep 2026 \u{b7} 23:10",
            px(12.0),
        ))
        .child(button(
            "settings-backup-now",
            crate::msg::desktop_settings_backup_now(),
            true,
            move |window, cx| on_backup_now_click(window, cx),
        ))
        .child(button(
            "settings-export-ledger",
            crate::msg::desktop_settings_backup_export(),
            false,
            move |window, cx| on_export_ledger_click(window, cx),
        ))
        .into_any_element()
}

fn row(label: String, value: &'static str, value_size: gpui::Pixels) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .items_center()
        .py(px(8.0))
        .child(div().text_color(color::INK_SECONDARY).child(label))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(value_size)
                .child(value),
        )
}

/// A section button: `padding:8px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800` -- `top_gap` adds the mockup's own `margin-top:8px` on
/// the first button only; the second relies on the column's own `gap:12px` alone, matching the
/// raw markup exactly (no inline margin on "Export ledger (CSV)").
fn button(
    id: &'static str,
    label: String,
    top_gap: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut button = div()
        .id(id)
        .cursor_pointer()
        .when(top_gap, |this| this.mt(px(8.0)))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::CHROME)
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label);
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
