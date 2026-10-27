//! The Inventory Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`]. Removing a Location
//! also drops the Document Links its Items had, in the same Ledger command.

use gpui::App;
use lib_core::{AreaId, LocationId};
use lib_inventory::{AreaError, LocationError};
use lib_ledger::{
    AddArea, AddLocation, EditArea, EditLocation, Ledger, LedgerCommand, RemoveArea, RemoveLocation,
};
use lib_toast::ToastKind;

use super::{InventoryRow, dialogs::InventoryDialog};
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, DialogToast, Landed},
};

/// What confirming an Inventory Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryCommand {
    AddLocation(AddLocation),
    EditLocation(EditLocation),
    RemoveLocation(RemoveLocation),
    AddArea(AddArea),
    EditArea(EditArea),
    /// Remove the Area, moving its Items to the Area named `destination`: the form holds Area
    /// names, so `run` resolves the name to an id against the Location.
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

/// The Toast a landed command raises; the Area-blocked notice raises none.
impl OutcomeToast for InventoryOutcome {
    fn toast(&self) -> Option<DialogToast> {
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
                let unit = form.unit_id().ok_or(FormError)?;
                let draft = form.draft();
                Ok(InventoryCommand::AddLocation(AddLocation { draft, unit }))
            }
            Self::Edit(id, form) => {
                let draft = form.draft();
                Ok(InventoryCommand::EditLocation(EditLocation {
                    id: *id,
                    draft,
                }))
            }
            Self::Remove(id, _) => Ok(InventoryCommand::RemoveLocation(RemoveLocation(*id))),
            Self::AddArea(location, form) => Ok(InventoryCommand::AddArea(AddArea {
                location: *location,
                name: form.name.text().trim().to_string(),
            })),
            Self::EditArea(id, form) => Ok(InventoryCommand::EditArea(EditArea {
                area: *id,
                name: form.name.text().trim().to_string(),
            })),
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

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<InventoryOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

/// Names are read before the write removes them, for the Toast and the cursor that follow.
impl LedgerCommand for InventoryCommand {
    type Outcome = InventoryOutcome;
    type Refusal = InventoryRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<InventoryOutcome, InventoryRefusal> {
        match self {
            Self::AddLocation(add) => {
                let name = add.draft.name.trim().to_string();
                let id = add.run(ledger).map_err(InventoryRefusal::Location)?;
                Ok(InventoryOutcome::LocationAdded { id, name })
            }
            Self::EditLocation(edit) => {
                let (id, name) = (edit.id, edit.draft.name.trim().to_string());
                edit.run(ledger).map_err(InventoryRefusal::Location)?;
                Ok(InventoryOutcome::LocationSaved { id, name })
            }
            Self::RemoveLocation(remove) => {
                let id = remove.0;
                let inventory = ledger.inventory();
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
                remove.run(ledger).map_err(InventoryRefusal::Location)?;
                let locations = &ledger.inventory().locations;
                let selected = locations
                    .get(position.min(locations.len().saturating_sub(1)))
                    .map(|location| InventoryRow::Location(location.id));
                Ok(InventoryOutcome::LocationRemoved { id, name, selected })
            }
            Self::AddArea(add) => {
                let (location, name) = (add.location, add.name.clone());
                let id = add.run(ledger).map_err(InventoryRefusal::Area)?;
                Ok(InventoryOutcome::AreaAdded { location, id, name })
            }
            Self::EditArea(edit) => {
                let (id, name) = (edit.area, edit.name.clone());
                edit.run(ledger).map_err(InventoryRefusal::Area)?;
                Ok(InventoryOutcome::AreaSaved { id, name })
            }
            Self::RemoveArea { id, destination } => {
                let inventory = ledger.inventory();
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
                RemoveArea {
                    area: id,
                    destination: destination.as_ref().map(|(id, _)| *id),
                }
                .run(ledger)
                .map_err(InventoryRefusal::Area)?;
                // The cursor lands on the Area that takes the removed row's place, else the Location.
                let areas = ledger
                    .inventory()
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
            Self::Dismiss => Ok(InventoryOutcome::Dismissed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_inventory::Inventory;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        ledger_store::{LedgerStore, test_ledger},
        settings::inventory::form::{LocationContext, LocationForm, RemoveLocationForm},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        test_ledger(
            cx,
            LedgerParts {
                inventory: lib_inventory::default_inventory(today),
                ..LedgerParts::default()
            },
        )
    }

    fn location_ids(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> Vec<LocationId> {
        ledger.read_with(cx, |store, _| {
            store
                .ledger()
                .inventory()
                .locations
                .iter()
                .map(|l| l.id)
                .collect()
        })
    }

    #[gpui::test]
    fn removing_a_location_lands_selects_its_neighbour_and_toasts(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let first = location_ids(&ledger, cx)[0];
        let mut dialog = InventoryDialog::Remove(first, RemoveLocationForm::new("Home", false));

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("the seeded Location exists");

        assert!(matches!(
            landed.outcome,
            InventoryOutcome::LocationRemoved { id, selected: Some(_), .. } if id == first
        ));
        assert!(matches!(landed.toast, Some((ToastKind::Success, _))));
        assert!(!location_ids(&ledger, cx).contains(&first));
    }

    #[gpui::test]
    fn a_vanished_area_is_refused_and_the_notice_only_closes(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = location_ids(&ledger, cx);
        let mut vanished = InventoryDialog::RemoveArea(
            AreaId::seed(9999),
            crate::settings::inventory::form::RemoveAreaForm::new(Vec::new(), None, false),
        );
        assert!(
            cx.update(|cx| confirm_ledger(&mut vanished, &ledger, cx))
                .is_none()
        );
        assert_eq!(location_ids(&ledger, cx), before);

        let mut notice = InventoryDialog::AreaBlocked(AreaId::seed(1));
        let landed = cx
            .update(|cx| confirm_ledger(&mut notice, &ledger, cx))
            .expect("the notice closes");
        assert_eq!(landed.outcome, InventoryOutcome::Dismissed);
        assert_eq!(landed.toast, None);
    }

    #[gpui::test]
    fn an_invalid_form_is_not_confirmed(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = location_ids(&ledger, cx);
        let mut dialog = InventoryDialog::Add(LocationForm::for_add(
            None,
            LocationContext {
                inventory: Inventory::default(),
                unit_choices: Vec::new(),
            },
        ));
        assert_eq!(dialog.command(), Err(FormError));
        assert!(
            cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
                .is_none()
        );
        assert_eq!(location_ids(&ledger, cx), before);
    }
}
