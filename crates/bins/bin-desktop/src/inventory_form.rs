//! The **Property** (#494) and **Room** (#495) dialogs' drafts and the Inventory Dialog enum: the
//! typed text of every field, the focus between them and the checks that turn it into an
//! [`inventory::PropertyDraft`]. `gpui`-free, so each rule is unit-tested without a window; the
//! model's own rules (unique name, the cover's amounts) stay in `inventory` and this module only
//! words them per field.
//!
//! Everything a check needs from `Shell` (the Inventory, today, the date style, the fiat Units) is
//! copied in when the Dialog opens, so no form reaches back into it.
//!
//! An empty Insurer means no cover, and emptying it clears the other four cover fields.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{DateStyle, Money};

use crate::{
    dialog_host::{Dialog, DialogKey, DialogOutcome},
    field::TextField,
    inventory::{
        self, CoverDraft, CoverError, Inventory, NameError, Property, PropertyDraft, PropertyError,
    },
    select::SelectState,
    transaction_filter_form::parse_date,
};

/// The dialog open over Settings › Inventory: Property and Room dialogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryDialog {
    Add(PropertyForm),
    Edit(u32, PropertyForm),
    /// The Property and the name typed so far, which only a Property holding something asks for.
    Remove(u32, RemovePropertyForm),
    /// Add room to the Property.
    AddRoom(u32, RoomForm),
    EditRoom(u32, RoomForm),
    /// The Room, and the chosen destination for its Items (unused while it is empty).
    RemoveRoom(u32, RemoveRoomForm),
    /// The Room is its Property's last and holds Items.
    RoomBlocked(u32),
}

impl InventoryDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddRoom(_, form) | Self::EditRoom(_, form) => Some(form),
            Self::RemoveRoom(_, form) => Some(form),
            Self::RoomBlocked(_) => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddRoom(_, form) | Self::EditRoom(_, form) => Some(form),
            Self::RemoveRoom(_, form) => Some(form),
            Self::RoomBlocked(_) => None,
        }
    }
}

impl Dialog for InventoryDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(inner) = self.inner_mut() {
            inner.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyField {
    Name,
    Address,
    /// Add only: a Property's Unit is fixed when it is created.
    Unit,
    Insurer,
    PolicyNo,
    RenewsOn,
    SumInsured,
    ItemLimit,
}

/// What is wrong with the draft, one field at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    NameEmpty,
    NameTaken,
    /// The text of the Message `parse_date` built.
    RenewsOn(String),
    Amount(PropertyField),
    SumMissing,
    SumNotPositive,
    LimitNotPositive,
    LimitAboveSum,
    /// Add with no fiat Unit to pick.
    UnitMissing,
}

impl Problem {
    pub fn field(&self) -> PropertyField {
        match self {
            Self::NameEmpty | Self::NameTaken => PropertyField::Name,
            Self::RenewsOn(_) => PropertyField::RenewsOn,
            Self::Amount(field) => *field,
            Self::SumMissing | Self::SumNotPositive => PropertyField::SumInsured,
            Self::LimitNotPositive | Self::LimitAboveSum => PropertyField::ItemLimit,
            Self::UnitMissing => PropertyField::Unit,
        }
    }
}

/// What the Property form checks against, copied from `Shell` when the Dialog opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyContext {
    pub inventory: Inventory,
    pub today: NaiveDate,
    pub date_style: Option<DateStyle>,
    /// The fiat Units a Property can be kept in: each one's option label, then its code.
    pub unit_choices: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyForm {
    pub name: TextField,
    pub address: TextField,
    pub unit: SelectState,
    pub insurer: TextField,
    pub policy_no: TextField,
    pub renews_on: TextField,
    pub sum_insured: TextField,
    pub item_limit: TextField,
    pub focused: PropertyField,
    /// Whether this is Add, which is when the Unit is chosen.
    pub adding: bool,
    /// The fields that have been typed in, which is when their errors start to show.
    touched: Vec<PropertyField>,
    context: PropertyContext,
    /// The Property being edited; `None` on Add.
    own_id: Option<u32>,
}

/// An amount as the dialog shows it back for editing: no trailing zeros.
fn amount_text(amount: &Money) -> String {
    amount.0.normalized().to_string()
}

