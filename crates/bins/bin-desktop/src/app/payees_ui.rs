//! The Payees dialogs' side of the Dialog host. The Settings page, its keys and the dialogs it
//! opens are the `PayeesView` Entity's (`view::payees::state`); the pure rules live in `lib_payees`
//! and the dialogs' drawing in `view::payees`.

use super::Shell;
use crate::{
    chrome::dialog_apply,
    payees::{
        self, Payee,
        apply::{PayeeOutcome, PayeesApply},
        form::PayeesDialog,
    },
    view::payees::state::PayeesView,
};

use gpui::{App, Context, Entity};
use lib_toast::ToastKind;

impl Shell {
    /// The Payees rows, read through their store (ADR-0032).
    pub(super) fn payees_list<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.payees_store.read(cx).payees(cx)
    }

    /// The Payees page's Entity, which the Settings View owns.
    fn payees_page(&self, cx: &App) -> Entity<PayeesView> {
        self.views.settings.read(cx).payees().clone()
    }

    /// Confirms the open Payees dialog through the host's cycle: the form's command runs against
    /// the Ledger, a refusal stays on the form, and a landed one closes the dialog. Add and
    /// Edit then select the Payee; Delete toasts the outcome, and the page's selection falls back
    /// to its first row if the Payee is gone.
    pub(super) fn confirm_payees_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = PayeesApply {
            ledger: self.ledger.clone(),
        };
        let Some(dialog) = self.dialog_mut::<PayeesDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        match outcome {
            PayeeOutcome::Saved(id) => {
                self.payees_page(cx)
                    .update(cx, |page, cx| page.select_id(id, cx));
            }
            PayeeOutcome::Removed { name, action } => {
                self.raise_toast(
                    ToastKind::Success,
                    match action {
                        payees::form::DeleteAction::Delete => {
                            lib_locale::msg::toast_payee_deleted(&name)
                        }
                        payees::form::DeleteAction::Deactivate => {
                            lib_locale::msg::toast_payee_deactivated(&name)
                        }
                        payees::form::DeleteAction::Reactivate => {
                            lib_locale::msg::toast_payee_reactivated(&name)
                        }
                    },
                );
            }
        }
    }

    /// The Payee the Delete dialog is open on and what confirming it would do (#283).
    pub(super) fn delete_payee_target<'a>(
        &self,
        cx: &'a App,
    ) -> Option<(&'a Payee, payees::form::DeleteAction)> {
        let Some(PayeesDialog::Delete(id, form)) = self.dialog::<PayeesDialog>() else {
            return None;
        };
        Some((payees::get(self.payees_list(cx), *id)?, form.action()))
    }
}
