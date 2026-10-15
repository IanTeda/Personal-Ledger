//! The Settings Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it through [`LedgerStore::apply`] as a `lib_ledger` command.
//!
//! The README's own "Dialog lifecycle" rows: Add "validate -> append to Units table -> close",
//! Edit "Save -> update the in-memory row -> close" (a changed code just relabels the row here;
//! rewriting real references is out of scope), Delete "confirm -> remove + close".

use gpui::{App, Entity};

use lib_ledger::{AddInstitution, AddUnit, DeleteUnit, EditUnit, UnitGone};
use lib_units::UnitKind;

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
    AddUnit {
        code: String,
        name: String,
        kind: UnitKind,
    },
    EditUnit {
        index: usize,
        code: String,
        name: String,
        kind: UnitKind,
    },
    DeleteUnit {
        index: usize,
        code: String,
    },
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
    UnitDeleted { code: String },
    LogsConfirmed,
}

/// Why a Settings command didn't land. The forms validate first, so this only happens when the
/// Unit row was removed or moved while its Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRefusal {
    UnitGone,
}

impl DialogForm for SettingsDialog {
    type Command = SettingsCommand;
    type Refusal = SettingsRefusal;

    fn command(&self) -> Result<SettingsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            // Neither Add unit field carries a source or a base or default flag: the store
            // defaults a dialog-created unit to "Manual entry", not base and not default.
            Self::AddUnit(form) => SettingsCommand::AddUnit {
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
            },
            // Source and the base and default flags aren't Edit unit fields either, so the store
            // keeps them from the row being edited.
            Self::EditUnit(index, form) => SettingsCommand::EditUnit {
                index: *index,
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
            },
            Self::DeleteUnit(index, form) => SettingsCommand::DeleteUnit {
                index: *index,
                code: form.code.clone(),
            },
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

    /// A refusal means the Unit row vanished under an open, valid form, so there is nothing left
    /// for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: SettingsRefusal) {}
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
            SettingsCommand::AddUnit { code, name, kind } => {
                let Ok(()) = self.ledger.update(cx, |store, cx| {
                    store.apply(cx, AddUnit { code, name, kind })
                });
                Ok(SettingsOutcome::Saved)
            }
            SettingsCommand::EditUnit {
                index,
                code,
                name,
                kind,
            } => self
                .ledger
                .update(cx, |store, cx| {
                    store.apply(
                        cx,
                        EditUnit {
                            index,
                            code,
                            name,
                            kind,
                        },
                    )
                })
                .map(|()| SettingsOutcome::Saved)
                .map_err(|UnitGone| SettingsRefusal::UnitGone),
            SettingsCommand::DeleteUnit { index, code } => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, DeleteUnit { index, code }))
                .map(|unit| SettingsOutcome::UnitDeleted { code: unit.code })
                .map_err(|UnitGone| SettingsRefusal::UnitGone),
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
    use crate::ledger_store::test_ledger;
    use crate::{chrome::dialog_apply::confirm, units::form::DeleteUnitForm};

    fn apply(cx: &mut TestAppContext) -> SettingsApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                units: crate::units::default_units(),
                price_sources: crate::units::default_price_sources(),
                ..LedgerParts::default()
            },
        );
        SettingsApply { ledger }
    }

    #[gpui::test]
    fn deleting_a_unit_needs_its_code_typed_and_reports_it(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let code = cx.read(|cx| apply.ledger.read(cx).ledger().units()[0].code.clone());
        let mut dialog = SettingsDialog::DeleteUnit(0, DeleteUnitForm::new(code.as_str()));
        assert_eq!(
            dialog.command(),
            Err(FormError),
            "the code isn't typed back yet"
        );

        if let SettingsDialog::DeleteUnit(_, form) = &mut dialog {
            form.confirm_input = crate::form::field::TextField::new(code.as_str());
        }
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(SettingsOutcome::UnitDeleted { code }));
    }

    #[gpui::test]
    fn adding_and_editing_a_unit_land_in_the_ledger_and_keep_its_source(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let landed = cx.update(|cx| {
            apply.apply(
                SettingsCommand::AddUnit {
                    code: "eth".into(),
                    name: "Ethereum".into(),
                    kind: UnitKind::Crypto,
                },
                cx,
            )
        });
        assert_eq!(landed, Ok(SettingsOutcome::Saved));
        let edited = cx.update(|cx| {
            apply.apply(
                SettingsCommand::EditUnit {
                    index: 0,
                    code: "aud".into(),
                    name: "Aussie".into(),
                    kind: UnitKind::Fiat,
                },
                cx,
            )
        });
        assert_eq!(edited, Ok(SettingsOutcome::Saved));

        cx.read(|cx| {
            let units = apply.ledger.read(cx).ledger().units();
            assert_eq!(units.last().expect("the added Unit").code, "eth");
            assert_eq!(units[0].name, "Aussie");
            assert_eq!(units[0].source, "USD");
        });
    }

    #[gpui::test]
    fn an_edit_of_a_unit_that_vanished_is_refused(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                SettingsCommand::EditUnit {
                    index: 999,
                    code: "x".into(),
                    name: "x".into(),
                    kind: UnitKind::Fiat,
                },
                cx,
            )
        });
        assert_eq!(refused, Err(SettingsRefusal::UnitGone));
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
    fn a_unit_row_that_vanished_is_refused_and_clear_logs_only_confirms(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                SettingsCommand::DeleteUnit {
                    index: 999,
                    code: "XXX".into(),
                },
                cx,
            )
        });
        assert_eq!(refused, Err(SettingsRefusal::UnitGone));

        let mut dialog = SettingsDialog::ClearLogs;
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(SettingsOutcome::LogsConfirmed));
    }
}