impl PropertyForm {
    /// Add: `unit` is the option label to pre-select.
    pub fn for_add(unit: Option<String>, context: PropertyContext) -> Self {
        Self {
            name: TextField::default(),
            address: TextField::default(),
            unit: SelectState::new(unit),
            insurer: TextField::default(),
            policy_no: TextField::default(),
            renews_on: TextField::default(),
            sum_insured: TextField::default(),
            item_limit: TextField::default(),
            focused: PropertyField::Name,
            adding: true,
            touched: Vec::new(),
            context,
            own_id: None,
        }
    }

    pub fn from_property(property: &Property, context: PropertyContext) -> Self {
        let cover = property.cover.as_ref();
        Self {
            name: TextField::new(property.name.clone()),
            address: TextField::new(property.address.clone()),
            unit: SelectState::new(Some(property.unit.clone())),
            insurer: TextField::new(cover.map(|c| c.insurer.clone()).unwrap_or_default()),
            policy_no: TextField::new(cover.and_then(|c| c.policy_no.clone()).unwrap_or_default()),
            renews_on: TextField::new(
                cover
                    .and_then(|c| c.renews_on)
                    // ISO, which `parse_date` always takes back; the Locale's short form has a
                    // two-digit year it refuses.
                    .map(|date| date.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            ),
            sum_insured: TextField::new(
                cover
                    .map(|c| amount_text(&c.sum_insured))
                    .unwrap_or_default(),
            ),
            item_limit: TextField::new(
                cover
                    .and_then(|c| c.item_limit.as_ref())
                    .map(amount_text)
                    .unwrap_or_default(),
            ),
            focused: PropertyField::Name,
            adding: false,
            touched: Vec::new(),
            context,
            own_id: Some(property.id),
        }
    }

    /// The fields `tab` walks, in screen order.
    pub fn fields(&self) -> Vec<PropertyField> {
        let mut fields = vec![PropertyField::Name, PropertyField::Address];
        if self.adding {
            fields.push(PropertyField::Unit);
        }
        fields.extend([
            PropertyField::Insurer,
            PropertyField::PolicyNo,
            PropertyField::RenewsOn,
            PropertyField::SumInsured,
            PropertyField::ItemLimit,
        ]);
        fields
    }

    pub fn focus(&mut self, field: PropertyField) {
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

    fn field_mut(&mut self, field: PropertyField) -> Option<&mut TextField> {
        match field {
            PropertyField::Name => Some(&mut self.name),
            PropertyField::Address => Some(&mut self.address),
            PropertyField::Insurer => Some(&mut self.insurer),
            PropertyField::PolicyNo => Some(&mut self.policy_no),
            PropertyField::RenewsOn => Some(&mut self.renews_on),
            PropertyField::SumInsured => Some(&mut self.sum_insured),
            PropertyField::ItemLimit => Some(&mut self.item_limit),
            PropertyField::Unit => None,
        }
    }

    pub fn text(&self, field: PropertyField) -> &str {
        match field {
            PropertyField::Name => self.name.text(),
            PropertyField::Address => self.address.text(),
            PropertyField::Insurer => self.insurer.text(),
            PropertyField::PolicyNo => self.policy_no.text(),
            PropertyField::RenewsOn => self.renews_on.text(),
            PropertyField::SumInsured => self.sum_insured.text(),
            PropertyField::ItemLimit => self.item_limit.text(),
            PropertyField::Unit => "",
        }
    }

    fn mark_touched(&mut self, field: PropertyField) {
        if !self.touched.contains(&field) {
            self.touched.push(field);
        }
    }

    /// Fills the Insurer from a suggestion.
    pub fn set_insurer(&mut self, insurer: &str) {
        self.insurer = TextField::new(insurer);
        self.mark_touched(PropertyField::Insurer);
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
        self.focus(PropertyField::Unit);
        if self.unit.is_open() {
            self.unit.cancel();
        } else {
            self.unit.open(&self.unit_labels());
        }
    }

    pub fn choose_unit(&mut self, index: usize) {
        self.unit.choose(&self.unit_labels(), index);
    }

    /// Whether the cover section has an insurer, and so a cover to check.
    pub fn has_cover(&self) -> bool {
        !self.insurer.is_blank()
    }

    fn amount(&self, field: PropertyField) -> Result<Option<Money>, Problem> {
        let text: String = self
            .text(field)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if text.is_empty() {
            return Ok(None);
        }
        // A comma is a thousands separator only between groups of three digits: "1,5" is
        // refused rather than read as 15.
        let whole = text.split('.').next().unwrap_or_default();
        let mut groups = whole.split(',');
        let first_ok = groups.next().is_some_and(|g| !g.is_empty());
        if !first_ok || groups.any(|g| g.len() != 3) {
            return Err(Problem::Amount(field));
        }
        let text = text.replace(',', "");
        text.parse::<BigDecimal>()
            .map(|amount| Some(Money(amount.round(2))))
            .map_err(|_| Problem::Amount(field))
    }

    /// The draft the model checks, or the first thing wrong with what was typed.
    pub fn draft(&self) -> Result<PropertyDraft, Problem> {
        let cover = if self.has_cover() {
            let renews_on = parse_date(
                self.renews_on.text(),
                self.context.today,
                self.context.date_style,
            )
            .map_err(Problem::RenewsOn)?;
            CoverDraft {
                insurer: self.insurer.text().to_string(),
                policy_no: self.policy_no.text().to_string(),
                renews_on,
                sum_insured: self.amount(PropertyField::SumInsured)?,
                item_limit: self.amount(PropertyField::ItemLimit)?,
            }
        } else {
            CoverDraft::default()
        };
        Ok(PropertyDraft {
            name: self.name.text().to_string(),
            address: self.address.text().to_string(),
            cover,
        })
    }

    /// The first problem with the draft, in screen order.
    pub fn problem(&self) -> Option<Problem> {
        let unit = self.unit_code();
        let inventory = &self.context.inventory;
        let draft = match self.draft() {
            Ok(draft) => draft,
            Err(problem) => {
                // A name problem comes first on screen, so it wins over a later field's.
                return self
                    .name_problem()
                    .or(self.unit_problem(unit))
                    .or(Some(problem));
            }
        };
        let mut scratch = inventory.clone();
        let result = match self.own_id {
            Some(id) => inventory::edit_property(&mut scratch, id, &draft),
            None => inventory::add_property(&mut scratch, &draft, unit.unwrap_or("-")).map(|_| ()),
        };
        match result {
            Ok(()) => self.unit_problem(unit),
            Err(PropertyError::Name(NameError::Empty)) => Some(Problem::NameEmpty),
            Err(PropertyError::Name(NameError::Taken)) => Some(Problem::NameTaken),
            Err(PropertyError::Cover(CoverError::SumMissing)) => Some(Problem::SumMissing),
            Err(PropertyError::Cover(CoverError::SumNotPositive)) => Some(Problem::SumNotPositive),
            Err(PropertyError::Cover(CoverError::LimitNotPositive)) => {
                Some(Problem::LimitNotPositive)
            }
            Err(PropertyError::Cover(CoverError::LimitAboveSum)) => Some(Problem::LimitAboveSum),
            Err(PropertyError::UnitMissing) => Some(Problem::UnitMissing),
            Err(PropertyError::Unknown) => None,
        }
    }

    fn name_problem(&self) -> Option<Problem> {
        let name = self.name.text().trim();
        if name.is_empty() {
            return Some(Problem::NameEmpty);
        }
        self.context
            .inventory
            .properties
            .iter()
            .any(|p| Some(p.id) != self.own_id && p.name.to_lowercase() == name.to_lowercase())
            .then_some(Problem::NameTaken)
    }

    fn unit_problem(&self, unit: Option<&str>) -> Option<Problem> {
        (self.adding && unit.is_none()).then_some(Problem::UnitMissing)
    }

    /// Whether `problem` has been earned yet: its field was typed in, or (for a missing sum) the
    /// Insurer was. A fresh Add dialog opens with its confirm disabled and no error showing.
    pub fn shows(&self, problem: &Problem) -> bool {
        self.touched.contains(&problem.field())
            || (*problem == Problem::SumMissing && self.touched.contains(&PropertyField::Insurer))
    }

    /// Marks the focused field typed in and, when the key would empty the Insurer, clears the
    /// other four cover fields with it. Typing itself falls through to the shared handling.
    fn note_edit(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let field = self.focused;
        self.mark_touched(field);
        if field != PropertyField::Insurer {
            return None;
        }
        let mut next = self.insurer.clone();
        match key {
            DialogKey::Char(ch) => next.push(ch),
            DialogKey::Backspace => next.backspace(),
            _ => {}
        }
        if !next.is_blank() {
            return None;
        }
        // No insurer, no cover: the other four fields go with it.
        self.insurer = TextField::default();
        self.policy_no = TextField::default();
        self.renews_on = TextField::default();
        self.sum_insured = TextField::default();
        self.item_limit = TextField::default();
        Some(DialogOutcome::Handled)
    }
}

impl Dialog for PropertyForm {
    /// `Up`/`Down` walk the Unit list or step its value, `Space` opens it or commits the
    /// highlight and `Enter` commits an open list, all while the Unit is focused (which a
    /// control swallows typing on); `Shift-Tab` goes back a field.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_unit = self.focused == PropertyField::Unit;
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
            DialogKey::Char(_) | DialogKey::Backspace => return self.note_edit(key),
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        let field = self.focused;
        self.field_mut(field)
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

/// The Remove **Property** dialog's draft. Whether the name must be typed is copied in at open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovePropertyForm {
    name: String,
    pub typed: TextField,
    /// Only a Property holding Rooms or Items asks for its name back.
    needs_typed_name: bool,
}

impl RemovePropertyForm {
    pub fn new(name: impl Into<String>, needs_typed_name: bool) -> Self {
        Self {
            name: name.into(),
            typed: TextField::default(),
            needs_typed_name,
        }
    }
}

impl Dialog for RemovePropertyForm {
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

/// The Add and Edit **Room** dialog's draft (#495): a name and whether it has been typed in, which
/// is when its error starts to show. The rules stay in `inventory`; this only words them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomForm {
    pub name: TextField,
    touched: bool,
    inventory: Inventory,
    property: u32,
    /// The Room being edited; `None` on Add.
    own_id: Option<u32>,
}

impl RoomForm {
    pub fn for_add(inventory: Inventory, property: u32) -> Self {
        Self {
            name: TextField::default(),
            touched: false,
            inventory,
            property,
            own_id: None,
        }
    }

    pub fn for_edit(inventory: Inventory, property: u32, room: &inventory::Room) -> Self {
        Self {
            name: TextField::new(room.name.clone()),
            touched: false,
            inventory,
            property,
            own_id: Some(room.id),
        }
    }

    /// What is wrong with the name, if anything. The name is unique within the Property only, so
    /// the same name in another Property is fine.
    pub fn problem(&self) -> Option<NameError> {
        let mut scratch = self.inventory.clone();
        let name = self.name.text();
        let result = match self.own_id {
            Some(id) => inventory::edit_room(&mut scratch, id, name),
            None => inventory::add_room(&mut scratch, self.property, name).map(|_| ()),
        };
        match result {
            Err(inventory::RoomError::Name(error)) => Some(error),
            _ => None,
        }
    }

    pub fn shows_problem(&self) -> bool {
        self.touched
    }
}

impl Dialog for RoomForm {
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

/// The Remove **Room** dialog's destination select. The other Rooms' names and whether Items need
/// a home are copied in at open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveRoomForm {
    /// The Property's other Rooms, in tab order: where a removed Room's Items can go.
    pub destinations: Vec<String>,
    pub select: SelectState,
    needs_destination: bool,
}

impl RemoveRoomForm {
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

impl Dialog for RemoveRoomForm {
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

/// The insurers already on a Property, A-Z, each once ignoring case: the Insurer field's
/// suggestions.
pub fn insurers_in_use(inventory: &Inventory) -> Vec<String> {
    let mut insurers: Vec<String> = Vec::new();
    for cover in inventory.properties.iter().filter_map(|p| p.cover.as_ref()) {
        if !insurers
            .iter()
            .any(|i| i.to_lowercase() == cover.insurer.to_lowercase())
        {
            insurers.push(cover.insurer.clone());
        }
    }
    insurers.sort_by_key(|i| i.to_lowercase());
    insurers
}

/// The suggestions for what is typed: insurers in use that contain it, ignoring case, bar an exact
/// match.
pub fn suggestions(inventory: &Inventory, typed: &str) -> Vec<String> {
    let typed = typed.trim().to_lowercase();
    insurers_in_use(inventory)
        .into_iter()
        .filter(|i| {
            let lower = i.to_lowercase();
            lower != typed && lower.contains(&typed)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog_host::handle_key;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()
    }

    fn seed() -> Inventory {
        inventory::default_inventory(today())
    }

    fn context() -> PropertyContext {
        PropertyContext {
            inventory: seed(),
            today: today(),
            date_style: None,
            unit_choices: vec![
                ("AUD".to_string(), "aud".to_string()),
                ("NZD".to_string(), "nzd".to_string()),
            ],
        }
    }

    /// An Add form with the AUD Unit picked, as `Shell` opens it.
    fn add_form() -> PropertyForm {
        PropertyForm::for_add(Some("AUD".to_string()), context())
    }

    fn edit_form(property: &Property) -> PropertyForm {
        PropertyForm::from_property(property, context())
    }

    fn type_text(dialog: &mut impl Dialog, text: &str) {
        for ch in text.chars() {
            handle_key(dialog, DialogKey::Char(ch));
        }
    }

    fn type_into(form: &mut PropertyForm, field: PropertyField, text: &str) {
        form.focus(field);
        type_text(form, text);
    }

    fn problem(form: &PropertyForm) -> Option<Problem> {
        form.problem()
    }

    #[test]
    fn a_fresh_add_has_no_name_and_shows_no_error_yet() {
        let form = add_form();
        let found = problem(&form);
        assert_eq!(found, Some(Problem::NameEmpty));
        assert!(!form.shows(&found.unwrap()));
        assert!(!form.is_valid());
    }

    #[test]
    fn a_name_alone_is_valid_with_no_cover() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        assert_eq!(problem(&form), None);
        assert!(form.is_valid());
    }

    #[test]
    fn an_add_with_no_fiat_unit_is_refused() {
        let mut form = PropertyForm::for_add(None, context());
        type_into(&mut form, PropertyField::Name, "Beach house");
        assert_eq!(problem(&form), Some(Problem::UnitMissing));
    }

    #[test]
    fn a_taken_name_is_refused_ignoring_case_but_not_for_the_property_itself() {
        let inventory = seed();
        let existing = inventory.properties[0].clone();
        let mut form = add_form();
        type_into(
            &mut form,
            PropertyField::Name,
            &existing.name.to_uppercase(),
        );
        assert_eq!(problem(&form), Some(Problem::NameTaken));
        let mut editing = edit_form(&existing);
        editing.name = TextField::new(existing.name.to_uppercase());
        assert_eq!(problem(&editing), None);
    }

    #[test]
    fn an_insurer_needs_a_sum_insured() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        assert_eq!(problem(&form), Some(Problem::SumMissing));
        assert!(form.shows(&Problem::SumMissing));
        type_into(&mut form, PropertyField::SumInsured, "250,000");
        assert_eq!(problem(&form), None);
    }

    #[test]
    fn amounts_must_be_numbers_and_positive() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "abc");
        assert_eq!(
            problem(&form),
            Some(Problem::Amount(PropertyField::SumInsured))
        );
        form.sum_insured = TextField::new("0");
        assert_eq!(problem(&form), Some(Problem::SumNotPositive));
    }

    #[test]
    fn a_comma_is_only_a_thousands_separator() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1,5");
        assert_eq!(
            problem(&form),
            Some(Problem::Amount(PropertyField::SumInsured))
        );
        form.sum_insured = TextField::new("1,500.50");
        assert_eq!(problem(&form), None);
    }

