//! The Units Dialogs' confirm cycle (ADR-0037): the command each form describes and the command
//! runs through [`confirm_ledger`](crate::chrome::dialog_apply::confirm_ledger) as a `lib_ledger` command.
//!
//! The README's own "Dialog lifecycle" rows: Add "validate -> append to Units table -> close",
//! Edit "Save -> update the in-memory row -> close" (a changed code keeps the row's id, so the Accounts
//! on it stay on it), Delete "confirm -> remove + close".

use gpui::App;
use lib_ledger::{
    AddUnit, DeactivateUnit, DeleteUnit, EditUnit, Ledger, LedgerCommand, SaveUnitRefusal,
};
use lib_toast::ToastKind;

use super::{dialogs::UnitsDialog, form::NewBase};
use crate::chrome::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
};

/// What confirming a Units Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsCommand {
    Add(AddUnit),
    Edit(EditUnit),
    Delete(DeleteUnit),
    /// What the Delete Dialog does for a Unit that is in use.
    Deactivate(DeactivateUnit),
}

/// What a landed [`UnitsCommand`] did, for the Toast that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsOutcome {
    Saved { code: String },
    Deleted { code: String },
    Deactivated { code: String },
    Reactivated { code: String },
}

impl UnitsOutcome {
    /// The Toast the page raises once the Dialog has closed and its detail pane isn't docked: a
    /// save has none.
    pub fn toast(&self) -> Option<(ToastKind, String)> {
        let text = match self {
            Self::Saved { .. } => return None,
            Self::Deleted { code } => lib_locale::msg::toast_unit_deleted(code),
            Self::Deactivated { code } => lib_locale::msg::toast_unit_deactivated(code),
            Self::Reactivated { code } => lib_locale::msg::toast_unit_reactivated(code),
        };
        Some((ToastKind::Success, text))
    }
}

/// The host raises no Toast: the page decides between ACTIVITY and a Toast, since only it knows
/// whether its detail pane is docked (`UnitsPage::land`, through [`UnitsOutcome::toast`]).
impl OutcomeToast for UnitsOutcome {}

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
            // No Add unit field carries a base or default flag, so a dialog-created unit is
            // not base and not default.
            Self::Add(form) => UnitsCommand::Add(AddUnit {
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.selected_kind(),
                link: form.link(),
            }),
            // The base and default flags aren't Edit unit fields either: the Ledger keeps them.
            Self::Edit(id, form) => UnitsCommand::Edit(EditUnit {
                id: *id,
                code: form.code.text().to_string(),
                name: form.name.text().to_string(),
                kind: form.selected_kind(),
                link: form.link(),
            }),
            Self::Delete(id, form) if form.deactivates() => {
                UnitsCommand::Deactivate(DeactivateUnit(*id))
            }
            Self::Delete(id, form) => UnitsCommand::Delete(DeleteUnit {
                id: *id,
                new_base: form.new_base.as_ref().and_then(NewBase::chosen),
            }),
        })
    }

    /// A Unit that something names turns the Delete Dialog into the refusal, offering Deactivate.
    /// Any other refusal means the Unit row or its Price Source vanished under an open, valid
    /// form, so there is nothing left for the form to correct: the Dialog stays open and `Esc`
    /// closes it.
    fn refuse(&mut self, refusal: SaveUnitRefusal) {
        if let (Self::Delete(_, form), SaveUnitRefusal::UnitInUse(uses)) = (self, refusal) {
            form.use_instead(uses);
        }
    }
}

impl ConfirmDialog for UnitsDialog {
    type Outcome = UnitsOutcome;

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<UnitsOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

impl LedgerCommand for UnitsCommand {
    type Outcome = UnitsOutcome;
    type Refusal = SaveUnitRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<UnitsOutcome, SaveUnitRefusal> {
        match self {
            Self::Add(add) => {
                let code = add.code.clone();
                add.run(ledger)?;
                Ok(UnitsOutcome::Saved { code })
            }
            Self::Edit(edit) => {
                let code = edit.code.clone();
                edit.run(ledger)?;
                Ok(UnitsOutcome::Saved { code })
            }
            Self::Delete(delete) => {
                let unit = delete.run(ledger)?;
                Ok(UnitsOutcome::Deleted { code: unit.code })
            }
            Self::Deactivate(deactivate) => {
                let code = ledger.unit_code(deactivate.0).to_string();
                deactivate.run(ledger).map_err(SaveUnitRefusal::from)?;
                Ok(UnitsOutcome::Deactivated { code })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;
    use lib_units::{PriceLink, UnitId, UnitKind};

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::units::{dialogs::UnitsDialog, form::DeleteUnitForm},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                units: lib_units::default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        )
    }

    fn unit_count(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().units().len())
    }

