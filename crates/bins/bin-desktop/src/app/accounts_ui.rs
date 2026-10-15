//! The Accounts dialogs' side of the Dialog host, and the `accounts` command. The page itself, its
//! keys and the dialogs it opens are the `AccountsView` Entity's (`view::accounts::state`); the
//! pure rules live in `accounts`.

use super::Shell;

use crate::{
    accounts::{
        apply::{AccountOutcome, AccountsApply},
        form::{AccountField, AccountOptions, AccountsDialog},
    },
    chrome::dialog_apply,
    navigation::command::AccountsVerb,
    settings::SettingsPage,
    view::accounts::state::{AccountsView, dialog_options},
};

use gpui::{App, Context, Entity};
use lib_toast::ToastKind;

impl Shell {
    /// The Accounts page's Entity, which the Settings View owns.
    fn accounts_page(&self, cx: &App) -> Entity<AccountsView> {
        self.views.settings.read(cx).accounts().clone()
    }

    /// The Accounts dialogs' select options, for the Dialog host to draw them with.
    pub(super) fn account_dialog_options(&self, cx: &App) -> AccountOptions {
        dialog_options(&self.ledger, cx)
    }

    /// A click on a field of the Add account dialog: focuses a text field, or focuses a select
    /// and toggles its list.
    pub(super) fn handle_accounts_dialog_field_click(
        &mut self,
        field: AccountField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .dialog_mut::<AccountsDialog>()
            .and_then(AccountsDialog::form_mut)
        {
            if field.is_select() {
                form.click_select(field);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    /// A click on a row of an open dropdown list.
    pub(super) fn handle_accounts_dialog_option_click(
        &mut self,
        field: AccountField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .dialog_mut::<AccountsDialog>()
            .and_then(AccountsDialog::form_mut)
        {
            form.choose_option(field, index);
        }
        cx.notify();
    }

    /// Confirms the open Accounts dialog through the host's cycle: the form's command runs
    /// against the Accounts store, and a landed one closes the dialog. Add and Edit select the
    /// account (an Edit regroups it if its Type changed); Delete toasts the count of Transactions
    /// that went with it and keeps the selection in range, so it lands on the account that slid
    /// into the deleted row's place (or the last one).
    pub(super) fn confirm_accounts_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = AccountsApply {
            ledger: self.ledger.clone(),
        };
        let Some(dialog) = self.dialog_mut::<AccountsDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        let page = self.accounts_page(cx);
        match outcome {
            AccountOutcome::Saved(id) => page.update(cx, |page, cx| page.select_id(id, cx)),
            AccountOutcome::Deleted { name, transactions } => {
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_account_deleted(
                        &name,
                        i64::try_from(transactions).unwrap_or(i64::MAX),
                    ),
                );
                page.update(cx, |page, cx| page.clamp_selection(cx));
            }
        }
    }

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, whose Entity then
    /// opens the same dialog its `n`/`e`/`d` and buttons do.
    pub(super) fn run_accounts_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_settings_page(SettingsPage::Accounts, cx);
        self.accounts_page(cx).update(cx, |page, cx| {
            page.run_command(command_name, verb, argument, cx);
        });
    }
}
