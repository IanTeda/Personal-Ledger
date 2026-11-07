//! The Add and Edit account Dialogs (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3b and 3c)
//! on the shared `crate::dialog` frame, 440px wide rather than `dialog::WIDTH`'s 420px because the
//! two-up field rows need the room. Name, Opening balance and Account number are typed into;
//! Institution, Type and Unit are the shared dropdown (`crate::form::select_field`), whose list
//! the Presenter holds and every key of which it reads. Edit shows Unit and Opening balance
//! read-only, with the reason in the usage notice.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_accounts::presenter::{
    AccountField, AccountFormEvent, AccountFormPresenter, AccountFormViewModel,
};
use lib_shell::Hint;
use lib_viper::view::SelectList;

use crate::form::select_field::OnOptionClick;
use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::{
        colour_field::error_line,
        dialog_fields::{label, optional_label, suffixed_label, text_field, two_up},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    theme::color,
};

const WIDTH: gpui::Pixels = px(440.0);

/// The open form's clicks.
type Clicks = ModuleDialogClicks<AccountFormPresenter>;

/// The `lib-accounts` form Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056). Its
/// widget state is the top row of each select's list window (Institution, Type, Unit), which the
/// View keeps between draws.
impl DialogView for AccountFormPresenter {
    type Widgets = [usize; 3];

    fn kind(view: &AccountFormViewModel) -> &'static str {
        if view.editing {
            "EditAccount"
        } else {
            "AddAccount"
        }
    }

    fn render(
        view: &AccountFormViewModel,
        scrolls: &mut [usize; 3],
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scrolls, clicks, cx)
    }

    /// The focused field, the drafts, the selects, whether a list is open and whether it would
    /// confirm; Edit also reads back the `transactions` and `budgets` counts it quotes.
    fn readback(view: &AccountFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &AccountFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// `field`'s list as the Presenter holds it: the focused select's, else closed.
fn list_of(view: &AccountFormViewModel, field: AccountField) -> SelectList {
    if view.focused == field {
        view.list
    } else {
        SelectList::default()
    }
}

fn render(
    view: &AccountFormViewModel,
    scrolls: &mut [usize; 3],
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let [institution_scroll, type_scroll, unit_scroll] = scrolls;
    let institution_state = drawn(
        &view.institution,
        list_of(view, AccountField::Institution),
        institution_scroll,
    );
    let type_state = drawn(
        &view.account_type,
        list_of(view, AccountField::Type),
        type_scroll,
    );
    let unit_state = drawn(&view.unit, list_of(view, AccountField::Unit), unit_scroll);
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
    // A click on a text field focuses it, closing an open list.
    let on_text = |field: AccountField| actions.send(AccountFormEvent::Focus(field));
    // A click on a select's closed field focuses it and toggles its list. A field that refuses
    // focus takes no click.
    let on_select = |field: AccountField| actions.send(AccountFormEvent::ToggleList(field));
    // A click on row `index` of an open list picks its choice; the lists are fixed while the
    // Dialog is open.
    let institutions: Vec<_> = view.institution.options.iter().map(|c| c.value).collect();
    let types: Vec<_> = view
        .account_type
        .options
        .iter()
        .map(|c| c.value.clone())
        .collect();
    let units: Vec<_> = view.unit.options.iter().map(|c| c.value).collect();
    let on_option = |field: AccountField| -> OnOptionClick {
        match field {
            AccountField::Institution => {
                let institutions = institutions.clone();
                actions.option(move |index| {
                    institutions
                        .get(index)
                        .map(|id| AccountFormEvent::Institution(*id))
                        .into_iter()
                        .collect()
                })
            }
            AccountField::Type => {
                let types = types.clone();
                actions.option(move |index| {
                    types
                        .get(index)
                        .map(|account_type| AccountFormEvent::Type(account_type.clone()))
                        .into_iter()
                        .collect()
                })
            }
            AccountField::Unit => {
                let units = units.clone();
                actions.option(move |index| {
                    units
                        .get(index)
                        .map(|id| AccountFormEvent::Unit(*id))
                        .into_iter()
                        .collect()
                })
            }
            _ => actions.option(|_| Vec::new()),
        }
    };
    let select = |id: &'static str,
                  label: &str,
                  options: Vec<String>,
                  state: SelectState,
                  focused: bool,
                  field: AccountField| {
        select_field::render(
            SelectFieldProps {
                id: id.into(),
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
            unit_state,
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
                institution_state,
                view.institution.focused,
                AccountField::Institution,
            ),
            select(
                type_id,
                &view.account_type.label,
                labels(&view.account_type),
                type_state,
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
fn readback(view: &AccountFormViewModel) -> Vec<(&'static str, String)> {
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
        (
            "balance",
            view.balance_read_only.clone().unwrap_or_default(),
        ),
        ("account_number", view.account_number.text.clone()),
        ("select_open", view.list.is_open().to_string()),
        ("valid", view.frame.can_confirm.to_string()),
        ("error", view.error.clone().unwrap_or_default()),
    ];
    if let Some(usage) = &view.usage {
        values.push(("transactions", usage.transactions.to_string()));
        values.push(("budgets", usage.budgets.to_string()));
    }
    values
}
