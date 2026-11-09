//! Draws the Schedule tab's filter row and stat callout (`docs/ux/desktop-mockups/12-bills/README.md`'s
//! 8a, carrying over 8f's History pieces in #381): status chips, then the Bill, Category and
//! Account selects, and the Last paid / Average / Same period last year figures while scoped to
//! one Bill Plan. The selects are the dialogs' shared dropdown, opening inline beneath the row;
//! the Presenter holds which is focused and its open list, and every click is its event.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_bills::presenter::{
    BillsPageEvent, ChipView, FilterField, FiltersView, ScheduleEvent, StatsView,
};

use super::page::BillsPage;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::page_clicks::PageClicks;
use crate::theme::color;

const SELECT_WIDTH: gpui::Pixels = px(180.0);

fn select_id(field: FilterField) -> &'static str {
    match field {
        FilterField::Plan => "bills-filter-plan",
        FilterField::Category => "bills-filter-category",
        FilterField::Account => "bills-filter-account",
    }
}

/// The filter row, then the stat callout while scoped to one Bill Plan. `window` is the open
/// list's window, which the page keeps so it follows the highlight.
pub(super) fn render(
    view: &FiltersView,
    stats: Option<&StatsView>,
    window: &mut usize,
    clicks: &PageClicks<BillsPage>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(filter_row(view, window, clicks, cx))
        .children(stats.map(|stats| stat_callout(stats, cx)))
        .into_any_element()
}

fn filter_row(
    view: &FiltersView,
    window: &mut usize,
    clicks: &PageClicks<BillsPage>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .items_end()
        .gap(px(8.0))
        .mb(px(20.0))
        .children(
            view.chips
                .iter()
                .enumerate()
                .map(|(index, chip_view)| chip(index, chip_view, clicks, cx)),
        )
        .child(
            div()
                .w(px(1.0))
                .h(px(24.0))
                .mx(px(4.0))
                .mb(px(6.0))
                .bg(color::border(cx)),
        )
        .children(view.selects.iter().map(|select| {
            let field = select.field;
            let options = labels(&select.view);
            // Only the focused select's list is ever open, and only its window is kept.
            let mut closed = 0;
            let scroll = if select.list.is_open() {
                &mut *window
            } else {
                &mut closed
            };
            let state = drawn(&select.view, select.list, scroll);
            div()
                .w(SELECT_WIDTH)
                .flex_none()
                .child(select_field::render(
                    SelectFieldProps {
                        id: select_id(field).into(),
                        label: select.view.label.clone().into(),
                        options: &options,
                        state: &state,
                        focused: select.view.focused,
                        accent: select.accent,
                        read_only: None,
                        on_field_click: clicks
                            .click(BillsPageEvent::Schedule(ScheduleEvent::ToggleFilter(field))),
                        on_option_click: clicks.option(move |index| {
                            BillsPageEvent::Schedule(ScheduleEvent::FilterOption(field, index))
                        }),
                    },
                    cx,
                ))
        }))
}

/// A status chip: dark and `✓`-marked while on, outlined while off.
fn chip(index: usize, view: &ChipView, clicks: &PageClicks<BillsPage>, cx: &App) -> AnyElement {
    let on_click = clicks.send(BillsPageEvent::Schedule(ScheduleEvent::ToggleChip(
        view.status,
    )));
    let hover = color::hover(cx);
    let label = view.label.clone();
    div()
        .id(SharedString::from(format!("bills-filter-chip-{index}")))
        .debug_selector(move || format!("bills-filter-chip-{index}"))
        .cursor_pointer()
        .flex_none()
        .py(px(8.0))
        .px(px(12.0))
        .border_1()
        .text_size(px(12.0))
        .when(view.on, |this| {
            this.bg(color::selection_background(cx))
                .border_color(color::selection_background(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(format!("\u{2713} {label}"))
        })
        .when(!view.on, |this| {
            this.border_color(color::border(cx))
                .text_color(color::muted(cx))
                .hover(move |style| style.bg(hover))
                .child(label.clone())
        })
        .on_click(on_click)
        .into_any_element()
}

/// The scoped Bill Plan's figures.
fn stat_callout(view: &StatsView, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(24.0))
        .mb(px(20.0))
        .py(px(14.0))
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::foreground(cx))
        .text_size(px(13.0))
        .children(view.stats.iter().map(|stat| {
            div()
                .flex()
                .items_baseline()
                .gap(px(4.0))
                .child(div().text_color(color::muted(cx)).child(stat.label.clone()))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::foreground(cx))
                        .child(stat.figure.clone()),
                )
                .children(stat.note.as_ref().map(|note| {
                    div()
                        .text_color(color::faint_text(cx))
                        .child(format!("\u{b7} {note}"))
                }))
        }))
        .children(view.never_paid.as_ref().map(|never_paid| {
            div()
                .text_color(color::faint_text(cx))
                .child(never_paid.clone())
        }))
}
