//! The **Location** (#494) and **Area** (#495) dialogs' drafts: the typed text of every field, the
//! focus between them and the checks that turn it into an [`inventory::LocationDraft`]. `gpui`-free,
//! so each rule is unit-tested without a window; the model's own rules (unique names) stay in
//! `inventory` and this module only words them per field.
//!
//! Everything a check needs from the page (the Inventory, the fiat Units) is copied in when the
//! Dialog opens, so no form reaches back into it.
//!
//! Settings sets up only a Location's name and Unit (16q). Its address and cover are edited from
//! the Inventory surface, so Edit starts its draft from the Location as it is and changes only the
//! name: nothing else is lost.

use lib_core::{AreaId, LocationId};

use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectState,
    inventory::{self, Inventory, Location, LocationDraft, LocationError, NameError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationField {
    Name,
    /// Add only: a Location's Unit is fixed when it is created.
    Unit,
}

/// What is wrong with the draft, one field at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    NameEmpty,
    NameTaken,
    /// Add with no fiat Unit to pick.
    UnitMissing,
}

impl Problem {
    pub fn field(self) -> LocationField {
        match self {
            Self::NameEmpty | Self::NameTaken => LocationField::Name,
            Self::UnitMissing => LocationField::Unit,
        }
    }
}

/// What the Location form checks against, copied from the page when the Dialog opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationContext {
    pub inventory: Inventory,
    /// The fiat Units a Location can be kept in: each one's option label, then its code.
    pub unit_choices: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationForm {
    pub name: TextField,
    pub unit: SelectState,
    pub focused: LocationField,
    /// Whether this is Add, which is when the Unit is chosen.
    pub adding: bool,
    /// Whether the name has been typed in, which is when its error starts to show.
    touched: bool,
    context: LocationContext,
    /// The Location being edited; `None` on Add.
    own_id: Option<LocationId>,
    /// The details the dialog doesn't show (address and cover), carried through unchanged.
    kept: LocationDraft,
}

impl LocationForm {
    /// Add: `unit` is the option label to pre-select.
    pub fn for_add(unit: Option<String>, context: LocationContext) -> Self {
        Self {
            name: TextField::default(),
            unit: SelectState::new(unit),
            focused: LocationField::Name,
            adding: true,
            touched: false,
            context,
            own_id: None,
            kept: LocationDraft::default(),
        }
    }

    pub fn from_location(location: &Location, context: LocationContext) -> Self {
        Self {
            name: TextField::new(location.name.clone()),
            unit: SelectState::new(Some(location.unit.clone())),
            focused: LocationField::Name,
            adding: false,
            touched: false,
            context,
            own_id: Some(location.id),
            kept: LocationDraft::from(location),
        }
    }

    /// The fields `tab` walks, in screen order: Edit has the name alone.
    pub fn fields(&self) -> Vec<LocationField> {
        if self.adding {
            vec![LocationField::Name, LocationField::Unit]
        } else {
            vec![LocationField::Name]
        }
    }

    pub fn focus(&mut self, field: LocationField) {
        if self.fields().contains(&field) {
            self.focused = field;
        }
    }

    pub fn cycle_focus(&mut self, backwards: bool) {
        let fields = self.fields();
        let at = fields.iter().position(|f| *f == self.focused).unwrap_or(0);
        let next = if backwards {
            (at + fields.len() - 1) % fields.len()
        } else {
            (at + 1) % fields.len()
        };
        self.focused = fields[next];
    }

    /// The fiat Units' option labels, in order: the Unit select's list.
    pub fn unit_labels(&self) -> Vec<String> {
        self.context
            .unit_choices
            .iter()
            .map(|(label, _)| label.clone())
            .collect()
    }

    /// The Unit code the picker is on, if it has a Unit at all.
    pub fn unit_code(&self) -> Option<&str> {
        let label = self.unit.value()?;
        self.context
            .unit_choices
            .iter()
            .find(|(option, _)| option == label)
            .map(|(_, code)| code.as_str())
    }

    /// A click on the Unit field: focuses it and toggles its list.
    pub fn click_unit(&mut self) {
        self.focus(LocationField::Unit);
        if self.unit.is_open() {
            self.unit.cancel();
        } else {
            self.unit.open(&self.unit_labels());
        }
    }

    pub fn choose_unit(&mut self, index: usize) {
        self.unit.choose(&self.unit_labels(), index);
    }

