//! The Add and Edit location Dialogs (#494): one form, Edit pre-filled from the Location.
//!
//! - The fields, in `Tab` order: Name and, on Add only, the Unit, chosen from the fiat Units and
//!   fixed when the Location is added. Settings asks for nothing else (16q): a Location's address
//!   and cover are edited on its register in Inventory, so Edit starts its draft from the Location
//!   as it is and changes only the name, and nothing else is lost.
//! - The name is required and unique among the Locations, ignoring case, checked live. Its problem
//!   only shows once the name has been typed in, so a fresh Add Dialog opens with its confirm
//!   disabled and no error showing. A missing fiat Unit always shows, since nothing typed fixes it.
//! - The Units are read when the Dialog opens, as it is modal.
//!
//! A confirm raises its own Toast and hands the page the Location to select.

use lib_core::{LocationId, UnitId};
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::{location_name_error_text, location_refusal_text};
use crate::interactor::{AddLocation, EditLocation};
use crate::protocols::{
    InventoryChange, InventoryOutcome, InventoryRouter, InventoryRow, InventoryUnits,
    SharedInventory, UnitChoice,
};
use crate::{Inventory, LocationDraft, LocationError, NameError, msg};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocationField {
    #[default]
    Name,
    /// Add only: a Location's Unit is fixed when it is created.
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationFormEvent {
    /// The name's whole new text, as its View's input holds it.
    Name(String),
    Unit(UnitId),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(LocationField),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The name's problem, once the name has been typed in.
    pub name_error: Option<String>,
    pub unit: UnitField,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// Edit, rather than Add.
    pub editing: bool,
    pub focused: LocationField,
}

/// The Unit field as drawn: a select on Add, the Unit's read-only text on Edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitField {
    /// The fiat Units, labelled `AUD · Australian dollar`.
    pub select: SelectView<UnitId>,
    /// `Every value in this location is in this Unit. It can't be changed later.`
    pub hint: String,
    /// Edit's `AUD · fixed when the location was added`, in place of the select.
    pub fixed: Option<String>,
    /// Set while Add has no fiat Unit to pick.
    pub error: Option<String>,
}

/// The Location an Edit saves, as it opened.
struct Editing {
    id: LocationId,
    /// The details the Dialog doesn't show (address and cover), carried through unchanged.
    kept: LocationDraft,
    fixed_unit: String,
}

/// What is wrong with the draft, in screen order.
enum Problem {
    Name(NameError),
    UnitMissing,
}

pub struct LocationFormPresenter {
    inventory: SharedInventory,
    /// The fiat Units, copied in when the Dialog opened.
    choices: Vec<UnitChoice>,
    editing: Option<Editing>,
    name: String,
    unit: Option<UnitId>,
    focused: LocationField,
    /// Whether the name has been typed in, which is when its error starts to show.
    touched: bool,
    refusal: Option<String>,
}

impl LocationFormPresenter {
    /// A fresh Add form: no name, and `preferred` chosen as the Unit when it is a fiat one, else
    /// the first.
    pub fn add(
        inventory: &SharedInventory,
        units: &dyn InventoryUnits,
        preferred: Option<UnitId>,
    ) -> Self {
        let choices = units.currencies();
        let unit = preferred
            .filter(|id| choices.iter().any(|choice| choice.id == *id))
            .or_else(|| choices.first().map(|choice| choice.id));
        Self {
            inventory: inventory.clone(),
            choices,
            editing: None,
            name: String::new(),
            unit,
            focused: LocationField::Name,
            touched: false,
            refusal: None,
        }
    }

