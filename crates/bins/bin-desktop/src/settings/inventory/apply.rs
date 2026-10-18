//! The Inventory Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger, which also drops the Document Links a removed
//! Location's Items had.

use gpui::{App, Entity};
use lib_core::{AreaId, LocationId};
use lib_inventory::{AreaError, Inventory, LocationDraft, LocationError};
use lib_ledger::{AddArea, AddLocation, EditArea, EditLocation, RemoveArea, RemoveLocation};
use lib_toast::ToastKind;

use super::{InventoryRow, dialogs::InventoryDialog};
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm, FormError},
        dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
    },
    ledger_store::LedgerStore,
};

/// What confirming an Inventory Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryCommand {
    AddLocation {
        draft: LocationDraft,
        unit: String,
    },
    EditLocation(LocationId, LocationDraft),
    RemoveLocation(LocationId),
    AddArea {
        location: LocationId,
        name: String,
    },
    EditArea {
        id: AreaId,
        name: String,
    },
    /// Remove the Area, moving its Items to the Area named `destination`.
    RemoveArea {
        id: AreaId,
        destination: Option<String>,
    },
    /// The Area-blocked notice: nothing to change, the Dialog only closes.
    Dismiss,
}

/// What a landed [`InventoryCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryOutcome {
    LocationAdded {
        id: LocationId,
        name: String,
    },
    LocationSaved {
        id: LocationId,
        name: String,
    },
    /// Removed: the name, read before the write, and the row the cursor lands on.
    LocationRemoved {
        id: LocationId,
        name: String,
        selected: Option<InventoryRow>,
    },
    AreaAdded {
        location: LocationId,
        id: AreaId,
        name: String,
    },
    AreaSaved {
        id: AreaId,
        name: String,
    },
    /// Removed: how many Items moved, and to which Area, read before the write.
    AreaRemoved {
        name: String,
        items: usize,
        destination: Option<String>,
        selected: InventoryRow,
    },
    Dismissed,
}

impl InventoryOutcome {
    /// The Toast a landed command raises; the Area-blocked notice raises none.
    pub fn toast(&self) -> Option<(ToastKind, String)> {
        let message = match self {
            Self::LocationAdded { name, .. } => {
                crate::msg::desktop_inventory_toast_location_added(name)
            }
            Self::LocationSaved { name, .. } => {
                crate::msg::desktop_inventory_toast_location_saved(name)
            }
            Self::LocationRemoved { name, .. } => {
                crate::msg::desktop_inventory_toast_location_removed(name)
            }
            Self::AreaAdded { name, .. } => crate::msg::desktop_inventory_toast_area_added(name),
            Self::AreaSaved { name, .. } => crate::msg::desktop_inventory_toast_area_saved(name),
            Self::AreaRemoved {
                name,
                items,
                destination,
                ..
            } => match destination {
                Some(to) if *items > 0 => crate::msg::desktop_inventory_toast_area_removed_moved(
                    name,
                    &crate::msg::desktop_inventory_count_items(
                        i64::try_from(*items).unwrap_or(i64::MAX),
                    ),
                    to,
                ),
                _ => crate::msg::desktop_inventory_toast_area_removed(name),
            },
            Self::Dismissed => return None,
        };
        Some((ToastKind::Success, message))
    }
}

/// Why an Inventory command didn't land. The forms check the same rules live, so this only
/// happens when the Location or Area was removed while its Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRefusal {
    Location(LocationError),
    Area(AreaError),
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
                let draft = form.draft();
                Ok(InventoryCommand::AddLocation { draft, unit })
            }
            Self::Edit(id, form) => {
                let draft = form.draft();
                Ok(InventoryCommand::EditLocation(*id, draft))
            }
            Self::Remove(id, _) => Ok(InventoryCommand::RemoveLocation(*id)),
            Self::AddArea(location, form) => Ok(InventoryCommand::AddArea {
                location: *location,
                name: form.name.text().trim().to_string(),
            }),
            Self::EditArea(id, form) => Ok(InventoryCommand::EditArea {
                id: *id,
                name: form.name.text().trim().to_string(),
            }),
            Self::RemoveArea(id, form) => Ok(InventoryCommand::RemoveArea {
                id: *id,
                destination: form.select.value().map(str::to_string),
            }),
            Self::AreaBlocked(_) => Ok(InventoryCommand::Dismiss),
        }
    }

    /// The forms check names and Unit live, so a refusal means the row vanished under an
    /// open, valid form: there is nothing left for the form to correct, and `Esc` closes it.
    fn refuse(&mut self, _refusal: InventoryRefusal) {}
}

impl ConfirmDialog for InventoryDialog {
    type Outcome = InventoryOutcome;

    /// A landed confirm raises the Toast its outcome names and hands the outcome to the page; the
    /// Area-blocked notice raises none.
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<InventoryOutcome>> {
        let mut apply = InventoryApply {
            ledger: scope.ledger.clone(),
        };
        let outcome = dialog_apply::confirm(self, &mut apply, cx)?;
        Some(Landed {
            toast: outcome.toast(),
            outcome,
        })
    }
}

