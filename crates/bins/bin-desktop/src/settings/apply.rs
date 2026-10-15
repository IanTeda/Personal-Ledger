//! The confirm cycle (ADR-0037) of the Settings Dialogs no page owns yet: the command each form
//! describes and the [`Apply`] that runs it through [`LedgerStore::apply`] as a `lib_ledger`
//! command.

use gpui::{App, Entity};

use lib_ledger::AddInstitution;

use super::SettingsDialog;
use crate::{
    chrome::{
        dialog_apply::{Apply, DialogForm, FormError},
        dialog_host::Dialog,
    },
    ledger_store::LedgerStore,
};

/// What confirming a Settings Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCommand {
    AddInstitution {
        name: String,
        account_type: String,
    },
    /// Clear logs: the capture lives in the Settings View, which `Shell` empties.
    ClearLogs,
}

/// What a landed [`SettingsCommand`] did, for the Toast that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    Saved,
    LogsConfirmed,
}

/// Why a Settings command didn't land: none of them can be refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRefusal {}

impl DialogForm for SettingsDialog {
    type Command = SettingsCommand;
    type Refusal = SettingsRefusal;

    fn command(&self) -> Result<SettingsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            Self::AddInstitution(form) => SettingsCommand::AddInstitution {
                name: form.name.text().to_string(),
                account_type: form
                    .account_types
                    .iter()
                    .map(|account_type| account_type.label())
                    .collect::<Vec<_>>()
                    .join(" \u{b7} "),
            },
            Self::ClearLogs => SettingsCommand::ClearLogs,
        })
    }

    fn refuse(&mut self, refusal: SettingsRefusal) {
        match refusal {}
    }
}

/// Runs [`SettingsCommand`]s as `lib_ledger` commands.
pub struct SettingsApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<SettingsCommand> for SettingsApply {
    type Outcome = SettingsOutcome;
    type Refusal = SettingsRefusal;

    fn apply(
        &mut self,
        command: SettingsCommand,
        cx: &mut App,
    ) -> Result<SettingsOutcome, SettingsRefusal> {
        match command {
            SettingsCommand::AddInstitution { name, account_type } => {
                let Ok(()) = self.ledger.update(cx, |store, cx| {
                    store.apply(cx, AddInstitution { name, account_type })
                });
                Ok(SettingsOutcome::Saved)
            }
            SettingsCommand::ClearLogs => Ok(SettingsOutcome::LogsConfirmed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::chrome::dialog_apply::confirm;
    use crate::ledger_store::test_ledger;

    fn apply(cx: &mut TestAppContext) -> SettingsApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                units: lib_units::default_units(),
                ..LedgerParts::default()
            },
        );
        SettingsApply { ledger }
    }

    #[gpui::test]
    fn adding_an_institution_lands_in_the_ledger(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let landed = cx.update(|cx| {
            apply.apply(
                SettingsCommand::AddInstitution {
                    name: "Bank X".into(),
                    account_type: "savings".into(),
                },
                cx,
            )
        });
        assert_eq!(landed, Ok(SettingsOutcome::Saved));
        cx.read(|cx| {
            let institutions = apply.ledger.read(cx).ledger().institutions();
            assert_eq!(institutions.last().expect("the added row").name, "Bank X");
        });
    }

    #[gpui::test]
    fn clear_logs_only_confirms(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = SettingsDialog::ClearLogs;
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(SettingsOutcome::LogsConfirmed));
    }
}
