//! The Remove location Dialog: a plain confirm for an empty Location, and otherwise the destructive
//! frame that quotes what goes with it, its Areas and Items and the Documents that lose a Link to
//! them, behind the typed-back name (exact, case-sensitive).
//!
//! Its confirm is lent the reassign port, which removes the Location with its Areas and Items and
//! drops the Document Links to them in the one commit of the input; a refusal stays in the Dialog.
//! It raises its own Toast on landing, and tells the page which row the cursor lands on: the
//! neighbour that takes the removed row's place. The last Location can go.

use lib_core::LocationId;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{count, location_refusal_text};
use crate::msg;
use crate::protocols::{
    InventoryChange, InventoryOutcome, InventoryReassign, InventoryRouter, InventoryRow,
    LocationUsage, SharedInventory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveLocationEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveLocationViewModel {
    pub frame: DialogFrame,
    /// What removing does: the plain question for an empty Location, else what goes with it.
    pub warning: String,
    /// `Type 12 Elm St to confirm`; `None` for a Location holding nothing, which needs no typed
    /// name.
    pub confirm: Option<TextFieldView>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct RemoveLocationPresenter {
    inventory: SharedInventory,
    id: LocationId,
    name: String,
    /// What the Location holds, read when the Dialog opens.
    areas: usize,
    items: usize,
    documents: usize,
    confirm_text: String,
    refusal: Option<String>,
}

impl RemoveLocationPresenter {
    /// The Dialog for Location `id`, its Documents read from `usage`, or `None` when it is gone.
    pub fn new(
        inventory: &SharedInventory,
        usage: &dyn LocationUsage,
        id: LocationId,
    ) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let location = rows.location(id)?;
        Some(Self {
            inventory: inventory.clone(),
            id,
            name: location.name.clone(),
            areas: location.areas.len(),
            items: rows.location_items(id),
            documents: usage.location_usage(id).documents,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: RemoveLocationEvent, router: &mut dyn InventoryRouter) {
        match event {
            RemoveLocationEvent::ConfirmText(text) => self.confirm_text = text,
            RemoveLocationEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Removes the Location through `reassign` once the Dialog allows it, raises the Toast and
    /// closes the Dialog; a refusal stays in the Dialog.
    pub fn confirm(
        &mut self,
        reassign: &mut dyn InventoryReassign,
        router: &mut dyn InventoryRouter,
    ) {
        if !self.can_confirm() {
            return;
        }
        let position = self
            .inventory
            .borrow()
            .locations()
            .iter()
            .position(|location| location.id == self.id)
            .unwrap_or(0);
        match reassign.remove_location(self.id) {
            Ok(removed) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::inventory_toast_location_removed(&removed.name),
                ));
                // The cursor lands on the neighbour that takes the removed row's place.
                let locations = self.inventory.borrow().locations();
                let selected = locations
                    .get(position.min(locations.len().saturating_sub(1)))
                    .map(|location| InventoryRow::Location(location.id));
                router.close_dialog(Some(InventoryOutcome {
                    change: InventoryChange::LocationRemoved,
                    location: self.id,
                    selected,
                }));
            }
            Err(refusal) => self.refusal = Some(location_refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> RemoveLocationViewModel {
        let name = self.name.as_str();
        let holds_something = self.holds_something();
        RemoveLocationViewModel {
            frame: DialogFrame {
                title: msg::inventory_dialog_remove_title(),
                submit: msg::inventory_dialog_remove_submit(),
                can_confirm: self.can_confirm(),
                destructive: holds_something,
            },
            warning: if holds_something {
                msg::inventory_remove_warning(
                    name,
                    &msg::inventory_count_areas(count(self.areas)),
                    &msg::inventory_count_items(count(self.items)),
                    &msg::inventory_count_documents(count(self.documents)),
                )
            } else {
                msg::inventory_remove_plain(name)
            },
            confirm: holds_something.then(|| TextFieldView {
                label: msg::inventory_remove_confirm_label(name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            }),
            refusal: self.refusal.clone(),
        }
    }

    /// Only a Location holding Areas or Items asks for its name back.
    fn holds_something(&self) -> bool {
        self.areas > 0 || self.items > 0
    }

    /// Always for an empty Location, else once the name is typed exactly.
    fn can_confirm(&self) -> bool {
        !self.holds_something() || self.confirm_text == self.name
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{
        InMemoryInventory, InMemoryReassign, InMemoryUsage, RecordingRouter, RouterCall,
    };
    use crate::protocols::{InventoryRepository as _, LocationUse};

    struct Dialog {
        presenter: RemoveLocationPresenter,
        router: RecordingRouter,
        reassign: InMemoryReassign,
        inventory: std::rc::Rc<std::cell::RefCell<InMemoryInventory>>,
    }

    impl Dialog {
        fn on(index: usize) -> Self {
            Self::with_documents(index, 0)
        }

        fn with_documents(index: usize, documents: usize) -> Self {
            let (inventory, shared) = stub();
            let id = inventory.borrow().locations()[index].id;
            let mut usage = InMemoryUsage::default();
            usage.uses.insert(id, LocationUse { documents });
            Self {
                presenter: RemoveLocationPresenter::new(&shared, &usage, id).expect("seeded"),
                router: RecordingRouter::default(),
                reassign: InMemoryReassign::new(inventory.clone()),
                inventory,
            }
        }

        fn typed(&mut self, text: &str) {
            self.presenter.handle(
                RemoveLocationEvent::ConfirmText(text.into()),
                &mut self.router,
            );
        }

        fn confirm(&mut self) -> Vec<RouterCall> {
            self.presenter.confirm(&mut self.reassign, &mut self.router);
            self.router.take()
        }

        /// Location `index` emptied of its Areas and Items.
        fn emptied(index: usize) -> Self {
            let (inventory, shared) = stub();
            {
                let mut rows = inventory.borrow_mut();
                let id = rows.locations[index].id;
                rows.locations[index].areas.clear();
                rows.items.retain(|item| item.location != id);
            }
            let id = inventory.borrow().locations()[index].id;
            Self {
                presenter: RemoveLocationPresenter::new(&shared, &InMemoryUsage::default(), id)
                    .expect("seeded"),
                router: RecordingRouter::default(),
                reassign: InMemoryReassign::new(inventory.clone()),
                inventory,
            }
        }
    }

    #[test]
    fn a_location_with_contents_quotes_them_and_needs_its_name_typed_back() {
        let mut dialog = Dialog::with_documents(0, 2);
        let view = dialog.presenter.view_model();

        assert!(view.frame.destructive);
        assert!(!view.frame.can_confirm);
        assert_eq!(
            view.warning,
            "Removing 12 Elm St contents also removes its 10 areas and 214 items. 2 documents \
             will lose a link to them. The documents and the transactions that bought the items \
             are kept."
        );
        let field = view.confirm.expect("a typed name is asked for");
        assert_eq!(field.label, "Type 12 Elm St contents to confirm");
        assert_eq!(field.placeholder, "12 Elm St contents");

        assert_eq!(dialog.confirm(), [], "confirm does nothing until typed");
        dialog.typed("12 elm st contents");
        assert!(!dialog.presenter.view_model().frame.can_confirm);
        dialog.typed("12 Elm St contents");
        assert!(dialog.presenter.view_model().frame.can_confirm);
    }

    #[test]
    fn an_empty_location_is_a_plain_neutral_confirm() {
        let dialog = Dialog::emptied(2);
        let view = dialog.presenter.view_model();

        assert!(!view.frame.destructive);
        assert!(view.frame.can_confirm);
        assert_eq!(view.warning, "Remove Hilux? It has no areas or items.");
        assert_eq!(view.confirm, None);
    }

    #[test]
    fn confirming_removes_it_raises_its_toast_and_lands_the_cursor_on_the_neighbour() {
        let mut dialog = Dialog::on(0);
        let next = dialog.inventory.borrow().locations()[1].id;
        dialog.typed("12 Elm St contents");

        let calls = dialog.confirm();

        assert_eq!(dialog.inventory.borrow().locations().len(), 2);
        let removed = dialog.presenter.id;
        assert_eq!(
            calls,
            vec![
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Removed location \"12 Elm St contents\"".into()
                )),
                RouterCall::CloseDialog(Some(InventoryOutcome {
                    change: InventoryChange::LocationRemoved,
                    location: removed,
                    selected: Some(InventoryRow::Location(next)),
                })),
            ]
        );
    }

    #[test]
    fn removing_the_last_row_lands_on_the_one_before_and_the_only_location_leaves_none() {
        let mut dialog = Dialog::emptied(2);
        let before = dialog.inventory.borrow().locations()[1].id;
        let calls = dialog.confirm();
        assert!(matches!(
            calls.last(),
            Some(RouterCall::CloseDialog(Some(InventoryOutcome {
                selected: Some(InventoryRow::Location(id)),
                ..
            }))) if *id == before
        ));

        let mut only = Dialog::emptied(2);
        only.inventory.borrow_mut().locations.drain(..2);
        let calls = only.confirm();
        assert!(matches!(
            calls.last(),
            Some(RouterCall::CloseDialog(Some(InventoryOutcome {
                selected: None,
                ..
            })))
        ));
    }

    #[test]
    fn a_refusal_stays_in_the_dialog() {
        let mut dialog = Dialog::emptied(2);
        dialog.inventory.borrow_mut().locations.pop();

        assert_eq!(dialog.confirm(), []);
        assert_eq!(
            dialog.presenter.view_model().refusal.as_deref(),
            Some("That location or area has gone or changed, so nothing was saved.")
        );
    }

    #[test]
    fn cancel_closes_without_an_outcome_and_a_gone_location_builds_no_dialog() {
        let mut dialog = Dialog::on(0);
        dialog
            .presenter
            .handle(RemoveLocationEvent::Cancel, &mut dialog.router);
        assert_eq!(dialog.router.take(), [RouterCall::CloseDialog(None)]);

        let (_, shared) = stub();
        assert!(
            RemoveLocationPresenter::new(
                &shared,
                &InMemoryUsage::default(),
                LocationId::seed(9_999)
            )
            .is_none()
        );
    }
}