    /// The draft the model checks: the typed name over the details the dialog doesn't show.
    pub fn draft(&self) -> LocationDraft {
        LocationDraft {
            name: self.name.text().to_string(),
            ..self.kept.clone()
        }
    }

    /// The first problem with the draft, in screen order.
    pub fn problem(&self) -> Option<Problem> {
        let unit = self.unit_code();
        let draft = self.draft();
        let mut scratch = self.context.inventory.clone();
        let result = match self.own_id {
            Some(id) => scratch.edit_location(id, &draft),
            None => scratch
                .add_location(&draft, unit.unwrap_or("-"))
                .map(|_| ()),
        };
        match result {
            Err(LocationError::Name(NameError::Empty)) => Some(Problem::NameEmpty),
            Err(LocationError::Name(NameError::Taken)) => Some(Problem::NameTaken),
            Err(LocationError::UnitMissing) => Some(Problem::UnitMissing),
            // Settings never edits the cover, so a cover refusal can only be the Location's own.
            Ok(()) | Err(LocationError::Unknown | LocationError::Cover(_)) => {
                (self.adding && unit.is_none()).then_some(Problem::UnitMissing)
            }
        }
    }

    /// Whether `problem` has been earned yet: the name was typed in. A missing Unit always shows,
    /// since nothing typed can fix it. A fresh Add dialog opens with its confirm disabled and no
    /// error showing.
    pub fn shows(&self, problem: Problem) -> bool {
        match problem.field() {
            LocationField::Name => self.touched,
            LocationField::Unit => true,
        }
    }
}

impl Dialog for LocationForm {
    /// `Up`/`Down` walk the Unit list or step its value, `Space` opens it or commits the
    /// highlight and `Enter` commits an open list, all while the Unit is focused (which a
    /// control swallows typing on); `Shift-Tab` goes back a field.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_unit = self.focused == LocationField::Unit;
        match key {
            DialogKey::BackTab => {
                self.unit.cancel();
                self.cycle_focus(true);
            }
            DialogKey::Up | DialogKey::Down if on_unit => {
                let delta = if key == DialogKey::Up { -1 } else { 1 };
                let labels = self.unit_labels();
                if self.unit.is_open() {
                    self.unit.move_highlight(&labels, delta);
                } else {
                    self.unit.step(&labels, delta);
                }
            }
            DialogKey::Char(' ') if on_unit => {
                let labels = self.unit_labels();
                if self.unit.is_open() {
                    self.unit.commit(&labels);
                } else {
                    self.unit.open(&labels);
                }
            }
            DialogKey::Enter if on_unit && self.unit.is_open() => {
                self.unit.commit(&self.unit_labels());
            }
            DialogKey::Char(_) | DialogKey::Backspace if on_unit => {}
            DialogKey::Char(_) | DialogKey::Backspace => {
                // Typing falls through to the shared handling; this only marks the name typed in.
                self.touched = true;
                return None;
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        (self.focused == LocationField::Name).then_some(&mut self.name)
    }

    fn cycle_field(&mut self) {
        self.unit.cancel();
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        self.problem().is_none()
    }

    fn close_open_select(&mut self) -> bool {
        let was_open = self.unit.is_open();
        self.unit.cancel();
        was_open
    }
}

/// The Remove **Location** dialog's draft. Whether the name must be typed is copied in at open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveLocationForm {
    name: String,
    pub typed: TextField,
    /// Only a Location holding Areas or Items asks for its name back.
    needs_typed_name: bool,
}

impl RemoveLocationForm {
    pub fn new(name: impl Into<String>, needs_typed_name: bool) -> Self {
        Self {
            name: name.into(),
            typed: TextField::default(),
            needs_typed_name,
        }
    }
}

impl Dialog for RemoveLocationForm {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        // With nothing to confirm by typing, the field swallows typing rather than leaking it.
        match key {
            DialogKey::Char(_) | DialogKey::Backspace if !self.needs_typed_name => {
                Some(DialogOutcome::Handled)
            }
            _ => None,
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.needs_typed_name.then_some(&mut self.typed)
    }

    fn is_valid(&self) -> bool {
        !self.needs_typed_name || self.typed.text() == self.name
    }
}

