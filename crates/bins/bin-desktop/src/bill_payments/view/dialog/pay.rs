//! The Pay Dialog (`docs/ux/desktop-mockups/12-bills/README.md`'s 8d) on the shared
//! `crate::dialog` frame: the `lib-bill-payments` Pay Presenter's `DialogView`, which the generic
//! `ModuleDialog` hosts in the Dialog host (ADR-0056). It draws the segmented switch under the
//! title bar, then either the Match list or the Pay it directly fields. Every key and click
//! becomes a Presenter event, and this draws the view model, hints included, and nothing else.
//!
//! The body scrolls past [`BODY_MAX_HEIGHT`], so the switch and the buttons stay on screen however
//! many candidates the Match list holds, and the chosen row is scrolled into view as it moves.

use gpui::{AnyElement, App, ScrollHandle, SharedString, div, prelude::*, px};
use lib_bill_payments::presenter::{
    MatchChoice, PayEvent, PayField, PayMode, PayPresenter, PayViewModel,
};
use lib_shell::Hint;

use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::theme::color;

/// The Dialog's width: 8d's.
const WIDTH: gpui::Pixels = px(460.0);
/// The body's scroll cap, the Plan form's, which keeps the switch and the buttons inside a
/// 1080px window.
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);

/// The open Pay Dialog's clicks.
type Clicks = ModuleDialogClicks<PayPresenter>;

/// The body's scroll, and the panel and chosen Match row last drawn, so a step down the list keeps
/// the chosen row in view.
#[derive(Default)]
pub struct PayWidgets {
    scroll: ScrollHandle,
    followed: Option<(PayMode, Option<usize>)>,
}

/// The `lib-bill-payments` Pay Presenter's [`DialogView`], hosted in a `ModuleDialog`.
impl DialogView for PayPresenter {
    type Widgets = PayWidgets;

