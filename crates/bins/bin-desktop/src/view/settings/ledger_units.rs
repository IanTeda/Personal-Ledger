//! The **Ledger & units** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"):
//! two segmented-radio fields, "Default unit for new entries" and "Budget period". Built against
//! the concrete markup -- the shared design system's own `.field`/`.seg`/`.seg-opt` classes
//! (`docs/ux/desktop/Shell & Navigation/styles.css`), not this ticket's own coarser prose
//! ("standard field-label/select pattern, two-column layout... `Field column` spec"). The actual
//! "2a" mockup is a single 300px column of segmented radio controls with `gap:18px` between
//! them, not selects and not two columns -- the same kind of prose/mockup tension issue #148
//! already resolved once, in favour of the concrete markup.
//!
//! Genuinely interactive, unlike General's static fields (issue #175): a segmented control is
//! click-to-select, the same shape every other "selectable row" in this crate already uses (rail
//! rows, the settings index rail), so there was no new interaction pattern to invent here.
//! `Shell` owns the current selection and "saves" it in memory on click.

use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px};

use crate::{
    settings::{BudgetPeriod, DefaultUnit},
    theme::color,
};

pub type OnDefaultUnitClick = Rc<dyn Fn(DefaultUnit, &mut Window, &mut App)>;
pub type OnBudgetPeriodClick = Rc<dyn Fn(BudgetPeriod, &mut Window, &mut App)>;

/// A [`segmented_control`] row's own click handler, generic over which enum it selects.
type OnSegmentClick<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

pub fn render(
    default_unit: DefaultUnit,
    budget_period: BudgetPeriod,
    on_default_unit_click: OnDefaultUnitClick,
    on_budget_period_click: OnBudgetPeriodClick,
) -> gpui::AnyElement {
    div()
        .w(px(300.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(
            div()
                .child(field_label("Default unit for new entries"))
                .child(segmented_control(
                    "ledger-units-default-unit",
                    &DefaultUnit::ALL,
                    default_unit,
                    DefaultUnit::label,
                    on_default_unit_click,
                )),
        )
        .child(
            div()
                .child(field_label("Budget period"))
                .child(segmented_control(
                    "ledger-units-budget-period",
                    &BudgetPeriod::ALL,
                    budget_period,
                    BudgetPeriod::label,
                    on_budget_period_click,
                )),
        )
        .into_any_element()
}

/// `.field > label`: `display:block; font-size:12px; margin-bottom:5px; color: color-mix(text
/// 70%, transparent)` -- the *other* field-label style in this design system, distinct from
/// General's bold "Field label" (`super::field_label`, `font-weight:800; margin-bottom:6px`).
/// `.field`-family labels sit over segmented controls, never over a text `Input`/`select`.
fn field_label(label: &'static str) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .mb(px(5.0))
        .text_color(color::INK_SECONDARY)
        .child(label)
}

/// `.seg`/`.seg-opt`: a bordered, radius-0 pill row, each option separated by a 1px rule, the
/// selected option taking `background: var(--color-accent); color: var(--color-bg)` -- the
/// segmented control's own named exception to the shell's "accent never a background" rule (see
/// `theme::color::ACCENT`'s own doc).
fn segmented_control<T: Copy + PartialEq + 'static>(
    id_prefix: &'static str,
    options: &'static [T],
    current: T,
    label: fn(T) -> &'static str,
    on_click: OnSegmentClick<T>,
) -> impl IntoElement {
    div()
        .flex()
        .border_1()
        .border_color(color::DIVIDER)
        .children(options.iter().enumerate().map(|(index, &option)| {
            let selected = option == current;
            let on_click = on_click.clone();
            div()
                .id(SharedString::from(format!("{id_prefix}-{index}")))
                .cursor_pointer()
                .py(px(7.0))
                .px(px(12.0))
                .text_size(px(13.0))
                .when(index > 0, |this| {
                    this.border_l(px(1.0)).border_color(color::DIVIDER)
                })
                .when(selected, |this| {
                    this.bg(color::ACCENT).text_color(color::INK_ON_DARK)
                })
                .on_click(move |_event, window, cx| on_click(option, window, cx))
                .child(label(option))
        }))
}
