//! The Units Dialogs' confirm cycle (ADR-0037): the command each form describes and the [`Apply`]
//! that runs it through [`LedgerStore::apply`] as a `lib_ledger` command.
//!
//! The README's own "Dialog lifecycle" rows: Add "validate -> append to Units table -> close",
//! Edit "Save -> update the in-memory row -> close" (a changed code keeps the row's id, so the Accounts
//! on it stay on it), Delete "confirm -> remove + close".

use gpui::{App, Entity};
use lib_ledger::{AddUnit, DeleteUnit, EditUnit, SaveUnitRefusal};
use lib_toast::ToastKind;
use lib_units::{PriceLink, UnitId, UnitKind};

use super::dialogs::UnitsDialog;
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm, FormError},
        dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
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
        id: UnitId,
        code: String,
        name: String,
        kind: UnitKind,
        link: Option<PriceLink>,
    },
    Delete {
        id: UnitId,
    },
}

/// What a landed [`UnitsCommand`] did, for the Toast that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsOutcome {
    Saved { code: String },
    Deleted { code: String },
}

impl UnitsOutcome {
    /// The Toast raised once the Dialog has closed: only a delete has one.
    pub fn toast(&self) -> Option<(ToastKind, String)> {
        match self {
            Self::Saved { .. } => None,
            Self::Deleted { code } => Some((
                ToastKind::Success,
                lib_locale::msg::toast_unit_deleted(code),
            )),
        }
    }
}

impl DialogForm for UnitsDialog {
    type Command = UnitsCommand;
    /// The forms validate first, so a refusal only happens when the Unit row or its Price Source
    /// was removed while its Dialog was open.
    type Refusal = SaveUnitRefusal;

    fn command(&self) -> Result<UnitsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            // No Add unit field carries a Price Source or a base or default flag, so a
            // dialog-created unit lands on Manual entry, not base and not default.
            Self::Add(form) => UnitsCommand::Add {
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
            },
            // The link and the base and default flags aren't Edit unit fields either: the form
            // carries the row's link back, and the Ledger keeps the flags.
            Self::Edit(id, form) => UnitsCommand::Edit {
                id: *id,
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.kind,
                link: form.link.clone(),
            },
            Self::Delete(id, _) => UnitsCommand::Delete { id: *id },
        })
    }

    /// A refusal means the Unit row or its Price Source vanished under an open, valid form, so
    /// there is nothing left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: SaveUnitRefusal) {}
}

impl ConfirmDialog for UnitsDialog {
    type Outcome = UnitsOutcome;

    /// The Landed carries no Toast: the page decides between ACTIVITY and a Toast, since only it
    /// knows whether its detail pane is docked (`UnitsPage::land`).
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<UnitsOutcome>> {
        let mut apply = UnitsApply {
            ledger: scope.ledger.clone(),
        };
        let outcome = dialog_apply::confirm(self, &mut apply, cx)?;
        Some(Landed {
            toast: None,
            outcome,
        })
    }
}

/// Runs [`UnitsCommand`]s as `lib_ledger` commands.
pub struct UnitsApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<UnitsCommand> for UnitsApply {
    type Outcome = UnitsOutcome;
    type Refusal = SaveUnitRefusal;

    fn apply(
        &mut self,
        command: UnitsCommand,
        cx: &mut App,
    ) -> Result<UnitsOutcome, SaveUnitRefusal> {
        match command {
            UnitsCommand::Add { code, name, kind } => {
                let saved = code.clone();
                let link = self.manual_link(cx);
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            AddUnit {
                                code,
                                name,
                                kind,
                                link,
                            },
                        )
                    })
                    .map(|()| UnitsOutcome::Saved { code: saved })
            }
            UnitsCommand::Edit {
                id,
                code,
                name,
                kind,
                link,
            } => {
                let saved = code.clone();
                let link = link.unwrap_or_else(|| self.manual_link(cx));
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            EditUnit {
                                id,
                                code,
                                name,
                                kind,
                                link,
                            },
                        )
                    })
                    .map(|()| UnitsOutcome::Saved { code: saved })
            }
            UnitsCommand::Delete { id } => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, DeleteUnit { id }))
                .map(|unit| UnitsOutcome::Deleted { code: unit.code })
                .map_err(SaveUnitRefusal::from),
        }
    }
}

impl UnitsApply {
    /// A Manual entry link quoted in the base Unit. With no base Unit it falls back to the first
    /// Unit, and with no Units at all to an id naming none, which Manual entry never reads.
    fn manual_link(&self, cx: &App) -> PriceLink {
        let units = self.ledger.read(cx).ledger().units();
        let quote = lib_units::base_unit(units)
            .or(units.first())
            .map_or_else(UnitId::new, |unit| unit.id);
        PriceLink::manual(quote)
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
                units: lib_units::default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        );
        UnitsApply { ledger }
    }

    #[gpui::test]
    fn deleting_a_unit_needs_its_code_typed_and_reports_it(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let code = cx.read(|cx| apply.ledger.read(cx).ledger().units()[0].code.clone());
        let mut dialog = UnitsDialog::Delete(UnitId::SEED_AUD, DeleteUnitForm::new(code.as_str()));
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
    fn adding_and_editing_a_unit_land_in_the_ledger_and_keep_its_link(cx: &mut TestAppContext) {
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
        assert_eq!(landed, Ok(UnitsOutcome::Saved { code: "eth".into() }));
        let edited = cx.update(|cx| {
            apply.apply(
                UnitsCommand::Edit {
                    id: UnitId::SEED_AUD,
                    code: "aud".into(),
                    name: "Aussie".into(),
                    kind: UnitKind::Currency,
                    link: Some(PriceLink::manual(UnitId::SEED_AUD)),
                },
                cx,
            )
        });
        assert_eq!(edited, Ok(UnitsOutcome::Saved { code: "aud".into() }));

        cx.read(|cx| {
            let units = apply.ledger.read(cx).ledger().units();
            let added = units.last().expect("the added Unit");
            assert_eq!(added.code, "eth");
            assert_eq!(added.link, PriceLink::manual(UnitId::SEED_AUD));
            assert_eq!(units[0].name, "Aussie");
            assert_eq!(units[0].link, PriceLink::manual(UnitId::SEED_AUD));
        });
    }

    #[gpui::test]
    fn an_edit_of_a_unit_that_vanished_is_refused(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                UnitsCommand::Edit {
                    id: UnitId::new(),
                    code: "x".into(),
                    name: "x".into(),
                    kind: UnitKind::Currency,
                    link: None,
                },
                cx,
            )
        });
        assert_eq!(refused, Err(SaveUnitRefusal::UnitGone));
    }

    #[gpui::test]
    fn a_delete_of_a_unit_that_vanished_is_refused(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| apply.apply(UnitsCommand::Delete { id: UnitId::new() }, cx));
        assert_eq!(refused, Err(SaveUnitRefusal::UnitGone));
    }

    #[test]
    fn only_a_delete_raises_a_toast() {
        crate::locale::init_for_tests();
        assert_eq!(UnitsOutcome::Saved { code: "btc".into() }.toast(), None);
        assert_eq!(
            UnitsOutcome::Deleted { code: "btc".into() }.toast(),
            Some((
                ToastKind::Success,
                lib_locale::msg::toast_unit_deleted("btc")
            ))
        );
    }
}
