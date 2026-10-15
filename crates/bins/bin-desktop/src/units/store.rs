//! The shared Units store (ADR-0032).

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_units::{PriceSourceRow, UnitRow};

use crate::ledger_data::LedgerDataEvent;
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The shared Units table and Price Sources, read through [`LedgerStore`]. Settings writes them
/// through Ledger commands, and Accounts and the Institutions dialog read them, so a change is
/// seen everywhere on the next render.
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
}
