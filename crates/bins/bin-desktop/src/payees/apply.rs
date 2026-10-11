//! The Payees Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Payees store. When `LedgerStore` replaces the per-domain
//! stores only [`PayeesApply`] changes.

use gpui::{App, Entity};

use super::{
    Payee, PayeeDraft, PayeeError, PayeesStore,
    form::{DeleteAction, PayeesDialog},
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    transactions::TransactionsStore,
};

/// What confirming a Payees Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeCommand {
    Add(PayeeDraft),
    Edit(u32, PayeeDraft),
    Remove(u32, DeleteAction),
}

/// What a landed [`PayeeCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeOutcome {
    /// Added or edited: the Payee to select.
    Saved(u32),
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

/// Runs [`PayeeCommand`]s against the Payees store, reading the Transactions a delete is checked
/// against from theirs.
pub struct PayeesApply {
    pub payees: Entity<PayeesStore>,
    pub transactions: Entity<TransactionsStore>,
}

impl Apply<PayeeCommand> for PayeesApply {
    type Outcome = PayeeOutcome;
    type Refusal = PayeeError;

    fn apply(&mut self, command: PayeeCommand, cx: &mut App) -> Result<PayeeOutcome, PayeeError> {
        match command {
            PayeeCommand::Add(draft) => self
                .payees
                .update(cx, |store, cx| store.insert(cx, &draft))
                .map(PayeeOutcome::Saved),
            PayeeCommand::Edit(id, draft) => self
                .payees
                .update(cx, |store, cx| store.edit(cx, id, &draft))
                .map(|()| PayeeOutcome::Saved(id)),
            PayeeCommand::Remove(id, action) => {
                let name = super::get(self.payees.read(cx).payees(), id)
                    .map(|payee: &Payee| payee.name.clone())
                    .ok_or(PayeeError::NotFound)?;
                let transactions = self.transactions.read(cx).transactions().to_vec();
                self.payees
                    .update(cx, |store, cx| {
                        store.apply_delete(cx, id, action, &transactions)
                    })
                    .map(|()| PayeeOutcome::Removed { name, action })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{
        chrome::dialog_apply::confirm,
        payees::form::{PayeeForm, PayeeOptions},
    };

    #[gpui::test]
    fn a_taken_name_stays_open_with_the_refusal_and_a_free_one_lands(cx: &mut TestAppContext) {
        let rows = lib_payees::default_payees();
        let taken = rows[0].name.clone();
        let payees = cx.new(|_| PayeesStore::new(rows.clone()));
        let transactions = cx.new(|_| TransactionsStore::new(Vec::new()));
        let mut apply = PayeesApply {
            payees,
            transactions,
        };
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
