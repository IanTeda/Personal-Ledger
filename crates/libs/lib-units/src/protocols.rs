//! The ports the Units Interactors reach storage through. They are synchronous (ADR-0046): an
//! adapter serves reads from memory and queues each write, so a write never fails here; a failed
//! save is the adapter's to report. `fake` implements them in memory.

use lib_core::{PriceSourceId, UnitId};

use crate::{PriceSourceRecord, UnitRow};

/// The Units on the Ledger.
pub trait UnitRepository {
    /// Every Unit, in no particular order.
    fn units(&self) -> Vec<UnitRow>;

    fn unit(&self, id: UnitId) -> Option<UnitRow> {
        self.units().into_iter().find(|unit| unit.id == id)
    }

    /// Writes `unit`, replacing the row with its id or adding it.
    fn upsert_unit(&mut self, unit: UnitRow);

    fn delete_unit(&mut self, id: UnitId);
}

/// The stored Price Source rows: Custom sources and a person's overrides of the built-ins. Manual
/// entry and the built-in definitions are code, never stored (ADR-0043).
pub trait PriceSourceRepository {
    fn price_source_records(&self) -> Vec<PriceSourceRecord>;

    /// Writes `record`, replacing the row with its id or adding it.
    fn upsert_price_source(&mut self, record: PriceSourceRecord);

    fn delete_price_source(&mut self, id: PriceSourceId);
}

/// Which Unit is the base: what new Accounts default to and what the net worth is figured in.
pub trait BaseUnitRepository {
    fn base_unit(&self) -> UnitId;

    fn set_base_unit(&mut self, id: UnitId);
}

/// How many rows in other domains name a Unit. A read port, so `lib-units` needn't depend on the
/// Accounts, Bills or Budgets crates to refuse deleting a Unit in use.
pub trait UnitUsage {
    fn unit_usage(&self, id: UnitId) -> UnitUse;
}

/// How many rows name a Unit, carried by the delete refusal so a Dialog can say why and offer
/// Deactivate instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitUse {
    pub accounts: usize,
    pub bill_plans: usize,
    pub budgets: usize,
}

impl UnitUse {
    pub fn is_used(self) -> bool {
        self.accounts + self.bill_plans + self.budgets > 0
    }
}
