//! The shared Inventory store (ADR-0032).

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_inventory::{
    Inventory, PropertyDraft, PropertyError, RemovedProperty, RoomError, add_property, add_room,
    edit_property, edit_room, move_room, remove_property, remove_room,
};
use lib_ledger::{Domain, LedgerHatch};

use crate::ledger_data::LedgerDataEvent;
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The shared Inventory: its Properties, Rooms and Items, read through [`LedgerStore`]. Settings'
/// Inventory page edits it, and Documents reads it to resolve Links, so a write emits
/// [`LedgerDataEvent::Changed`] for `Shell` to refresh them. A refused write emits nothing and
/// changes nothing.
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

    pub fn add_property(
        &mut self,
        cx: &mut App,
        draft: &PropertyDraft,
        unit: &str,
    ) -> Result<u32, PropertyError> {
        self.try_write(cx, |ledger| add_property(ledger.inventory, draft, unit))
    }

    pub fn edit_property(
        &mut self,
        cx: &mut App,
        id: u32,
        draft: &PropertyDraft,
    ) -> Result<(), PropertyError> {
        self.try_write(cx, |ledger| edit_property(ledger.inventory, id, draft))
    }

    pub fn remove_property(
        &mut self,
        cx: &mut App,
        id: u32,
    ) -> Result<RemovedProperty, PropertyError> {
        self.try_write(cx, |ledger| remove_property(ledger.inventory, id))
    }

    pub fn add_room(&mut self, cx: &mut App, property: u32, name: &str) -> Result<u32, RoomError> {
        self.try_write(cx, |ledger| add_room(ledger.inventory, property, name))
    }

    pub fn edit_room(&mut self, cx: &mut App, room: u32, name: &str) -> Result<(), RoomError> {
        self.try_write(cx, |ledger| edit_room(ledger.inventory, room, name))
    }

    pub fn remove_room(
        &mut self,
        cx: &mut App,
        room: u32,
        destination: Option<u32>,
    ) -> Result<(), RoomError> {
        self.try_write(cx, |ledger| {
            remove_room(ledger.inventory, room, destination)
        })
    }

    /// Returns `false` when nothing moved, and emits nothing.
    pub fn move_room(&mut self, cx: &mut App, room: u32, delta: isize) -> bool {
        self.try_write(cx, |ledger| {
            move_room(ledger.inventory, room, delta)
                .then_some(())
                .ok_or(())
        })
        .is_ok()
    }

    #[expect(
        deprecated,
        reason = "Inventory writes move onto Ledger commands in #646"
    )]
    fn try_write<T, E>(
        &self,
        cx: &mut App,
        change: impl FnOnce(LedgerHatch<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.ledger.update(cx, |ledger, cx| {
            ledger.try_write(cx, &[Domain::Inventory], change)
        })
    }
}