/// The Add and Edit **Area** dialog's draft (#495): a name and whether it has been typed in, which
/// is when its error starts to show. The rules stay in `inventory`; this only words them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaForm {
    pub name: TextField,
    touched: bool,
    inventory: Inventory,
    location: LocationId,
    /// The Area being edited; `None` on Add.
    own_id: Option<AreaId>,
}

impl AreaForm {
    pub fn for_add(inventory: Inventory, location: LocationId) -> Self {
        Self {
            name: TextField::default(),
            touched: false,
            inventory,
            location,
            own_id: None,
        }
    }

    pub fn for_edit(inventory: Inventory, location: LocationId, area: &inventory::Area) -> Self {
        Self {
            name: TextField::new(area.name.clone()),
            touched: false,
            inventory,
            location,
            own_id: Some(area.id),
        }
    }

    /// What is wrong with the name, if anything. The name is unique within the Location only, so
    /// the same name in another Location is fine.
    pub fn problem(&self) -> Option<NameError> {
        let mut scratch = self.inventory.clone();
        let name = self.name.text();
        let result = match self.own_id {
            Some(id) => scratch.edit_area(id, name),
            None => scratch.add_area(self.location, name).map(|_| ()),
        };
        match result {
            Err(inventory::AreaError::Name(error)) => Some(error),
            _ => None,
        }
    }

    pub fn shows_problem(&self) -> bool {
        self.touched
    }
}

impl Dialog for AreaForm {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        // Typing falls through to the shared handling; this only marks the name typed in.
        if matches!(key, DialogKey::Char(_) | DialogKey::Backspace) {
            self.touched = true;
        }
        None
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        Some(&mut self.name)
    }

    fn is_valid(&self) -> bool {
        self.problem().is_none()
    }
}

/// The Remove **Area** dialog's destination select. The other Areas' names and whether Items need
/// a home are copied in at open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveAreaForm {
    /// The Location's other Areas, in tab order: where a removed Area's Items can go.
    pub destinations: Vec<String>,
    pub select: SelectState,
    needs_destination: bool,
}

impl RemoveAreaForm {
    pub fn new(destinations: Vec<String>, preselected: Option<String>, holds_items: bool) -> Self {
        Self {
            destinations,
            select: SelectState::new(preselected),
            needs_destination: holds_items,
        }
    }

    /// A click on the select toggles its list.
    pub fn click_select(&mut self) {
        if self.select.is_open() {
            self.select.cancel();
        } else {
            self.select.open(&self.destinations);
        }
    }

    pub fn choose(&mut self, index: usize) {
        self.select.choose(&self.destinations, index);
    }
}

