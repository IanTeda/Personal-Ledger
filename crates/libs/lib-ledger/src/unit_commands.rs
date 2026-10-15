//! The Units writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording the row it wrote
//! (ADR-0039).
//!
//! A Unit row has no id yet, so a command addresses it by its position and a change records that
//! position; it takes an id with the UUIDv7 newtypes (ADR-0040). Price Sources have no write yet.

use lib_units::{UnitKind, UnitRow, add_unit, edit_unit, remove_unit};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Unit row a command addressed is no longer at that position (or no longer holds that code),
/// which only happens when it was removed or moved while its Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unit not found")]
pub struct UnitGone;

impl Ledger {
    pub fn add_unit(&mut self, code: String, name: String, kind: UnitKind) {
        let unit = add_unit(&mut self.units, code, name, kind);
        let index = self.units.len() - 1;
        self.pending.push(LedgerChange::UpsertUnit { index, unit });
    }

    pub fn edit_unit(
        &mut self,
        index: usize,
        code: String,
        name: String,
        kind: UnitKind,
    ) -> Result<(), UnitGone> {
        let unit = edit_unit(&mut self.units, index, code, name, kind)
            .ok_or(UnitGone)?
            .clone();
        self.pending.push(LedgerChange::UpsertUnit { index, unit });
        Ok(())
    }

    /// Removes the Unit at `index` if it still holds `code`, the code the confirm dialog opened on.
    pub fn delete_unit(&mut self, index: usize, code: &str) -> Result<UnitRow, UnitGone> {
        let unit = remove_unit(&mut self.units, index, code).ok_or(UnitGone)?;
        self.pending.push(LedgerChange::DeleteUnit { index });
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
    pub index: usize,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
}

impl LedgerCommand for EditUnit {
    type Outcome = ();
    type Refusal = UnitGone;
    fn run(self, ledger: &mut Ledger) -> Result<(), UnitGone> {
        ledger.edit_unit(self.index, self.code, self.name, self.kind)
    }
}

/// [`Ledger::delete_unit`] as a value; the outcome is the Unit removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnit {
    pub index: usize,
    pub code: String,
}

impl LedgerCommand for DeleteUnit {
    type Outcome = UnitRow;
    type Refusal = UnitGone;
    fn run(self, ledger: &mut Ledger) -> Result<UnitRow, UnitGone> {
        ledger.delete_unit(self.index, &self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LedgerChange;

    fn code_at(ledger: &Ledger, index: usize) -> String {
        ledger.units()[index].code.clone()
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
            vec![LedgerChange::UpsertUnit { index, unit: added }]
        );
    }

    #[test]
    fn editing_a_unit_keeps_its_source_and_flags_and_records_the_row() {
        let mut ledger = Ledger::stub();
        let before = ledger.units()[0].clone();
        assert!(before.is_base && before.is_default);

        EditUnit {
            index: 0,
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
            vec![LedgerChange::UpsertUnit {
                index: 0,
                unit: edited
            }]
        );
    }

    #[test]
    fn editing_a_unit_that_is_gone_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        assert_eq!(
            EditUnit {
                index: 99,
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
        let code = code_at(&ledger, 1);
        let count = ledger.units().len();

        let removed = DeleteUnit {
            index: 1,
            code: code.clone(),
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(removed.code, code);
        assert_eq!(ledger.units().len(), count - 1);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::DeleteUnit { index: 1 }]
        );
    }

    #[test]
    fn deleting_with_the_wrong_code_or_index_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.units().to_vec();

        let wrong_code = DeleteUnit {
            index: 1,
            code: code_at(&ledger, 0),
        };
        assert_eq!(wrong_code.run(&mut ledger), Err(UnitGone));
        let past_end = DeleteUnit {
            index: 99,
            code: "x".into(),
        };
        assert_eq!(past_end.run(&mut ledger), Err(UnitGone));

        assert_eq!(ledger.units(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_refused_unit_write_leaves_earlier_recorded_changes_alone() {
        let mut ledger = Ledger::stub();
        ledger.add_unit("eth".into(), "Ethereum".into(), UnitKind::Crypto);
        assert!(ledger.delete_unit(99, "x").is_err());
        assert!(
            ledger
                .edit_unit(99, "x".into(), "x".into(), UnitKind::Fiat)
                .is_err()
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn the_price_sources_are_not_touched_by_a_unit_write() {
        let mut ledger = Ledger::stub();
        let before = ledger.price_sources().to_vec();
        DeleteUnit {
            index: 1,
            code: code_at(&ledger, 1),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(ledger.price_sources(), before);
    }
}
