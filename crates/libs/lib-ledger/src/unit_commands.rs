//! The Units writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording the row it wrote
//! (ADR-0039).
//!
//! A command addresses a Unit by its [`UnitId`] (ADR-0040), so editing its code leaves the Accounts
//! on it alone. The Price Source writes are in `price_source_commands`.

use lib_core::{PriceSourceId, UnitId};
use lib_units::{MANUAL_ENTRY, PriceLink, UnitKind, UnitRow, add_unit, edit_unit, remove_unit};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Unit a command addressed is gone, which only happens when it was removed while its Dialog
/// was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unit not found")]
pub struct UnitGone;

/// Why an add, edit or delete of a Unit was refused. The first two only happen when something was
/// removed while the Dialog was open; the last two are a delete of the base Unit that names no
/// Unit to take over as base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SaveUnitRefusal {
    /// The Unit being edited is gone. An add never refuses with this.
    #[error("unit not found")]
    UnitGone,
    /// The link names a Price Source that doesn't exist.
    #[error("price source not found")]
    PriceSourceGone,
    /// The Unit being deleted is the base Unit and no new base was given: the Ledger always has one.
    #[error("the base unit needs a new base unit before it can be deleted")]
    BaseUnitNeeded,
    /// The new base Unit is the Unit being deleted, or isn't on the Ledger.
    #[error("new base unit not found")]
    NewBaseGone,
}

impl From<UnitGone> for SaveUnitRefusal {
    fn from(UnitGone: UnitGone) -> Self {
        Self::UnitGone
    }
}

impl Ledger {
    pub fn add_unit(
        &mut self,
        code: String,
        name: String,
        kind: UnitKind,
        link: PriceLink,
    ) -> Result<(), SaveUnitRefusal> {
        self.check_price_source(link.source_id)?;
        let unit = add_unit(&mut self.units, code, name, kind, link);
        self.pending.push(LedgerChange::UpsertUnit(unit));
        Ok(())
    }

    pub fn edit_unit(
        &mut self,
        id: UnitId,
        code: String,
        name: String,
        kind: UnitKind,
        link: PriceLink,
    ) -> Result<(), SaveUnitRefusal> {
        if self.unit(id).is_none() {
            return Err(SaveUnitRefusal::UnitGone);
        }
        self.check_price_source(link.source_id)?;
        let unit = edit_unit(&mut self.units, id, code, name, kind, link)
            .ok_or(SaveUnitRefusal::UnitGone)?
            .clone();
        self.pending.push(LedgerChange::UpsertUnit(unit));
        Ok(())
    }

    /// Deletes a Unit. The base Unit can only go together with a move to `new_base`, which is
    /// checked before anything is written so the Ledger is never left without a base; for any other
    /// Unit `new_base` is ignored.
    pub fn delete_unit(
        &mut self,
        id: UnitId,
        new_base: Option<UnitId>,
    ) -> Result<UnitRow, SaveUnitRefusal> {
        if self.unit(id).is_none() {
            return Err(SaveUnitRefusal::UnitGone);
        }
        let moves_base = if id == self.base_unit {
            let new_base = new_base.ok_or(SaveUnitRefusal::BaseUnitNeeded)?;
            if new_base == id || self.unit(new_base).is_none() {
                return Err(SaveUnitRefusal::NewBaseGone);
            }
            Some(new_base)
        } else {
            None
        };
        if let Some(new_base) = moves_base {
            self.set_base_unit(new_base)?;
        }
        let unit = remove_unit(&mut self.units, id).ok_or(SaveUnitRefusal::UnitGone)?;
        self.pending.push(LedgerChange::DeleteUnit(id));
        Ok(unit)
    }

    /// Makes `id` the base Unit, which new Accounts default to. Refuses a Unit that isn't there.
    pub fn set_base_unit(&mut self, id: UnitId) -> Result<(), UnitGone> {
        self.unit(id).ok_or(UnitGone)?;
        self.base_unit = id;
        self.pending.push(LedgerChange::SetBaseUnit(id));
        Ok(())
    }

    /// Refuses a link to a Price Source that is neither Manual entry, a built-in nor a stored Custom
    /// source.
    fn check_price_source(&self, id: PriceSourceId) -> Result<(), SaveUnitRefusal> {
        let exists = id == MANUAL_ENTRY
            || lib_units::built_in(id).is_some()
            || self.price_sources.iter().any(|record| record.id() == id);
        if exists {
            Ok(())
        } else {
            Err(SaveUnitRefusal::PriceSourceGone)
        }
    }
}

/// [`Ledger::add_unit`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUnit {
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub link: PriceLink,
}

impl LedgerCommand for AddUnit {
    type Outcome = ();
    /// The form has already validated the Unit, so an add is refused only when its Price Source
    /// is gone.
    type Refusal = SaveUnitRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), Self::Refusal> {
        ledger.add_unit(self.code, self.name, self.kind, self.link)
    }
}

