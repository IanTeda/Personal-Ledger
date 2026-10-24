//! Renders the **Add account** dialog (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3b) on the shared
//! `crate::dialog` frame. `Root` owns the live form (`settings::accounts::form::AccountForm`) and every
//! keystroke while it is open (`InputMode::Dialog`); this module only draws it.
//!
//! Width is 440px rather than `dialog::WIDTH`'s 420px, the README's own note: only because the
//! two-up field rows need the room. Text fields follow the Settings dialogs' append/pop-only
//! model. Institution, Type and Unit are the shared dropdown (`crate::form::select_field`); Institution
//! reads as the read-only placeholder while Type is Cash.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use lib_accounts::NO_INSTITUTION;

use super::WIDTH;
use crate::{
    dialog,
    form::{
        dialog_fields::{label, optional_label, text_field, two_up},
        select_field::{self, SelectFieldProps},
    },
    settings::accounts::{
        form::{AccountField, AccountForm, AccountOptions},
        institution_label,
    },
};

pub type OnFieldClick = Rc<dyn Fn(AccountField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(AccountField, usize, &mut Window, &mut App)>;
pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

pub fn render(
    form: &AccountForm,
    options: &AccountOptions,
    on_field_click: OnFieldClick,
    on_option_click: OnOptionClick,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
    cx: &App,
) -> AnyElement {
    let focused = |field: AccountField| form.focused == field;
    let click = |field: AccountField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: AccountField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_option_click(field, index, window, cx)
        })
    };

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_accounts_add_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            text_field(
                "add-account-name",
                label(lib_locale::msg::column_name()),
                form.name.text(),
                &crate::msg::desktop_accounts_name_placeholder(),
                focused(AccountField::Name),
                click(AccountField::Name),
                cx,
            ),
            two_up([
                select_field::render(
                    SelectFieldProps {
                        id: "add-account-institution",
                        label: lib_locale::msg::column_institution().into(),
                        options: &options.institutions,
                        state: &form.institution,
                        focused: focused(AccountField::Institution),
                        accent: false,
                        read_only: form
                            .is_cash()
                            .then(|| institution_label(NO_INSTITUTION).into()),
                        on_field_click: click(AccountField::Institution),
                        on_option_click: option_click(AccountField::Institution),
                    },
                    cx,
                ),
                select_field::render(
                    SelectFieldProps {
                        id: "add-account-type",
                        label: lib_locale::msg::column_type().into(),
                        options: &options.types,
                        state: &form.account_type,
                        focused: focused(AccountField::Type),
                        accent: false,
                        read_only: None,
                        on_field_click: click(AccountField::Type),
                        on_option_click: option_click(AccountField::Type),
                    },
                    cx,
                ),
            ]),
            two_up([
                select_field::render(
                    SelectFieldProps {
                        id: "add-account-unit",
                        label: lib_locale::msg::column_unit().into(),
                        options: &options.units,
                        state: &form.unit,
                        focused: focused(AccountField::Unit),
                        accent: false,
                        read_only: None,
                        on_field_click: click(AccountField::Unit),
                        on_option_click: option_click(AccountField::Unit),
                    },
                    cx,
                ),
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(text_field(
                        "add-account-balance",
                        label(crate::msg::desktop_accounts_field_opening_balance()),
                        form.opening_balance.text(),
                        "0.00",
                        focused(AccountField::OpeningBalance),
                        click(AccountField::OpeningBalance),
                        cx,
                    ))
                    .into_any_element(),
            ]),
            text_field(
                "add-account-number",
                optional_label(crate::msg::desktop_accounts_field_number(), cx),
                form.account_number.text(),
                "\u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} 1234",
                focused(AccountField::AccountNumber),
                click(AccountField::AccountNumber),
                cx,
            ),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-account-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "add-account-confirm",
                    crate::msg::desktop_accounts_add_submit(),
                    form.is_valid(),
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}
