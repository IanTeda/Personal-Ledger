//! Settings › Ledger drawn from its view model: the field column (Ledger name, Owner, Financial
//! year starts, Base unit) beside the THIS LEDGER summary. The text fields draw the draft and a
//! caret while edited; the selects are the shared dropdown, their lists the Presenter's. Each
//! click is a Presenter event through [`PageClicks`]; a click on the page away from the fields
//! saves what is open.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_core::FinancialYearStart;
use lib_settings::LedgerEvent;
use lib_settings::presenter::{
    BaseUnitView, FinancialYearView, LedgerField, LedgerTextView, LedgerViewModel,
};

use super::{LedgerPage, LedgerWidgets};
use crate::form::fields::field_label;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::shell::view::page_clicks::PageClicks;
use crate::theme::color;

/// The page drawn from `model`.
pub(super) fn page(
    model: &LedgerViewModel,
    clicks: &PageClicks<LedgerPage>,
    widgets: &mut LedgerWidgets,
    cx: &App,
) -> AnyElement {
    div()
        .id("settings-ledger")
        .debug_selector(|| "settings-ledger".to_string())
        .flex()
        .gap(px(40.0))
        .on_click(clicks.send(LedgerEvent::ClickAway))
        .child(field_column(model, clicks, widgets, cx))
        .child(summary_panel(model, cx))
        .into_any_element()
}

