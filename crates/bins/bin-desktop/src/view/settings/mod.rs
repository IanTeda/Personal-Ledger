//! The Settings body (`docs/ux/desktop/Settings/README.md`'s "2a resting state", "Body"): one
//! continuous scroll, not a pane switcher -- every section stays mounted, and the settings index
//! rail (`rail::settings_index`) scrolls to a heading rather than swapping views. The shell
//! scaffold (issue #173) laid out every section as a placeholder; each section's real content
//! lands as its own submodule here, one ticket at a time
//! (`crate::settings::SettingsSection::placeholder_issue` names the one still owed).
//!
//! Direct children of the scrollable container, in order: the page heading block (child `0`),
//! then each of the nine sections (children `1..=9`) -- `SettingsSection::body_child_index`
//! documents this offset, since `gpui::ScrollHandle::scroll_to_top_of_item` addresses direct
//! children by index.

mod general;
pub mod ledger_units;

use gpui::{AnyElement, ScrollHandle, SharedString, div, prelude::*, px};

use crate::{
    settings::{BudgetPeriod, DefaultUnit, SettingsSection},
    theme::color,
};

/// Interactive bits a section's own content needs, gathered in one bundle so `render`'s own
/// signature doesn't grow a new positional parameter per section (mirrors `shell::SettingsIndexProps`'s
/// reason for existing). Every section function takes `&SettingsBodyProps` and reads whatever
/// subset it needs -- most of it, today, is only for `ledger_units`.
pub struct SettingsBodyProps {
    pub default_unit: DefaultUnit,
    pub budget_period: BudgetPeriod,
    pub on_default_unit_click: ledger_units::OnDefaultUnitClick,
    pub on_budget_period_click: ledger_units::OnBudgetPeriodClick,
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: SettingsBodyProps,
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
fn section_block(section: SettingsSection, props: &SettingsBodyProps) -> impl IntoElement {
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
                        .child(section.scope_note()),
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

/// Each section's real content, once its own ticket has landed -- everything else still falls
/// back to the placeholder `section_block` originally rendered for all nine.
fn section_content(section: SettingsSection, props: &SettingsBodyProps) -> AnyElement {
    match section {
        SettingsSection::General => general::render(),
        SettingsSection::LedgerUnits => ledger_units::render(
            props.default_unit,
            props.budget_period,
            props.on_default_unit_click.clone(),
            props.on_budget_period_click.clone(),
        ),
        other => placeholder(other),
    }
}

fn placeholder(section: SettingsSection) -> AnyElement {
    div()
        .text_color(color::INK_TERTIARY)
        .child(format!(
            "{} -- not yet built (see issue #{})",
            section.label(),
            section.placeholder_issue()
        ))
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
