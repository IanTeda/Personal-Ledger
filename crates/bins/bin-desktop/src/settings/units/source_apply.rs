//! The price source Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it through [`LedgerStore::apply`] as a `lib_ledger` command (#712).

use gpui::App;
use lib_ledger::{
    AddPriceSource, DeletePriceSource, DeletePriceSourceRefusal, EditPriceSource,
    EditPriceSourceRefusal, Ledger, LedgerCommand,
};
use lib_toast::ToastKind;

use super::dialogs::sources::PriceSourceDialog;
use crate::chrome::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
};

/// What confirming a price source Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCommand {
    Add(AddPriceSource),
    Edit(EditPriceSource),
    Delete(DeletePriceSource),
}

/// What a landed [`SourceCommand`] did, for the Toast or the ACTIVITY line that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOutcome {
    Saved {
        name: String,
    },
    /// `moved` Units fell back to Manual entry.
    Deleted {
        name: String,
        moved: usize,
    },
}

impl SourceOutcome {
    /// The Toast raised below X-Large once the Dialog has closed.
    pub fn toast(&self) -> (ToastKind, String) {
        let text = match self {
            Self::Saved { name } => lib_locale::msg::toast_price_source_saved(name),
            Self::Deleted { name, moved } => lib_locale::msg::toast_price_source_deleted(
                name,
                i64::try_from(*moved).unwrap_or(i64::MAX),
            ),
        };
        (ToastKind::Success, text)
    }

    /// The line ACTIVITY logs at X-Large.
    pub fn activity(&self) -> String {
        match self {
            Self::Saved { name } => crate::msg::desktop_settings_units_activity_source_saved(name),
            Self::Deleted { name, .. } => {
                crate::msg::desktop_settings_units_activity_source_deleted(name)
            }
        }
    }
}

/// Why the Ledger refused a price source write. The forms validate first, so a refusal only
/// happens when the source was deleted under its open Dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRefusal {
    Edit(EditPriceSourceRefusal),
    Delete(DeletePriceSourceRefusal),
}

impl DialogForm for PriceSourceDialog {
    type Command = SourceCommand;
    type Refusal = SourceRefusal;

    fn command(&self) -> Result<SourceCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            Self::Add(form) => SourceCommand::Add(AddPriceSource {
                name: form.name(),
                endpoint: form.endpoint(),
                updates: form.updates,
            }),
            Self::EditCustom(id, form) => SourceCommand::Edit(EditPriceSource {
                id: *id,
                name: form.name(),
                updates: form.updates,
                endpoint: Some(form.endpoint()),
            }),
            Self::EditBuiltIn(id, form) => SourceCommand::Edit(EditPriceSource {
                id: *id,
                name: form.name(),
                updates: form.updates,
                endpoint: None,
            }),
            Self::Delete(id, _) => SourceCommand::Delete(DeletePriceSource { id: *id }),
        })
    }

    /// Nothing is left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: SourceRefusal) {}
}

