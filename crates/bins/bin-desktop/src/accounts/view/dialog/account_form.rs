//! The Add and Edit account Dialogs (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3b and 3c)
//! on the shared `crate::dialog` frame, 440px wide rather than `dialog::WIDTH`'s 420px because the
//! two-up field rows need the room. Name, Opening balance and Account number are typed into;
//! Institution, Type and Unit are the shared dropdown (`crate::form::select_field`), which takes
//! the arrows, `Enter` and `Space` and swallows other typing rather than letting it reach the
//! Shell. Edit shows Unit and Opening balance read-only, with the reason in the usage notice.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_accounts::presenter::{AccountField, AccountFormEvent, AccountFormViewModel};

use super::{AccountsDialog, Clicks, Selects};
use crate::form::viper::{
    choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
};
use crate::{
    dialog,
    form::{
        colour_field::error_line,
        dialog_fields::{label, optional_label, suffixed_label, text_field, two_up},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

const WIDTH: gpui::Pixels = px(440.0);

/// A key on the Dialog: `Tab` commits an open list's highlight and moves field; a select takes
/// the arrows, `Enter` and `Space`; a text field takes typing, and `Enter` confirms a valid form.
pub(super) fn key(dialog: &mut AccountsDialog, key: DialogKey) -> DialogOutcome {
    let view = dialog.presenter.view_model();
    if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
        if let Some(event) = select_event(dialog, &view, DialogKey::Enter, true) {
            dialog.send(event);
        }
        let event = if key == DialogKey::Tab {
            AccountFormEvent::NextField
        } else {
            AccountFormEvent::PrevField
        };
        dialog.send(event);
        dialog.selects.close_all();
        return DialogOutcome::Handled;
    }
    match view.focused {
        AccountField::Institution | AccountField::Type | AccountField::Unit => {
            let taken = matches!(
                key,
                DialogKey::Up | DialogKey::Down | DialogKey::Enter | DialogKey::Char(' ')
            );
            if let Some(event) = select_event(dialog, &view, key, false) {
                dialog.send(event);
            }
            match key {
                // A select swallows typing rather than letting it fall through to the Shell.
                DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
                _ if taken => DialogOutcome::Handled,
                _ => DialogOutcome::Ignored,
            }
        }
        AccountField::Name => text_key(dialog, &view, key, &view.name.text, AccountFormEvent::Name),
        AccountField::OpeningBalance => text_key(
            dialog,
            &view,
            key,
            &view.opening_balance.text,
            AccountFormEvent::OpeningBalance,
        ),
        AccountField::AccountNumber => text_key(
            dialog,
            &view,
            key,
            &view.account_number.text,
            AccountFormEvent::AccountNumber,
        ),
    }
}

/// Runs `key` through the focused select, returning the event for a changed choice. With
/// `only_open`, only an open list takes the key (a `Tab` commits its highlight).
fn select_event(
    dialog: &mut AccountsDialog,
    view: &AccountFormViewModel,
    key: DialogKey,
    only_open: bool,
) -> Option<AccountFormEvent> {
    let selects = &mut dialog.selects;
    match view.focused {
        AccountField::Institution if !only_open || selects.institution.is_open() => {
            select_key(&mut selects.institution, &view.institution, key)
                .1
                .map(AccountFormEvent::Institution)
        }
        AccountField::Type if !only_open || selects.account_type.is_open() => {
            select_key(&mut selects.account_type, &view.account_type, key)
                .1
                .map(AccountFormEvent::Type)
        }
        AccountField::Unit if !only_open || selects.unit.is_open() => {
            select_key(&mut selects.unit, &view.unit, key)
                .1
                .map(AccountFormEvent::Unit)
        }
        _ => None,
    }
}

/// A key on a text field: typing edits it, `Enter` confirms a valid form, the arrows do nothing.
fn text_key(
    dialog: &mut AccountsDialog,
    view: &AccountFormViewModel,
    key: DialogKey,
    text: &str,
    event: fn(String) -> AccountFormEvent,
) -> DialogOutcome {
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Up | DialogKey::Down => DialogOutcome::Handled,
        _ => match typed(text, key) {
            Some(text) => {
                dialog.send(event(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

/// A click on a text field focuses it, closing a list open on another field.
fn click_text(dialog: &mut AccountsDialog, field: AccountField) {
    if dialog.presenter.view_model().focused != field {
        dialog.selects.close_all();
    }
    dialog.send(AccountFormEvent::Focus(field));
}

/// A click on a select's closed field focuses it and toggles its list. A field that refuses focus
/// takes no click.
fn click_select(dialog: &mut AccountsDialog, field: AccountField) {
    dialog.send(AccountFormEvent::Focus(field));
    let view = dialog.presenter.view_model();
    if view.focused != field {
        return;
    }
    dialog.selects.close_except(field);
    match field {
        AccountField::Institution => {
            toggle_select(&mut dialog.selects.institution, &view.institution);
        }
        AccountField::Type => toggle_select(&mut dialog.selects.account_type, &view.account_type),
        AccountField::Unit => toggle_select(&mut dialog.selects.unit, &view.unit),
        _ => {}
    }
}

/// A click on row `index` of an open list.
fn click_option(dialog: &mut AccountsDialog, field: AccountField, index: usize) {
    let view = dialog.presenter.view_model();
    let selects = &mut dialog.selects;
    let event = match field {
        AccountField::Institution => {
            choose_option(&mut selects.institution, &view.institution, index)
                .map(AccountFormEvent::Institution)
        }
        AccountField::Type => choose_option(&mut selects.account_type, &view.account_type, index)
            .map(AccountFormEvent::Type),
        AccountField::Unit => {
            choose_option(&mut selects.unit, &view.unit, index).map(AccountFormEvent::Unit)
        }
        _ => None,
    };
    if let Some(event) = event {
        dialog.send(event);
    }
}

pub(super) fn render(
    view: &AccountFormViewModel,
    selects: &Selects,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let (name_id, institution_id, type_id, unit_id, balance_id, number_id, cancel_id, confirm_id) =
        if view.editing {
            (
                "edit-account-name",
                "edit-account-institution",
                "edit-account-type",
                "edit-account-unit",
                "edit-account-balance",
                "edit-account-number",
                "edit-account-cancel",
                "edit-account-confirm",
            )
        } else {
            (
                "add-account-name",
                "add-account-institution",
                "add-account-type",
                "add-account-unit",
                "add-account-balance",
                "add-account-number",
                "add-account-cancel",
                "add-account-confirm",
            )
        };
    let on_text = |field: AccountField| actions.click(move |dialog| click_text(dialog, field));
    let on_select = |field: AccountField| actions.click(move |dialog| click_select(dialog, field));
    let on_option = |field: AccountField| {
        actions.option(move |dialog, index| click_option(dialog, field, index))
    };
    let select = |id: &'static str,
                  label: &str,
                  options: Vec<String>,
                  state: SelectState,
                  focused: bool,
                  field: AccountField| {
        select_field::render(
            SelectFieldProps {
                id,
                label: label.to_string().into(),
                options: &options,
                state: &state,
                focused,
                accent: false,
                read_only: None,
                on_field_click: on_select(field),
                on_option_click: on_option(field),
            },
            cx,
        )
    };

    let unit = match &view.unit_read_only {
        Some(code) => read_only_field(
            unit_id,
            suffixed_label(
                view.unit.label.clone(),
                crate::msg::desktop_field_fixed(),
                cx,
            ),
            code.clone(),
            cx,
        ),
        None => select(
            unit_id,
            &view.unit.label,
            labels(&view.unit),
            synced(&selects.unit, &view.unit),
            view.unit.focused,
            AccountField::Unit,
        ),
    };
    let balance = match &view.balance_read_only {
        Some(text) => read_only_field(
            balance_id,
            suffixed_label(
                view.opening_balance.label.clone(),
                crate::msg::desktop_field_fixed(),
                cx,
            ),
            text.clone(),
            cx,
        ),
        None => div()
            .flex_1()
            .min_w(px(0.0))
            .child(text_field(
                balance_id,
                label(view.opening_balance.label.clone()),
                &view.opening_balance.text,
                &view.opening_balance.placeholder,
                view.opening_balance.focused,
                on_text(AccountField::OpeningBalance),
                cx,
            ))
            .into_any_element(),
    };
    let mut body = vec![
        text_field(
            name_id,
            label(view.name.label.clone()),
            &view.name.text,
            &view.name.placeholder,
            view.name.focused,
            on_text(AccountField::Name),
            cx,
        ),
        two_up([
            select(
                institution_id,
                &view.institution.label,
                labels(&view.institution),
                synced(&selects.institution, &view.institution),
                view.institution.focused,
                AccountField::Institution,
            ),
            select(
                type_id,
                &view.account_type.label,
                labels(&view.account_type),
                synced(&selects.account_type, &view.account_type),
                view.account_type.focused,
                AccountField::Type,
            ),
        ]),
        two_up([unit, balance]),
        text_field(
            number_id,
            optional_label(view.account_number.label.clone(), cx),
            &view.account_number.text,
            &view.account_number.placeholder,
            view.account_number.focused,
            on_text(AccountField::AccountNumber),
            cx,
        ),
    ];
    if let Some(usage) = &view.usage {
        body.push(dialog::info_panel(usage.notice.clone(), cx).into_any_element());
    }
    if let Some(error) = view.error.clone() {
        body.push(error_line(error, cx).into_any_element());
    }
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button(cancel_id, actions.cancel.clone(), cx).into_any_element(),
                dialog::confirm_button(
                    confirm_id,
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

/// A label above a non-interactive box in the `chrome` tint: the Unit and Opening balance an
/// existing account can no longer change.
fn read_only_field(id: &'static str, label: AnyElement, value: String, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .child(label)
        .child(
            div()
                .id(id)
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .text_size(px(13.0))
                .text_color(color::faint_text(cx))
                .child(SharedString::from(value)),
        )
        .into_any_element()
}

/// The form's fields, for a test: the focused field's variant name, the drafts, the selects'
/// chosen labels, the Unit codes on offer, and whether a list is open. Edit adds the counts it
/// quotes.
pub(super) fn readback(
    view: &AccountFormViewModel,
    select_open: bool,
) -> Vec<(&'static str, String)> {
    let mut values = vec![
        ("focused", format!("{:?}", view.focused)),
        ("name", view.name.text.clone()),
        (
            "institution",
            chosen_label(&view.institution).unwrap_or_default(),
        ),
        ("type", chosen_label(&view.account_type).unwrap_or_default()),
        ("unit", chosen_label(&view.unit).unwrap_or_default()),
        ("unit_options", labels(&view.unit).join("|")),
        ("opening_balance", view.opening_balance.text.clone()),
        ("account_number", view.account_number.text.clone()),
        ("select_open", select_open.to_string()),
        ("valid", view.frame.can_confirm.to_string()),
        ("error", view.error.clone().unwrap_or_default()),
    ];
    if let Some(usage) = &view.usage {
        values.push(("transactions", usage.transactions.to_string()));
        values.push(("budgets", usage.budgets.to_string()));
    }
    values
}
