//! The Settings body (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Body"): one
//! continuous scroll, not a pane switcher -- every section stays mounted, and the settings index
//! rail (`rail::settings_index`) scrolls to a heading rather than swapping views. The shell
//! scaffold (issue #173) laid out every section as a placeholder; each section's real content
//! landed as its own submodule here, one ticket at a time, with issue #179 (Display) the last
//! (`crate::settings::SettingsSection::placeholder_issue` still names each section's own
//! building ticket, for reference).
//!
//! Direct children of the scrollable container, in order: the page heading block (child `0`),
//! then each of the eight sections (children `1..=8`) -- `SettingsSection::body_child_index`
//! documents this offset, since `gpui::ScrollHandle::scroll_to_top_of_item` addresses direct
//! children by index.

mod about;
pub mod add_institution_dialog;
pub mod add_unit_dialog;
pub mod data_backup;
pub mod delete_unit_dialog;
pub mod display;
pub mod edit_unit_dialog;
mod general;
pub mod institutions;
pub mod sync_server;
pub mod tracing;
pub mod units;

use gpui::{AnyElement, ScrollHandle, SharedString, div, prelude::*, px};

use crate::{
    settings::{
        DateFormat, DecimalSeparator, InstitutionRow, PriceSourceRow, RowDensity, SettingsSection,
        StatusGlyphs, TracingLevel, UnitRow,
    },
    theme::color,
};

/// Interactive bits a section's own content needs, gathered in one bundle so `render`'s own
/// signature doesn't grow a new positional parameter per section (mirrors `shell::SettingsPanelProps`'s
/// reason for existing). Every section function takes `&SettingsBodyProps` and reads whatever
/// subset it needs.
pub struct SettingsBodyProps<'a> {
    pub date_format: DateFormat,
    pub decimal_separator: DecimalSeparator,
    pub row_density: RowDensity,
    pub status_glyphs: StatusGlyphs,
    pub start_sidebar_minimised: bool,
    pub on_start_sidebar_minimised_click: display::OnPlainClick,
    pub on_date_format_click: display::OnDateFormatClick,
    pub on_decimal_separator_click: display::OnDecimalSeparatorClick,
    pub on_row_density_click: display::OnRowDensityClick,
    pub on_status_glyphs_click: display::OnStatusGlyphsClick,
    pub units: &'a [UnitRow],
    pub on_unit_edit_click: units::OnRowIndexClick,
    pub on_unit_delete_click: units::OnRowIndexClick,
    pub on_add_unit_click: units::OnAddClick,
    pub price_sources: &'a [PriceSourceRow],
    pub on_price_source_test_click: units::OnRowIndexClick,
    pub on_price_source_edit_click: units::OnRowIndexClick,
    pub on_price_source_delete_click: units::OnRowIndexClick,
    pub on_add_price_source_click: units::OnAddClick,
    pub institutions: &'a [InstitutionRow],
    pub on_institution_edit_click: institutions::OnRowIndexClick,
    pub on_institution_delete_click: institutions::OnRowIndexClick,
    pub on_add_institution_click: institutions::OnAddClick,
    pub on_sync_now_click: sync_server::OnSyncNowClick,
    pub on_backup_now_click: data_backup::OnBackupNowClick,
    pub on_export_ledger_click: data_backup::OnExportLedgerClick,
    pub tracing_level: TracingLevel,
    pub log_lines: &'a [&'static str],
    pub on_tracing_level_click: tracing::OnLevelClick,
    pub on_clear_logs_click: tracing::OnClearLogsClick,
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: SettingsBodyProps<'_>,
) -> gpui::AnyElement {
    div()
        .id("settings-body")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .py(px(22.0))
        .px(px(28.0))
        .flex()
        .flex_col()
        .child(page_heading())
        .children(
            SettingsSection::ALL
                .into_iter()
                .map(|section| section_block(section, &props)),
        )
        .into_any_element()
}

fn page_heading() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::INK)
                        .child("Settings"),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::INK_TERTIARY)
                        .child("preferences · synced"),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(18.0)),
        )
}

