//! The New budget dialog (11c) and its edit mode on the shared `crate::dialog` frame at the
//! handoff's 640px: Name and Unit, the four method cards, the on-budget Account chips and Start
//! from. Only Category limits is built, so the other three cards are drawn dimmed and take no
//! click (#400). Edit mode locks the Unit and leaves Start from out.
//!
//! What each key means is the Presenter's (`lib_budgets::presenter::BudgetFormPresenter`); this
//! View draws the form and turns its clicks into Presenter events.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_budgets::presenter::{
    AccountChip, BudgetField, BudgetFormEvent, BudgetFormPresenter, BudgetFormViewModel,
    MethodCard, StartChoice,
};
use lib_core::AccountId;
use lib_locale::format::upper;

use super::{Clicks, Form, FormDialog, Router, help, picker, segmented};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::theme::color;

/// The dialog's width: the handoff's 11c.
const WIDTH: gpui::Pixels = px(640.0);

fn handle(presenter: &mut BudgetFormPresenter, event: BudgetFormEvent) {
    presenter.handle(event, &mut Router::default());
}

fn form(dialog: &mut FormDialog) -> Option<&mut BudgetFormPresenter> {
    match &mut dialog.view {
        Form::Budget(presenter, _) => Some(presenter),
        _ => None,
    }
}

fn focus(dialog: &mut FormDialog, field: BudgetField) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, BudgetFormEvent::Focus(field));
    }
}

/// A click on the Unit field focuses it and toggles its list; Edit's is locked.
fn click_unit(dialog: &mut FormDialog) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, BudgetFormEvent::ToggleUnitList);
    }
}

/// A click on row `index` of the Unit's open list picks it; Edit's is locked.
fn click_option(dialog: &mut FormDialog, index: usize) {
    if let Some(presenter) = form(dialog) {
        let view = presenter.view_model();
        if view.editing {
            return;
        }
        if let Some(choice) = view.unit.options.get(index) {
            handle(presenter, BudgetFormEvent::Focus(BudgetField::Unit));
            handle(presenter, BudgetFormEvent::Unit(choice.value));
        }
    }
}

fn toggle_account(dialog: &mut FormDialog, id: AccountId) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, BudgetFormEvent::ToggleAccount(id));
    }
}

fn choose_start(dialog: &mut FormDialog, choice: StartChoice) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, BudgetFormEvent::StartFrom(choice));
    }
}

/// The chosen card takes the 2px ink border and the `chrome` fill; the rest are dimmed and say
/// they come later.
fn method_card(card: &MethodCard, later: &str, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .p(px(12.0))
        .border(px(2.0))
        .border_color(if card.enabled {
            color::foreground(cx)
        } else {
            color::border(cx)
        })
        .when(card.enabled, |this| this.bg(color::chrome(cx)))
        .when(!card.enabled, |this| this.opacity(0.55))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(13.0))
                        .child(card.title.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::muted(cx))
                        .child(upper(&card.tag)),
                ),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(card.detail.clone()),
        )
        .when(!card.enabled, |this| {
            this.child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::muted(cx))
                    .child(upper(later)),
            )
        })
        .into_any_element()
}

fn method_grid(view: &BudgetFormViewModel, cx: &App) -> AnyElement {
    let rows = view.methods.chunks(2).map(|cards| {
        div().flex().gap(px(10.0)).children(
            cards
                .iter()
                .map(|card| method_card(card, &view.method_later, cx)),
        )
    });
    div()
        .child(label(view.method_label.clone()))
        .child(div().flex().flex_col().gap(px(10.0)).children(rows))
        .into_any_element()
}

/// The Account chips: a selected one is solid ink with a `✓`; the keyboard cursor takes the
/// accent border while the chips are focused.
fn account_chips(view: &BudgetFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let choose = actions.choose(toggle_account);
    let chips = view.accounts.iter().map(|chip: &AccountChip| {
        let on_click = picker(choose.clone(), chip.id);
        let id = chip.id;
        div()
            .id(SharedString::from(format!("budgets-new-account-{id}")))
            .debug_selector(move || format!("budgets-new-account-{id}"))
            .cursor_pointer()
            .py(px(5.0))
            .px(px(10.0))
            .border(px(if chip.cursor { 2.0 } else { 1.0 }))
            .border_color(if chip.cursor {
                color::accent(cx)
            } else if chip.selected {
                color::foreground(cx)
            } else {
                color::border(cx)
            })
            .text_size(px(12.5))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .when(chip.selected, |this| {
                this.bg(color::foreground(cx))
                    .text_color(color::background(cx))
            })
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(if chip.selected {
                format!("\u{2713} {}", chip.name)
            } else {
                chip.name.clone()
            })
    });
    div()
        .child(label(view.accounts_label.clone()))
        .child(div().flex().flex_wrap().gap(px(6.0)).children(chips))
        .child(help(view.accounts_help.clone(), cx))
        .into_any_element()
}

pub(super) fn render(
    view: &BudgetFormViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let mut fields = vec![
        two_up([
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(text_field(
                    "budgets-new-name",
                    label(view.name.label.clone()),
                    &view.name.text,
                    &view.name.placeholder,
                    view.name.focused,
                    actions.click(|dialog| focus(dialog, BudgetField::Name)),
                    cx,
                ))
                .into_any_element(),
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(select_field::render(
                    SelectFieldProps {
                        id: "budgets-new-unit".into(),
                        label: view.unit.label.clone().into(),
                        options: &labels(&view.unit),
                        state: &drawn(&view.unit, view.unit_list, scroll),
                        focused: view.unit.focused,
                        accent: false,
                        read_only: view.unit_fixed.clone().map(SharedString::from),
                        on_field_click: actions.click(click_unit),
                        on_option_click: actions.option(click_option),
                    },
                    cx,
                ))
                .into_any_element(),
        ]),
        method_grid(view, cx),
        account_chips(view, actions, cx),
    ];
    if let Some(start) = &view.start {
        let choose = actions.choose(choose_start);
        fields.push(
            div()
                .child(segmented(
                    "budgets-new-start",
                    start.label.clone(),
                    start.focused,
                    start
                        .options
                        .iter()
                        .map(|choice| {
                            (
                                choice.label.clone(),
                                start.chosen == Some(choice.value),
                                picker(choose.clone(), choice.value),
                            )
                        })
                        .collect(),
                    cx,
                ))
                .child(help(view.start_help.clone(), cx))
                .into_any_element(),
        );
    }
    if let Some(error) = &view.error {
        fields.push(error_line(error.clone(), cx));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-new-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "budgets-new-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