impl ConfirmDialog for PriceSourceDialog {
    type Outcome = SourceOutcome;

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<SourceOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

/// The host raises no Toast: the page decides between ACTIVITY and a Toast
/// (`UnitsPage::land_source`, through [`SourceOutcome::toast`]).
impl OutcomeToast for SourceOutcome {}

impl LedgerCommand for SourceCommand {
    type Outcome = SourceOutcome;
    type Refusal = SourceRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<SourceOutcome, SourceRefusal> {
        match self {
            Self::Add(add) => {
                let name = add.name.clone();
                match add.run(ledger) {
                    Ok(_) => Ok(SourceOutcome::Saved { name }),
                    Err(never) => match never {},
                }
            }
            Self::Edit(edit) => {
                let name = edit.name.clone();
                edit.run(ledger).map_err(SourceRefusal::Edit)?;
                Ok(SourceOutcome::Saved { name })
            }
            Self::Delete(delete) => {
                let deleted = delete.run(ledger).map_err(SourceRefusal::Delete)?;
                Ok(SourceOutcome::Deleted {
                    name: deleted.source.name,
                    moved: deleted.moved,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;
    use lib_units::{COINGECKO, MANUAL_ENTRY, PriceSourceId, UnitId, Updates};

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::units::source_form::{
            BuiltInSourceForm, CustomSourceForm, DeleteSourceForm, SourceField,
        },
    };

    const MANUAL: &str = "Manual entry";

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                units: lib_units::default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        )
    }

    fn filled(name: &str) -> CustomSourceForm {
        let mut form = CustomSourceForm::default();
        form.name = TextField::new(name);
        form.address = TextField::new("https://broker.example");
        form.request = TextField::new("/{symbol}");
        form.price_path = TextField::new("/price");
        form.focused = SourceField::Name;
        form
    }

    fn confirm(
        dialog: &mut PriceSourceDialog,
        ledger: &Entity<LedgerStore>,
        cx: &mut TestAppContext,
    ) -> Option<Landed<SourceOutcome>> {
        cx.update(|cx| confirm_ledger(dialog, ledger, cx))
    }

    fn source_id(ledger: &Entity<LedgerStore>, name: &str, cx: &TestAppContext) -> PriceSourceId {
        ledger.read_with(cx, |store, _| {
            store
                .ledger()
                .price_sources(MANUAL)
                .iter()
                .find(|source| source.name == name)
                .expect("the source is listed")
                .id
        })
    }

    #[gpui::test]
    fn an_invalid_form_is_not_confirmed(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = PriceSourceDialog::Add(CustomSourceForm::default());
        assert_eq!(dialog.command(), Err(FormError));
        assert!(confirm(&mut dialog, &ledger, cx).is_none());
    }

    #[gpui::test]
    fn adding_editing_and_deleting_a_custom_source_land_in_the_ledger(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut add = PriceSourceDialog::Add(filled("Broker"));
        let landed = confirm(&mut add, &ledger, cx).expect("a valid source lands");
        assert_eq!(
            landed.outcome,
            SourceOutcome::Saved {
                name: "Broker".into()
            }
        );
        assert_eq!(landed.toast, None, "the page picks ACTIVITY or a Toast");
        let id = source_id(&ledger, "Broker", cx);

        let mut edit = PriceSourceDialog::EditCustom(id, filled("My broker"));
        let edited = confirm(&mut edit, &ledger, cx).expect("the edit lands");
        assert_eq!(
            edited.outcome,
            SourceOutcome::Saved {
                name: "My broker".into()
            }
        );

        let mut form = DeleteSourceForm::new("My broker");
        form.confirm_input = TextField::new("My broker");
        let mut delete = PriceSourceDialog::Delete(id, form);
        let deleted = confirm(&mut delete, &ledger, cx).expect("the delete lands");
        assert_eq!(
            deleted.outcome,
            SourceOutcome::Deleted {
                name: "My broker".into(),
                moved: 0
            }
        );
        ledger.read_with(cx, |store, _| {
            assert!(store.ledger().price_source(MANUAL, id).is_none());
        });
    }

    #[gpui::test]
    fn a_built_in_is_renamed_but_neither_re_pointed_nor_deleted(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut rename = PriceSourceDialog::EditBuiltIn(
            COINGECKO,
            BuiltInSourceForm::new("Gecko", Updates::Daily),
        );
        assert!(confirm(&mut rename, &ledger, cx).is_some());

        for (id, name) in [(COINGECKO, "Gecko"), (MANUAL_ENTRY, MANUAL)] {
            let mut form = DeleteSourceForm::new(name);
            form.confirm_input = TextField::new(name);
            let mut delete = PriceSourceDialog::Delete(id, form);
            assert!(confirm(&mut delete, &ledger, cx).is_none(), "{name} stays");
        }
        ledger.read_with(cx, |store, _| {
            let ledger = store.ledger();
            assert_eq!(
                ledger.price_source(MANUAL, COINGECKO).unwrap().name,
                "Gecko"
            );
            assert_eq!(
                ledger.unit(UnitId::SEED_BTC).unwrap().link.source_id,
                COINGECKO
            );
        });
    }

    #[test]
    fn both_outcomes_toast_and_log_by_name() {
        crate::locale::init_for_tests();
        let deleted = SourceOutcome::Deleted {
            name: "Broker".into(),
            moved: 1,
        };
        assert_eq!(deleted.toast().0, ToastKind::Success);
        assert!(deleted.toast().1.contains("Broker"));
        assert_eq!(
            SourceOutcome::Saved {
                name: "Broker".into()
            }
            .activity(),
            "Saved price source Broker"
        );
    }
}
