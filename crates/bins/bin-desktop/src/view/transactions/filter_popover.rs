//! The **4b** filter popover (`docs/ux/desktop/Transactions/README.md`): a 400px card *anchored*
//! just below the chip row (not centred like the dialogs and the palette), over a transparent
//! full-window layer that cancels it when clicked. It holds one field per filter dimension and
//! edits a draft (`transaction_filter_form::FilterForm`); nothing reaches the applied filters until
//! **apply**.
//!
//! Account and Category are the shared dropdown (`view::accounts::select_field`); Payee and Tag are
//! plain text; From and To are text dates that show an accent border and a hint while unparseable
//! (which also disables **apply**); Status is the shared segmented control. The page behind is dimmed
//! by the page itself (`opacity .55`, the map's blur-free replacement for the mockup's blur), not by
//! a scrim here, so the rails and header stay at full strength.

use std::rc::Rc;

use gpui::{AnyElement, App, BoxShadow, Pixels, SharedString, Window, div, point, prelude::*, px};

use crate::{
    dialog,
    theme::color,
    transaction_filter_form::{FilterForm, FormField, FormOptions},
    transaction_query::StatusFilter,
    view::{
        accounts::{
            add_dialog::{label, text_field, two_up},
            select_field::{self, SelectFieldProps},
        },
        settings::add_unit_dialog::segmented_control,
    },
};

pub type OnFieldClick = Rc<dyn Fn(FormField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(FormField, usize, &mut Window, &mut App)>;
pub type OnStatusClick = Rc<dyn Fn(StatusFilter, &mut Window, &mut App)>;

/// The card's width: the bundle's own 400px.
const WIDTH: Pixels = px(400.0);

pub struct PopoverProps<'a> {
    pub form: &'a FilterForm,
    pub options: &'a FormOptions,
    /// The hint under From / To while its text is not a date.
    pub start_hint: Option<String>,
    pub end_hint: Option<String>,
    /// Whether **apply** may run (both dates parse).
    pub can_apply: bool,
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

pub fn render(props: PopoverProps<'_>) -> AnyElement {
    let PopoverProps {
        form,
        options,
        start_hint,
        end_hint,
        can_apply,
        left,
        top,
        on_field_click,
        on_option_click,
        on_status_click,
        on_reset,
        on_apply,
        on_cancel,
    } = props;

    let focused = |field: FormField| form.focused == field;
    let click = |field: FormField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: FormField| -> select_field::OnOptionClick {
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
        .bg(color::GROUND)
        .border_1()
        .border_color(color::BORDER)
        .shadow(vec![BoxShadow {
            color: color::DIALOG_SHADOW.into(),
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
                .border_color(color::STRUCTURAL_RULE)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(13.5))
                .child(crate::msg::desktop_transactions_filter_title()),
        )
        .child(
            div()
                .px(px(18.0))
                .py(px(16.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(select_field::render(SelectFieldProps {
                    id: "filter-account",
                    label: lib_locale::msg::column_account().into(),
                    options: options.for_field(FormField::Account),
                    state: &form.account,
                    focused: focused(FormField::Account),
                    read_only: None,
                    on_field_click: click(FormField::Account),
                    on_option_click: option_click(FormField::Account),
                }))
                .child(select_field::render(SelectFieldProps {
                    id: "filter-category",
                    label: lib_locale::msg::column_category().into(),
                    options: options.for_field(FormField::Category),
                    state: &form.category,
                    focused: focused(FormField::Category),
                    read_only: None,
                    on_field_click: click(FormField::Category),
                    on_option_click: option_click(FormField::Category),
                }))
                .child(text_field(
                    "filter-payee",
                    label(lib_locale::msg::column_payee()),
                    &form.payee,
                    &crate::msg::desktop_transactions_filter_any_payee(),
                    focused(FormField::Payee),
                    click(FormField::Payee),
                ))
                .child(text_field(
                    "filter-tag",
                    label(crate::msg::desktop_transactions_field_tag()),
                    &form.tag,
                    &crate::msg::desktop_transactions_filter_any_tag(),
                    focused(FormField::Tag),
                    click(FormField::Tag),
                ))
                .child(two_up([
                    date_field(
                        "filter-from",
                        lib_locale::msg::column_from(),
                        &form.from,
                        &crate::msg::desktop_transactions_filter_no_start(),
                        focused(FormField::From),
                        start_hint,
                        click(FormField::From),
                    ),
                    date_field(
                        "filter-to",
                        crate::msg::desktop_transactions_field_to(),
                        &form.to,
                        &crate::msg::desktop_transactions_filter_no_end(),
                        focused(FormField::To),
                        end_hint,
                        click(FormField::To),
                    ),
                ]))
                .child(status_field(form.status, focused(FormField::Status), {
                    let on_status_click = on_status_click.clone();
                    Rc::new(
                        move |status: StatusFilter, window: &mut Window, cx: &mut App| {
                            on_status_click(status, window, cx)
                        },
                    )
                })),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(10.0))
                .px(px(18.0))
                .py(px(14.0))
                .border_t(px(1.0))
                .border_color(color::HAIRLINE)
                .child(reset_button(on_reset))
                .child(apply_button(can_apply, on_apply)),
        );

    div()
        .id("filter-popover-layer")
        .absolute()
        .inset_0()
        .on_click(move |_event, window, cx| on_cancel(window, cx))
        .child(card)
        .into_any_element()
}

/// A From / To field: the label over a text box, the box turning `ACCENT` with a hint beneath it
/// while the text is not a date.
fn date_field(
    id: &'static str,
    title: String,
    value: &str,
    placeholder: &str,
    focused: bool,
    error: Option<String>,
    on_click: dialog::OnClick,
) -> AnyElement {
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::INK_TERTIARY,
        )
    } else {
        (SharedString::from(format!("{value}{caret}")), color::INK)
    };
    let border = if error.is_some() || focused {
        color::ACCENT
    } else {
        color::BORDER
    };
    div()
        .flex_1()
        .min_w(px(0.0))
        .child(label(title))
        .child(
            div()
                .id(id)
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
                .text_color(color::ACCENT_TEXT)
                .child(message)
        }))
        .into_any_element()
}

