use lib_core::UnitId;

use crate::protocols::{BaseUnitRepository, UnitRepository, UnitUsage};
use crate::{Error, Result, UnitRow};

/// Delete a Unit. The base Unit can only go together with a move of the base to `new_base`, so
/// the Ledger is never left without one; for any other Unit `new_base` is ignored.
///
/// A Unit in use is refused with its counts, and that refusal is the one place "in use, so
/// deactivate instead" is decided: no caller checks usage first to pick Delete or Deactivate.
/// A Unit other Units are quoted in is refused with how many, since storage keeps every quote on a
/// Unit; this reads the Units themselves, so it needs no usage port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteUnit {
    pub id: UnitId,
    pub new_base: Option<UnitId>,
}

impl DeleteUnit {
    /// Returns the Unit removed.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        base_unit: &mut dyn BaseUnitRepository,
        usage: &dyn UnitUsage,
    ) -> Result<UnitRow> {
        let outcome = self.apply(units, base_unit, usage);
        let new_base = self.new_base.map(tracing::field::display);
        match &outcome {
            Ok(_) => tracing::info!(unit_id = %self.id, new_base, "Deleted a Unit"),
            Err(refusal) => {
                tracing::info!(unit_id = %self.id, new_base, %refusal, "Refused to delete a Unit");
            }
        }
        outcome
    }

    fn apply(
        self,
        units: &mut dyn UnitRepository,
        base_unit: &mut dyn BaseUnitRepository,
        usage: &dyn UnitUsage,
    ) -> Result<UnitRow> {
        let all = units.units();
        let unit = all
            .iter()
            .find(|unit| unit.id == self.id)
            .cloned()
            .ok_or(Error::UnitGone)?;
        if all.len() == 1 {
            return Err(Error::OnlyUnit);
        }
        let in_use = usage.unit_usage(self.id);
        if in_use.is_used() {
            return Err(Error::UnitInUse(in_use));
        }
        let moves_base = if self.id == base_unit.base_unit() {
            let new_base = self.new_base.ok_or(Error::BaseUnitNeeded)?;
            let new_base = all
                .iter()
                .find(|unit| unit.id == new_base && unit.id != self.id)
                .ok_or(Error::NewBaseGone)?;
            if !new_base.is_active {
                return Err(Error::InactiveBase);
            }
            Some(new_base.id)
        } else {
            None
        };
        let quoted_by = all
            .iter()
            .filter(|other| other.id != self.id && other.link.quote == self.id)
            .count();
        if quoted_by > 0 {
            return Err(Error::QuotedBy(quoted_by));
        }
        if let Some(new_base) = moves_base {
            base_unit.set_base_unit(new_base);
        }
        units.delete_unit(self.id);
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::{InMemoryPorts, InMemoryUnits};
    use crate::presenter::test_support::unquote_aud;
    use crate::protocols::UnitUse;

    fn delete(ports: &mut InMemoryPorts, id: UnitId, new_base: Option<UnitId>) -> Result<UnitRow> {
        DeleteUnit { id, new_base }.run(&mut ports.units, &mut ports.base_unit, &ports.usage)
    }

    /// Runs a delete that should be refused and checks it wrote nothing.
    fn refused(ports: &mut InMemoryPorts, id: UnitId, new_base: Option<UnitId>) -> Error {
        let before = ports.clone();
        let refusal = delete(ports, id, new_base).expect_err("the delete should be refused");
        assert_eq!(*ports, before, "a refused delete writes nothing");
        refusal
    }

    #[test]
    fn deleting_a_unit_removes_it_and_returns_it() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let removed = delete(&mut ports, UnitId::SEED_BTC, None).unwrap();

        assert_eq!(removed.code, "btc");
        assert_eq!(ports.units.unit(UnitId::SEED_BTC), None);
        assert_eq!(ports.units.units.len(), 2);
        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn a_new_base_is_ignored_when_the_deleted_unit_is_not_the_base() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        delete(&mut ports, UnitId::SEED_BTC, Some(UnitId::SEED_VAS)).unwrap();

        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn deleting_the_base_unit_moves_the_base_to_the_new_base() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        unquote_aud(&mut ports);

        let removed = delete(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_BTC)).unwrap();

        assert_eq!(removed.id, UnitId::SEED_AUD);
        assert_eq!(ports.base_unit.0, UnitId::SEED_BTC);
        assert_eq!(ports.units.unit(UnitId::SEED_AUD), None);
    }

    #[test]
    fn deleting_a_unit_that_is_gone_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        assert_eq!(refused(&mut ports, UnitId::new(), None), Error::UnitGone);
    }

    #[test]
    fn deleting_the_only_unit_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units = InMemoryUnits {
            units: vec![ports.units.unit(UnitId::SEED_AUD).unwrap()],
        };
        for new_base in [None, Some(UnitId::SEED_AUD)] {
            assert_eq!(
                refused(&mut ports, UnitId::SEED_AUD, new_base),
                Error::OnlyUnit
            );
        }
    }

    #[test]
    fn deleting_a_unit_in_use_is_refused_with_its_counts() {
        for uses in [
            UnitUse {
                accounts: 2,
                ..UnitUse::default()
            },
            UnitUse {
                bill_plans: 1,
                ..UnitUse::default()
            },
            UnitUse {
                budgets: 3,
                ..UnitUse::default()
            },
            UnitUse {
                locations: 2,
                ..UnitUse::default()
            },
        ] {
            let mut ports = InMemoryPorts::stub(Utc::now());
            ports.usage.uses.insert(UnitId::SEED_BTC, uses);
            assert_eq!(
                refused(&mut ports, UnitId::SEED_BTC, None),
                Error::UnitInUse(uses)
            );
        }
    }

    #[test]
    fn deleting_a_unit_other_units_quote_is_refused_with_how_many() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_BTC)),
            Error::QuotedBy(2),
            "btc and vas are quoted in aud"
        );

        ports.units.units[2].link.quote = UnitId::SEED_BTC;
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_VAS)),
            Error::QuotedBy(1)
        );
        assert_eq!(
            ports.base_unit.0,
            UnitId::SEED_AUD,
            "the base doesn't move for a refused delete"
        );
    }

    #[test]
    fn a_unit_quoted_only_by_itself_can_be_deleted() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units.units[1].link.quote = UnitId::SEED_BTC;

        delete(&mut ports, UnitId::SEED_BTC, None).expect("only btc quotes btc");
    }

    #[test]
    fn deleting_the_base_unit_without_a_new_base_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, None),
            Error::BaseUnitNeeded
        );
    }

    #[test]
    fn a_new_base_that_is_the_deleted_unit_or_unknown_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        for new_base in [UnitId::SEED_AUD, UnitId::new()] {
            assert_eq!(
                refused(&mut ports, UnitId::SEED_AUD, Some(new_base)),
                Error::NewBaseGone
            );
        }
    }

    #[test]
    fn a_deactivated_new_base_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units.units[1].is_active = false;
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_BTC)),
            Error::InactiveBase
        );
    }
}
