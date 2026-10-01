//! The Settings body (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Body"): one page
//! at a time, not a continuous scroll. The settings index rail (`rail::settings_index`) swaps the
//! page, and only the selected page is mounted. Each page keeps the same anatomy: the "Settings"
//! heading and scope note over a 2px rule, then the page's own heading with its right-aligned
//! meta over another, then the page's content.

mod about;
pub mod accounts;
pub mod add_institution_dialog;
pub mod add_unit_dialog;
pub mod categories;
pub mod colour_theme;
pub mod data_backup;
pub mod delete_unit_dialog;
pub mod display;
pub mod edit_unit_dialog;
mod general;
pub mod institutions;
pub mod payees;
pub mod sync_server;
pub mod tags;
pub mod tracing;
pub mod units;

use gpui::{AnyElement, App, ScrollHandle, SharedString, div, prelude::*, px};

use lib_core::DateStyle;

use crate::{
    nav::Noun,
    settings::{
        InstitutionRow, PriceSourceRow, RowDensity, SettingsSection, StatusGlyphs, TracingLevel,
        UnitRow,
    },
    theme::color,
};

/// Interactive bits a section's own content needs, gathered in one bundle so `render`'s own
/// signature doesn't grow a new positional parameter per section (mirrors `shell::SettingsPanelProps`'s
/// reason for existing). Every section function takes `&SettingsBodyProps` and reads whatever
/// subset it needs.
pub struct SettingsBodyProps<'a> {
    /// The page on show.
    pub selected: SettingsSection,
    /// Whether the page (not the index) holds keyboard focus.
    pub page_focused: bool,
    pub accounts: accounts::AccountsPageProps<'a>,
    pub categories: categories::CategoriesPageProps<'a>,
    pub tags: tags::TagsPageProps<'a>,
    pub payees: payees::PayeesPageProps<'a>,
    pub date_style: Option<DateStyle>,
    pub row_density: RowDensity,
    pub status_glyphs: StatusGlyphs,
    pub start_sidebar_minimised: bool,
    pub on_start_sidebar_minimised_click: display::OnPlainClick,
    pub on_date_style_click: display::OnDateStyleClick,
    pub on_row_density_click: display::OnRowDensityClick,
    pub on_status_glyphs_click: display::OnStatusGlyphsClick,
    pub toasts_on: bool,
    pub on_toasts_click: display::OnToastsClick,
    /// The Colour Theme card the keyboard is on, when the grid has focus.
    pub colour_theme_focus: Option<usize>,
    /// The Display page's keyboard-focused control, when the page has focus and not the grid.
    pub display_field: Option<usize>,
    pub on_colour_theme_click: colour_theme::OnColourThemeClick,
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
    cx: &App,
) -> AnyElement {
    div()
        .id("settings-body")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .py(px(22.0))
        .px(px(28.0))
        .flex()
        .flex_col()
        .child(page_heading(cx))
        .child(section_block(props.selected, &props, cx))
        .into_any_element()
}

fn page_heading(cx: &App) -> impl IntoElement {
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
                        .text_color(color::foreground(cx))
                        .child(Noun::Settings.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(crate::msg::desktop_settings_scope_preferences()),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::structural_rule(cx))
                .mt(px(14.0))
                .mb(px(18.0)),
        )
}

/// One page: heading + right-aligned scope note, a 2px rule, the page's content, then a **48px**
/// bottom gap -- every page, no exceptions (README's implementation note 4: mixed top/bottom
/// margin ownership is how this gap goes missing, so it's carried on a single edge, here).
fn section_block(
    section: SettingsSection,
    props: &SettingsBodyProps<'_>,
    cx: &App,
) -> impl IntoElement {
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
                        .text_color(color::foreground(cx))
                        .child(section.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(scope_meta(section, props, cx)),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .bg(color::structural_rule(cx))
                .mt(px(14.0))
                .mb(px(18.0)),
        )
        .child(section_content(section, props, cx))
}

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The section heading's own right-aligned scope note. Static for every section except Units,
/// which the mockup gives a **dynamic**, row-count-based note ("3 units · synced") instead of
/// the generic "synced · change sets" `SettingsSection::scope_note` otherwise returns --
/// confirmed against the raw markup, not just the README's coarser Components table.
fn scope_note(section: SettingsSection, props: &SettingsBodyProps<'_>) -> String {
    match section {
        SettingsSection::Units => {
            crate::msg::desktop_settings_scope_units(count(props.units.len()))
        }
        SettingsSection::Institutions => {
            crate::msg::desktop_settings_scope_institutions(count(props.institutions.len()))
        }
        SettingsSection::Accounts => {
            accounts::scope_note(props.accounts.accounts, props.accounts.units)
        }
        SettingsSection::Categories => categories::scope_note(props.categories.categories),
        other => other.scope_note(),
    }
}

/// The heading's meta element: the plain scope note, except Tags, whose duplicate count is drawn
/// in the accent text colour.
fn scope_meta(section: SettingsSection, props: &SettingsBodyProps<'_>, cx: &App) -> AnyElement {
    match section {
        SettingsSection::Tags => tags::scope_note(props.tags.tags, props.tags.groups, cx),
        SettingsSection::Payees => payees::scope_note(props.payees.payees, cx),
        other => scope_note(other, props).into_any_element(),
    }
}

/// Each page's content. Every moved noun now has its page.
fn section_content(
    section: SettingsSection,
    props: &SettingsBodyProps<'_>,
    cx: &App,
) -> AnyElement {
    match section {
        SettingsSection::General => general::render(cx),
        SettingsSection::Display => display::render(
            props.date_style,
            props.row_density,
            props.status_glyphs,
            props.start_sidebar_minimised,
            props.on_start_sidebar_minimised_click.clone(),
            props.on_date_style_click.clone(),
            props.on_row_density_click.clone(),
            props.on_status_glyphs_click.clone(),
            props.toasts_on,
            props.on_toasts_click.clone(),
            props.colour_theme_focus,
            props.display_field,
            props.on_colour_theme_click.clone(),
            cx,
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
            cx,
        ),
        SettingsSection::Institutions => institutions::render(
            props.institutions,
            props.on_institution_edit_click.clone(),
            props.on_institution_delete_click.clone(),
            props.on_add_institution_click.clone(),
            cx,
        ),
        SettingsSection::Accounts => accounts::render(&props.accounts, props.page_focused, cx),
        SettingsSection::Categories => {
            categories::render(&props.categories, props.page_focused, cx)
        }
        SettingsSection::Tags => tags::render(&props.tags, props.page_focused, cx),
        SettingsSection::Payees => payees::render(&props.payees, props.page_focused, cx),
        SettingsSection::SyncServer => sync_server::render(props.on_sync_now_click.clone(), cx),
        SettingsSection::DataBackup => data_backup::render(
            props.on_backup_now_click.clone(),
            props.on_export_ledger_click.clone(),
            cx,
        ),
        SettingsSection::Tracing => tracing::render(
            props.tracing_level,
            props.log_lines,
            props.on_tracing_level_click.clone(),
            props.on_clear_logs_click.clone(),
            cx,
        ),
        SettingsSection::About => about::render(cx),
    }
}

/// A page that isn't rebuilt yet.
fn placeholder(cx: &App) -> AnyElement {
    div()
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_settings_page_placeholder())
        .into_any_element()
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
pub(super) fn field_value(
    value: impl Into<SharedString>,
    select_style: bool,
    cx: &App,
) -> impl IntoElement {
    div()
        .w_full()
        .py(px(8.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(13.0))
        .when(select_style, |this| this.bg(color::background(cx)))
        .child(value.into())
}
