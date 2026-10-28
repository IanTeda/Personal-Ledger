//! The Add payee and Edit payee Dialogs (`docs/ux/desktop-mockups/20-payees/README.md`'s 6b and
//! 6c) on the shared `crate::dialog` frame: Name, Default category (the shared `select_field`
//! dropdown), the Match rules field and a callout.
//!
//! `Enter` in a rule input holding text adds it as a chip rather than confirming, and `Backspace`
//! in the empty input removes the last chip. The select takes the arrows, `Enter` and `Space`, and
//! swallows other typing rather than letting it reach the Shell.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_payees::presenter::{PayeeField, PayeeFormEvent, PayeeFormPresenter, PayeeFormViewModel};

use super::rules_field::{self, RulesFieldProps};
use super::{Clicks, PayeesDialog, View, send};
use crate::form::viper::{choose_option, chosen_label, enter, labels, select_key, synced};
use crate::{
    dialog,
    form::{
        dialog_fields::{label, text_field},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

/// The Dialogs' width: the handoff's 6b and 6c.
const WIDTH: gpui::Pixels = px(460.0);

fn handle(presenter: &mut PayeeFormPresenter, event: PayeeFormEvent) {
    send(presenter, event, PayeeFormPresenter::handle);
}

pub(super) fn key(
    presenter: &mut PayeeFormPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Tab | DialogKey::BackTab => {
            // Leaving the select commits an open list's highlight, as a click on it would.
            if select.is_open() {
                let (_, value) = select_key(select, &view.default_category, DialogKey::Enter);
                if let Some(id) = value {
                    handle(presenter, PayeeFormEvent::Category(id));
                }
            }
            let event = if key == DialogKey::Tab {
                PayeeFormEvent::NextField
            } else {
                PayeeFormEvent::PrevField
            };
            handle(presenter, event);
            return DialogOutcome::Handled;
        }
        _ => {}
    }
    match view.focused {
        PayeeField::DefaultCategory => {
            let (taken, value) = select_key(select, &view.default_category, key);
            if let Some(id) = value {
                handle(presenter, PayeeFormEvent::Category(id));
            }
            match key {
                DialogKey::Enter if !taken => enter(view.frame.can_confirm),
                // A select swallows typing rather than letting it fall through to the Shell.
                DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
                _ if taken => DialogOutcome::Handled,
                _ => DialogOutcome::Ignored,
            }
        }
        PayeeField::Rule => match key {
            DialogKey::Enter if !view.rule_input.text.is_empty() => {
                handle(presenter, PayeeFormEvent::AddRule);
                DialogOutcome::Handled
            }
            DialogKey::Backspace if view.rule_input.text.is_empty() => {
                handle(presenter, PayeeFormEvent::RemoveLastRule);
                DialogOutcome::Handled
            }
            _ => text_key(presenter, &view, key, PayeeFormEvent::RuleInput),
        },
        PayeeField::Name => text_key(presenter, &view, key, PayeeFormEvent::Name),
    }
}

/// A key on a text field: typing edits it, `Enter` confirms a valid form, the arrows do nothing.
fn text_key(
    presenter: &mut PayeeFormPresenter,
    view: &PayeeFormViewModel,
    key: DialogKey,
    event: fn(String) -> PayeeFormEvent,
) -> DialogOutcome {
    let text = if view.focused == PayeeField::Rule {
        &view.rule_input.text
    } else {
        &view.name.text
    };
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Up | DialogKey::Down => DialogOutcome::Handled,
        _ => match crate::form::viper::typed(text, key) {
            Some(text) => {
                handle(presenter, event(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

fn form(dialog: &mut PayeesDialog) -> Option<(&mut PayeeFormPresenter, &mut SelectState)> {
    match &mut dialog.view {
        View::Form(presenter, select) => Some((presenter, select)),
        View::Delete(_) => None,
    }
}

/// A click on a text field focuses it.
fn click_field(dialog: &mut PayeesDialog, field: PayeeField) {
    if let Some((presenter, select)) = form(dialog) {
        select.cancel();
        handle(presenter, PayeeFormEvent::Focus(field));
    }
}

/// A click on the select's closed field focuses it and toggles its list.
fn click_select(dialog: &mut PayeesDialog) {
    if let Some((presenter, select)) = form(dialog) {
        handle(
            presenter,
            PayeeFormEvent::Focus(PayeeField::DefaultCategory),
        );
        let view = presenter.view_model();
        *select = synced(select, &view.default_category);
        if select.is_open() {
            select.cancel();
        } else {
            select.open(&labels(&view.default_category));
        }
    }
}

fn click_option(dialog: &mut PayeesDialog, index: usize) {
    if let Some((presenter, select)) = form(dialog)
        && let Some(id) = choose_option(select, &presenter.view_model().default_category, index)
    {
        handle(presenter, PayeeFormEvent::Category(id));
    }
}

fn click_add_rule(dialog: &mut PayeesDialog) {
    if let Some((presenter, _)) = form(dialog) {
        handle(presenter, PayeeFormEvent::AddRule);
    }
}

fn click_remove_rule(dialog: &mut PayeesDialog, index: usize) {
    if let Some((presenter, _)) = form(dialog) {
        handle(presenter, PayeeFormEvent::RemoveRule(index));
    }
}

pub(super) fn render(
    view: &PayeeFormViewModel,
    select: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let id = if view.editing {
        "edit-payee"
    } else {
        "add-payee"
    };
    let element_id = |suffix: &str| gpui::SharedString::from(format!("{id}-{suffix}"));
    // `select_field` and `text_field` take a `'static` id, so these two are spelt out.
    let (name_id, category_id) = if view.editing {
        ("edit-payee-name", "edit-payee-category")
    } else {
        ("add-payee-name", "add-payee-category")
    };
    let options = labels(&view.default_category);
    let on_field = |field: PayeeField| actions.click(move |dialog| click_field(dialog, field));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            div()
                .child(text_field(
                    name_id,
                    label(view.name.label.clone()),
                    &view.name.text,
                    &view.name.placeholder,
                    view.name.focused,
                    on_field(PayeeField::Name),
                    cx,
                ))
                .children(
                    view.name_error
                        .clone()
                        .map(|error| rules_field::error_line(error, cx)),
                )
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: category_id.into(),
                    label: view.default_category.label.clone().into(),
                    options: &options,
                    state: &synced(select, &view.default_category),
                    focused: view.default_category.focused,
                    accent: false,
                    read_only: None,
                    on_field_click: actions.click(click_select),
                    on_option_click: actions.option(click_option),
                },
                cx,
            ),
            div()
                .child(rules_field::render(
                    RulesFieldProps {
                        id,
                        label: view.rule_input.label.clone(),
                        rules: &view.rules,
                        no_rules: view.no_rules.clone(),
                        input: &view.rule_input.text,
                        placeholder: view.rule_input.placeholder.clone(),
                        add: view.rule_add.clone(),
                        focused: view.rule_input.focused,
                        on_input_click: on_field(PayeeField::Rule),
                        on_add_click: actions.click(click_add_rule),
                        on_remove_rule: actions.choose(click_remove_rule),
                    },
                    cx,
                ))
                .children(
                    view.rule_error
                        .clone()
                        .map(|error| rules_field::error_line(error, cx)),
                )
                .into_any_element(),
            callout(view, cx),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button(element_id("cancel"), actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    element_id("confirm"),
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

/// Add's dark-bordered note on how rules match; Edit's red-bordered usage warning while Splits
/// carry the Payee.
fn callout(view: &PayeeFormViewModel, cx: &App) -> AnyElement {
    let border = if view.callout.warning {
        color::negative(cx)
    } else {
        color::foreground(cx)
    };
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(border)
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(view.callout.text.clone())
        .into_any_element()
}

/// The form's fields, for a test.
pub(super) fn readback(view: &PayeeFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("field", format!("{:?}", view.focused)),
        ("name", view.name.text.clone()),
        (
            "category",
            chosen_label(&view.default_category).unwrap_or_default(),
        ),
        ("rules", view.rules.join(", ")),
        ("rule_input", view.rule_input.text.clone()),
        (
            "error",
            view.name_error
                .clone()
                .or_else(|| view.rule_error.clone())
                .unwrap_or_default(),
        ),
    ]
}