    #[gpui::test]
    fn an_invalid_form_is_not_confirmed(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = unit_count(&ledger, cx);
        let mut dialog = UnitsDialog::Delete(UnitId::SEED_BTC, DeleteUnitForm::new("btc"));
        assert_eq!(
            dialog.command(),
            Err(FormError),
            "the code isn't typed back yet"
        );
        assert!(
            cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
                .is_none()
        );
        assert_eq!(unit_count(&ledger, cx), before);
    }

    #[gpui::test]
    fn deleting_a_unit_needs_its_code_typed_and_reports_it(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = unit_count(&ledger, cx);
        let code = ledger.read_with(cx, |store, _| store.ledger().units()[1].code.clone());
        let mut form = DeleteUnitForm::new(code.as_str());
        form.confirm_input = TextField::new(code.as_str());
        let mut dialog = UnitsDialog::Delete(UnitId::SEED_BTC, form);

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("the typed code lands");

        assert_eq!(landed.outcome, UnitsOutcome::Deleted { code });
        assert_eq!(landed.toast, None, "the page picks ACTIVITY or a Toast");
        assert_eq!(unit_count(&ledger, cx), before - 1);
    }

    #[gpui::test]
    fn deleting_the_base_unit_moves_the_base_to_the_chosen_unit(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let candidates = vec![
            (UnitId::SEED_BTC, "btc".to_string()),
            (UnitId::SEED_VAS, "vas".to_string()),
        ];
        let mut form = DeleteUnitForm::for_base("aud", candidates);
        form.confirm_input = TextField::new("aud");
        form.new_base.as_mut().unwrap().choose(1);
        let mut dialog = UnitsDialog::Delete(UnitId::SEED_AUD, form);

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert_eq!(
            landed.map(|landed| landed.outcome),
            Some(UnitsOutcome::Deleted { code: "aud".into() })
        );
        ledger.read_with(cx, |store, _| {
            assert_eq!(store.ledger().base_unit(), UnitId::SEED_VAS);
            assert!(store.ledger().unit(UnitId::SEED_AUD).is_none());
        });
    }

    #[gpui::test]
    fn adding_a_unit_lands_in_the_ledger_with_the_link_it_carries(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let command = UnitsCommand::Add(AddUnit {
            code: "eth".into(),
            name: "Ethereum".into(),
            kind: UnitKind::Crypto,
            link: PriceLink {
                source_id: lib_units::COINGECKO,
                symbol: "ethereum".into(),
                quote: UnitId::SEED_AUD,
            },
        });
        let outcome = cx.update(|cx| ledger.update(cx, |store, cx| store.apply(cx, command)));
        assert_eq!(outcome, Ok(UnitsOutcome::Saved { code: "eth".into() }));
        ledger.read_with(cx, |store, _| {
            let added = store.ledger().units().last().expect("the added Unit");
            assert_eq!(added.code, "eth");
            assert_eq!(added.link.source_id, lib_units::COINGECKO);
            assert_eq!(added.link.symbol, "ethereum");
        });
    }

    #[gpui::test]
    fn an_edit_of_a_unit_that_vanished_is_refused(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let command = UnitsCommand::Edit(EditUnit {
            id: UnitId::new(),
            code: "x".into(),
            name: "x".into(),
            kind: UnitKind::Currency,
            link: PriceLink::manual(UnitId::SEED_AUD),
        });
        let refused = cx.update(|cx| ledger.update(cx, |store, cx| store.apply(cx, command)));
        assert_eq!(refused, Err(SaveUnitRefusal::UnitGone));
    }

    #[gpui::test]
    fn a_delete_of_a_unit_that_vanished_stays_open(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut form = DeleteUnitForm::new("x");
        form.confirm_input = TextField::new("x");
        let mut dialog = UnitsDialog::Delete(UnitId::new(), form);
        assert!(
            cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
                .is_none()
        );
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
