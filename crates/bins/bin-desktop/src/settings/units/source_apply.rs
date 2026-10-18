//! The price source Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it through [`LedgerStore::apply`] as a `lib_ledger` command (#712).

use gpui::{App, Entity};
use lib_ledger::{
    AddPriceSource, DeletePriceSource, DeletePriceSourceRefusal, EditPriceSource,
    EditPriceSourceRefusal,
};
use lib_toast::ToastKind;
use lib_units::{Endpoint, PriceSourceId, Updates};

use super::dialogs::sources::PriceSourceDialog;
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm, FormError},
        dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
    },
    ledger_store::LedgerStore,
};

/// What confirming a price source Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCommand {
    Add {
        name: String,
        endpoint: Endpoint,
        updates: Updates,
    },
    /// `endpoint` is `None` for a built-in, whose endpoint is code.
    Edit {
        id: PriceSourceId,
        name: String,
        updates: Updates,
        endpoint: Option<Endpoint>,
    },
    Delete {
        id: PriceSourceId,
    },
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
            Self::Add(form) => SourceCommand::Add {
                name: form.name(),
                endpoint: form.endpoint(),
                updates: form.updates,
            },
            Self::EditCustom(id, form) => SourceCommand::Edit {
                id: *id,
                name: form.name(),
                updates: form.updates,
                endpoint: Some(form.endpoint()),
            },
            Self::EditBuiltIn(id, form) => SourceCommand::Edit {
                id: *id,
                name: form.name(),
                updates: form.updates,
                endpoint: None,
            },
            Self::Delete(id, _) => SourceCommand::Delete { id: *id },
        })
    }

    /// Nothing is left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: SourceRefusal) {}
}

impl ConfirmDialog for PriceSourceDialog {
    type Outcome = SourceOutcome;

    /// The page decides between ACTIVITY and a Toast (`UnitsPage::land_source`).
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<SourceOutcome>> {
        let mut apply = SourceApply {
            ledger: scope.ledger.clone(),
        };
        let outcome = dialog_apply::confirm(self, &mut apply, cx)?;
        Some(Landed {
            toast: None,
            outcome,
        })
    }
}

/// Runs [`SourceCommand`]s as `lib_ledger` commands.
pub struct SourceApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<SourceCommand> for SourceApply {
    type Outcome = SourceOutcome;
    type Refusal = SourceRefusal;

    fn apply(
        &mut self,
        command: SourceCommand,
        cx: &mut App,
    ) -> Result<SourceOutcome, SourceRefusal> {
        self.ledger.update(cx, |store, cx| match command {
            SourceCommand::Add {
                name,
                endpoint,
                updates,
            } => {
                let saved = name.clone();
                match store.apply(
                    cx,
                    AddPriceSource {
                        name,
                        endpoint,
                        updates,
                    },
                ) {
                    Ok(_) => Ok(SourceOutcome::Saved { name: saved }),
                    Err(never) => match never {},
                }
            }
            SourceCommand::Edit {
                id,
                name,
                updates,
                endpoint,
            } => {
                let saved = name.clone();
                store
                    .apply(
                        cx,
                        EditPriceSource {
                            id,
                            name,
                            updates,
                            endpoint,
                        },
                    )
                    .map(|()| SourceOutcome::Saved { name: saved })
                    .map_err(SourceRefusal::Edit)
            }
            SourceCommand::Delete { id } => store
                .apply(cx, DeletePriceSource { id })
                .map(|deleted| SourceOutcome::Deleted {
                    name: deleted.source.name,
                    moved: deleted.moved,
                })
                .map_err(SourceRefusal::Delete),
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;
    use lib_units::{COINGECKO, MANUAL_ENTRY, PriceSourceAuth, UnitId};

    use super::*;
    use crate::ledger_store::test_ledger;

    const MANUAL: &str = "Manual entry";

    fn apply(cx: &mut TestAppContext) -> SourceApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                units: lib_units::default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        );
        SourceApply { ledger }
    }

    fn endpoint() -> Endpoint {
        Endpoint {
            address: "https://broker.example".to_string().into(),
            request: "/{symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::None,
        }
    }

    fn add(apply: &mut SourceApply, cx: &mut TestAppContext) -> PriceSourceId {
        let landed = cx.update(|cx| {
            apply.apply(
                SourceCommand::Add {
                    name: "Broker".into(),
                    endpoint: endpoint(),
                    updates: Updates::Daily,
                },
                cx,
            )
        });
        assert_eq!(
            landed,
            Ok(SourceOutcome::Saved {
                name: "Broker".into()
            })
        );
        cx.read(|cx| {
            apply
                .ledger
                .read(cx)
                .ledger()
                .price_sources(MANUAL)
                .iter()
                .find(|source| source.name == "Broker")
                .expect("the added source is listed")
                .id
        })
    }

    #[gpui::test]
    fn adding_editing_and_deleting_a_custom_source_land_in_the_ledger(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let id = add(&mut apply, cx);

        let edited = cx.update(|cx| {
            apply.apply(
                SourceCommand::Edit {
                    id,
                    name: "My broker".into(),
                    updates: Updates::Hourly,
                    endpoint: Some(endpoint()),
                },
                cx,
            )
        });
        assert_eq!(
            edited,
            Ok(SourceOutcome::Saved {
                name: "My broker".into()
            })
        );

        let deleted = cx.update(|cx| apply.apply(SourceCommand::Delete { id }, cx));
        assert_eq!(
            deleted,
            Ok(SourceOutcome::Deleted {
                name: "My broker".into(),
                moved: 0
            })
        );
        cx.read(|cx| {
            let ledger = apply.ledger.read(cx).ledger();
            assert!(ledger.price_source(MANUAL, id).is_none());
            assert!(ledger.price_source_records().is_empty());
        });
    }

    #[gpui::test]
    fn a_built_in_is_renamed_but_neither_re_pointed_nor_deleted(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let renamed = cx.update(|cx| {
            apply.apply(
                SourceCommand::Edit {
                    id: COINGECKO,
                    name: "Gecko".into(),
                    updates: Updates::Daily,
                    endpoint: None,
                },
                cx,
            )
        });
        assert!(renamed.is_ok());
        let deleted = cx.update(|cx| apply.apply(SourceCommand::Delete { id: COINGECKO }, cx));
        assert_eq!(
            deleted,
            Err(SourceRefusal::Delete(DeletePriceSourceRefusal::BuiltIn))
        );
        let manual = cx.update(|cx| apply.apply(SourceCommand::Delete { id: MANUAL_ENTRY }, cx));
        assert_eq!(
            manual,
            Err(SourceRefusal::Delete(DeletePriceSourceRefusal::ManualEntry))
        );
        cx.read(|cx| {
            let ledger = apply.ledger.read(cx).ledger();
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
