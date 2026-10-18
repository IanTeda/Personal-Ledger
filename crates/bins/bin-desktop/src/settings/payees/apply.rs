//! The Payees Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger.

use gpui::{App, Entity};
use lib_core::PayeeId;
use lib_ledger::{
    AddPayee, DeletePayee, DeletePayeeRefusal, EditPayee, SetPayeeActive, SetPayeeActiveRefusal,
};
use lib_toast::ToastKind;

use lib_payees::{Payee, PayeeDraft, PayeeError};

use super::{dialogs::PayeesDialog, form::DeleteAction};
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm, FormError},
        dialog_host::{ConfirmDialog, DialogScope, Landed},
    },
    ledger_store::LedgerStore,
};

/// What confirming a Payees Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeCommand {
    Add(PayeeDraft),
    Edit(PayeeId, PayeeDraft),
    Remove(PayeeId, DeleteAction),
}

/// What a landed [`PayeeCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeOutcome {
    /// Added or edited: the Payee to select.
    Saved(PayeeId),
    /// Deleted, deactivated or reactivated; the name is read before the write removes it.
    Removed { name: String, action: DeleteAction },
}

impl DialogForm for PayeesDialog {
    type Command = PayeeCommand;
    type Refusal = PayeeError;

    fn command(&self) -> Result<PayeeCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(PayeeCommand::Add(form.draft())),
            Self::Edit(id, form) if form.is_valid() => Ok(PayeeCommand::Edit(*id, form.draft())),
            Self::Delete(id, form) if form.is_valid() => {
                Ok(PayeeCommand::Remove(*id, form.action()))
            }
            _ => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: PayeeError) {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.error = Some(refusal),
            Self::Delete(_, form) => form.error = Some(refusal),
        }
    }
}

impl ConfirmDialog for PayeesDialog {
    type Outcome = PayeeOutcome;

    /// A removal raises its Toast; a save selects its Payee instead.
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<PayeeOutcome>> {
        let mut apply = PayeesApply {
            ledger: scope.ledger.clone(),
        };
        let outcome = dialog_apply::confirm(self, &mut apply, cx)?;
        Some(Landed {
            toast: outcome.toast(),
            outcome,
        })
    }
}

impl PayeeOutcome {
    /// The Toast raised once the Dialog has closed: only a removal has one, since a save
    /// selects its Payee instead.
    fn toast(&self) -> Option<(ToastKind, String)> {
        match self {
            Self::Saved(_) => None,
            Self::Removed { name, action } => Some((
                ToastKind::Success,
                match action {
                    DeleteAction::Delete => lib_locale::msg::toast_payee_deleted(name),
                    DeleteAction::Deactivate => lib_locale::msg::toast_payee_deactivated(name),
                    DeleteAction::Reactivate => lib_locale::msg::toast_payee_reactivated(name),
                },
            )),
        }
    }
}

/// Runs [`PayeeCommand`]s against the Ledger as Ledger commands. A Payee the Ledger refuses to
/// delete because a Split or Bill Plan names it comes back as [`PayeeError::InUse`], the refusal
/// the forms already show as a Message.
pub struct PayeesApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<PayeeCommand> for PayeesApply {
    type Outcome = PayeeOutcome;
    type Refusal = PayeeError;

    fn apply(&mut self, command: PayeeCommand, cx: &mut App) -> Result<PayeeOutcome, PayeeError> {
        match command {
            PayeeCommand::Add(draft) => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, AddPayee(draft)))
                .map(PayeeOutcome::Saved),
            PayeeCommand::Edit(id, draft) => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, EditPayee { id, draft }))
                .map(|()| PayeeOutcome::Saved(id)),
            PayeeCommand::Remove(id, action) => {
                let name = lib_payees::get(self.ledger.read(cx).ledger().payees(), id)
                    .map(|payee: &Payee| payee.name.clone())
                    .ok_or(PayeeError::NotFound)?;
                self.ledger.update(cx, |store, cx| match action {
                    DeleteAction::Delete => {
                        store
                            .apply(cx, DeletePayee(id))
                            .map_err(|refusal| match refusal {
                                DeletePayeeRefusal::NotFound => PayeeError::NotFound,
                                DeletePayeeRefusal::InUse { .. } => PayeeError::InUse,
                            })
                    }
                    DeleteAction::Deactivate | DeleteAction::Reactivate => store
                        .apply(
                            cx,
                            SetPayeeActive {
                                id,
                                is_active: action == DeleteAction::Reactivate,
                            },
                        )
                        .map_err(|SetPayeeActiveRefusal::NotFound| PayeeError::NotFound),
                })?;
                Ok(PayeeOutcome::Removed { name, action })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{
        chrome::dialog_apply::confirm,
        settings::payees::form::{PayeeForm, PayeeOptions},
    };

    #[gpui::test]
    fn a_taken_name_stays_open_with_the_refusal_and_a_free_one_lands(cx: &mut TestAppContext) {
        let rows = lib_payees::default_payees();
        let taken = rows[0].name.clone();
        let ledger = test_ledger(
            cx,
            LedgerParts {
                payees: rows.clone(),
                ..LedgerParts::default()
            },
        );
        let mut apply = PayeesApply { ledger };
        let options = PayeeOptions::new(&[], "None".into());
        let mut dialog = PayeesDialog::Add(PayeeForm::new(&options, &rows));

        assert_eq!(
            dialog.command(),
            Err(FormError),
            "a blank name isn't a command"
        );

        let form = dialog.form_mut().expect("Add has a form");
        form.name = crate::form::field::TextField::new(taken.as_str());
        // The form's own check already refuses a taken name, so force the store's refusal.
        let refused = cx.update(|cx| apply.apply(PayeeCommand::Add(form.draft()), cx));
        assert!(matches!(refused, Err(PayeeError::DuplicateName(_))));
        dialog.refuse(refused.unwrap_err());
        assert!(dialog.form_mut().unwrap().error.is_some());

        let form = dialog.form_mut().unwrap();
        form.name = crate::form::field::TextField::new("Corner Shop");
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert!(matches!(outcome, Some(PayeeOutcome::Saved(_))));
    }
}
