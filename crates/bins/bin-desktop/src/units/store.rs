//! The shared Units store (ADR-0032).

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::{Domain, LedgerHatch};
use lib_units::{PriceSourceRow, UnitKind, UnitRow, add_unit, edit_unit, remove_unit};

use crate::ledger_data::LedgerDataEvent;
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The shared Units table and Price Sources, read through [`LedgerStore`]. Settings edits them,
/// and Accounts and the Institutions dialog read them, so a change is seen everywhere on the next
/// render.
pub struct UnitsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<LedgerDataEvent> for UnitsStore {}

impl UnitsStore {
    /// A store over `ledger`'s Units and Price Sources.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Units) {
                cx.emit(LedgerDataEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn units<'a>(&self, cx: &'a App) -> &'a [UnitRow] {
        self.ledger.read(cx).ledger().units()
    }

    pub fn unit<'a>(&self, cx: &'a App, index: usize) -> Option<&'a UnitRow> {
        self.units(cx).get(index)
    }

    pub fn price_sources<'a>(&self, cx: &'a App) -> &'a [PriceSourceRow] {
        self.ledger.read(cx).ledger().price_sources()
    }

    pub fn add_unit(&mut self, cx: &mut App, code: String, name: String, kind: UnitKind) {
        self.write(cx, |ledger| add_unit(ledger.units, code, name, kind));
    }

    /// Returns `false` when `index` is out of bounds, and emits nothing.
    pub fn edit_unit(
        &mut self,
        cx: &mut App,
        index: usize,
        code: String,
        name: String,
        kind: UnitKind,
    ) -> bool {
        self.try_write(cx, |ledger| {
            edit_unit(ledger.units, index, code, name, kind)
                .then_some(())
                .ok_or(())
        })
        .is_ok()
    }

    pub fn remove_unit(&mut self, cx: &mut App, index: usize, code: &str) -> Option<UnitRow> {
        self.try_write(cx, |ledger| {
            remove_unit(ledger.units, index, code).ok_or(())
        })
        .ok()
    }

    #[expect(deprecated, reason = "Units writes move onto Ledger commands in #647")]
    fn write<R>(&self, cx: &mut App, change: impl FnOnce(LedgerHatch<'_>) -> R) -> R {
        self.ledger
            .update(cx, |ledger, cx| ledger.write(cx, &[Domain::Units], change))
    }

    /// Writes the Units, announcing them only when `change` returns `Ok`, so a write that changed
    /// nothing emits nothing.
    #[expect(deprecated, reason = "Units writes move onto Ledger commands in #647")]
    fn try_write<T>(
        &self,
        cx: &mut App,
        change: impl FnOnce(LedgerHatch<'_>) -> Result<T, ()>,
    ) -> Result<T, ()> {
        self.ledger.update(cx, |ledger, cx| {
            ledger.try_write(cx, &[Domain::Units], change)
        })
    }
}
