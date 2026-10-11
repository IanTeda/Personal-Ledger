//! The Settings Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Units and Institutions stores. When `LedgerStore` replaces
//! the per-domain stores only [`SettingsApply`] changes.
//!
//! The README's own "Dialog lifecycle" rows: Add "validate -> append to Units table -> close",
//! Edit "Save -> update the in-memory row -> close" (a changed code just relabels the row here;
//! rewriting real references is out of scope), Delete "confirm -> remove + close".

use gpui::{App, Entity};

use super::SettingsDialog;
use crate::{
    chrome::{
        dialog_apply::{Apply, DialogForm, FormError},
        dialog_host::Dialog,
    },
    institutions::InstitutionsStore,
    units::UnitsStore,
};

/// What confirming a Settings Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCommand {
    AddUnit {
        code: String,
        name: String,
        kind: String,
    },
    EditUnit {
        index: usize,
        code: String,
        name: String,
        kind: String,
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
                kind: form.kind.label().to_string(),
            },
            // Source and the base and default flags aren't Edit unit fields either, so the store
            // keeps them from the row being edited.
            Self::EditUnit(index, form) => SettingsCommand::EditUnit {
                index: *index,
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind.label().to_string(),
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

/// Runs [`SettingsCommand`]s against the Units and Institutions stores.
pub struct SettingsApply {
    pub units: Entity<UnitsStore>,
    pub institutions: Entity<InstitutionsStore>,
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
                self.units
                    .update(cx, |store, cx| store.add_unit(cx, code, name, kind));
                Ok(SettingsOutcome::Saved)
            }
            SettingsCommand::EditUnit {
                index,
                code,
                name,
                kind,
            } => self
                .units
                .update(cx, |store, cx| store.edit_unit(cx, index, code, name, kind))
                .then_some(SettingsOutcome::Saved)
                .ok_or(SettingsRefusal::UnitGone),
            SettingsCommand::DeleteUnit { index, code } => self
                .units
                .update(cx, |store, cx| store.remove_unit(cx, index, &code))
                .map(|unit| SettingsOutcome::UnitDeleted { code: unit.code })
                .ok_or(SettingsRefusal::UnitGone),
            SettingsCommand::AddInstitution { name, account_type } => {
                self.institutions
                    .update(cx, |store, cx| store.add(cx, name, account_type));
                Ok(SettingsOutcome::Saved)
            }
            SettingsCommand::ClearLogs => Ok(SettingsOutcome::LogsConfirmed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{chrome::dialog_apply::confirm, units::form::DeleteUnitForm};

    fn apply(cx: &mut TestAppContext) -> SettingsApply {
        SettingsApply {
            units: cx.new(|_| {
                UnitsStore::new(
                    crate::units::default_units(),
                    crate::units::default_price_sources(),
                )
            }),
            institutions: cx.new(|_| InstitutionsStore::new(Vec::new())),
        }
    }

    #[gpui::test]
    fn deleting_a_unit_needs_its_code_typed_and_reports_it(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let code = cx.read(|cx| {
            apply
                .units
                .read(cx)
                .unit(0)
                .expect("a seeded Unit")
                .code
                .clone()
        });
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
