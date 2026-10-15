//! The Institutions Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it through [`LedgerStore::apply`] as a `lib_ledger` command.

use gpui::{App, Entity};
use lib_ledger::AddInstitution;

use super::dialogs::InstitutionsDialog;
use crate::{
    chrome::{
        dialog_apply::{Apply, DialogForm, FormError},
        dialog_host::Dialog,
    },
    ledger_store::LedgerStore,
};

/// What confirming an Institutions Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsCommand {
    Add { name: String, account_type: String },
}

/// What a landed [`InstitutionsCommand`] did: nothing needs a Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsOutcome {
    Saved,
}

/// Why an Institutions command didn't land: none of them can be refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionsRefusal {}

impl DialogForm for InstitutionsDialog {
    type Command = InstitutionsCommand;
    type Refusal = InstitutionsRefusal;

    fn command(&self) -> Result<InstitutionsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        let Self::Add(form) = self;
        Ok(InstitutionsCommand::Add {
            name: form.name.text().to_string(),
            account_type: form
                .account_types
                .iter()
                .map(|account_type| account_type.label())
                .collect::<Vec<_>>()
                .join(" \u{b7} "),
        })
    }

    fn refuse(&mut self, refusal: InstitutionsRefusal) {
        match refusal {}
    }
}

/// Runs [`InstitutionsCommand`]s as `lib_ledger` commands.
pub struct InstitutionsApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<InstitutionsCommand> for InstitutionsApply {
    type Outcome = InstitutionsOutcome;
    type Refusal = InstitutionsRefusal;

    fn apply(
        &mut self,
        command: InstitutionsCommand,
        cx: &mut App,
    ) -> Result<InstitutionsOutcome, InstitutionsRefusal> {
        let InstitutionsCommand::Add { name, account_type } = command;
        let Ok(()) = self.ledger.update(cx, |store, cx| {
            store.apply(cx, AddInstitution { name, account_type })
        });
        Ok(InstitutionsOutcome::Saved)
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;

    #[gpui::test]
    fn adding_an_institution_lands_in_the_ledger(cx: &mut TestAppContext) {
        let ledger = test_ledger(cx, LedgerParts::default());
        let mut apply = InstitutionsApply { ledger };
        let landed = cx.update(|cx| {
            apply.apply(
                InstitutionsCommand::Add {
                    name: "Bank X".into(),
                    account_type: "savings".into(),
                },
                cx,
            )
        });
        assert_eq!(landed, Ok(InstitutionsOutcome::Saved));
        cx.read(|cx| {
            let institutions = apply.ledger.read(cx).ledger().institutions();
            assert_eq!(institutions.last().expect("the added row").name, "Bank X");
        });
    }
}
