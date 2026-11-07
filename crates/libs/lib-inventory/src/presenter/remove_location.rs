//! The Remove location Dialog: a plain confirm for an empty Location, and otherwise the destructive
//! frame that quotes what goes with it, its Areas and Items and the Documents that lose a Link to
//! them, behind the typed-back name (exact, case-sensitive).
//!
//! Its confirm runs the Remove Location Interactor on the Inventory it holds, which removes the
//! Location with its Areas and Items; the file's cascade drops the Document Links to them in the
//! one commit of the input. A refusal stays in the Dialog.
//! It raises its own Toast on landing, and tells the page which row the cursor lands on: the
//! neighbour that takes the removed row's place. The last Location can go.

use lib_core::LocationId;
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{DialogFrame, Hint, TextFieldView, count};

use super::location_refusal_text;
use crate::interactor::RemoveLocation;
use crate::msg;
use crate::protocols::{
    InventoryChange, InventoryOutcome, InventoryRow, LocationUsage, SharedInventory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveLocationEvent {
    /// A Dialog command or typed key: `select` and `submit` remove the Location once the Dialog
    /// allows it, and typing goes to the typed-back name, the only field, which an empty Location
    /// doesn't have.
    Input(DialogInput),
    /// The confirmation field's whole new text.
    ConfirmText(String),
    /// Removes the Location once the Dialog allows it, as its confirm button does.
    Confirm,
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
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct RemoveLocationPresenter {
    keys: DialogKeys,
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
    /// `keys` are the Dialog keymap's chords, which its hints show.
    pub fn new(
        inventory: &SharedInventory,
        usage: &dyn LocationUsage,
        id: LocationId,
        keys: DialogKeys,
    ) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let location = rows.location(id)?;
        Some(Self {
            keys,
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
}

impl Presenter for RemoveLocationPresenter {
    type Event = RemoveLocationEvent;
    type ViewModel = RemoveLocationViewModel;
    type Router<'r> = dyn DialogRouter<InventoryOutcome> + 'r;

    fn handle(&mut self, event: RemoveLocationEvent, router: &mut Self::Router<'_>) {
        match event {
            RemoveLocationEvent::Input(input) => self.input(input, router),
            RemoveLocationEvent::ConfirmText(text) => self.confirm_text = text,
            RemoveLocationEvent::Confirm => self.save(router),
            RemoveLocationEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> RemoveLocationViewModel {
        let name = self.name.as_str();
        let holds_something = self.holds_something();
        RemoveLocationViewModel {
            frame: DialogFrame {
                title: msg::inventory_dialog_remove_title(),
                submit: msg::inventory_dialog_remove_submit(),
                can_confirm: self.can_confirm(),
                destructive: holds_something,
                hints: Vec::new(),
            },
            hints: self.hints(),
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
}

impl DialogPresenter for RemoveLocationPresenter {
    type Outcome = InventoryOutcome;

    fn input_event(input: DialogInput) -> RemoveLocationEvent {
        RemoveLocationEvent::Input(input)
    }

    fn confirm_event() -> RemoveLocationEvent {
        RemoveLocationEvent::Confirm
    }

    fn cancel_event() -> RemoveLocationEvent {
        RemoveLocationEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        // Always for an empty Location, else once the name is typed exactly.
        !self.holds_something() || self.confirm_text == self.name
    }
}

impl RemoveLocationPresenter {
    /// Cancel and confirm, from the Dialog keys.
    fn hints(&self) -> Vec<Hint> {
        vec![
            self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
            self.keys
                .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
        ]
    }

    /// Reads a Dialog command or typed key: `select` and `submit` remove the Location once the
    /// Dialog allows it, and typing goes to the typed-back name, the only field, which an empty
    /// Location doesn't have.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<InventoryOutcome>) {
        let typed_back = self.holds_something();
        match input {
            DialogInput::Select | DialogInput::Submit => self.save(router),
            DialogInput::Char(ch) if typed_back => self.confirm_text.push(ch),
            DialogInput::Backspace if typed_back => {
                self.confirm_text.pop();
            }
            _ => {}
        }
    }

    /// Removes the Location once the Dialog allows it, raises the Toast and
    /// closes the Dialog; a refusal stays in the Dialog.
    fn save(&mut self, router: &mut dyn DialogRouter<InventoryOutcome>) {
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
        let removed = RemoveLocation(self.id).run(&mut *self.inventory.borrow_mut());
        match removed {
            Ok(_) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::inventory_toast_location_removed(&self.name),
                ));
                // The cursor lands on the neighbour that takes the removed row's place.
                let locations = self.inventory.borrow().locations();
                let selected = locations
                    .get(position.min(locations.len().saturating_sub(1)))
                    .map(|location| InventoryRow::Location(location.id));
                router.close(Some(InventoryOutcome {
                    change: InventoryChange::LocationRemoved,
                    location: self.id,
                    selected,
                }));
            }
            Err(refusal) => self.refusal = Some(location_refusal_text(refusal)),
        }
    }

    /// Only a Location holding Areas or Items asks for its name back.
    fn holds_something(&self) -> bool {
        self.areas > 0 || self.items > 0
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::sample;
    use super::*;
    use crate::fake::{InMemoryInventory, InMemoryUsage, RecordingRouter, RouterCall};
    use crate::protocols::{InventoryRepository as _, LocationUse};

    struct Dialog {
        presenter: RemoveLocationPresenter,
        router: RecordingRouter,
        inventory: std::rc::Rc<std::cell::RefCell<InMemoryInventory>>,
    }

    impl Dialog {
        fn on(index: usize) -> Self {
            Self::with_documents(index, 0)
        }

        fn with_documents(index: usize, documents: usize) -> Self {
            let (inventory, shared) = sample();
            let id = inventory.borrow().locations()[index].id;
            let mut usage = InMemoryUsage::default();
            usage.uses.insert(id, LocationUse { documents });
            Self {
                presenter: RemoveLocationPresenter::new(&shared, &usage, id, DialogKeys::default())
                    .expect("seeded"),
                router: RecordingRouter::default(),
                inventory,
            }
        }

        fn press(&mut self, input: DialogInput) {
            self.presenter.input(input, &mut self.router);
        }

        fn typed(&mut self, text: &str) {
            self.presenter.handle(
                RemoveLocationEvent::ConfirmText(text.into()),
                &mut self.router,
            );
        }

        fn confirm(&mut self) -> Vec<RouterCall> {
            self.presenter
                .handle(RemoveLocationEvent::Confirm, &mut self.router);
            self.router.take()
        }

        /// Location `index` emptied of its Areas and Items.
        fn emptied(index: usize) -> Self {
            let (inventory, shared) = sample();
            {
                let mut rows = inventory.borrow_mut();
                let id = rows.locations[index].id;
                rows.locations[index].areas.clear();
                rows.items.retain(|item| item.location != id);
            }
            let id = inventory.borrow().locations()[index].id;
            Self {
                presenter: RemoveLocationPresenter::new(
                    &shared,
                    &InMemoryUsage::default(),
                    id,
                    DialogKeys::default(),
                )
                .expect("seeded"),
                router: RecordingRouter::default(),
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

        let (_, shared) = sample();
        assert!(
            RemoveLocationPresenter::new(
                &shared,
                &InMemoryUsage::default(),
                LocationId::seed(9_999),
                DialogKeys::default()
            )
            .is_none()
        );
    }

    #[test]
    fn typing_builds_the_name_and_select_removes_only_once_it_is_exact() {
        let mut dialog = Dialog::on(0);
        let id = dialog.inventory.borrow().locations()[0].id;
        let name = dialog.inventory.borrow().locations()[0].name.clone();

        for ch in name.chars() {
            dialog.press(DialogInput::Char(ch));
        }
        dialog.press(DialogInput::Char('x'));
        dialog.press(DialogInput::Select);
        dialog.press(DialogInput::NextField);
        assert!(
            dialog.inventory.borrow().location(id).is_some(),
            "not the name yet"
        );
        dialog.press(DialogInput::Backspace);
        dialog.press(DialogInput::Submit);

        assert!(dialog.inventory.borrow().location(id).is_none());
        assert!(matches!(
            dialog.router.take().last(),
            Some(RouterCall::CloseDialog(Some(_)))
        ));
    }

    #[test]
    fn an_empty_location_takes_no_typing_and_select_removes_it() {
        let mut dialog = Dialog::emptied(0);
        let id = dialog.inventory.borrow().locations()[0].id;

        dialog.press(DialogInput::Char('x'));
        assert_eq!(dialog.presenter.view_model().confirm, None);
        dialog.press(DialogInput::Select);

        assert!(dialog.inventory.borrow().location(id).is_none());
    }
}