impl Dialog for RemoveAreaForm {
    /// `Up`/`Down` walk the open list or step the value; `Space` opens it or commits the
    /// highlight; `Enter` commits an open list, else falls through to confirm.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Up | DialogKey::Down => {
                let delta = if key == DialogKey::Up { -1 } else { 1 };
                if self.select.is_open() {
                    self.select.move_highlight(&self.destinations, delta);
                } else {
                    self.select.step(&self.destinations, delta);
                }
            }
            DialogKey::Char(' ') if self.select.is_open() => {
                self.select.commit(&self.destinations);
            }
            DialogKey::Char(' ') => self.select.open(&self.destinations),
            DialogKey::Enter if self.select.is_open() => self.select.commit(&self.destinations),
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        !self.needs_destination
            || self
                .select
                .value()
                .is_some_and(|chosen| self.destinations.iter().any(|name| name == chosen))
    }

    fn close_open_select(&mut self) -> bool {
        let was_open = self.select.is_open();
        self.select.cancel();
        was_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::inventory::dialogs::InventoryDialog;

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()
    }

    fn seed() -> Inventory {
        inventory::default_inventory(today())
    }

    fn context() -> LocationContext {
        LocationContext {
            inventory: seed(),
            unit_choices: vec![
                ("AUD".to_string(), "aud".to_string()),
                ("NZD".to_string(), "nzd".to_string()),
            ],
        }
    }

    /// An Add form with the AUD Unit picked, as the page opens it.
    fn add_form() -> LocationForm {
        LocationForm::for_add(Some("AUD".to_string()), context())
    }

    fn edit_form(location: &Location) -> LocationForm {
        LocationForm::from_location(location, context())
    }

    fn type_text(dialog: &mut impl Dialog, text: &str) {
        for ch in text.chars() {
            handle_key(dialog, DialogKey::Char(ch));
        }
    }

    fn type_into(form: &mut LocationForm, field: LocationField, text: &str) {
        form.focus(field);
        type_text(form, text);
    }

    #[test]
    fn tab_walks_name_and_unit_on_add_and_edit_has_the_name_alone() {
        let inventory = seed();
        let mut edit = edit_form(&inventory.locations[0]);
        assert_eq!(edit.fields(), [LocationField::Name]);
        edit.focus(LocationField::Unit);
        assert_eq!(
            edit.focused,
            LocationField::Name,
            "a disabled field takes no focus"
        );
        handle_key(&mut edit, DialogKey::Tab);
        assert_eq!(edit.focused, LocationField::Name);

        let mut add = add_form();
        handle_key(&mut add, DialogKey::Tab);
        assert_eq!(add.focused, LocationField::Unit);
        handle_key(&mut add, DialogKey::BackTab);
        assert_eq!(add.focused, LocationField::Name);
    }

    #[test]
    fn edit_renames_and_keeps_the_address_and_cover_settings_does_not_show() {
        let inventory = seed();
        let covered = inventory
            .locations
            .iter()
            .find(|l| l.cover.is_some())
            .expect("the seed has a covered Location");
        let mut form = edit_form(covered);
        form.name = TextField::new("Renamed");
        let draft = form.draft();
        assert_eq!(draft.name, "Renamed");
        assert_eq!(draft.address, covered.address);
        assert_eq!(
            draft.cover,
            inventory::CoverDraft::from(covered.cover.as_ref().unwrap())
        );
        assert_eq!(form.problem(), None);
    }

    fn problem(form: &LocationForm) -> Option<Problem> {
        form.problem()
    }

    #[test]
    fn a_fresh_add_has_no_name_and_shows_no_error_yet() {
        let form = add_form();
        let found = problem(&form);
        assert_eq!(found, Some(Problem::NameEmpty));
        assert!(!form.shows(found.unwrap()));
        assert!(!form.is_valid());
    }

    #[test]
    fn a_name_and_unit_are_all_add_needs() {
        let mut form = add_form();
        type_into(&mut form, LocationField::Name, "Beach house");
        assert_eq!(problem(&form), None);
        assert!(form.is_valid());
    }

    #[test]
    fn an_add_with_no_fiat_unit_is_refused() {
        let mut form = LocationForm::for_add(None, context());
        type_into(&mut form, LocationField::Name, "Beach house");
        assert_eq!(problem(&form), Some(Problem::UnitMissing));
    }

    #[test]
    fn a_taken_name_is_refused_ignoring_case_but_not_for_the_location_itself() {
        let inventory = seed();
        let existing = inventory.locations[0].clone();
        let mut form = add_form();
        type_into(
            &mut form,
            LocationField::Name,
            &existing.name.to_uppercase(),
        );
        assert_eq!(problem(&form), Some(Problem::NameTaken));
        let mut editing = edit_form(&existing);
        editing.name = TextField::new(existing.name.to_uppercase());
        assert_eq!(problem(&editing), None);
    }

    #[test]
    fn the_unit_select_walks_commits_and_first_esc_closes_it() {
        let mut form = add_form();
        form.focus(LocationField::Unit);
        // Typing on the select is swallowed, not appended anywhere.
        assert_eq!(
            handle_key(&mut form, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(form.name.text(), "");
        handle_key(&mut form, DialogKey::Char(' '));
        assert!(form.unit.is_open());
        handle_key(&mut form, DialogKey::Down);
        assert!(form.close_open_select());
        assert!(!form.close_open_select());
        assert_eq!(form.unit.value(), Some("AUD"));
        handle_key(&mut form, DialogKey::Char(' '));
        handle_key(&mut form, DialogKey::Down);
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert_eq!(form.unit.value(), Some("NZD"));
        assert_eq!(form.unit_code(), Some("nzd"));
    }

    #[test]
    fn enter_confirms_only_when_the_location_form_is_valid() {
        let mut form = add_form();
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut form, "Beach house");
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn an_area_name_is_unique_within_its_location_only() {
        let inventory = seed();
        let elm = &inventory.locations[0];
        let storage = &inventory.locations[1];
        let mut form = AreaForm::for_add(inventory.clone(), elm.id);
        assert_eq!(form.problem(), Some(NameError::Empty));
        assert!(!form.shows_problem(), "a fresh dialog shows no error yet");
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut form, &elm.areas[1].name.to_uppercase());
        assert!(form.shows_problem());
        assert_eq!(form.problem(), Some(NameError::Taken));
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );

        let mut editing = AreaForm::for_edit(inventory.clone(), elm.id, &elm.areas[1]);
        editing.name = TextField::new(elm.areas[1].name.to_uppercase());
        assert_eq!(editing.problem(), None);

        let mut elsewhere = AreaForm::for_add(inventory.clone(), storage.id);
        elsewhere.name = TextField::new(elm.areas[1].name.to_uppercase());
        assert_eq!(elsewhere.problem(), None);
        assert_eq!(
            handle_key(&mut elsewhere, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_location_asks_for_the_name_only_when_it_holds_something() {
        let mut empty = RemoveLocationForm::new("Shed", false);
        assert_eq!(
            handle_key(&mut empty, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(empty.typed.text(), "");
        assert_eq!(
            handle_key(&mut empty, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        let mut full = RemoveLocationForm::new("Elm St", true);
        assert_eq!(
            handle_key(&mut full, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut full, "Elm S");
        handle_key(&mut full, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut full, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut full, "t");
        assert_eq!(
            handle_key(&mut full, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_area_needs_a_destination_only_when_it_holds_items() {
        let names = vec!["Kitchen".to_string(), "Garage".to_string()];
        let mut empty = RemoveAreaForm::new(names.clone(), None, false);
        assert_eq!(
            handle_key(&mut empty, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        let mut full = RemoveAreaForm::new(names, None, true);
        assert_eq!(
            handle_key(&mut full, DialogKey::Enter),
            DialogOutcome::Handled
        );
        // Space opens the list; the first Esc closes it and keeps the dialog.
        handle_key(&mut full, DialogKey::Char(' '));
        assert!(full.select.is_open());
        assert!(full.close_open_select());
        assert!(!full.close_open_select());
        handle_key(&mut full, DialogKey::Down);
        assert_eq!(full.select.value(), Some("Kitchen"));
        assert_eq!(
            handle_key(&mut full, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn the_area_blocked_notice_confirms_with_enter_and_swallows_tab() {
        let mut dialog = InventoryDialog::AreaBlocked(AreaId::seed(1));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    use crate::chrome::dialog_host::{DialogKey, DialogOutcome, OpenDialog, handle_key};

    fn location_context() -> LocationContext {
        LocationContext {
            inventory: Inventory::default(),
            unit_choices: vec![
                ("AUD".to_string(), "aud".to_string()),
                ("NZD".to_string(), "nzd".to_string()),
            ],
        }
    }

    fn add_location() -> OpenDialog {
        OpenDialog::Inventory(InventoryDialog::Add(LocationForm::for_add(
            Some("AUD".to_string()),
            location_context(),
        )))
    }

    fn location_form(dialog: &OpenDialog) -> &LocationForm {
        match dialog {
            OpenDialog::Inventory(InventoryDialog::Add(form) | InventoryDialog::Edit(_, form)) => {
                form
            }
            other => panic!("expected a location form, got {other:?}"),
        }
    }

    #[test]
    fn location_typing_tab_and_enter_only_when_valid() {
        let mut dialog = add_location();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Beach housex");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(location_form(&dialog).name.text(), "Beach house");
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(location_form(&dialog).focused, LocationField::Unit);
        type_text(&mut dialog, "x");
        assert_eq!(location_form(&dialog).name.text(), "Beach house");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(location_form(&dialog).focused, LocationField::Name);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn location_unit_select_first_esc_closes_it_and_keeps_the_dialog() {
        let mut dialog = add_location();
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(location_form(&dialog).focused, LocationField::Unit);
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(location_form(&dialog).unit.is_open());
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        assert_eq!(location_form(&dialog).unit.value(), Some("AUD"));
    }

    #[test]
    fn remove_location_with_holdings_confirms_only_on_the_typed_name() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::Remove(
            LocationId::seed(1),
            RemoveLocationForm::new("Elm St", true),
        ));
        type_text(&mut dialog, "Elm");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, " St");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_area_select_first_esc_closes_it_and_enter_needs_a_destination() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::RemoveArea(
            AreaId::seed(1),
            RemoveAreaForm::new(vec!["Kitchen".to_string()], None, true),
        ));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn the_area_blocked_notice_confirms_with_enter_and_swallows_tab_in_the_host() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::AreaBlocked(AreaId::seed(1)));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }
}
