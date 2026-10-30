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
//! a scrim here, so the rails and header stay at full strength.

use std::rc::Rc;

use gpui::{AnyElement, App, BoxShadow, Pixels, SharedString, Window, div, point, prelude::*, px};

use lib_transactions::presenter::{FilterField, FilterViewModel, status_filter_label};
use lib_viper::view::TextFieldView;

use super::state::select_labels;
use crate::{
    dialog,
    form::fields::segmented_control,
    form::{
        dialog_fields::{label, text_field, two_up},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    theme::color,
    transactions::query::StatusFilter,
};

pub type OnFieldClick = Rc<dyn Fn(FilterField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(FilterField, usize, &mut Window, &mut App)>;
pub type OnStatusClick = Rc<dyn Fn(StatusFilter, &mut Window, &mut App)>;

/// The card's width: the bundle's own 400px.
const WIDTH: Pixels = px(400.0);

pub struct PopoverProps<'a> {
    pub view: FilterViewModel,
    /// A select's open list, and the field it belongs to.
    pub list: Option<&'a (FilterField, SelectState)>,
    /// The card's top-left corner in window coordinates: just below the triggering chip.
    pub left: Pixels,
    pub top: Pixels,
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_status_click: OnStatusClick,
    pub on_reset: dialog::OnClick,
    pub on_apply: dialog::OnClick,
    /// A click outside the card.
    pub on_cancel: dialog::OnClick,
}

pub fn render(props: PopoverProps<'_>, cx: &App) -> AnyElement {
    let PopoverProps {
        view,
        list,
        left,
        top,
        on_field_click,
        on_option_click,
        on_status_click,
        on_reset,
        on_apply,
        on_cancel,
    } = props;

    // A select shows its own open list, else its choice closed.
    let select = |field: FilterField| -> (Vec<String>, SelectState) {
        let (options, chosen) = select_labels(&view, field);
        let state = match list {
            Some((at, state)) if *at == field => state.clone(),
            _ => SelectState::new(chosen),
        };
        (options, state)
    };
    let (account_options, account_state) = select(FilterField::Account);
    let (category_options, category_state) = select(FilterField::Category);
    let can_apply = view.frame.can_confirm;
    let click = |field: FilterField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: FilterField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_option_click(field, index, window, cx)
        })
    };

    let card = div()
        .id("filter-popover")
        .absolute()
        .left(left)
        .top(top)
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
                        on_option_click: option_click(FilterField::Account),
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
                        on_option_click: option_click(FilterField::Category),
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
                    {
                        let on_status_click = on_status_click.clone();
                        Rc::new(
                            move |status: StatusFilter, window: &mut Window, cx: &mut App| {
                                on_status_click(status, window, cx)
                            },
                        )
                    },
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
                .child(reset_button(view.reset.clone(), on_reset, cx))
                .child(apply_button(
                    view.frame.submit.clone(),
                    can_apply,
                    on_apply,
                    cx,
                )),
        );

    div()
        .id("filter-popover-layer")
        .absolute()
        .inset_0()
        .on_click(move |_event, window, cx| on_cancel(window, cx))
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
    on_click: OnStatusClick,
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
fn reset_button(text: String, on_click: dialog::OnClick, cx: &App) -> impl IntoElement {
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
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(text)
}

/// `apply`: the primary button with its `enter` hint, at 45% opacity and inert while a date is bad.
fn apply_button(
    text: String,
    enabled: bool,
    on_click: dialog::OnClick,
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
        .when(enabled, |this| {
            this.on_click(move |_event, window, cx| on_click(window, cx))
        })
        .child(text)
        .child(
            div()
                .opacity(0.75)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::NORMAL)
                .child("enter"),
        )
}
