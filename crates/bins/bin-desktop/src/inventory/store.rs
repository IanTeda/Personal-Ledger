//! The shared Inventory store (ADR-0032).

use gpui::{Context, EventEmitter};
use lib_inventory::{
    Inventory, InventoryService, PropertyDraft, PropertyError, RemovedProperty, RoomError,
};

use crate::ledger_data::LedgerDataEvent;

/// The shared Inventory: its Properties, Rooms and Items. Settings' Inventory page edits it, and
/// Documents reads it to resolve Links, so a write emits [`LedgerDataEvent::Changed`] for `Shell`
/// to refresh them. A refused write emits nothing and changes nothing.
pub struct InventoryStore {
    service: InventoryService,
}

impl EventEmitter<LedgerDataEvent> for InventoryStore {}

impl InventoryStore {
    pub fn new(inventory: Inventory) -> Self {
        Self {
            service: InventoryService::new(inventory),
        }
    }

    pub fn inventory(&self) -> &Inventory {
        self.service.inventory()
    }

    pub fn add_property(
        &mut self,
        cx: &mut Context<'_, Self>,
        draft: &PropertyDraft,
        unit: &str,
    ) -> Result<u32, PropertyError> {
        let result = self.service.add_property(draft, unit);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    pub fn edit_property(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        draft: &PropertyDraft,
    ) -> Result<(), PropertyError> {
        let result = self.service.edit_property(id, draft);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    pub fn remove_property(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
    ) -> Result<RemovedProperty, PropertyError> {
        let result = self.service.remove_property(id);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    pub fn add_room(
        &mut self,
        cx: &mut Context<'_, Self>,
        property: u32,
        name: &str,
    ) -> Result<u32, RoomError> {
        let result = self.service.add_room(property, name);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    pub fn edit_room(
        &mut self,
        cx: &mut Context<'_, Self>,
        room: u32,
        name: &str,
    ) -> Result<(), RoomError> {
        let result = self.service.edit_room(room, name);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    pub fn remove_room(
        &mut self,
        cx: &mut Context<'_, Self>,
        room: u32,
        destination: Option<u32>,
    ) -> Result<(), RoomError> {
        let result = self.service.remove_room(room, destination);
        if result.is_ok() {
            cx.emit(LedgerDataEvent::Changed);
        }
        result
    }

    /// Returns `false` when nothing moved, and emits nothing.
    pub fn move_room(&mut self, cx: &mut Context<'_, Self>, room: u32, delta: isize) -> bool {
        let moved = self.service.move_room(room, delta);
        if moved {
            cx.emit(LedgerDataEvent::Changed);
        }
        moved
    }
}
