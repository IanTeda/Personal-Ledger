//! Renders the **Delete account** destructive-confirm dialog (`docs/ux/desktop/Accounts/README.md`'s
//! 3d) on the shared `crate::dialog` chrome's destructive variant, following the Settings Delete
//! unit dialog: the consequences are named and the account's own name must be typed back exactly
//! (`accounts::DeleteAccountForm::matches`, case-sensitive) before **Delete account** enables.
//!
//! The balance is the account's own; the transaction and budget counts are its stub figures
//! (nothing real backs them in this map). Transfer-on-delete, which the TUI design makes the
//! primary path, is out of scope here: the desktop design has the transactions deleted with the
//! account, and says so.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use super::add_dialog::{WIDTH, text_field};
use crate::{
    accounts::{self, Account, DeleteAccountForm},
    dialog,
};

pub fn render(
    account: &Account,
    form: &DeleteAccountForm,
    on_cancel: dialog::OnClick,
    on_confirm: dialog::OnClick,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_accounts_delete_title(&account.name),
            true,
        ))
        .child(dialog::body([
            warning_copy(account),
            dialog::info_panel(reference_notice(account)).into_any_element(),
            text_field(
                "delete-account-confirm-input",
                confirm_label(&account.name),
                &form.confirm_input,
                &account.name,
                true,
                Rc::new(|_window: &mut Window, _cx: &mut App| {}),
            ),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("delete-account-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "delete-account-confirm",
                crate::msg::desktop_accounts_delete_submit(),
                form.matches(&account.name),
                true,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(WIDTH, true, card)
}

/// `-2,318.44 aud`, the balance in the account's own Unit.
fn balance_text(account: &Account) -> String {
    format!(
        "{} {}",
        crate::format::amount(&account.balance).1,
        account.unit
    )
}

/// `204 transactions will be permanently deleted · referenced in 2 budgets`.
fn reference_notice(account: &Account) -> String {
    crate::msg::desktop_accounts_delete_reference(
        i64::from(account.transaction_count),
        i64::from(account.budget_count),
    )
}

/// `This account has a balance of **−2,318.44 aud** and **204 transactions**. Deleting it cannot
/// be undone.` -- the two figures bold, as in the mockup.
fn warning_copy(account: &Account) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .text_size(px(13.0))
        .children(
            crate::msg::desktop_accounts_delete_warning(
                &balance_text(account),
                i64::from(account.transaction_count),
            )
            .into_iter()
            .map(|segment| match segment.tag.as_deref() {
                Some("strong") => div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(segment.text)
                    .into_any_element(),
                _ => div().child(segment.text).into_any_element(),
            }),
        )
        .into_any_element()
}

fn confirm_label(name: &str) -> AnyElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(crate::msg::desktop_accounts_delete_confirm_label(name))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded(name: &str) -> Account {
        accounts::default_accounts()
            .into_iter()
            .find(|account| account.name == name)
            .expect("seeded")
    }

    #[test]
    fn the_reference_notice_names_the_transactions_and_budgets() {
        crate::locale::init_for_tests();
        assert_eq!(
            reference_notice(&seeded("Amex Platinum")),
            "204 transactions will be permanently deleted \u{b7} referenced in 2 budgets"
        );
    }

    #[test]
    fn the_reference_notice_handles_singular_and_none() {
        crate::locale::init_for_tests();
        let mut account = seeded("Amex Platinum");
        account.transaction_count = 1;
        account.budget_count = 1;
        assert_eq!(
            reference_notice(&account),
            "1 transaction will be permanently deleted \u{b7} referenced in 1 budget"
        );
        account.budget_count = 0;
        assert!(reference_notice(&account).ends_with("not referenced by any budget"));
    }

    #[test]
    fn the_balance_is_stated_in_the_accounts_own_unit_with_a_real_minus() {
        assert_eq!(
            balance_text(&seeded("Amex Platinum")),
            "\u{2212}2,318.44 aud"
        );
        assert_eq!(balance_text(&seeded("Bitcoin")), "0.4120 btc");
    }

    #[test]
    fn the_warning_emphasises_the_balance_and_the_transaction_count() {
        crate::locale::init_for_tests();
        let strong: Vec<String> = crate::msg::desktop_accounts_delete_warning("1.00 aud", 3)
            .into_iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text)
            .collect();
        assert_eq!(
            strong,
            vec!["1.00 aud".to_string(), "3 transactions.".to_string()]
        );
    }

    #[test]
    fn the_accounts_messages_render_in_the_pseudo_locale() {
        crate::locale::init_for_tests();
        lib_locale::with_locale(lib_locale::Locale::EnXa, || {
            let account = seeded("Amex Platinum");
            assert!(reference_notice(&account).starts_with('['));
            assert!(crate::msg::desktop_accounts_delete_title("x").starts_with('['));
            assert!(
                crate::msg::desktop_accounts_delete_warning("1", 2)
                    .iter()
                    .any(|segment| segment.tag.as_deref() == Some("strong"))
            );
        });
    }
}