    #[test]
    fn the_item_limit_is_positive_and_no_more_than_the_sum() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        type_into(&mut form, PropertyField::ItemLimit, "0");
        assert_eq!(problem(&form), Some(Problem::LimitNotPositive));
        form.item_limit = TextField::new("1000.01");
        assert_eq!(problem(&form), Some(Problem::LimitAboveSum));
        form.item_limit = TextField::new("1000");
        assert_eq!(problem(&form), None);
    }

    #[test]
    fn a_bad_renewal_date_is_refused_only_with_cover() {
        crate::locale::init_for_tests();
        let mut form = add_form();
        type_into(&mut form, PropertyField::Name, "Beach house");
        form.renews_on = TextField::new("not a date");
        assert_eq!(problem(&form), None, "no insurer, so no cover to check");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        assert!(matches!(problem(&form), Some(Problem::RenewsOn(_))));
    }

    #[test]
    fn emptying_the_insurer_clears_the_other_cover_fields() {
        let mut form = add_form();
        type_into(&mut form, PropertyField::Insurer, "AA");
        type_into(&mut form, PropertyField::PolicyNo, "P1");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        form.focus(PropertyField::Insurer);
        handle_key(&mut form, DialogKey::Backspace);
        assert!(form.has_cover());
        assert_eq!(form.policy_no.text(), "P1");
        handle_key(&mut form, DialogKey::Backspace);
        assert!(!form.has_cover());
        assert_eq!(form.policy_no.text(), "");
        assert_eq!(form.sum_insured.text(), "");
        let draft = form.draft().unwrap();
        assert_eq!(draft.cover, CoverDraft::default());
    }

    #[test]
    fn tab_skips_the_unit_on_edit_and_wraps() {
        let inventory = seed();
        let mut edit = edit_form(&inventory.properties[0]);
        assert!(!edit.fields().contains(&PropertyField::Unit));
        edit.focus(PropertyField::Unit);
        assert_eq!(
            edit.focused,
            PropertyField::Name,
            "a disabled field takes no focus"
        );
        handle_key(&mut edit, DialogKey::BackTab);
        assert_eq!(edit.focused, PropertyField::ItemLimit);
        handle_key(&mut edit, DialogKey::Tab);
        assert_eq!(edit.focused, PropertyField::Name);

        let mut add = add_form();
        handle_key(&mut add, DialogKey::Tab);
        handle_key(&mut add, DialogKey::Tab);
        assert_eq!(add.focused, PropertyField::Unit);
    }

    #[test]
    fn the_unit_select_walks_commits_and_first_esc_closes_it() {
        let mut form = add_form();
        form.focus(PropertyField::Unit);
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
    fn enter_confirms_only_when_the_property_form_is_valid() {
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
    fn edit_starts_from_the_property_as_it_is() {
        crate::locale::init_for_tests();
        let inventory = seed();
        let covered = inventory
            .properties
            .iter()
            .find(|p| p.cover.is_some())
            .expect("the seed has a covered Property");
        let form = edit_form(covered);
        let cover = covered.cover.as_ref().unwrap();
        assert_eq!(form.insurer.text(), cover.insurer);
        let draft = form.draft().unwrap();
        assert_eq!(draft.cover.sum_insured, Some(cover.sum_insured.clone()));
        assert_eq!(draft.cover.renews_on, cover.renews_on);
    }

    #[test]
    fn suggestions_are_the_insurers_in_use_matching_what_is_typed() {
        let inventory = seed();
        let all = insurers_in_use(&inventory);
        assert!(!all.is_empty());
        assert_eq!(suggestions(&inventory, ""), all);
        let first = all[0].clone();
        assert!(
            suggestions(&inventory, &first).is_empty(),
            "an exact match needs no suggestion"
        );
        let part = first[..2].to_lowercase();
        assert!(suggestions(&inventory, &part).contains(&first));
    }

    #[test]
    fn a_room_name_is_unique_within_its_property_only() {
        let inventory = seed();
        let elm = &inventory.properties[0];
        let storage = &inventory.properties[1];
        let mut form = RoomForm::for_add(inventory.clone(), elm.id);
        assert_eq!(form.problem(), Some(NameError::Empty));
        assert!(!form.shows_problem(), "a fresh dialog shows no error yet");
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut form, &elm.rooms[1].name.to_uppercase());
        assert!(form.shows_problem());
        assert_eq!(form.problem(), Some(NameError::Taken));
        assert_eq!(
            handle_key(&mut form, DialogKey::Enter),
            DialogOutcome::Handled
        );

        let mut editing = RoomForm::for_edit(inventory.clone(), elm.id, &elm.rooms[1]);
        editing.name = TextField::new(elm.rooms[1].name.to_uppercase());
        assert_eq!(editing.problem(), None);

        let mut elsewhere = RoomForm::for_add(inventory.clone(), storage.id);
        elsewhere.name = TextField::new(elm.rooms[1].name.to_uppercase());
        assert_eq!(elsewhere.problem(), None);
        assert_eq!(
            handle_key(&mut elsewhere, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_property_asks_for_the_name_only_when_it_holds_something() {
        let mut empty = RemovePropertyForm::new("Shed", false);
        assert_eq!(
            handle_key(&mut empty, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(empty.typed.text(), "");
        assert_eq!(
            handle_key(&mut empty, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        let mut full = RemovePropertyForm::new("Elm St", true);
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
    fn remove_room_needs_a_destination_only_when_it_holds_items() {
        let names = vec!["Kitchen".to_string(), "Garage".to_string()];
        let mut empty = RemoveRoomForm::new(names.clone(), None, false);
        assert_eq!(
            handle_key(&mut empty, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        let mut full = RemoveRoomForm::new(names, None, true);
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
    fn the_room_blocked_notice_confirms_with_enter_and_swallows_tab() {
        let mut dialog = InventoryDialog::RoomBlocked(1);
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