    fn kind(_view: &PayViewModel) -> &'static str {
        "Pay"
    }

    /// Scrolls a Match row chosen on the Match list into view before drawing. Coming to the Match
    /// list from the other panel leaves it where it was, whatever row is chosen.
    fn render(
        view: &PayViewModel,
        widgets: &mut PayWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        let drawn = (view.mode, chosen_row(view));
        if let (PayMode::Match, Some(row)) = drawn
            && widgets
                .followed
                .is_some_and(|(mode, followed)| mode == PayMode::Match && followed != Some(row))
        {
            widgets.scroll.scroll_to_item(row);
        }
        widgets.followed = Some(drawn);
        render(view, &widgets.scroll, clicks, cx)
    }

    /// The panel, what it shows and whether it would confirm.
    fn readback(view: &PayViewModel) -> Vec<(&'static str, String)> {
        let choice = match view.choice {
            Some(MatchChoice::Candidate(index)) => index.to_string(),
            Some(MatchChoice::NoneOfThese) => "none".to_string(),
            None => String::new(),
        };
        framed(
            &view.frame,
            vec![
                ("mode", format!("{:?}", view.mode)),
                ("candidates", view.candidates.len().to_string()),
                ("choice", choice),
                ("amount", view.amount.text.clone()),
                ("date", view.date.text.clone()),
                ("refusal", view.refusal.clone().unwrap_or_default()),
            ],
        )
    }

    fn hints(view: &PayViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &PayViewModel, scroll: &ScrollHandle, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = match view.mode {
        PayMode::Match => match_panel(view, actions, cx),
        PayMode::Direct => direct_panel(view, actions, cx),
    };
    if let Some(refusal) = &view.refusal {
        fields.push(error_line(refusal.clone(), cx));
    }
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(
            div()
                .px(px(24.0))
                .pt(px(16.0))
                .child(switch(view, actions, cx)),
        )
        .child(
            dialog::body(fields)
                .id("pay-bill-body")
                .debug_selector(|| "pay-bill-body".to_string())
                .max_h(BODY_MAX_HEIGHT)
                .overflow_y_scroll()
                .track_scroll(scroll),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button("pay-bill-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "pay-bill-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    view.frame.destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// The two-cell switch: the chosen cell filled dark (8d's `.seg-opt`), not the accent the Settings
/// segmented control uses.
fn switch(view: &PayViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    div()
        .flex()
        .border_1()
        .border_color(color::border(cx))
        .children(view.modes.iter().enumerate().map(|(index, mode)| {
            let on_click = actions.send(PayEvent::Mode(mode.value));
            div()
                .id(SharedString::from(format!("pay-bill-mode-{index}")))
                .debug_selector(move || format!("pay-bill-mode-{index}"))
                .cursor_pointer()
                .flex_1()
                .py(px(8.0))
                .px(px(12.0))
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(mode.value == view.mode, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                })
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(mode.label.clone())
        }))
        .into_any_element()
}

/// The chosen Match row's index among the body's children, in [`match_panel`]'s order: the
/// heading, the no-candidates line if any, the candidates, then "None of these".
fn chosen_row(view: &PayViewModel) -> Option<usize> {
    let first = 1 + usize::from(view.no_candidates.is_some());
    match view.choice? {
        MatchChoice::Candidate(index) => Some(first + index),
        MatchChoice::NoneOfThese => Some(first + view.candidates.len()),
    }
}

fn match_panel(view: &PayViewModel, actions: &Clicks, cx: &App) -> Vec<AnyElement> {
    let mut rows = vec![
        div()
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::faint_text(cx))
            .child(view.candidates_heading.clone())
            .into_any_element(),
    ];
    if let Some(none) = &view.no_candidates {
        rows.push(
            div()
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .child(none.clone())
                .into_any_element(),
        );
    }
    for (index, candidate) in view.candidates.iter().enumerate() {
        let choice = MatchChoice::Candidate(index);
        rows.push(radio_row(
            choice,
            view.choice == Some(choice),
            candidate.summary.clone(),
            Some(candidate.account.clone()),
            actions,
            cx,
        ));
    }
    rows.push(radio_row(
        MatchChoice::NoneOfThese,
        view.choice == Some(MatchChoice::NoneOfThese),
        view.none_of_these.clone(),
        None,
        actions,
        cx,
    ));
    rows.push(dialog::info_panel(view.match_notice.clone(), cx).into_any_element());
    rows
}

/// A bordered radio row; the chosen one gets the accent border and tint.
fn radio_row(
    choice: MatchChoice,
    selected: bool,
    text: String,
    trailing: Option<String>,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let on_click = actions.send(PayEvent::Choose(choice));
    let id = match choice {
        MatchChoice::Candidate(index) => format!("pay-bill-candidate-{index}"),
        MatchChoice::NoneOfThese => "pay-bill-none-of-these".to_string(),
    };
    div()
        .debug_selector({
            let id = id.clone();
            move || id.clone()
        })
        .id(SharedString::from(id))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(10.0))
        .py(px(10.0))
        .px(px(12.0))
        .border_1()
        .border_color(if selected {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .when(selected, |this| this.bg(color::accent_tint(cx)))
        .text_size(px(13.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(
            div()
                .size(px(14.0))
                .flex_none()
                .rounded_full()
                .border_1()
                .flex()
                .items_center()
                .justify_center()
                .border_color(if selected {
                    color::accent(cx)
                } else {
                    color::muted(cx)
                })
                .when(selected, |this| {
                    this.child(div().size(px(6.0)).rounded_full().bg(color::accent(cx)))
                }),
        )
        .child(div().flex_1().min_w(px(0.0)).child(text))
        .children(trailing.map(|account| {
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(account)
        }))
        .into_any_element()
}

fn direct_panel(view: &PayViewModel, actions: &Clicks, cx: &App) -> Vec<AnyElement> {
    let with_error = |field: AnyElement, error: Option<String>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(error.map(|text| error_line(text, cx)))
            .into_any_element()
    };
    vec![
        two_up([
            with_error(
                text_field(
                    "pay-bill-amount",
                    label(view.amount.label.clone()),
                    &view.amount.text,
                    &view.amount.placeholder,
                    view.amount.focused,
                    actions.send(PayEvent::Focus(PayField::Amount)),
                    cx,
                ),
                view.amount_error.clone(),
            ),
            with_error(
                text_field(
                    "pay-bill-date",
                    label(view.date.label.clone()),
                    &view.date.text,
                    &view.date.placeholder,
                    view.date.focused,
                    actions.send(PayEvent::Focus(PayField::Date)),
                    cx,
                ),
                view.date_error.clone(),
            ),
        ]),
        div()
            .flex()
            .flex_wrap()
            .gap(px(6.0))
            .children(view.chips.iter().map(|text| {
                div()
                    .py(px(3.0))
                    .px(px(8.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(text.clone())
            }))
            .into_any_element(),
        dialog::info_panel(view.direct_notice.clone(), cx).into_any_element(),
    ]
}

fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}
