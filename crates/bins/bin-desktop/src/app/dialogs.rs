//! The Dialog host plumbing of `Shell`: opening, closing, confirming and the typed per-domain
//! accessors. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::settings::payees::PayeesPage;
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm},
        dialog_host::{Dialog, DialogVariant, OpenDialog},
    },
    navigation::nav::InputMode,
    settings::{
        accounts::{
            apply::{AccountOutcome, AccountsApply},
            dialogs::AccountsDialog,
        },
        institutions::{apply::InstitutionsApply, dialogs::InstitutionsDialog},
        payees::{
            apply::{PayeeOutcome, PayeesApply},
            dialogs::PayeesDialog,
        },
        units::{apply::UnitsApply, dialogs::UnitsDialog},
    },
};
use lib_toast::ToastKind;

impl Shell {
    /// Opens `dialog` in the Dialog host, entering `InputMode::Dialog` with it.
    pub(super) fn open_dialog(&mut self, dialog: OpenDialog) {
        self.chrome.dialog = Some(dialog);
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Closes the Dialog host's Dialog, if one is open, without applying it.
    pub(super) fn close_dialog(&mut self) {
        if self.chrome.dialog.take().is_some() {
            self.nav.exit_mode();
        }
    }

    /// `Enter` and the confirm button both land here: applies the open Dialog if its form is
    /// valid, then closes it. A no-op while the form is invalid, leaving the Dialog open.
    pub(super) fn confirm_open_dialog(&mut self, cx: &mut gpui::Context<'_, Self>) {
        if !self.chrome.dialog.as_ref().is_some_and(Dialog::is_valid) {
            return;
        }
        // Every domain's Dialog runs through the host's cycle (`DialogForm`/`Apply`, ADR-0037).
        match self.chrome.dialog {
            Some(OpenDialog::Settings(_)) => self.confirm_settings_dialog(cx),
            Some(OpenDialog::Units(_)) => {
                let mut apply = UnitsApply {
                    ledger: self.ledger.clone(),
                };
                let toast = self
                    .confirm_dialog::<UnitsDialog, _>(&mut apply, cx)
                    .and_then(|outcome| outcome.toast());
                if let Some((kind, message)) = toast {
                    self.raise_toast(kind, message);
                }
            }
            Some(OpenDialog::Institutions(_)) => {
                let mut apply = InstitutionsApply {
                    ledger: self.ledger.clone(),
                };
                self.confirm_dialog::<InstitutionsDialog, _>(&mut apply, cx);
            }
            Some(OpenDialog::Accounts(_)) => self.confirm_accounts_dialog(cx),
            Some(OpenDialog::Categories(_)) => self.confirm_categories_dialog(cx),
            Some(OpenDialog::Payees(_)) => {
                let mut apply = PayeesApply {
                    ledger: self.ledger.clone(),
                };
                let outcome = self.confirm_dialog::<PayeesDialog, _>(&mut apply, cx);
                if let Some(PayeeOutcome::Saved(id)) = outcome {
                    self.payees_page(cx)
                        .update(cx, |page, cx| page.select_id(id, cx));
                }
                if let Some((kind, message)) = outcome.as_ref().and_then(PayeeOutcome::toast) {
                    self.raise_toast(kind, message);
                }
            }
            Some(OpenDialog::DocumentTypes(_)) => self.confirm_document_types_dialog(cx),
            Some(OpenDialog::Inventory(_)) => self.confirm_inventory_dialog(cx),
            Some(OpenDialog::Tags(_)) => self.confirm_tags_dialog(cx),
            Some(OpenDialog::Bills(_)) => self.confirm_bills_dialog(cx),
            Some(OpenDialog::Budgets(_)) => self.confirm_budgets_dialog(cx),
            Some(OpenDialog::Documents(_)) => self.confirm_documents_dialog(cx),
            // Read-only: it has nothing to apply.
            Some(OpenDialog::ToastHistory(_)) => self.close_dialog(),
            None => {}
        }
    }

    /// The Payees page's Entity, which the Settings View owns.
    pub(super) fn payees_page(&self, cx: &gpui::App) -> gpui::Entity<PayeesPage> {
        self.views.settings.read(cx).payees().clone()
    }

    /// Confirms the open `T` Dialog through the host's cycle (ADR-0037): the form's command runs
    /// through `apply`, and a landed one closes the Dialog. `None` while the form is invalid or
    /// the command was refused, leaving the Dialog open.
    fn confirm_dialog<T, A>(&mut self, apply: &mut A, cx: &mut gpui::App) -> Option<A::Outcome>
    where
        T: DialogVariant + DialogForm,
        A: Apply<T::Command, Refusal = T::Refusal>,
    {
        let outcome = dialog_apply::confirm(self.dialog_mut::<T>()?, apply, cx)?;
        self.close_dialog();
        Some(outcome)
    }

    /// The cancel button: closes the Dialog without applying it.
    pub(super) fn handle_dialog_cancel(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    /// The confirm button, the click twin of `Enter`.
    pub(super) fn handle_dialog_confirm(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// A click inside the open `T` Dialog: runs `change` on it and repaints. A no-op when another
    /// feature's Dialog is open.
    pub(super) fn edit_dialog<T: DialogVariant>(
        &mut self,
        cx: &mut gpui::Context<'_, Self>,
        change: impl FnOnce(&mut T),
    ) {
        if let Some(dialog) = self.dialog_mut::<T>() {
            change(dialog);
        }
        cx.notify();
    }

    /// The open Dialog as a `T`, `None` when no Dialog or another feature's is open.
    pub(super) fn dialog<T: DialogVariant>(&self) -> Option<&T> {
        T::from_open(self.chrome.dialog.as_ref()?)
    }

    pub(super) fn dialog_mut<T: DialogVariant>(&mut self) -> Option<&mut T> {
        T::from_open_mut(self.chrome.dialog.as_mut()?)
    }
}

impl Shell {
    /// Confirms the open Accounts dialog through the host's cycle: the form's command runs against
    /// the Ledger, and a landed one closes the dialog. Add and Edit select the account (an Edit
    /// regroups it if its Type changed); Delete toasts the count of Transactions that went with it
    /// and keeps the selection in range, so it lands on the account that slid into the deleted
    /// row's place (or the last one).
    pub(super) fn confirm_accounts_dialog(&mut self, cx: &mut gpui::Context<'_, Self>) {
        let mut apply = AccountsApply {
            ledger: self.ledger.clone(),
        };
        let Some(outcome) = self.confirm_dialog::<AccountsDialog, _>(&mut apply, cx) else {
            return;
        };
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
}
