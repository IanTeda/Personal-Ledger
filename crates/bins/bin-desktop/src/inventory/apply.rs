//! The Inventory Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Inventory store, and against the Ledger for the Document
//! Links a removed Property drops.

use gpui::{App, Entity};
use lib_inventory::{PropertyDraft, PropertyError, RoomError};
use lib_ledger::{AddProperty, AddRoom, EditProperty, EditRoom, RemoveProperty, RemoveRoom};

use super::{
    InventoryStore,
    form::{InventoryDialog, Problem},
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    chrome::dialog_host::Dialog,
    ledger_store::LedgerStore,
    settings::inventory::InventoryRow,
};

/// What confirming an Inventory Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryCommand {
    AddProperty {
        draft: PropertyDraft,
        unit: String,
    },
    EditProperty(u32, PropertyDraft),
    RemoveProperty(u32),
    AddRoom {
        property: u32,
        name: String,
    },
    EditRoom {
        id: u32,
        name: String,
    },
    /// Remove the Room, moving its Items to the Room named `destination`.
    RemoveRoom {
        id: u32,
        destination: Option<String>,
    },
    /// The Room-blocked notice: nothing to change, the Dialog only closes.
    Dismiss,
}

/// What a landed [`InventoryCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryOutcome {
    PropertyAdded {
        id: u32,
        name: String,
    },
    PropertySaved {
        id: u32,
        name: String,
    },
    /// Removed: the name, read before the write, and the row the cursor lands on.
    PropertyRemoved {
        id: u32,
        name: String,
        selected: Option<InventoryRow>,
    },
    RoomAdded {
        property: u32,
        id: u32,
        name: String,
    },
    RoomSaved {
        id: u32,
        name: String,
    },
    /// Removed: how many Items moved, and to which Room, read before the write.
    RoomRemoved {
        name: String,
        items: usize,
        destination: Option<String>,
        selected: InventoryRow,
    },
    Dismissed,
}

/// Why an Inventory command didn't land. The forms check the same rules live, so this only
/// happens when the Property or Room was removed while its Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRefusal {
    Property(PropertyError),
    Room(RoomError),
}

impl DialogForm for InventoryDialog {
    type Command = InventoryCommand;
    type Refusal = InventoryRefusal;

    fn command(&self) -> Result<InventoryCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        match self {
            Self::Add(form) => {
                let unit = form.unit_code().ok_or(FormError)?.to_string();
                let draft = form.draft().map_err(|_: Problem| FormError)?;
                Ok(InventoryCommand::AddProperty { draft, unit })
            }
            Self::Edit(id, form) => {
                let draft = form.draft().map_err(|_: Problem| FormError)?;
                Ok(InventoryCommand::EditProperty(*id, draft))
            }
            Self::Remove(id, _) => Ok(InventoryCommand::RemoveProperty(*id)),
            Self::AddRoom(property, form) => Ok(InventoryCommand::AddRoom {
                property: *property,
                name: form.name.text().trim().to_string(),
            }),
            Self::EditRoom(id, form) => Ok(InventoryCommand::EditRoom {
                id: *id,
                name: form.name.text().trim().to_string(),
            }),
            Self::RemoveRoom(id, form) => Ok(InventoryCommand::RemoveRoom {
                id: *id,
                destination: form.select.value().map(str::to_string),
            }),
            Self::RoomBlocked(_) => Ok(InventoryCommand::Dismiss),
        }
    }

    /// The forms check names, cover and Unit live, so a refusal means the row vanished under an
    /// open, valid form: there is nothing left for the form to correct, and `Esc` closes it.
    fn refuse(&mut self, _refusal: InventoryRefusal) {}
}

/// Runs [`InventoryCommand`]s against the Inventory store, dropping the Document Links of a
/// removed Property's Items through the Ledger.
pub struct InventoryApply {
    pub inventory: Entity<InventoryStore>,
    pub ledger: Entity<LedgerStore>,
}

impl Apply<InventoryCommand> for InventoryApply {
    type Outcome = InventoryOutcome;
    type Refusal = InventoryRefusal;