/// One section wrapper: heading + right-aligned scope note, a 2px rule, placeholder content,
/// then a **48px** bottom gap -- every section, no exceptions (README's implementation note 4:
/// mixed top/bottom margin ownership is how this gap goes missing, so it's carried on a single
/// edge, here).
fn section_block(section: SettingsSection, props: &SettingsBodyProps<'_>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mb(px(48.0))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(20.0))
                        .text_color(color::INK)
                        .child(section.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::INK_TERTIARY)
                        .child(scope_note(section, props)),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(18.0)),
        )
        .child(section_content(section, props))
}

/// The section heading's own right-aligned scope note. Static for every section except Units,
/// which the mockup gives a **dynamic**, row-count-based note ("3 units · synced") instead of
/// the generic "synced · change sets" `SettingsSection::scope_note` otherwise returns --
/// confirmed against the raw markup, not just the README's coarser Components table.
fn scope_note(section: SettingsSection, props: &SettingsBodyProps<'_>) -> String {
    match section {
        SettingsSection::Units => format!("{} units \u{b7} synced", props.units.len()),
        SettingsSection::Institutions => {
            format!("{} institutions \u{b7} synced", props.institutions.len())
        }
        other => other.scope_note().to_string(),
    }
}

/// Each section's real content -- issue #179 (Display) was the last section still on the
/// placeholder `section_block` originally rendered for all nine; every `SettingsSection` variant
/// now has a real arm here, so there is no longer a catch-all fallback.
fn section_content(section: SettingsSection, props: &SettingsBodyProps<'_>) -> AnyElement {
    match section {
        SettingsSection::General => general::render(),
        SettingsSection::Display => display::render(
            props.date_format,
            props.decimal_separator,
            props.row_density,
            props.status_glyphs,
            props.start_sidebar_minimised,
            props.on_start_sidebar_minimised_click.clone(),
            props.on_date_format_click.clone(),
            props.on_decimal_separator_click.clone(),
            props.on_row_density_click.clone(),
            props.on_status_glyphs_click.clone(),
        ),
        SettingsSection::Units => units::render(
            props.units,
            props.on_unit_edit_click.clone(),
            props.on_unit_delete_click.clone(),
            props.on_add_unit_click.clone(),
            props.price_sources,
            props.on_price_source_test_click.clone(),
            props.on_price_source_edit_click.clone(),
            props.on_price_source_delete_click.clone(),
            props.on_add_price_source_click.clone(),
        ),
        SettingsSection::Institutions => institutions::render(
            props.institutions,
            props.on_institution_edit_click.clone(),
            props.on_institution_delete_click.clone(),
            props.on_add_institution_click.clone(),
        ),
        SettingsSection::SyncServer => sync_server::render(props.on_sync_now_click.clone()),
        SettingsSection::DataBackup => data_backup::render(
            props.on_backup_now_click.clone(),
            props.on_export_ledger_click.clone(),
        ),
        SettingsSection::Tracing => tracing::render(
            props.tracing_level,
            props.log_lines,
            props.on_tracing_level_click.clone(),
            props.on_clear_logs_click.clone(),
        ),
        SettingsSection::About => about::render(),
    }
}

/// A field label: `display:block; font-weight:800; font-size:12px; margin-bottom:6px`
/// (`docs/ux/desktop/Settings/README.md`'s Components table) -- shared by every section that
/// lays out `Field label` + `Input/select` pairs.
pub(super) fn field_label(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(label.into())
}

/// An `Input`/`select`-styled box showing `value` as static text: `width:100%; padding:8px 10px;
/// border:1px solid rgba(32,30,29,.30); font-size:13px` -- `select_style` adds the `select`
/// row's own `background:#f3f2f2` (README: "selects add `background:#f3f2f2`").
///
/// Deliberately **not** a real editable text input or an openable dropdown -- see this map's own
/// Destination: every section here is stubbed/dummy data, and free-text editing/dropdown-open
/// behaviour is real interaction-pattern work with no existing precedent in this crate yet
/// (`gpui-component` ships an `Input` widget, but adopting it is a bigger, crate-wide styling
/// decision than one section's ticket should make on its own -- left for whichever future
/// ticket needs it first). "Save on change" therefore has nothing to save yet.
pub(super) fn field_value(value: impl Into<SharedString>, select_style: bool) -> impl IntoElement {
    div()
        .w_full()
        .py(px(8.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::BORDER)
        .text_size(px(13.0))
        .when(select_style, |this| this.bg(color::GROUND))
        .child(value.into())
}