/// The Status field: the label over the shared segmented control, ringed in accent while focused
/// (`Left` / `Right` step it).
fn status_field(current: StatusFilter, focused: bool, on_click: OnStatusClick) -> AnyElement {
    div()
        .child(label(lib_locale::msg::column_status()))
        .child(
            div()
                .p(px(2.0))
                .border_1()
                .border_color(if focused {
                    color::ACCENT
                } else {
                    color::GROUND
                })
                .child(segmented_control(
                    "filter-status",
                    &StatusFilter::ALL,
                    current,
                    StatusFilter::label,
                    on_click,
                )),
        )
        .into_any_element()
}

/// `reset`: the ghost button.
fn reset_button(on_click: dialog::OnClick) -> impl IntoElement {
    div()
        .id("filter-reset")
        .cursor_pointer()
        .py(px(8.0))
        .px(px(16.0))
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::INK)
        .hover(|style| style.bg(color::HOVER_TINT))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_hint_reset())
}

/// `apply`: the primary button with its `enter` hint, at 45% opacity and inert while a date is bad.
fn apply_button(enabled: bool, on_click: dialog::OnClick) -> impl IntoElement {
    div()
        .id("filter-apply")
        .when(enabled, |this| this.cursor_pointer())
        .when(!enabled, |this| this.opacity(0.45))
        .flex()
        .items_center()
        .gap(px(8.0))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::INK)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .when(enabled, |this| {
            this.on_click(move |_event, window, cx| on_click(window, cx))
        })
        .child(crate::msg::desktop_hint_apply())
        .child(
            div()
                .opacity(0.75)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::NORMAL)
                .child("enter"),
        )
}
