//! The Payees Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use gpui::App;
use lib_core::PayeeId;
use lib_ledger::{
    AddPayee, DeletePayee, DeletePayeeRefusal, EditPayee, Ledger, LedgerCommand, SetPayeeActive,
    SetPayeeActiveRefusal,
};
use lib_payees::{PayeeError, get};
use lib_toast::ToastKind;

use super::{dialogs::PayeesDialog, form::DeleteAction};
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, DialogScope, DialogToast, Landed},
};

/// What confirming a Payees Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeCommand {
    Add(AddPayee),
    Edit(EditPayee),
    /// Hard-delete a Payee nothing references.
    Delete(DeletePayee),
    /// Deactivate or reactivate a Payee.
    SetActive(SetPayeeActive),
}

/// What a landed [`PayeeCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeeOutcome {
    /// Added or edited: the Payee to select.
    Saved(PayeeId),
    /// Deleted, deactivated or reactivated; the name is read before the write removes it.
    Removed { name: String, action: DeleteAction },
}

/// A removal raises its Toast; a save selects its Payee instead.
impl OutcomeToast for PayeeOutcome {
    fn toast(&self) -> Option<DialogToast> {
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

/// A Payee the Ledger refuses to delete because a Split or Bill Plan names it comes back as
/// [`PayeeError::InUse`], the refusal the forms already show as a Message.
impl LedgerCommand for PayeeCommand {
    type Outcome = PayeeOutcome;
    type Refusal = PayeeError;

    fn run(self, ledger: &mut Ledger) -> Result<PayeeOutcome, PayeeError> {
        match self {
            Self::Add(add) => add.run(ledger).map(PayeeOutcome::Saved),
            Self::Edit(edit) => {
                let id = edit.id;
                edit.run(ledger).map(|()| PayeeOutcome::Saved(id))
            }
            Self::Delete(delete) => {
                let name = payee_name(ledger, delete.0)?;
                delete.run(ledger).map_err(|refusal| match refusal {
                    DeletePayeeRefusal::NotFound => PayeeError::NotFound,
                    DeletePayeeRefusal::InUse { .. } => PayeeError::InUse,
                })?;
                Ok(PayeeOutcome::Removed {
                    name,
                    action: DeleteAction::Delete,
                })
            }
            Self::SetActive(set) => {
                let name = payee_name(ledger, set.id)?;
                let action = if set.is_active {
                    DeleteAction::Reactivate
                } else {
                    DeleteAction::Deactivate
                };
                set.run(ledger)
                    .map_err(|SetPayeeActiveRefusal::NotFound| PayeeError::NotFound)?;
                Ok(PayeeOutcome::Removed { name, action })
            }
        }
    }
}

fn payee_name(ledger: &Ledger, id: PayeeId) -> Result<String, PayeeError> {
    get(ledger.payees(), id)
        .map(|payee| payee.name.clone())
        .ok_or(PayeeError::NotFound)
}

impl DialogForm for PayeesDialog {
    type Command = PayeeCommand;
    type Refusal = PayeeError;

    fn command(&self) -> Result<PayeeCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(PayeeCommand::Add(AddPayee(form.draft()))),
            Self::Edit(id, form) if form.is_valid() => Ok(PayeeCommand::Edit(EditPayee {
                id: *id,
                draft: form.draft(),
            })),
            Self::Delete(id, form) if form.is_valid() => Ok(match form.action() {
                DeleteAction::Delete => PayeeCommand::Delete(DeletePayee(*id)),
                action @ (DeleteAction::Deactivate | DeleteAction::Reactivate) => {
                    PayeeCommand::SetActive(SetPayeeActive {
                        id: *id,
                        is_active: action == DeleteAction::Reactivate,
                    })
                }
            }),
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

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<PayeeOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::payees::form::{DeletePayeeForm, PayeeForm, PayeeOptions},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                payees: lib_payees::default_payees(),
                ..LedgerParts::default()
            },
        )
    }

    fn payee_count(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().payees().len())
    }

    fn add_dialog(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> PayeesDialog {
        let rows = ledger.read_with(cx, |store, _| store.ledger().payees().to_vec());
        let options = PayeeOptions::new(&[], "None".into());
        PayeesDialog::Add(PayeeForm::new(&options, &rows))
    }

    #[gpui::test]
    fn an_add_without_a_name_stays_open_and_adds_nothing(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = payee_count(&ledger, cx);
        let mut dialog = add_dialog(&ledger, cx);

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert_eq!(payee_count(&ledger, cx), before);
    }

    #[gpui::test]
    fn a_taken_name_is_refused_onto_the_form_and_a_free_one_lands(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = payee_count(&ledger, cx);
        let taken = ledger.read_with(cx, |store, _| store.ledger().payees()[0].name.clone());
        let mut dialog = add_dialog(&ledger, cx);
        // The form's own check already refuses a taken name, so build the command by hand to
        // reach the Ledger's refusal.
        let draft = {
            let form = dialog.form_mut().expect("Add has a form");
            form.name = TextField::new(taken.as_str());
            form.draft()
        };
        let refused = ledger.update(cx, |store, cx| store.apply(cx, AddPayee(draft)));
        assert!(matches!(refused, Err(PayeeError::DuplicateName(_))));
        dialog.refuse(refused.unwrap_err());
        assert!(dialog.form_mut().unwrap().error.is_some());
        assert_eq!(payee_count(&ledger, cx), before);

        dialog.form_mut().unwrap().name = TextField::new("Corner Shop");
        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a free name lands");

        assert_eq!(landed.toast, None);
        assert!(matches!(landed.outcome, PayeeOutcome::Saved(_)));
        assert_eq!(payee_count(&ledger, cx), before + 1);
    }

    #[gpui::test]
    fn a_delete_of_a_vanished_payee_is_refused_and_stays_open(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = payee_count(&ledger, cx);
        let mut dialog = PayeesDialog::Delete(
            PayeeId::new(),
            DeletePayeeForm::new(&lib_payees::default_payees()[0], DeleteAction::Deactivate),
        );

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert_eq!(payee_count(&ledger, cx), before);
    }

    #[gpui::test]
    fn a_confirmed_delete_removes_the_payee_and_toasts_its_name(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let payee = ledger.read_with(cx, |store, _| store.ledger().payees()[0].clone());
        let (id, name) = (payee.id, payee.name.clone());
        let mut form = DeletePayeeForm::new(&payee, DeleteAction::Delete);
        form.confirmation_name = TextField::new(name.as_str());
        let mut dialog = PayeesDialog::Delete(id, form);

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("an unused payee deletes");

        assert_eq!(
            landed.outcome,
            PayeeOutcome::Removed {
                name: name.clone(),
                action: DeleteAction::Delete
            }
        );
        assert_eq!(
            landed.toast,
            Some((
                ToastKind::Success,
                lib_locale::msg::toast_payee_deleted(&name)
            ))
        );
        ledger.read_with(cx, |store, _| {
            assert!(store.ledger().payees().iter().all(|p| p.id != id));
        });
    }
}