/// The 320px field column: Ledger name, Owner, Financial year starts, Base unit -- in the
/// mockup's own order.
fn field_column(
    model: &LedgerViewModel,
    clicks: &PageClicks<LedgerPage>,
    widgets: &mut LedgerWidgets,
    cx: &App,
) -> impl IntoElement {
    div()
        .w(px(320.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(text_field(
            "settings-ledger-name",
            &model.name,
            LedgerField::Name,
            clicks,
            cx,
        ))
        .child(text_field(
            "settings-ledger-owner",
            &model.owner,
            LedgerField::Owner,
            clicks,
            cx,
        ))
        .child(financial_year_select(
            &model.financial_year,
            clicks,
            &mut widgets.financial_year_scroll,
            cx,
        ))
        .child(base_unit_select(
            &model.base_unit,
            clicks,
            &mut widgets.base_unit_scroll,
            cx,
        ))
}

/// A text field: what is stored, or the draft and a caret while it is edited. The border is the
/// accent while it has the keyboard, the negative colour once a save was refused.
fn text_field(
    id: &'static str,
    view: &LedgerTextView,
    field: LedgerField,
    clicks: &PageClicks<LedgerPage>,
    cx: &App,
) -> impl IntoElement {
    let caret = if view.editing { "\u{2502}" } else { "" };
    let (text, text_color) = if view.text.is_empty() && !view.editing {
        (view.placeholder.clone(), color::faint_text(cx))
    } else {
        (format!("{}{caret}", view.text), color::foreground(cx))
    };
    let border = if view.refused {
        color::negative(cx)
    } else if view.focused {
        color::accent(cx)
    } else {
        color::border(cx)
    };
    div().child(field_label(view.label.clone())).child(
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_text()
            .w_full()
            .min_h(px(34.0))
            .py(px(8.0))
            .px(px(10.0))
            .border_1()
            .border_color(border)
            .bg(color::background(cx))
            .text_size(px(13.0))
            .text_color(text_color)
            .on_click({
                let press = pressed(clicks, LedgerEvent::Press(field));
                move |_event, window, cx| {
                    press(window, cx);
                    cx.stop_propagation();
                }
            })
            .child(text),
    )
}

/// The *Financial year starts* select: the twelve months, the chosen one shown.
fn financial_year_select(
    view: &FinancialYearView,
    clicks: &PageClicks<LedgerPage>,
    scroll: &mut usize,
    cx: &App,
) -> AnyElement {
    let state = drawn(
        view.options.get(view.chosen).cloned(),
        view.highlight,
        view.options.len(),
        scroll,
    );
    select(
        SelectFieldProps {
            id: "settings-ledger-financial-year".into(),
            label: view.label.clone().into(),
            options: &view.options,
            state: &state,
            focused: view.focused,
            accent: false,
            read_only: None,
            on_field_click: pressed(clicks, LedgerEvent::Press(LedgerField::FinancialYear)),
            on_option_click: clicks.option(|index| {
                let month = u32::try_from(index + 1).unwrap_or(1);
                LedgerEvent::ChooseMonth(
                    FinancialYearStart::new(month).unwrap_or(FinancialYearStart::JANUARY),
                )
            }),
        },
        cx,
    )
}

/// The *Base unit* select: every active Unit, the base one shown.
fn base_unit_select(
    view: &BaseUnitView,
    clicks: &PageClicks<LedgerPage>,
    scroll: &mut usize,
    cx: &App,
) -> AnyElement {
    let labels: Vec<String> = view.options.iter().map(|o| o.label.clone()).collect();
    let ids: Vec<_> = view.options.iter().map(|o| o.id).collect();
    let chosen = view
        .options
        .iter()
        .find(|option| Some(option.id) == view.chosen)
        .map(|option| option.label.clone());
    let state = drawn(chosen, view.highlight, labels.len(), scroll);
    let fallback = view.chosen;
    select(
        SelectFieldProps {
            id: "settings-ledger-base-unit".into(),
            label: view.label.clone().into(),
            options: &labels,
            state: &state,
            focused: view.focused,
            accent: false,
            read_only: None,
            on_field_click: pressed(clicks, LedgerEvent::Press(LedgerField::BaseUnit)),
            on_option_click: clicks.option(move |index| {
                match ids.get(index).copied().or(fallback) {
                    Some(id) => LedgerEvent::ChooseBaseUnit(id),
                    None => LedgerEvent::ClickAway,
                }
            }),
        },
        cx,
    )
}

/// A click on a field: `event`, then the page takes the keyboard, as a row's click hands it.
fn pressed(clicks: &PageClicks<LedgerPage>, event: LedgerEvent) -> crate::dialog::OnClick {
    let send = clicks.row::<_, ()>(event);
    std::rc::Rc::new(move |window, cx| send(&(), window, cx))
}

/// A select of `len` options showing `chosen`, its list open on `highlight`. `scroll` is the
/// open list's window, kept between draws so it follows the highlight.
fn drawn(
    chosen: Option<String>,
    highlight: Option<usize>,
    len: usize,
    scroll: &mut usize,
) -> SelectState {
    let state = SelectState::drawn(chosen, highlight, *scroll, len);
    *scroll = state.scroll();
    state
}

/// The shared dropdown, wrapped so a click on it reaches no click-away behind it.
fn select(props: SelectFieldProps<'_>, cx: &App) -> AnyElement {
    let id: SharedString = format!("{}-box", props.id).into();
    div()
        .id(id)
        .on_click(|_event, _window, cx| cx.stop_propagation())
        .child(select_field::render(props, cx))
        .into_any_element()
}

/// The right-hand "THIS LEDGER" panel: a bordered figure table, then a note explaining the page's
/// own Preference-vs-Configuration scope (mirrors the section's own "ledger identity" scope
/// note).
fn summary_panel(model: &LedgerViewModel, cx: &App) -> impl IntoElement {
    let last = model.summary.len().saturating_sub(1);
    div()
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(400.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .mb(px(10.0))
                .child(lib_locale::format::upper(&model.summary_title)),
        )
        .child(
            div()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .flex()
                .flex_col()
                .children(
                    model
                        .summary
                        .iter()
                        .enumerate()
                        .map(|(index, (label, value))| {
                            summary_row(label.clone(), value.clone(), index == last, cx)
                        }),
                ),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(model.note.clone()),
        )
}

fn summary_row(label: String, value: String, last: bool, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .px(px(12.0))
        .py(px(10.0))
        .text_size(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(div().text_color(color::muted(cx)).child(label))
        .child(div().font_weight(gpui::FontWeight::EXTRA_BOLD).child(value))
}