    /// The Edit form pre-filled from Location `id`, or `None` when it is gone.
    pub fn edit(
        inventory: &SharedInventory,
        units: &dyn InventoryUnits,
        id: LocationId,
    ) -> Option<Self> {
        let location = inventory.borrow().location(id)?;
        let choices = units.currencies();
        let code = units.unit_code(location.unit).unwrap_or_default();
        Some(Self {
            inventory: inventory.clone(),
            unit: choices
                .iter()
                .find(|choice| choice.id == location.unit)
                .map(|choice| choice.id),
            choices,
            editing: Some(Editing {
                id,
                kept: LocationDraft::from(&location),
                fixed_unit: msg::inventory_field_unit_fixed(&code.to_uppercase()),
            }),
            name: location.name,
            focused: LocationField::Name,
            touched: false,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: LocationFormEvent, router: &mut dyn InventoryRouter) {
        match event {
            LocationFormEvent::Name(text) => {
                self.name = text;
                self.touched = true;
                self.refusal = None;
            }
            LocationFormEvent::Unit(id) => {
                if self.editing.is_none() && self.choices.iter().any(|choice| choice.id == id) {
                    self.unit = Some(id);
                }
            }
            LocationFormEvent::NextField => self.cycle(false),
            LocationFormEvent::PrevField => self.cycle(true),
            LocationFormEvent::Focus(field) => {
                if self.order().contains(&field) {
                    self.focused = field;
                }
            }
            LocationFormEvent::Cancel => router.close_dialog(None),
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
            match &self.editing {
                Some(editing) => EditLocation {
                    id: editing.id,
                    draft: LocationDraft {
                        name: self.name.clone(),
                        ..editing.kept.clone()
                    },
                }
                .run(&mut *inventory)
                .map(|()| {
                    (
                        editing.id,
                        InventoryChange::LocationSaved,
                        msg::inventory_toast_location_saved(&name),
                    )
                }),
                None => match self.unit {
                    Some(unit) => AddLocation {
                        draft: LocationDraft {
                            name: self.name.clone(),
                            ..LocationDraft::default()
                        },
                        unit,
                    }
                    .run(&mut *inventory)
                    .map(|id| {
                        (
                            id,
                            InventoryChange::LocationAdded,
                            msg::inventory_toast_location_added(&name),
                        )
                    }),
                    None => return,
                },
            }
        };
        match saved {
            Ok((id, change, toast)) => {
                router.notify(Notice::Toast(ToastKind::Success, toast));
                router.close_dialog(Some(InventoryOutcome {
                    change,
                    location: id,
                    selected: Some(InventoryRow::Location(id)),
                }));
            }
            Err(refusal) => self.refusal = Some(location_refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> LocationFormViewModel {
        let editing = self.editing.is_some();
        let (title, submit) = if editing {
            (
                msg::inventory_dialog_edit_title(),
                msg::inventory_dialog_save(),
            )
        } else {
            (
                msg::inventory_dialog_add_title(),
                msg::inventory_dialog_add_submit(),
            )
        };
        let problem = self.problem();
        LocationFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: problem.is_none(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: msg::inventory_field_name(),
                text: self.name.clone(),
                placeholder: msg::inventory_field_name_placeholder(),
                focused: self.focused == LocationField::Name,
            },
            name_error: match problem {
                Some(Problem::Name(error)) if self.touched => Some(location_name_error_text(error)),
                _ => None,
            },
            unit: UnitField {
                select: SelectView {
                    label: msg::inventory_field_unit(),
                    options: self
                        .choices
                        .iter()
                        .map(|choice| Choice {
                            value: choice.id,
                            label: msg::inventory_unit_option(
                                &choice.code.to_uppercase(),
                                &choice.name,
                            ),
                        })
                        .collect(),
                    chosen: self.unit,
                    focused: self.focused == LocationField::Unit,
                },
                hint: msg::inventory_field_unit_hint(),
                fixed: self
                    .editing
                    .as_ref()
                    .map(|editing| editing.fixed_unit.clone()),
                error: (!editing && self.unit.is_none()).then(msg::inventory_error_unit_missing),
            },
            refusal: self.refusal.clone(),
            editing,
            focused: self.focused,
        }
    }

    fn order(&self) -> &'static [LocationField] {
        if self.editing.is_some() {
            &[LocationField::Name]
        } else {
            &[LocationField::Name, LocationField::Unit]
        }
    }

    fn cycle(&mut self, backward: bool) {
        let order = self.order();
        let at = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            at + order.len() - 1
        } else {
            at + 1
        };
        self.focused = order[next % order.len()];
    }

    fn can_confirm(&self) -> bool {
        self.problem().is_none()
    }

    /// The first problem with the draft, in screen order. The name is checked against the other
    /// Locations alone, on a scratch Inventory without the Items.
    fn problem(&self) -> Option<Problem> {
        let mut scratch = Inventory {
            locations: self.inventory.borrow().locations(),
            items: Vec::new(),
        };
        let draft = LocationDraft {
            name: self.name.clone(),
            ..LocationDraft::default()
        };
        let result = match &self.editing {
            Some(editing) => scratch.edit_location(editing.id, &draft),
            None => scratch
                .add_location(&draft, self.unit.unwrap_or_else(UnitId::new))
                .map(|_| ()),
        };
        match result {
            Err(LocationError::Name(error)) => Some(Problem::Name(error)),
            _ => (self.editing.is_none() && self.unit.is_none()).then_some(Problem::UnitMissing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{InMemoryInventory, InMemoryUnits, RecordingRouter, RouterCall};
    use crate::protocols::InventoryRepository as _;

    struct Form {
        presenter: LocationFormPresenter,
        router: RecordingRouter,
        inventory: std::rc::Rc<std::cell::RefCell<InMemoryInventory>>,
    }

    impl Form {
        fn add() -> Self {
            Self::add_with(&InMemoryUnits::stub(), None)
        }

        fn add_with(units: &InMemoryUnits, preferred: Option<UnitId>) -> Self {
            let (inventory, shared) = stub();
            Self {
                presenter: LocationFormPresenter::add(&shared, units, preferred),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn edit() -> Self {
            let (inventory, shared) = stub();
            let id = inventory.borrow().locations()[0].id;
            Self {
                presenter: LocationFormPresenter::edit(&shared, &InMemoryUnits::stub(), id)
                    .expect("seeded"),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn send(&mut self, event: LocationFormEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> LocationFormViewModel {
            self.presenter.view_model()
        }

        fn type_name(&mut self, name: &str) {
            self.send(LocationFormEvent::Name(name.into()));
        }

        fn confirm(&mut self) -> Vec<RouterCall> {
            self.presenter.confirm(&mut self.router);
            self.router.take()
        }
    }

    #[test]
    fn a_fresh_add_form_has_the_first_fiat_unit_and_cannot_confirm_or_show_an_error() {
        let view = Form::add().view();

        assert_eq!(view.frame.title, "Add location");
        assert_eq!(view.frame.submit, "Add location");
        assert!(!view.frame.can_confirm);
        assert_eq!(view.name_error, None);
        assert_eq!(view.unit.select.chosen, Some(UnitId::SEED_AUD));
        let labels: Vec<_> = view
            .unit
            .select
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(
            labels,
            ["AUD \u{b7} Australian dollar", "USD \u{b7} US dollar"]
        );
        assert_eq!(view.unit.fixed, None);
    }

    #[test]
    fn add_preselects_the_preferred_unit_only_when_it_is_a_fiat_one() {
        let usd = UnitId::seed(20);
        let units = InMemoryUnits::stub();

        assert_eq!(
            Form::add_with(&units, Some(usd)).view().unit.select.chosen,
            Some(usd)
        );
        assert_eq!(
            Form::add_with(&units, Some(UnitId::SEED_BTC))
                .view()
                .unit
                .select
                .chosen,
            Some(UnitId::SEED_AUD)
        );
    }

    #[test]
    fn a_unit_not_offered_is_ignored_and_none_to_pick_blocks_the_form() {
        let mut form = Form::add();
        form.send(LocationFormEvent::Unit(UnitId::seed(20)));
        assert_eq!(form.view().unit.select.chosen, Some(UnitId::seed(20)));
        form.send(LocationFormEvent::Unit(UnitId::SEED_BTC));
        assert_eq!(form.view().unit.select.chosen, Some(UnitId::seed(20)));

        let mut none = Form::add_with(&InMemoryUnits::default(), None);
        none.type_name("Cabin");
        let view = none.view();
        assert!(!view.frame.can_confirm);
        assert_eq!(
            view.unit.error.as_deref(),
            Some("Add a fiat Unit in Settings \u{203a} Units first.")
        );
        assert_eq!(none.confirm(), []);
    }

    #[test]
    fn the_name_is_required_and_unique_among_locations_ignoring_case_once_typed() {
        let mut form = Form::add();

        form.type_name("");
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("Give the location a name.")
        );
        form.type_name("storage UNIT");
        assert_eq!(
            form.view().name_error.as_deref(),
            Some("Another location already has that name.")
        );
        assert!(!form.view().frame.can_confirm);
        form.type_name("Cabin");
        assert_eq!(form.view().name_error, None);
        assert!(form.view().frame.can_confirm);
    }

    #[test]
    fn tab_walks_name_then_unit_on_add_and_edit_has_the_name_alone() {
        let mut form = Form::add();
        assert_eq!(form.view().focused, LocationField::Name);
        form.send(LocationFormEvent::NextField);
        assert_eq!(form.view().focused, LocationField::Unit);
        form.send(LocationFormEvent::NextField);
        assert_eq!(form.view().focused, LocationField::Name);
        form.send(LocationFormEvent::PrevField);
        assert_eq!(form.view().focused, LocationField::Unit);
        form.send(LocationFormEvent::Focus(LocationField::Name));
        assert_eq!(form.view().focused, LocationField::Name);

        let mut edit = Form::edit();
        edit.send(LocationFormEvent::NextField);
        edit.send(LocationFormEvent::Focus(LocationField::Unit));
        assert_eq!(edit.view().focused, LocationField::Name);
    }

    #[test]
    fn adding_writes_the_location_in_its_unit_raises_its_toast_then_closes_on_it() {
        let mut form = Form::add();
        form.send(LocationFormEvent::Unit(UnitId::seed(20)));
        form.type_name(" Cabin ");

        let calls = form.confirm();

        let added = form
            .inventory
            .borrow()
            .locations()
            .last()
            .cloned()
            .expect("the added row");
        assert_eq!(
            (added.name.as_str(), added.unit),
            ("Cabin", UnitId::seed(20))
        );
        assert_eq!(
            calls,
            vec![
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Added location \"Cabin\"".into()
                )),
                RouterCall::CloseDialog(Some(InventoryOutcome {
                    change: InventoryChange::LocationAdded,
                    location: added.id,
                    selected: Some(InventoryRow::Location(added.id)),
                })),
            ]
        );
    }

    #[test]
    fn editing_changes_the_name_and_keeps_the_unit_cover_and_areas() {
        let mut form = Form::edit();
        let before = form.inventory.borrow().locations()[0].clone();
        let view = form.view();
        assert_eq!(view.name.text, before.name);
        assert_eq!(view.frame.title, "Edit location");
        assert_eq!(view.frame.submit, "Save");
        assert_eq!(
            view.unit.fixed.as_deref(),
            Some("AUD \u{b7} fixed when the location was added")
        );

        form.type_name("Elm");
        let calls = form.confirm();

        let after = form.inventory.borrow().locations()[0].clone();
        assert_eq!(after.name, "Elm");
        assert_eq!(
            (after.unit, &after.cover, &after.areas, &after.address),
            (before.unit, &before.cover, &before.areas, &before.address)
        );
        assert_eq!(
            calls.first(),
            Some(&RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Saved location \"Elm\"".into()
            )))
        );
    }

    #[test]
    fn a_location_removed_under_the_dialog_is_refused_and_the_form_stays() {
        let mut form = Form::edit();
        form.type_name("Elm");
        form.inventory.borrow_mut().locations.clear();

        assert_eq!(form.confirm(), []);
        assert_eq!(
            form.view().refusal.as_deref(),
            Some("That location or area has gone or changed, so nothing was saved.")
        );
    }

    #[test]
    fn cancel_closes_without_an_outcome_and_a_gone_location_builds_no_edit() {
        let mut form = Form::add();
        assert_eq!(
            form.send(LocationFormEvent::Cancel),
            [RouterCall::CloseDialog(None)]
        );

        let (_, shared) = stub();
        assert!(
            LocationFormPresenter::edit(&shared, &InMemoryUnits::stub(), LocationId::seed(9_999))
                .is_none()
        );
    }
}