/// Runs [`InventoryCommand`]s against the Ledger as Ledger commands. Removing a Location drops the
/// Document Links of its Items in the same command.
pub struct InventoryApply {
    pub ledger: Entity<LedgerStore>,
}

impl InventoryApply {
    fn inventory<'a>(&self, cx: &'a App) -> &'a Inventory {
        self.ledger.read(cx).ledger().inventory()
    }
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
            InventoryCommand::AddLocation { draft, unit } => {
                let name = draft.name.trim().to_string();
                let id = self
                    .ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            AddLocation {
                                draft: draft.clone(),
                                unit,
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Location)?;
                Ok(InventoryOutcome::LocationAdded { id, name })
            }
            InventoryCommand::EditLocation(id, draft) => {
                let name = draft.name.trim().to_string();
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, EditLocation { id, draft }))
                    .map_err(InventoryRefusal::Location)?;
                Ok(InventoryOutcome::LocationSaved { id, name })
            }
            InventoryCommand::RemoveLocation(id) => {
                let inventory = self.inventory(cx);
                let name = inventory
                    .location(id)
                    .map(|location| location.name.clone())
                    .ok_or(InventoryRefusal::Location(LocationError::Unknown))?;
                // The cursor lands on the neighbour that takes the removed row's place.
                let position = inventory
                    .locations
                    .iter()
                    .position(|location| location.id == id)
                    .unwrap_or(0);
                // The command also drops the Document Links to the Location's Items.
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, RemoveLocation(id)))
                    .map_err(InventoryRefusal::Location)?;
                let locations = &self.inventory(cx).locations;
                let selected = locations
                    .get(position.min(locations.len().saturating_sub(1)))
                    .map(|location| InventoryRow::Location(location.id));
                Ok(InventoryOutcome::LocationRemoved { id, name, selected })
            }
            InventoryCommand::AddArea { location, name } => {
                let id = self
                    .ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            AddArea {
                                location,
                                name: name.clone(),
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Area)?;
                Ok(InventoryOutcome::AreaAdded { location, id, name })
            }
            InventoryCommand::EditArea { id, name } => {
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            EditArea {
                                area: id,
                                name: name.clone(),
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Area)?;
                Ok(InventoryOutcome::AreaSaved { id, name })
            }
            InventoryCommand::RemoveArea { id, destination } => {
                let inventory = self.inventory(cx);
                let (location, area) = inventory
                    .area(id)
                    .ok_or(InventoryRefusal::Area(AreaError::UnknownArea))?;
                let (location_id, name) = (location.id, area.name.clone());
                let position = location
                    .areas
                    .iter()
                    .position(|area| area.id == id)
                    .unwrap_or(0);
                let items = inventory.area_items(id);
                let destination = destination.and_then(|chosen| {
                    location
                        .areas
                        .iter()
                        .find(|area| area.id != id && area.name == chosen)
                        .map(|area| (area.id, area.name.clone()))
                });
                let destination_id = destination.as_ref().map(|(id, _)| *id);
                self.ledger
                    .update(cx, |store, cx| {
                        store.apply(
                            cx,
                            RemoveArea {
                                area: id,
                                destination: destination_id,
                            },
                        )
                    })
                    .map_err(InventoryRefusal::Area)?;
                // The cursor lands on the Area that takes the removed row's place, else the Location.
                let areas = self
                    .inventory(cx)
                    .location(location_id)
                    .map(|location| location.areas.as_slice())
                    .unwrap_or_default();
                let selected = areas
                    .get(position.min(areas.len().saturating_sub(1)))
                    .map_or(InventoryRow::Location(location_id), |area| {
                        InventoryRow::Area(area.id)
                    });
                Ok(InventoryOutcome::AreaRemoved {
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
    use gpui::TestAppContext;
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
        InventoryApply { ledger }
    }

    #[gpui::test]
    fn removing_a_location_selects_its_neighbour(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let first = cx.read(|cx| apply.inventory(cx).locations[0].id);
        let outcome = cx
            .update(|cx| apply.apply(InventoryCommand::RemoveLocation(first), cx))
            .expect("the seeded Location exists");
        assert!(matches!(
            outcome,
            InventoryOutcome::LocationRemoved { id, selected: Some(_), .. } if id == first
        ));
    }

    #[gpui::test]
    fn a_vanished_area_is_refused_and_the_notice_only_closes(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let refused = cx.update(|cx| {
            apply.apply(
                InventoryCommand::RemoveArea {
                    id: AreaId::seed(9999),
                    destination: None,
                },
                cx,
            )
        });
        assert_eq!(refused, Err(InventoryRefusal::Area(AreaError::UnknownArea)));

        let mut notice = InventoryDialog::AreaBlocked(AreaId::seed(1));
        let outcome = cx.update(|cx| confirm(&mut notice, &mut apply, cx));
        assert_eq!(outcome, Some(InventoryOutcome::Dismissed));
    }
}
