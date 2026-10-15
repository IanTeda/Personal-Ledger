//! The Units Dialogs' confirm cycle (ADR-0037): the command each form describes and the [`Apply`]
//! that runs it through [`LedgerStore::apply`] as a `lib_ledger` command.
//!
//! The README's own "Dialog lifecycle" rows: Add "validate -> append to Units table -> close",
//! Edit "Save -> update the in-memory row -> close" (a changed code just relabels the row here;
//! rewriting real references is out of scope), Delete "confirm -> remove + close".

use gpui::{App, Entity};
use lib_ledger::{AddUnit, DeleteUnit, EditUnit, UnitGone};
use lib_toast::ToastKind;
use lib_units::UnitKind;

use super::dialogs::UnitsDialog;
use crate::{
    chrome::{
        dialog_apply::{Apply, DialogForm, FormError},
        dialog_host::Dialog,
    },
    ledger_store::LedgerStore,
};

/// What confirming a Units Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsCommand {
    Add {
        code: String,
        name: String,
        kind: UnitKind,
    },
    Edit {
        index: usize,
        code: String,
        name: String,
        kind: UnitKind,
    },
    Delete {
        index: usize,
        code: String,
    },
}

/// What a landed [`UnitsCommand`] did, for the Toast that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsOutcome {
    Saved,
    Deleted { code: String },
}

impl UnitsOutcome {
    /// The Toast `Shell` raises once the Dialog has closed: only a delete has one.
    pub fn toast(&self) -> Option<(ToastKind, String)> {
        match self {
            Self::Saved => None,
            Self::Deleted { code } => Some((
                ToastKind::Success,
                lib_locale::msg::toast_unit_deleted(code),
            )),
        }
    }
}

impl DialogForm for UnitsDialog {
    type Command = UnitsCommand;
    /// The forms validate first, so a refusal only happens when the Unit row was removed or
    /// moved while its Dialog was open.
    type Refusal = UnitGone;

    fn command(&self) -> Result<UnitsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            // Neither Add unit field carries a source or a base or default flag: the Ledger
            // defaults a dialog-created unit to "Manual entry", not base and not default.
            Self::Add(form) => UnitsCommand::Add {
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
            },
            // Source and the base and default flags aren't Edit unit fields either, so the Ledger
            // keeps them from the row being edited.
            Self::Edit(index, form) => UnitsCommand::Edit {
                index: *index,
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
            },
            Self::Delete(index, form) => UnitsCommand::Delete {
                index: *index,
                code: form.code.clone(),
            },
        })
    }

    /// A refusal means the Unit row vanished under an open, valid form, so there is nothing left
    /// for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: UnitGone) {}
}

/// Runs [`UnitsCommand`]s as `lib_ledger` commands.
pub struct UnitsApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<UnitsCommand> for UnitsApply {
    type Outcome = UnitsOutcome;
    type Refusal = UnitGone;

    fn apply(&mut self, command: UnitsCommand, cx: &mut App) -> Result<UnitsOutcome, UnitGone> {
        match command {
            UnitsCommand::Add { code, name, kind } => {
                let Ok(()) = self.ledger.update(cx, |store, cx| {
                    store.apply(cx, AddUnit { code, name, kind })
                });
                Ok(UnitsOutcome::Saved)
            }
            UnitsCommand::Edit {
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
                .map(|()| UnitsOutcome::Saved),
            UnitsCommand::Delete { index, code } => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, DeleteUnit { index, code }))
                .map(|unit| UnitsOutcome::Deleted { code: unit.code }),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        chrome::dialog_apply::confirm, form::field::TextField, ledger_store::test_ledger,
        settings::units::form::DeleteUnitForm,
    };

    fn apply(cx: &mut TestAppContext) -> UnitsApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                units: lib_units::default_units(),
                price_sources: lib_units::default_price_sources(),
                ..LedgerParts::default()
            },
        );
        UnitsApply { ledger }
    }

    #[gpui::test]
    fn deleting_a_unit_needs_its_code_typed_and_reports_it(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let code = cx.read(|cx| apply.ledger.read(cx).ledger().units()[0].code.clone());
        let mut dialog = UnitsDialog::Delete(0, DeleteUnitForm::new(code.as_str()));
        assert_eq!(
            dialog.command(),
            Err(FormError),
            "the code isn't typed back yet"
        );

        if let UnitsDialog::Delete(_, form) = &mut dialog {
            form.confirm_input = TextField::new(code.as_str());
        }
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(UnitsOutcome::Deleted { code }));
    }

    #[gpui::test]
    fn adding_and_editing_a_unit_land_in_the_ledger_and_keep_its_source(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let landed = cx.update(|cx| {
            apply.apply(
                UnitsCommand::Add {
                    code: "eth".into(),
                    name: "Ethereum".into(),
                    kind: UnitKind::Crypto,
                },
                cx,
            )
        });
        assert_eq!(landed, Ok(UnitsOutcome::Saved));
        let edited = cx.update(|cx| {
            apply.apply(
                UnitsCommand::Edit {
                    index: 0,
                    code: "aud".into(),
                    name: "Aussie".into(),
                    kind: UnitKind::Fiat,
                },
                cx,
            )
        });
        assert_eq!(edited, Ok(UnitsOutcome::Saved));

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
                UnitsCommand::Edit {
                    index: 999,
                    code: "x".into(),
                    name: "x".into(),
                    kind: UnitKind::Fiat,
                },
                cx,
            )
        });
        assert_eq!(refused, Err(UnitGone));
    }

    #[gpui::test]
    fn a_delete_of_a_unit_that_vanished_is_refused(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                UnitsCommand::Delete {
                    index: 999,
                    code: "XXX".into(),
                },
                cx,
            )
        });
        assert_eq!(refused, Err(UnitGone));
    }

    #[test]
    fn only_a_delete_raises_a_toast() {
        crate::locale::init_for_tests();
        assert_eq!(UnitsOutcome::Saved.toast(), None);
        assert_eq!(
            UnitsOutcome::Deleted { code: "btc".into() }.toast(),
            Some((
                ToastKind::Success,
                lib_locale::msg::toast_unit_deleted("btc")
            ))
        );
    }
}
