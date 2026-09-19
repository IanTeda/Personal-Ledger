//! Renders the **Edit account** dialog (`docs/ux/desktop/Accounts/README.md`'s 3c) on the same
//! form and chrome as Add (`super::add_dialog`), pre-filled from the account.
//!
//! **Editable**: Name, Institution, Type, and the UI-only Account number. **Shown read-only**:
//! Unit and Opening balance -- the glossary fixes a Unit at creation, and an opening balance
//! can't move once transactions have been posted against it -- with the reason in the info panel
//! rather than the mockup's own "changing the unit is not recommended" (a deliberate departure,
//! recorded on the Desktop Accounts map). Institution reads as the placeholder while Type is
//! Cash, as in Add.

use gpui::{AnyElement, SharedString, div, prelude::*, px};

use super::{
    add_dialog::{
        self, OnCancel, OnConfirm, OnFieldClick, OnOptionClick, label, suffixed_label, text_field,
        two_up,
    },
    select_field::{self, SelectFieldProps},
};
use crate::{
    accounts::{self, Account, AccountField, AccountForm, AccountOptions, NO_INSTITUTION},
    dialog,
    theme::color,
};

pub fn render(
    form: &AccountForm,
    account: &Account,
    options: &AccountOptions,
    on_field_click: OnFieldClick,
    on_option_click: OnOptionClick,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let focused = |field: AccountField| form.focused == field;
    let click = |field: AccountField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        std::rc::Rc::new(move |window: &mut gpui::Window, cx: &mut gpui::App| {
            on_field_click(field, window, cx)
        })
    };
    let option_click = |field: AccountField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        std::rc::Rc::new(
            move |index: usize, window: &mut gpui::Window, cx: &mut gpui::App| {
                on_option_click(field, index, window, cx)
            },
        )
    };

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header("Edit account", false))
        .child(dialog::body([
            text_field(
                "edit-account-name",
                label("Name"),
                &form.name,
                "e.g. Everyday Account",
                focused(AccountField::Name),
                click(AccountField::Name),
            ),
            two_up([
                select_field::render(SelectFieldProps {
                    id: "edit-account-institution",
                    label: "Institution".into(),
                    options: &options.institutions,
                    state: &form.institution,
                    focused: focused(AccountField::Institution),
                    read_only: form.is_cash().then_some(NO_INSTITUTION),
                    on_field_click: click(AccountField::Institution),
                    on_option_click: option_click(AccountField::Institution),
                }),
                select_field::render(SelectFieldProps {
                    id: "edit-account-type",
                    label: "Type".into(),
                    options: &options.types,
                    state: &form.account_type,
                    focused: focused(AccountField::Type),
                    read_only: None,
                    on_field_click: click(AccountField::Type),
                    on_option_click: option_click(AccountField::Type),
                }),
            ]),
            two_up([
                read_only_field(
                    "edit-account-unit",
                    suffixed_label("Unit", "(fixed)"),
                    account.unit.clone(),
                ),
                read_only_field(
                    "edit-account-balance",
                    suffixed_label("Opening balance", "(fixed)"),
                    accounts::format_amount(&account.balance).1,
                ),
            ]),
            text_field(
                "edit-account-number",
                add_dialog::suffixed_label("Account number", "(optional)"),
                &form.account_number,
                "\u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} 1234",
                focused(AccountField::AccountNumber),
                click(AccountField::AccountNumber),
            ),
            dialog::info_panel(usage_notice(account)).into_any_element(),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("edit-account-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "edit-account-confirm",
                "Save",
                form.is_valid(),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(add_dialog::WIDTH, false, card)
}

/// `Opened Mar 2019 · 312 transactions. Renaming is safe. ...` -- the usage notice, with the
/// reason Unit and Opening balance can't be edited. Counts are the stub figures on the account.
fn usage_notice(account: &Account) -> String {
    let count = account.transaction_count;
    let noun = if count == 1 {
        "transaction"
    } else {
        "transactions"
    };
    format!(
        "Opened {} \u{b7} {count} {noun}. Renaming is safe. Unit and opening balance are fixed \
         once an account exists.",
        account.opened_at.format("%b %Y"),
    )
}

/// A label above a non-interactive box in the chrome tint: the Unit and Opening balance an
/// existing account can no longer change.
fn read_only_field(id: &'static str, label: AnyElement, value: String) -> AnyElement {
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
                .border_color(color::BORDER)
                .bg(color::CHROME)
                .text_size(px(13.0))
                .text_color(color::INK_TERTIARY)
                .child(SharedString::from(value)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_notice_names_the_month_the_count_and_the_reason() {
        let mut account = accounts::default_accounts()
            .into_iter()
            .find(|a| a.name == "ANZ Everyday")
            .expect("seeded");
        let notice = usage_notice(&account);
        assert!(notice.starts_with("Opened Mar 2019 \u{b7} 312 transactions."));
        assert!(notice.contains("Renaming is safe"));
        assert!(notice.contains("fixed once an account exists"));

        account.transaction_count = 1;
        assert!(usage_notice(&account).contains("1 transaction."));
        account.transaction_count = 0;
        assert!(usage_notice(&account).contains("0 transactions."));
    }
}
