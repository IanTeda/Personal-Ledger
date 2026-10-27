//! The Add and Edit area Dialogs (#495): one form of a single Name field, Edit pre-filled from the
//! Area. An Area's name is unique within its Location only, ignoring case, so the same name in
//! another Location is fine.
//!
//! - The name is checked live against the Location's other Areas, but its problem only shows once
//!   the name has been typed in: a fresh Add Dialog opens with its confirm disabled and no error
//!   showing.
//! - A confirm raises its own Toast and hands the page the Area to select.

use lib_core::{AreaId, LocationId};
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{area_name_error_text, area_refusal_text};
use crate::interactor::{AddArea, EditArea};
use crate::protocols::{
    InventoryChange, InventoryOutcome, InventoryRouter, InventoryRow, SharedInventory,
};
use crate::{AreaError, Inventory, NameError, msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaFormEvent {
    /// The name's whole new text, as its View's input holds it.
    Name(String),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The name's problem, once the name has been typed in.
    pub name_error: Option<String>,
    /// `In 12 Elm St. Its items stay with it if you rename it.`, shown while there is no error.
    pub note: String,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// Edit, rather than Add.
    pub editing: bool,
}

pub struct AreaFormPresenter {
    inventory: SharedInventory,
    location: LocationId,
    location_name: String,
    /// The Area an Edit saves; `None` on Add.
    editing: Option<AreaId>,
    name: String,
    /// Whether the name has been typed in, which is when its error starts to show.
    touched: bool,
    refusal: Option<String>,
}

impl AreaFormPresenter {
    /// A fresh Add form for Location `location`, or `None` when it is gone.
    pub fn add(inventory: &SharedInventory, location: LocationId) -> Option<Self> {
        let found = inventory.borrow().location(location)?;
        Some(Self {
            inventory: inventory.clone(),
            location,
            location_name: found.name,
            editing: None,
            name: String::new(),
            touched: false,
            refusal: None,
        })
    }

    /// The Edit form pre-filled from Area `area`, or `None` when it is gone.
    pub fn edit(inventory: &SharedInventory, area: AreaId) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let (location, found) = rows.area(area)?;
        Some(Self {
            inventory: inventory.clone(),
            location: location.id,
            location_name: location.name.clone(),
            editing: Some(area),
            name: found.name.clone(),
            touched: false,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: AreaFormEvent, router: &mut dyn InventoryRouter) {
        match event {
            AreaFormEvent::Name(text) => {
                self.name = text;
                self.touched = true;
                self.refusal = None;
            }
            AreaFormEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Saves the form through the Interactors, raises the Toast and closes the Dialog; a refusal
    /// stays on the form. A form that can't confirm does nothing.
    pub fn confirm(&mut self, router: &mut dyn InventoryRouter) {
        if !self.can_confirm() {
            return;
        }
        let name = self.name.trim().to_string();
        let saved = {
            let mut inventory = self.inventory.borrow_mut();
            match self.editing {
                Some(area) => EditArea {
                    area,
                    name: self.name.clone(),
                }
                .run(&mut *inventory)
                .map(|()| {
                    (
                        area,
                        InventoryChange::AreaSaved,
                        msg::inventory_toast_area_saved(&name),
                    )
                }),
                None => AddArea {
                    location: self.location,
                    name: self.name.clone(),
                }
                .run(&mut *inventory)
                .map(|area| {
                    (
                        area,
                        InventoryChange::AreaAdded,
                        msg::inventory_toast_area_added(&name),
                    )
                }),
            }
        };
        match saved {
            Ok((area, change, toast)) => {
                router.notify(Notice::Toast(ToastKind::Success, toast));
                router.close_dialog(Some(InventoryOutcome {
                    change,
                    location: self.location,
                    selected: Some(InventoryRow::Area(area)),
                }));
            }
            Err(refusal) => self.refusal = Some(area_refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> AreaFormViewModel {
        let editing = self.editing.is_some();
        let (title, submit) = if editing {
            (
                msg::inventory_area_edit_title(),
                msg::inventory_area_edit_submit(),
            )
        } else {
            (
                msg::inventory_area_add_title(),
                msg::inventory_area_add_submit(),
            )
        };
        AreaFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_confirm(),
                destructive: false,
            },
            name: TextFieldView {
                label: msg::inventory_area_field_name(),
                text: self.name.clone(),
                placeholder: msg::inventory_area_field_name_placeholder(),
                focused: true,
            },
            name_error: self
                .problem()
                .filter(|_| self.touched)
                .map(area_name_error_text),
            note: msg::inventory_area_in_location(&self.location_name),
            refusal: self.refusal.clone(),
            editing,
        }
    }

    fn can_confirm(&self) -> bool {
        self.problem().is_none()
    }

    /// What is wrong with the name, if anything. Checked on the Location alone, as the rule is
    /// the Location's: a scratch Inventory of just that row, so the Items aren't copied.
    fn problem(&self) -> Option<NameError> {
        let location = self.inventory.borrow().location(self.location)?;
        let mut scratch = Inventory {
            locations: vec![location],
            items: Vec::new(),
        };
        let result = match self.editing {
            Some(area) => scratch.edit_area(area, &self.name),
            None => scratch.add_area(self.location, &self.name).map(|_| ()),
        };
        match result {
            Err(AreaError::Name(error)) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::protocols::InventoryRepository as _;

    struct Form {
        presenter: AreaFormPresenter,
        router: RecordingRouter,
        inventory: std::rc::Rc<std::cell::RefCell<crate::fake::InMemoryInventory>>,
    }

    impl Form {
        fn add() -> Self {
            let (inventory, shared) = stub();
            let location = inventory.borrow().locations()[0].id;
            Self {
                presenter: AreaFormPresenter::add(&shared, location).expect("seeded"),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn edit() -> Self {
            let (inventory, shared) = stub();
            let area = inventory.borrow().locations()[0].areas[0].id;
            Self {
                presenter: AreaFormPresenter::edit(&shared, area).expect("seeded"),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn send(&mut self, event: AreaFormEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> AreaFormViewModel {
            self.presenter.view_model()
        }

        fn type_name(&mut self, name: &str) {
            self.send(AreaFormEvent::Name(name.into()));
        }

        fn confirm(&mut self) -> Vec<RouterCall> {
            self.presenter.confirm(&mut self.router);
            self.router.take()
        }

        fn areas(&self) -> Vec<String> {
            self.inventory.borrow().locations()[0]
                .areas
                .iter()
                .map(|a| a.name.clone())
                .collect()
        }
    }

    #[test]
    fn a_fresh_add_form_cannot_confirm_and_shows_no_error_until_typed_in() {
        let mut form = Form::add();
        let view = form.view();

        assert!(!view.frame.can_confirm);
        assert_eq!(view.frame.title, "Add area");
        assert_eq!(view.name_error, None);
        assert_eq!(
            view.note,
            "In 12 Elm St contents. Its items stay with it if you rename it."
        );
        assert_eq!(form.confirm(), [], "an empty name confirms nothing");

        form.type_name("");
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("Give the area a name.")
        );
    }

    #[test]
    fn a_name_in_the_locations_other_areas_is_refused_ignoring_case_but_not_its_own() {
        let mut form = Form::add();
        let existing = form.areas()[0].to_uppercase();

        form.type_name(&existing);
        assert!(!form.view().frame.can_confirm);
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("Another area in this location already has that name.")
        );

        let mut edit = Form::edit();
        let own = edit.areas()[0].to_uppercase();
        edit.type_name(&own);
        assert!(edit.view().frame.can_confirm, "an Area keeps its own name");
    }

    #[test]
    fn adding_writes_the_area_last_raises_its_toast_then_closes_on_it() {
        let mut form = Form::add();
        let location = form.inventory.borrow().locations()[0].id;
        form.type_name(" Attic ");

        let calls = form.confirm();

        let areas = form.inventory.borrow().locations()[0].areas.clone();
        let added = areas.last().cloned().expect("the added row");
        assert_eq!(added.name, "Attic");
        assert_eq!(
            calls,
            vec![
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Added area \"Attic\"".into()
                )),
                RouterCall::CloseDialog(Some(InventoryOutcome {
                    change: InventoryChange::AreaAdded,
                    location,
                    selected: Some(InventoryRow::Area(added.id)),
                })),
            ]
        );
    }

    #[test]
    fn editing_renames_the_area_in_place() {
        let mut form = Form::edit();
        let before = form.areas();
        assert_eq!(form.view().name.text, before[0]);
        assert_eq!(form.view().frame.title, "Edit area");

        form.type_name("Lounge");
        let calls = form.confirm();

        let after = form.areas();
        assert_eq!(
            (&after[0], after.len()),
            (&"Lounge".to_string(), before.len())
        );
        assert!(matches!(
            calls.last(),
            Some(RouterCall::CloseDialog(Some(InventoryOutcome {
                change: InventoryChange::AreaSaved,
                ..
            })))
        ));
        assert_eq!(
            calls.first(),
            Some(&RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Saved area \"Lounge\"".into()
            )))
        );
    }

    #[test]
    fn an_area_removed_under_the_dialog_is_refused_and_the_form_stays() {
        let mut form = Form::edit();
        form.type_name("Lounge");
        form.inventory.borrow_mut().locations[0].areas.clear();

        assert_eq!(form.confirm(), []);
        assert_eq!(
            form.view().refusal.as_deref(),
            Some("That location or area has gone or changed, so nothing was saved.")
        );
    }

    #[test]
    fn cancel_closes_without_an_outcome() {
        let mut form = Form::add();

        assert_eq!(
            form.send(AreaFormEvent::Cancel),
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn the_dialog_for_a_gone_row_is_not_built() {
        let (_, shared) = stub();

        assert!(AreaFormPresenter::add(&shared, LocationId::seed(9_999)).is_none());
        assert!(AreaFormPresenter::edit(&shared, AreaId::seed(9_999)).is_none());
    }
}