/// [`Ledger::edit_unit`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditUnit {
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub link: PriceLink,
}

impl LedgerCommand for EditUnit {
    type Outcome = ();
    type Refusal = SaveUnitRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), SaveUnitRefusal> {
        ledger.edit_unit(self.id, self.code, self.name, self.kind, self.link)
    }
}

/// [`Ledger::delete_unit`] as a value; the outcome is the Unit removed. `new_base` is the Unit that
/// becomes the base when `id` is the base, and is ignored otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnit {
    pub id: UnitId,
    pub new_base: Option<UnitId>,
}

impl LedgerCommand for DeleteUnit {
    type Outcome = UnitRow;
    type Refusal = SaveUnitRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<UnitRow, SaveUnitRefusal> {
        ledger.delete_unit(self.id, self.new_base)
    }
}

/// [`Ledger::set_base_unit`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetBaseUnit(pub UnitId);

impl LedgerCommand for SetBaseUnit {
    type Outcome = ();
    type Refusal = UnitGone;
    fn run(self, ledger: &mut Ledger) -> Result<(), UnitGone> {
        ledger.set_base_unit(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LedgerChange;
    use lib_core::AccountId;

    fn id_at(ledger: &Ledger, index: usize) -> UnitId {
        ledger.units()[index].id
    }

    fn manual() -> PriceLink {
        PriceLink::manual(UnitId::SEED_AUD)
    }

    #[test]
    fn adding_a_unit_appends_it_with_its_link_and_records_it() {
        let mut ledger = Ledger::stub();
        let index = ledger.units().len();
        let link = PriceLink {
            source_id: lib_units::COINGECKO,
            symbol: "ethereum".into(),
            quote: UnitId::SEED_AUD,
        };

        AddUnit {
            code: "eth".into(),
            name: "Ethereum".into(),
            kind: UnitKind::Crypto,
            link: link.clone(),
        }
        .run(&mut ledger)
        .unwrap();

        let added = ledger.units()[index].clone();
        assert_eq!(added.code, "eth");
        assert_eq!(added.link, link);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertUnit(added)]
        );
    }

