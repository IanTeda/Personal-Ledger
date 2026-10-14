//! The shared Inventory store (ADR-0032), a read view over the Ledger's Inventory.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_inventory::Inventory;
use lib_ledger::Domain;

use crate::ledger_data::LedgerDataEvent;
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The shared Inventory: its Properties, Rooms and Items, read through [`LedgerStore`]. Settings'
/// Inventory page writes it through the Ledger's commands, and Documents reads it to resolve Links,
/// so a write that names the Inventory emits [`LedgerDataEvent::Changed`] for `Shell` to refresh
/// them.
pub struct InventoryStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<LedgerDataEvent> for InventoryStore {}

impl InventoryStore {
    /// A store over `ledger`'s Inventory.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Inventory) {
                cx.emit(LedgerDataEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn inventory<'a>(&self, cx: &'a App) -> &'a Inventory {
        self.ledger.read(cx).ledger().inventory()
    }
}
