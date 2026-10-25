//! The Institutions Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use gpui::App;
use lib_institutions::InstitutionError;
use lib_ledger::{AddInstitution, Ledger, LedgerCommand};

use super::dialogs::InstitutionsDialog;
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
};

/// What confirming an Institutions Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsCommand {
    Add(AddInstitution),
}

/// What a landed [`InstitutionsCommand`] did: nothing needs a Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsOutcome {
    Saved,
}

impl OutcomeToast for InstitutionsOutcome {}

impl LedgerCommand for InstitutionsCommand {
    type Outcome = InstitutionsOutcome;
    type Refusal = InstitutionError;

    fn run(self, ledger: &mut Ledger) -> Result<InstitutionsOutcome, InstitutionError> {
        let Self::Add(add) = self;
        add.run(ledger).map(|_| InstitutionsOutcome::Saved)
    }
}

impl DialogForm for InstitutionsDialog {
    type Command = InstitutionsCommand;
    type Refusal = InstitutionError;

    fn command(&self) -> Result<InstitutionsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        let Self::Add(form) = self;
        Ok(InstitutionsCommand::Add(AddInstitution {
            name: form.name.text().to_string(),
            account_type: form
                .account_types
                .iter()
                .map(|account_type| account_type.label())
                .collect::<Vec<_>>()
                .join(" \u{b7} "),
        }))
    }

    /// Shown under the name until it's edited.
    fn refuse(&mut self, refusal: InstitutionError) {
        let Self::Add(form) = self;
        form.refuse(refusal);
    }
}

impl ConfirmDialog for InstitutionsDialog {
    type Outcome = InstitutionsOutcome;

    fn confirm(
        &mut self,
        scope: &DialogScope,
        cx: &mut App,
    ) -> Option<Landed<InstitutionsOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;
    use lib_units::default_units;

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::institutions::form::{AddInstitutionForm, InstitutionProduct},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                units: default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        )
    }

    fn add_dialog(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> InstitutionsDialog {
        let units = ledger.read_with(cx, |store, _| store.ledger().units().to_vec());
        InstitutionsDialog::Add(AddInstitutionForm::new(&units))
    }

    #[gpui::test]
    fn an_add_without_a_name_stays_open_and_adds_nothing(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = add_dialog(&ledger, cx);

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        ledger.read_with(cx, |store, _| {
            assert!(store.ledger().institutions().is_empty())
        });
    }

    #[gpui::test]
    fn adding_an_institution_lands_in_the_ledger_with_no_toast(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = add_dialog(&ledger, cx);
        let InstitutionsDialog::Add(form) = &mut dialog;
        form.name = TextField::new("Bank X");
        form.toggle_account_type(InstitutionProduct::Loan);

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a named Institution lands");

        assert_eq!(landed.outcome, InstitutionsOutcome::Saved);
        assert_eq!(landed.toast, None);
        ledger.read_with(cx, |store, _| {
            let row = store.ledger().institutions().last().expect("the added row");
            assert_eq!(row.name, "Bank X");
            assert_eq!(
                row.account_type,
                format!(
                    "{} \u{b7} {}",
                    InstitutionProduct::Savings.label(),
                    InstitutionProduct::Loan.label()
                )
            );
        });
    }

    #[gpui::test]
    fn a_taken_name_stays_open_with_its_refusal_and_adds_nothing(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = add_dialog(&ledger, cx);
        let InstitutionsDialog::Add(form) = &mut dialog;
        form.name = TextField::new("Bank X");
        cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("the first Bank X lands");
        let mut again = add_dialog(&ledger, cx);
        let InstitutionsDialog::Add(form) = &mut again;
        form.name = TextField::new("bank x");

        let landed = cx.update(|cx| confirm_ledger(&mut again, &ledger, cx));

        assert!(landed.is_none());
        let InstitutionsDialog::Add(form) = &again;
        assert_eq!(
            form.name_error(),
            Some(&InstitutionError::DuplicateName("Bank X".into()))
        );
        ledger.read_with(cx, |store, _| {
            assert_eq!(store.ledger().institutions().len(), 1)
        });
    }
}
