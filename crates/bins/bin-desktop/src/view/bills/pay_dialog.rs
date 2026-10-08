//! Renders the **Pay** dialog (`docs/ux/desktop-mockups/12-bills/README.md`'s 8d) on the shared
//! `crate::dialog` chrome: the segmented switch directly under the title bar, then either the Match
//! list or the Pay it directly fields. `Shell` owns the live form (`bills::pay_form::PayForm`) and every
//! keystroke while it is open, and hands this the candidates already worded.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use lib_locale::format::upper;

use crate::{
    bills::pay_form::{MatchChoice, PAY_MODES, PayField, PayForm, PayMode},
    dialog,
    theme::color,
    view::accounts::add_dialog::{label, text_field, two_up},
};

/// The dialog's width: 8d's.
pub const WIDTH: gpui::Pixels = px(460.0);

pub type OnModeClick = Rc<dyn Fn(PayMode, &mut Window, &mut App)>;
pub type OnChoiceClick = Rc<dyn Fn(MatchChoice, &mut Window, &mut App)>;
pub type OnFieldClick = Rc<dyn Fn(PayField, &mut Window, &mut App)>;

pub struct PayDialogHandlers {
    pub on_mode_click: OnModeClick,
    pub on_choice_click: OnChoiceClick,
    pub on_field_click: OnFieldClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

/// One Match candidate, worded: "Telstra · 21 sep · −79.99", and its Account.
pub struct CandidateRow {
    pub summary: String,
    pub account: String,
}

pub struct PayDialogProps<'a> {
    pub plan_name: &'a str,
    pub due: String,
    pub form: &'a PayForm,
    pub candidates: Vec<CandidateRow>,
    /// The Plan's planned amount with its unit, for the Match notice.
    pub planned: String,
    /// The seeded Category, Payee and Account, read-only on Pay it directly.
    pub chips: [String; 3],
    pub amount_invalid: bool,
    pub date_error: Option<String>,
    /// A refused confirm.
    pub error: Option<String>,
    pub valid: bool,
    pub handlers: PayDialogHandlers,
}

fn mode_label(mode: PayMode) -> String {
    match mode {
        PayMode::Direct => crate::msg::desktop_bills_pay_mode_direct(),
        PayMode::Match => crate::msg::desktop_bills_pay_mode_match(),
    }
}

pub fn render(props: PayDialogProps<'_>, cx: &App) -> AnyElement {
    let PayDialogProps {
        plan_name,
        due,
        form,
        candidates,
        planned,
        chips,
        amount_invalid,
        date_error,
        error,
        valid,
        handlers,
    } = props;
    let PayDialogHandlers {
        on_mode_click,
        on_choice_click,
        on_field_click,
        on_cancel,
        on_confirm,
    } = handlers;

    let mut fields = match form.mode {
        PayMode::Match => match_panel(form, candidates, &planned, &on_choice_click, cx),
        PayMode::Direct => {
            direct_panel(form, chips, amount_invalid, date_error, &on_field_click, cx)
        }
    };
    if let Some(error) = error {
        fields.push(error_line(error, cx));
    }
    let submit = match form.mode {
        PayMode::Direct => crate::msg::desktop_bills_pay_submit_direct(),
        PayMode::Match => crate::msg::desktop_bills_pay_submit_match(),
    };

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_bills_pay_title(plan_name, &due),
            false,
            cx,
        ))
        .child(
            div()
                .px(px(24.0))
                .pt(px(16.0))
                .child(switch(form.mode, &on_mode_click, cx)),
        )
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("pay-bill-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button("pay-bill-confirm", submit, valid, false, on_confirm, cx)
                    .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}

/// The two-cell switch: the selected cell filled dark (8d's `.seg-opt`), not the accent the
/// Settings segmented control uses.
fn switch(current: PayMode, on_click: &OnModeClick, cx: &App) -> AnyElement {
    div()
        .flex()
        .border_1()
        .border_color(color::border(cx))
        .children(PAY_MODES.iter().enumerate().map(|(index, &mode)| {
            let on_click = on_click.clone();
            div()
                .id(SharedString::from(format!("pay-bill-mode-{index}")))
                .debug_selector(move || format!("pay-bill-mode-{index}"))
                .cursor_pointer()
                .flex_1()
                .py(px(8.0))
                .px(px(12.0))
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(mode == current, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                })
                .on_click(move |_event, window, cx| on_click(mode, window, cx))
                .child(mode_label(mode))
        }))
        .into_any_element()
}

fn match_panel(
    form: &PayForm,
    candidates: Vec<CandidateRow>,
    planned: &str,
    on_click: &OnChoiceClick,
    cx: &App,
) -> Vec<AnyElement> {
    let mut rows = vec![
        div()
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::faint_text(cx))
            .child(upper(&crate::msg::desktop_bills_pay_candidates()))
            .into_any_element(),
    ];
    if candidates.is_empty() {
        rows.push(
            div()
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_bills_pay_no_candidates())
                .into_any_element(),
        );
    }
    for (index, candidate) in candidates.into_iter().enumerate() {
        let choice = MatchChoice::Candidate(index);
        rows.push(radio_row(
            choice,
            form.choice == Some(choice),
            candidate.summary,
            Some(candidate.account),
            on_click,
            cx,
        ));
    }
    rows.push(radio_row(
        MatchChoice::NoneOfThese,
        form.choice == Some(MatchChoice::NoneOfThese),
        crate::msg::desktop_bills_pay_none_of_these(),
        None,
        on_click,
        cx,
    ));
    rows.push(
        dialog::info_panel(crate::msg::desktop_bills_pay_match_notice(planned), cx)
            .into_any_element(),
    );
    rows
}

/// A bordered radio row; the chosen one gets the accent border and tint.
fn radio_row(
    choice: MatchChoice,
    selected: bool,
    text: String,
    trailing: Option<String>,
    on_click: &OnChoiceClick,
    cx: &App,
) -> AnyElement {
    let on_click = on_click.clone();
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
        .on_click(move |_event, window, cx| on_click(choice, window, cx))
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

fn direct_panel(
    form: &PayForm,
    chips: [String; 3],
    amount_invalid: bool,
    date_error: Option<String>,
    on_click: &OnFieldClick,
    cx: &App,
) -> Vec<AnyElement> {
    let click = |field: PayField| -> dialog::OnClick {
        let on_click = on_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
    };
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
                    label(crate::msg::desktop_bills_pay_field_amount()),
                    form.amount.text(),
                    "0.00",
                    form.focused == PayField::Amount,
                    click(PayField::Amount),
                    cx,
                ),
                amount_invalid.then(crate::msg::desktop_bills_pay_error_amount),
            ),
            with_error(
                text_field(
                    "pay-bill-date",
                    label(crate::msg::desktop_bills_pay_field_date()),
                    form.date.text(),
                    &lib_locale::msg::date_word_today(),
                    form.focused == PayField::Date,
                    click(PayField::Date),
                    cx,
                ),
                date_error,
            ),
        ]),
        div()
            .flex()
            .flex_wrap()
            .gap(px(6.0))
            .children(chips.into_iter().map(|text| {
                div()
                    .py(px(3.0))
                    .px(px(8.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(text)
            }))
            .into_any_element(),
        dialog::info_panel(crate::msg::desktop_bills_pay_direct_notice(), cx).into_any_element(),
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
