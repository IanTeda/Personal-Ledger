//! The Remove area Dialog (#487): a plain confirm while the Area is empty, else the destructive
//! frame with a "Move items to" select of the Location's other Areas, pre-selecting the Area above
//! (below when it is first). The Location's last Area holding Items gets the notice instead
//! (`AreaBlockedPresenter`), as nowhere is left to move them.
//!
//! The Area's Items are read when the Dialog opens, as it is modal. A confirm moves them, removes
//! the Area, raises its own Toast and tells the page which row the cursor lands on: the Area that
//! takes the removed one's place, else its Location.

use lib_core::{AreaId, LocationId};
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{Choice, DialogFrame, SelectView};

use super::{area_refusal_text, count};
use crate::default_destination;
use crate::interactor::RemoveArea;
use crate::msg;
use crate::protocols::{
    InventoryChange, InventoryOutcome, InventoryRouter, InventoryRow, SharedInventory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveAreaEvent {
    /// The Area that takes the Items.
    Destination(AreaId),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveAreaViewModel {
    pub frame: DialogFrame,
    /// The plain question for an empty Area, else what it holds.
    pub warning: String,
    /// The destination select; `None` for an empty Area, which has nothing to move.
    pub destination: Option<SelectView<AreaId>>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct RemoveAreaPresenter {
    inventory: SharedInventory,
    id: AreaId,
    name: String,
    location: LocationId,
    items: usize,
    /// The Location's other Areas, in tab order.
    destinations: Vec<(AreaId, String)>,
    chosen: Option<AreaId>,
    refusal: Option<String>,
}

impl RemoveAreaPresenter {
    /// The Dialog for Area `id`, or `None` when it is gone.
    pub fn new(inventory: &SharedInventory, id: AreaId) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let (location, area) = rows.area(id)?;
        let items = rows.area_items(id);
        Some(Self {
            inventory: inventory.clone(),
            id,
            name: area.name.clone(),
            location: location.id,
            items,
            destinations: location
                .areas
                .iter()
                .filter(|other| other.id != id)
                .map(|other| (other.id, other.name.clone()))
                .collect(),
            chosen: (items > 0)
                .then(|| default_destination(&rows, id))
                .flatten(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: RemoveAreaEvent, router: &mut dyn InventoryRouter) {
        match event {
            RemoveAreaEvent::Destination(id) => {
                if self.destinations.iter().any(|(area, _)| *area == id) {
                    self.chosen = Some(id);
                    self.refusal = None;
                }
            }
            RemoveAreaEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Removes the Area, moving its Items, raises the Toast and closes the Dialog; a refusal stays
    /// in the Dialog.
    pub fn confirm(&mut self, router: &mut dyn InventoryRouter) {
        if !self.can_confirm() {
            return;
        }
        let position = self
            .inventory
            .borrow()
            .location(self.location)
            .and_then(|location| location.areas.iter().position(|area| area.id == self.id))
            .unwrap_or(0);
        let destination = self
            .chosen
            .filter(|_| self.items > 0)
            .and_then(|id| self.destinations.iter().find(|(area, _)| *area == id));
        let removed = RemoveArea {
            area: self.id,
            destination: destination.map(|(id, _)| *id),
        }
        .run(&mut *self.inventory.borrow_mut());
        match removed {
            Ok(_) => {
                let toast = match destination {
                    Some((_, to)) if self.items > 0 => msg::inventory_toast_area_removed_moved(
                        &self.name,
                        &msg::inventory_count_items(count(self.items)),
                        to,
                    ),
                    _ => msg::inventory_toast_area_removed(&self.name),
                };
                router.notify(Notice::Toast(ToastKind::Success, toast));
                // The cursor lands on the Area that takes the removed row's place, else the
                // Location.
                let areas = self
                    .inventory
                    .borrow()
                    .location(self.location)
                    .map(|location| location.areas)
                    .unwrap_or_default();
                let selected = areas
                    .get(position.min(areas.len().saturating_sub(1)))
                    .map_or(InventoryRow::Location(self.location), |area| {
                        InventoryRow::Area(area.id)
                    });
                router.close_dialog(Some(InventoryOutcome {
                    change: InventoryChange::AreaRemoved,
                    location: self.location,
                    selected: Some(selected),
                }));
            }
            Err(refusal) => self.refusal = Some(area_refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> RemoveAreaViewModel {
        let holds_items = self.items > 0;
        RemoveAreaViewModel {
            frame: DialogFrame {
                title: msg::inventory_area_remove_title(),
                submit: msg::inventory_area_remove_submit(),
                can_confirm: self.can_confirm(),
                destructive: holds_items,
                hints: Vec::new(),
            },
            warning: if holds_items {
                msg::inventory_area_remove_with_items(
                    &self.name,
                    &msg::inventory_count_items(count(self.items)),
                )
            } else {
                msg::inventory_area_remove_plain(&self.name)
            },
            destination: holds_items.then(|| SelectView {
                label: msg::inventory_area_remove_destination(),
                options: self
                    .destinations
                    .iter()
                    .map(|(id, name)| Choice {
                        value: *id,
                        label: name.clone(),
                    })
                    .collect(),
                chosen: self.chosen,
                focused: true,
            }),
            refusal: self.refusal.clone(),
        }
    }

    /// An empty Area always; one with Items once a destination is chosen.
    fn can_confirm(&self) -> bool {
        self.items == 0
            || self
                .chosen
                .is_some_and(|id| self.destinations.iter().any(|(area, _)| *area == id))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{InMemoryInventory, RecordingRouter, RouterCall};
    use crate::protocols::InventoryRepository as _;

    struct Dialog {
        presenter: RemoveAreaPresenter,
        router: RecordingRouter,
        inventory: std::rc::Rc<std::cell::RefCell<InMemoryInventory>>,
    }

    impl Dialog {
        /// The Dialog on Elm St's Area `index`.
        fn on(index: usize) -> Self {
            let (inventory, shared) = stub();
            let area = inventory.borrow().locations()[0].areas[index].id;
            Self {
                presenter: RemoveAreaPresenter::new(&shared, area).expect("seeded"),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn areas(&self) -> Vec<AreaId> {
            self.inventory.borrow().locations()[0]
                .areas
                .iter()
                .map(|area| area.id)
                .collect()
        }

        fn items_in(&self, area: AreaId) -> usize {
            self.inventory.borrow().inventory().area_items(area)
        }

        fn confirm(&mut self) -> Vec<RouterCall> {
            self.presenter.confirm(&mut self.router);
            self.router.take()
        }
    }

    #[test]
    fn an_area_with_items_preselects_the_area_above_and_below_when_first() {
        let second = Dialog::on(1);
        let areas = second.areas();
        let view = second.presenter.view_model();
        let select = view.destination.expect("Items need a home");
        assert_eq!(select.chosen, Some(areas[0]));
        assert!(view.frame.destructive);
        assert_eq!(select.options.len(), areas.len() - 1);
        assert!(select.options.iter().all(|choice| choice.value != areas[1]));

        let first = Dialog::on(0);
        assert_eq!(
            first
                .presenter
                .view_model()
                .destination
                .expect("select")
                .chosen,
            Some(areas[1])
        );
    }

    #[test]
    fn confirming_moves_the_items_removes_the_area_and_lands_on_its_neighbour() {
        let mut dialog = Dialog::on(1);
        let areas = dialog.areas();
        let moving = dialog.items_in(areas[1]);
        let above = dialog.items_in(areas[0]);
        assert!(moving > 0);

        let calls = dialog.confirm();

        assert_eq!(dialog.items_in(areas[0]), above + moving, "no Item is lost");
        assert!(!dialog.areas().contains(&areas[1]));
        let location = dialog.inventory.borrow().locations()[0].id;
        assert!(matches!(
            &calls[0],
            RouterCall::Notice(Notice::Toast(ToastKind::Success, text))
                if text.starts_with("Removed area") && text.contains("moved its")
        ));
        assert_eq!(
            calls[1],
            RouterCall::CloseDialog(Some(InventoryOutcome {
                change: InventoryChange::AreaRemoved,
                location,
                selected: Some(InventoryRow::Area(areas[2])),
            }))
        );
    }

    #[test]
    fn the_destination_can_be_changed_but_only_to_another_area_of_the_location() {
        let mut dialog = Dialog::on(0);
        let areas = dialog.areas();
        let third_before = dialog.items_in(areas[2]);
        let moving = dialog.items_in(areas[0]);

        dialog
            .presenter
            .handle(RemoveAreaEvent::Destination(areas[2]), &mut dialog.router);
        dialog.presenter.handle(
            RemoveAreaEvent::Destination(AreaId::seed(9_999)),
            &mut dialog.router,
        );
        assert_eq!(
            dialog
                .presenter
                .view_model()
                .destination
                .expect("select")
                .chosen,
            Some(areas[2])
        );
        dialog.confirm();

        assert_eq!(dialog.items_in(areas[2]), third_before + moving);
    }

    #[test]
    fn an_empty_area_is_a_plain_confirm_with_no_destination() {
        let mut dialog = Dialog::on(0);
        let attic = {
            let location = dialog.inventory.borrow().locations()[0].id;
            let mut rows = dialog.inventory.borrow_mut();
            let id = AreaId::new();
            rows.locations
                .iter_mut()
                .find(|l| l.id == location)
                .expect("seeded")
                .areas
                .push(crate::Area {
                    id,
                    name: "Attic".into(),
                });
            id
        };
        let shared: SharedInventory = dialog.inventory.clone();
        dialog.presenter = RemoveAreaPresenter::new(&shared, attic).expect("added");

        let view = dialog.presenter.view_model();
        assert_eq!(view.warning, "Remove Attic? It has no items.");
        assert_eq!(view.destination, None);
        assert!(!view.frame.destructive && view.frame.can_confirm);

        let calls = dialog.confirm();
        assert_eq!(
            calls[0],
            RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Removed area \"Attic\"".into()
            ))
        );
    }

    #[test]
    fn a_vanished_area_is_refused_and_cancel_closes_without_an_outcome() {
        let mut dialog = Dialog::on(0);
        dialog.inventory.borrow_mut().locations[0].areas.clear();

        assert_eq!(dialog.confirm(), []);
        assert!(dialog.presenter.view_model().refusal.is_some());
        dialog
            .presenter
            .handle(RemoveAreaEvent::Cancel, &mut dialog.router);
        assert_eq!(dialog.router.take(), [RouterCall::CloseDialog(None)]);

        let (_, shared) = stub();
        assert!(RemoveAreaPresenter::new(&shared, AreaId::seed(9_999)).is_none());
    }
}
