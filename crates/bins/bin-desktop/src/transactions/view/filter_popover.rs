//! The **4b** filter popover (`docs/ux/desktop-mockups/03-transactions/README.md`): a 400px card *anchored*
//! just below the chip row (not centred like the dialogs and the palette), over a transparent
//! full-window layer that cancels it when clicked. It holds one field per filter dimension and
//! draws the view model of `lib_transactions`' `FilterPresenter`, which holds the draft; nothing
//! reaches the applied filters until **apply**.
//!
//! Account and Category are the shared dropdown (`form::select_field`); Payee and Tag are
//! plain text; From and To are text dates that show an accent border and a hint while unparseable
//! (which also disables **apply**); Status is the shared segmented control. The page behind is dimmed
//! by the page itself (`opacity .55`, the map's blur-free replacement for the mockup's blur), not by
//! a scrim here, so the rails and header stay at full strength. Every click is a
//! [`FilterEvent`] to the page's Presenter, sent through [`PageClicks`].

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, Pixels, Point, SharedString, Window, div, point,
    prelude::*, px,
};

use lib_transactions::presenter::{
    FilterEvent, FilterField, FilterViewModel, TransactionsPageEvent, status_filter_label,
};
use lib_viper::view::TextFieldView;

use super::page::{TransactionsPage, select_labels};
use crate::{
    dialog,
    form::fields::{OnSegmentClick, segmented_control},
    form::{
        dialog_fields::{label, text_field, two_up},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    shell::view::page_clicks::PageClicks,
    theme::color,
};
use lib_transactions::query::StatusFilter;

/// The card's width: the bundle's own 400px.
const WIDTH: Pixels = px(400.0);

/// The card at `at`, its top-left corner in window coordinates just below the triggering chip,
/// and `list` a select's open list with the field it belongs to.
pub fn render(
    view: &FilterViewModel,
    list: Option<&(FilterField, SelectState)>,
    at: Point<Pixels>,
    clicks: &PageClicks<TransactionsPage>,
    cx: &App,
) -> AnyElement {
    let filter = |event: FilterEvent| TransactionsPageEvent::Filter(event);
    // A select shows its own open list, else its choice closed.
    let select = |field: FilterField| -> (Vec<String>, SelectState) {
        let (options, chosen) = select_labels(view, field);
        let state = match list {
            Some((at, state)) if *at == field => state.clone(),
            _ => SelectState::new(chosen),
        };
        (options, state)
    };
    let (account_options, account_state) = select(FilterField::Account);
    let (category_options, category_state) = select(FilterField::Category);
    let can_apply = view.frame.can_confirm;
    let click = |field: FilterField| clicks.click(filter(FilterEvent::Focus(field)));
    // An option's index is its place in the select's options, as `select_labels` lists them.
    let accounts: Vec<_> = view.account.options.iter().map(|o| o.value).collect();
    let categories: Vec<_> = view.category.options.iter().map(|o| o.value).collect();
    let account_option = clicks
        .option(move |index| filter(FilterEvent::Account(accounts.get(index).copied().flatten())));
    let category_option = clicks.option(move |index| {
        filter(FilterEvent::Category(
            categories.get(index).copied().flatten(),
        ))
    });
    let on_status_click: OnSegmentClick<StatusFilter> = {
        let send =
            clicks.send_with(move |status: &StatusFilter| filter(FilterEvent::Status(*status)));
        Rc::new(move |status, window, cx| send(&status, window, cx))
    };

    let card = div()
        .id("filter-popover")
        .absolute()
        .left(at.x)
        .top(at.y)
        .w(WIDTH)
        .flex()
        .flex_col()
        .bg(color::background(cx))
        .border_1()
        .border_color(color::border(cx))
        .shadow(vec![BoxShadow {
            color: color::dialog_shadow(cx).into(),
            offset: point(px(0.0), px(18.0)),
            blur_radius: px(44.0),
            spread_radius: px(0.0),
        }])
        // A click inside the card must not reach the cancelling layer beneath it.
        .on_click(|_event, _window, cx| cx.stop_propagation())
        .child(
            div()
                .px(px(18.0))
                .py(px(14.0))
                .border_b(px(2.0))
                .border_color(color::structural_rule(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(13.5))
                .child(view.frame.title.clone()),
        )
        .child(
            div()
                .px(px(18.0))
                .py(px(16.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(select_field::render(
                    SelectFieldProps {
                        id: "filter-account".into(),
                        label: view.account.label.clone().into(),
                        options: &account_options,
                        state: &account_state,
                        focused: view.account.focused,
                        accent: false,
                        read_only: None,
                        on_field_click: click(FilterField::Account),
                        on_option_click: account_option,
                    },
                    cx,
                ))
                .child(select_field::render(
                    SelectFieldProps {
                        id: "filter-category".into(),
                        label: view.category.label.clone().into(),
                        options: &category_options,
                        state: &category_state,
                        focused: view.category.focused,
                        accent: false,
                        read_only: None,
                        on_field_click: click(FilterField::Category),
                        on_option_click: category_option,
                    },
                    cx,
                ))
                .child(text_field(
                    "filter-payee",
                    label(view.payee.label.clone()),
                    &view.payee.text,
                    &view.payee.placeholder,
                    view.payee.focused,
                    click(FilterField::Payee),
                    cx,
                ))
                .child(text_field(
                    "filter-tag",
                    label(view.tag.label.clone()),
                    &view.tag.text,
                    &view.tag.placeholder,
                    view.tag.focused,
                    click(FilterField::Tag),
                    cx,
                ))
                .child(two_up([
                    date_field(
                        "filter-from",
                        &view.from,
                        view.from_error.clone(),
                        click(FilterField::From),
                        cx,
                    ),
                    date_field(
                        "filter-to",
                        &view.to,
                        view.to_error.clone(),
                        click(FilterField::To),
                        cx,
                    ),
                ]))
                .child(status_field(
                    &view.status.label,
                    view.status.chosen.unwrap_or_default(),
                    view.status.focused,
                    on_status_click,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(10.0))
                .px(px(18.0))
                .py(px(14.0))
                .border_t(px(1.0))
                .border_color(color::hairline(cx))
                .child(reset_button(
                    view.reset.clone(),
                    clicks.send(filter(FilterEvent::Reset)),
                    cx,
                ))
                .child(apply_button(
                    view.frame.submit.clone(),
                    can_apply,
                    clicks.send(filter(FilterEvent::Apply)),
                    cx,
                )),
        );

    div()
        .id("filter-popover-layer")
        .absolute()
        .inset_0()
        // A click outside the card cancels the popover, its draft discarded.
        .on_click(clicks.send(filter(FilterEvent::Cancel)))
        .child(card)
        .into_any_element()
}

/// A From / To field: the label over a text box, the box turning `negative` with a hint beneath it
/// while the text is not a date, and `accent` while focused.
fn date_field(
    id: &'static str,
    field: &TextFieldView,
    error: Option<String>,
    on_click: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let TextFieldView {
        label: title,
        text: value,
        placeholder,
        focused,
    } = field;
    let focused = *focused;
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{value}{caret}")),
            color::foreground(cx),
        )
    };
    let border = if error.is_some() {
        color::negative(cx)
    } else if focused {
        color::accent(cx)
    } else {
        color::border(cx)
    };
    div()
        .flex_1()
        .min_w(px(0.0))
        .child(label(title.clone()))
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(border)
                .text_size(px(13.0))
                .text_color(text_color)
                .truncate()
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .children(error.map(|message| {
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .text_color(color::negative_text(cx))
                .child(message)
        }))
        .into_any_element()
}

/// The Status field: the label over the shared segmented control, ringed in accent while focused
/// (`Left` / `Right` step it).
fn status_field(
    title: &str,
    current: StatusFilter,
    focused: bool,
    on_click: OnSegmentClick<StatusFilter>,
    cx: &App,
) -> AnyElement {
    div()
        .child(label(title.to_string()))
        .child(
            div()
                .p(px(2.0))
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::background(cx)
                })
                .child(segmented_control(
                    "filter-status",
                    &StatusFilter::ALL,
                    current,
                    status_filter_label,
                    on_click,
                    cx,
                )),
        )
        .into_any_element()
}

/// `reset`: the ghost button.
fn reset_button(
    text: String,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id("filter-reset")
        .debug_selector(|| "filter-reset".to_string())
        .cursor_pointer()
        .py(px(8.0))
        .px(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::foreground(cx))
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(text)
}

/// `apply`: the primary button with its `enter` hint, at 45% opacity and inert while a date is bad.
fn apply_button(
    text: String,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("filter-apply")
        .debug_selector(|| "filter-apply".to_string())
        .when(enabled, |this| this.cursor_pointer())
        .when(!enabled, |this| this.opacity(0.45))
        .flex()
        .items_center()
        .gap(px(8.0))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .when(enabled, |this| this.on_click(on_click))
        .child(text)
        .child(
            div()
                .opacity(0.75)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::NORMAL)
                .child("enter"),
        )
}
