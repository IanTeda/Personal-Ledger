//! The shared Units store (ADR-0032).

use gpui::{Context, EventEmitter};
use lib_units::{PriceSourceRow, UnitRow, UnitService};

use crate::ledger_data::LedgerDataEvent;

/// The shared Units table and Price Sources. Settings edits them, and Accounts and the
/// Institutions dialog read them, so a change is seen everywhere on the next render.
pub struct UnitsStore {
    service: UnitService,
}

impl EventEmitter<LedgerDataEvent> for UnitsStore {}

impl UnitsStore {
    /// A store over the seeded Units and Price Sources, which the Desktop takes from
    /// `lib_units::default_units` and `lib_units::default_price_sources`.
    pub fn new(units: Vec<UnitRow>, price_sources: Vec<PriceSourceRow>) -> Self {
        Self {
            service: UnitService::new(units, price_sources),
        }
    }

    pub fn units(&self) -> &[UnitRow] {
        self.service.units()
    }

    pub fn unit(&self, index: usize) -> Option<&UnitRow> {
        self.service.unit(index)
    }

    pub fn price_sources(&self) -> &[PriceSourceRow] {
        self.service.price_sources()
    }

    pub fn add_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        code: String,
        name: String,
        kind: String,
    ) {
        self.service.add_unit(code, name, kind);
        cx.emit(LedgerDataEvent::Changed);
    }

    /// Returns `false` when `index` is out of bounds, and emits nothing.
    pub fn edit_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        index: usize,
        code: String,
        name: String,
        kind: String,
    ) -> bool {
        let edited = self.service.edit_unit(index, code, name, kind);
        if edited {
            cx.emit(LedgerDataEvent::Changed);
        }
        edited
    }

    pub fn remove_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        index: usize,
        code: &str,
    ) -> Option<UnitRow> {
        let removed = self.service.remove_unit(index, code);
        if removed.is_some() {
            cx.emit(LedgerDataEvent::Changed);
        }
        removed
    }
}