    fn apply(
        &mut self,
        command: InventoryCommand,
        cx: &mut App,
    ) -> Result<InventoryOutcome, InventoryRefusal> {
        match command {
            InventoryCommand::AddProperty { draft, unit } => {
                let name = draft.name.trim().to_string();
                let id = self
                    .ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            AddProperty {
                                draft: draft.clone(),
                                unit,
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Property)?;
                Ok(InventoryOutcome::PropertyAdded { id, name })
            }
            InventoryCommand::EditProperty(id, draft) => {
                let name = draft.name.trim().to_string();
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, EditProperty { id, draft }))
                    .map_err(InventoryRefusal::Property)?;
                Ok(InventoryOutcome::PropertySaved { id, name })
            }
            InventoryCommand::RemoveProperty(id) => {
                let inventory = self.inventory.read(cx).inventory(cx);
                let name = inventory
                    .property(id)
                    .map(|property| property.name.clone())
                    .ok_or(InventoryRefusal::Property(PropertyError::Unknown))?;
                // The cursor lands on the neighbour that takes the removed row's place.
                let position = inventory
                    .properties
                    .iter()
                    .position(|property| property.id == id)
                    .unwrap_or(0);
                // The command also drops the Document Links to the Property's Items.
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, RemoveProperty(id)))
                    .map_err(InventoryRefusal::Property)?;
                let properties = &self.inventory.read(cx).inventory(cx).properties;
                let selected = properties
                    .get(position.min(properties.len().saturating_sub(1)))
                    .map(|property| InventoryRow::Property(property.id));
                Ok(InventoryOutcome::PropertyRemoved { id, name, selected })
            }
            InventoryCommand::AddRoom { property, name } => {
                let id = self
                    .ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            AddRoom {
                                property,
                                name: name.clone(),
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Room)?;
                Ok(InventoryOutcome::RoomAdded { property, id, name })
            }
            InventoryCommand::EditRoom { id, name } => {
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            EditRoom {
                                room: id,
                                name: name.clone(),
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Room)?;
                Ok(InventoryOutcome::RoomSaved { id, name })
            }
            InventoryCommand::RemoveRoom { id, destination } => {
                let inventory = self.inventory.read(cx).inventory(cx);
                let (property, room) = inventory
                    .room(id)
                    .ok_or(InventoryRefusal::Room(RoomError::UnknownRoom))?;
                let (property_id, name) = (property.id, room.name.clone());
                let position = property
                    .rooms
                    .iter()
                    .position(|room| room.id == id)
                    .unwrap_or(0);
                let items = inventory.room_items(id);
                let destination = destination.and_then(|chosen| {
                    property
                        .rooms
                        .iter()
                        .find(|room| room.id != id && room.name == chosen)
                        .map(|room| (room.id, room.name.clone()))
                });
                let destination_id = destination.as_ref().map(|(id, _)| *id);
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            RemoveRoom {
                                room: id,
                                destination: destination_id,
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Room)?;
                // The cursor lands on the Room that takes the removed row's place, else the Property.
                let rooms = self
                    .inventory
                    .read(cx)
                    .inventory(cx)
                    .property(property_id)
                    .map(|property| property.rooms.as_slice())
                    .unwrap_or_default();
                let selected = rooms
                    .get(position.min(rooms.len().saturating_sub(1)))
                    .map_or(InventoryRow::Property(property_id), |room| {
                        InventoryRow::Room(room.id)
                    });
                Ok(InventoryOutcome::RoomRemoved {
                    name,
                    items,
                    destination: destination.map(|(_, name)| name),
                    selected,
                })
            }
            InventoryCommand::Dismiss => Ok(InventoryOutcome::Dismissed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::chrome::dialog_apply::confirm;
    use crate::ledger_store::test_ledger;

    fn apply(cx: &mut TestAppContext) -> InventoryApply {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let ledger = test_ledger(
            cx,
            LedgerParts {
                inventory: lib_inventory::default_inventory(today),
                ..LedgerParts::default()
            },
        );
        let inventory = cx.new(|cx| InventoryStore::new(ledger.clone(), cx));
        InventoryApply { inventory, ledger }
    }

    #[gpui::test]
    fn removing_a_property_selects_its_neighbour(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let first = cx.read(|cx| apply.inventory.read(cx).inventory(cx).properties[0].id);
        let outcome = cx
            .update(|cx| apply.apply(InventoryCommand::RemoveProperty(first), cx))
            .expect("the seeded Property exists");
        assert!(matches!(
            outcome,
            InventoryOutcome::PropertyRemoved { id, selected: Some(_), .. } if id == first
        ));
    }

    #[gpui::test]
    fn a_vanished_room_is_refused_and_the_notice_only_closes(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                InventoryCommand::RemoveRoom {
                    id: 9999,
                    destination: None,
                },
                cx,
            )
        });
        assert_eq!(refused, Err(InventoryRefusal::Room(RoomError::UnknownRoom)));

        let mut notice = InventoryDialog::RoomBlocked(1);
        let outcome = cx.update(|cx| confirm(&mut notice, &mut apply, cx));
        assert_eq!(outcome, Some(InventoryOutcome::Dismissed));
    }
}