    #[test]
    fn editing_a_unit_writes_its_link_and_records_the_row() {
        let mut ledger = Ledger::stub();
        let before = ledger.units()[0].clone();

        EditUnit {
            id: UnitId::SEED_AUD,
            code: "aud2".into(),
            name: "Aussie".into(),
            kind: UnitKind::Currency,
            link: manual(),
        }
        .run(&mut ledger)
        .unwrap();

        let edited = ledger.units()[0].clone();
        assert_eq!(
            (edited.code.as_str(), edited.name.as_str()),
            ("aud2", "Aussie")
        );
        assert_eq!(edited.link, before.link);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertUnit(edited)]
        );
    }

    #[test]
    fn editing_a_unit_that_is_gone_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        assert_eq!(
            EditUnit {
                id: UnitId::new(),
                code: "x".into(),
                name: "x".into(),
                kind: UnitKind::Currency,
                link: manual(),
            }
            .run(&mut ledger),
            Err(SaveUnitRefusal::UnitGone)
        );
        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn the_stub_holds_aud_as_its_base_unit() {
        let ledger = Ledger::stub();
        assert_eq!(ledger.base_unit(), UnitId::SEED_AUD);
        assert_eq!(
            ledger.base_unit_row().map(|unit| unit.code.as_str()),
            Some("aud")
        );
    }

    #[test]
    fn setting_the_base_unit_moves_it_and_records_the_change() {
        let mut ledger = Ledger::stub();

        SetBaseUnit(UnitId::SEED_BTC).run(&mut ledger).unwrap();

        assert_eq!(ledger.base_unit(), UnitId::SEED_BTC);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::SetBaseUnit(UnitId::SEED_BTC)]
        );
    }

    #[test]
    fn setting_an_unknown_unit_as_base_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();

        assert_eq!(SetBaseUnit(UnitId::new()).run(&mut ledger), Err(UnitGone));

        assert_eq!(ledger.base_unit(), UnitId::SEED_AUD);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn setting_the_base_unit_leaves_the_unit_rows_alone() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();
        SetBaseUnit(UnitId::SEED_VAS).run(&mut ledger).unwrap();
        assert_eq!(ledger.units(), before);
    }

    #[test]
    fn deleting_a_unit_removes_it_returns_it_and_records_the_delete() {
        let mut ledger = Ledger::stub();
        let id = id_at(&ledger, 1);
        let count = ledger.units().len();

        let removed = DeleteUnit { id, new_base: None }.run(&mut ledger).unwrap();

        assert_eq!(removed.id, id);
        assert_eq!(ledger.units().len(), count - 1);
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::DeleteUnit(id)]);
    }

    #[test]
    fn deleting_the_base_unit_moves_the_base_first_and_records_both() {
        let mut ledger = Ledger::stub();

        let removed = DeleteUnit {
            id: UnitId::SEED_AUD,
            new_base: Some(UnitId::SEED_BTC),
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(removed.id, UnitId::SEED_AUD);
        assert_eq!(ledger.base_unit(), UnitId::SEED_BTC);
        assert!(ledger.base_unit_row().is_some());
        assert_eq!(
            ledger.drain_changes(),
            vec![
                LedgerChange::SetBaseUnit(UnitId::SEED_BTC),
                LedgerChange::DeleteUnit(UnitId::SEED_AUD)
            ]
        );
    }

    #[test]
    fn deleting_the_base_unit_without_a_new_base_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        assert_eq!(
            DeleteUnit {
                id: UnitId::SEED_AUD,
                new_base: None
            }
            .run(&mut ledger),
            Err(SaveUnitRefusal::BaseUnitNeeded)
        );

        assert_eq!(ledger.units(), before);
        assert_eq!(ledger.base_unit(), UnitId::SEED_AUD);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_new_base_that_is_the_deleted_unit_or_unknown_is_refused() {
        let mut ledger = Ledger::stub();
        for new_base in [UnitId::SEED_AUD, UnitId::new()] {
            assert_eq!(
                DeleteUnit {
                    id: UnitId::SEED_AUD,
                    new_base: Some(new_base)
                }
                .run(&mut ledger),
                Err(SaveUnitRefusal::NewBaseGone)
            );
        }
        assert_eq!(ledger.base_unit(), UnitId::SEED_AUD);
        assert!(ledger.unit(UnitId::SEED_AUD).is_some());
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_new_base_is_ignored_when_the_deleted_unit_is_not_the_base() {
        let mut ledger = Ledger::stub();
        DeleteUnit {
            id: UnitId::SEED_BTC,
            new_base: Some(UnitId::SEED_VAS),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(ledger.base_unit(), UnitId::SEED_AUD);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::DeleteUnit(UnitId::SEED_BTC)]
        );
    }

    #[test]
    fn deleting_an_unknown_id_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        assert_eq!(
            DeleteUnit {
                id: UnitId::new(),
                new_base: None
            }
            .run(&mut ledger),
            Err(SaveUnitRefusal::UnitGone)
        );

        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_refused_unit_write_leaves_earlier_recorded_changes_alone() {
        let mut ledger = Ledger::stub();
        ledger
            .add_unit("eth".into(), "Ethereum".into(), UnitKind::Crypto, manual())
            .unwrap();
        assert!(ledger.delete_unit(UnitId::new(), None).is_err());
        assert!(
            ledger
                .edit_unit(
                    UnitId::new(),
                    "x".into(),
                    "x".into(),
                    UnitKind::Currency,
                    manual()
                )
                .is_err()
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn editing_a_units_code_keeps_its_accounts_on_that_unit() {
        let mut ledger = Ledger::stub();
        let on_btc: Vec<AccountId> = ledger
            .accounts()
            .iter()
            .filter(|account| account.unit == UnitId::SEED_BTC)
            .map(|account| account.id)
            .collect();
        assert!(!on_btc.is_empty(), "the seed has an Account in btc");

        EditUnit {
            id: UnitId::SEED_BTC,
            code: "xbt".into(),
            name: "Bitcoin".into(),
            kind: UnitKind::Crypto,
            link: ledger.units()[1].link.clone(),
        }
        .run(&mut ledger)
        .unwrap();

        for id in on_btc {
            let account = ledger.accounts().iter().find(|a| a.id == id).unwrap();
            assert_eq!(account.unit, UnitId::SEED_BTC);
            let unit = ledger.unit(account.unit).expect("the Unit still exists");
            assert_eq!(unit.code, "xbt");
        }
    }

    #[test]
    fn the_price_sources_are_not_touched_by_a_unit_write() {
        let mut ledger = Ledger::stub();
        let before = ledger.price_source_records().to_vec();
        DeleteUnit {
            id: id_at(&ledger, 1),
            new_base: None,
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(ledger.price_source_records(), before);
    }

    #[test]
    fn a_link_to_a_price_source_that_is_gone_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();
        let gone = PriceLink {
            source_id: PriceSourceId::new(),
            symbol: "x".into(),
            quote: UnitId::SEED_AUD,
        };

        assert_eq!(
            AddUnit {
                code: "eth".into(),
                name: "Ethereum".into(),
                kind: UnitKind::Crypto,
                link: gone.clone(),
            }
            .run(&mut ledger),
            Err(SaveUnitRefusal::PriceSourceGone)
        );
        assert_eq!(
            EditUnit {
                id: UnitId::SEED_BTC,
                code: "btc".into(),
                name: "Bitcoin".into(),
                kind: UnitKind::Crypto,
                link: gone,
            }
            .run(&mut ledger),
            Err(SaveUnitRefusal::PriceSourceGone)
        );
        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }
}
