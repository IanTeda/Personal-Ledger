//! The Units writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording the row it wrote
//! (ADR-0039).
//!
//! A command addresses a Unit by its [`UnitId`] (ADR-0040), so editing its code leaves the Accounts
//! on it alone. Price Sources have no write yet.

use lib_core::UnitId;
use lib_units::{UnitKind, UnitRow, add_unit, edit_unit, remove_unit};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Unit a command addressed is gone, which only happens when it was removed while its Dialog
/// was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unit not found")]
pub struct UnitGone;

impl Ledger {
    pub fn add_unit(&mut self, code: String, name: String, kind: UnitKind) {
        let unit = add_unit(&mut self.units, code, name, kind);
        self.pending.push(LedgerChange::UpsertUnit(unit));
    }

    pub fn edit_unit(
        &mut self,
        id: UnitId,
        code: String,
        name: String,
        kind: UnitKind,
    ) -> Result<(), UnitGone> {
        let unit = edit_unit(&mut self.units, id, code, name, kind)
            .ok_or(UnitGone)?
            .clone();
        self.pending.push(LedgerChange::UpsertUnit(unit));
        Ok(())
    }

    pub fn delete_unit(&mut self, id: UnitId) -> Result<UnitRow, UnitGone> {
        let unit = remove_unit(&mut self.units, id).ok_or(UnitGone)?;
        self.pending.push(LedgerChange::DeleteUnit(id));
        Ok(unit)
    }
}

/// [`Ledger::add_unit`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUnit {
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
}

impl LedgerCommand for AddUnit {
    type Outcome = ();
    /// The form has already validated the Unit, so an add can't be refused.
    type Refusal = std::convert::Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<(), Self::Refusal> {
        ledger.add_unit(self.code, self.name, self.kind);
        Ok(())
    }
}

/// [`Ledger::edit_unit`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditUnit {
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
}

impl LedgerCommand for EditUnit {
    type Outcome = ();
    type Refusal = UnitGone;
    fn run(self, ledger: &mut Ledger) -> Result<(), UnitGone> {
        ledger.edit_unit(self.id, self.code, self.name, self.kind)
    }
}

/// [`Ledger::delete_unit`] as a value; the outcome is the Unit removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnit {
    pub id: UnitId,
}

impl LedgerCommand for DeleteUnit {
    type Outcome = UnitRow;
    type Refusal = UnitGone;
    fn run(self, ledger: &mut Ledger) -> Result<UnitRow, UnitGone> {
        ledger.delete_unit(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LedgerChange;

    fn id_at(ledger: &Ledger, index: usize) -> UnitId {
        ledger.units()[index].id
    }

    #[test]
    fn adding_a_unit_appends_a_manual_unit_and_records_it() {
        let mut ledger = Ledger::stub();
        let index = ledger.units().len();

        AddUnit {
            code: "eth".into(),
            name: "Ethereum".into(),
            kind: UnitKind::Crypto,
        }
        .run(&mut ledger)
        .unwrap();

        let added = ledger.units()[index].clone();
        assert_eq!(added.code, "eth");
        assert_eq!(added.source, "Manual entry");
        assert!(!added.is_base && !added.is_default);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertUnit(added)]
        );
    }

    #[test]
    fn editing_a_unit_keeps_its_source_and_flags_and_records_the_row() {
        let mut ledger = Ledger::stub();
        let before = ledger.units()[0].clone();
        assert!(before.is_base && before.is_default);

        EditUnit {
            id: UnitId::SEED_AUD,
            code: "aud2".into(),
            name: "Aussie".into(),
            kind: UnitKind::Fiat,
        }
        .run(&mut ledger)
        .unwrap();

        let edited = ledger.units()[0].clone();
        assert_eq!(
            (edited.code.as_str(), edited.name.as_str()),
            ("aud2", "Aussie")
        );
        assert_eq!(edited.source, before.source);
        assert!(edited.is_base && edited.is_default);
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
                kind: UnitKind::Fiat
            }
            .run(&mut ledger),
            Err(UnitGone)
        );
        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn deleting_a_unit_removes_it_returns_it_and_records_the_delete() {
        let mut ledger = Ledger::stub();
        let id = id_at(&ledger, 1);
        let count = ledger.units().len();

        let removed = DeleteUnit { id }.run(&mut ledger).unwrap();

        assert_eq!(removed.id, id);
        assert_eq!(ledger.units().len(), count - 1);
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::DeleteUnit(id)]);
    }

    #[test]
    fn deleting_an_unknown_id_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        assert_eq!(
            DeleteUnit { id: UnitId::new() }.run(&mut ledger),
            Err(UnitGone)
        );

        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_refused_unit_write_leaves_earlier_recorded_changes_alone() {
        let mut ledger = Ledger::stub();
        ledger.add_unit("eth".into(), "Ethereum".into(), UnitKind::Crypto);
        assert!(ledger.delete_unit(UnitId::new()).is_err());
        assert!(
            ledger
                .edit_unit(UnitId::new(), "x".into(), "x".into(), UnitKind::Fiat)
                .is_err()
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn editing_a_units_code_keeps_its_accounts_on_that_unit() {
        let mut ledger = Ledger::stub();
        let on_btc: Vec<u32> = ledger
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
        let before = ledger.price_sources().to_vec();
        DeleteUnit {
            id: id_at(&ledger, 1),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(ledger.price_sources(), before);
    }
}
