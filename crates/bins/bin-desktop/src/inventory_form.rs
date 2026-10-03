//! The Add and Edit **Property** dialog's draft (#494): the typed text of every field, the focus
//! between them and the checks that turn it into an [`inventory::PropertyDraft`]. `gpui`-free, so
//! each rule is unit-tested without a window; the model's own rules (unique name, the cover's
//! amounts) stay in `inventory` and this module only words them per field.
//!
//! An empty Insurer means no cover, and emptying it clears the other four cover fields.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{DateStyle, Money};

use crate::{
    inventory::{
        self, CoverDraft, CoverError, Inventory, NameError, Property, PropertyDraft, PropertyError,
    },
    select::SelectState,
    transaction_filter_form::parse_date,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyForm {
    pub name: String,
    pub address: String,
    pub unit: SelectState,
    pub insurer: String,
    pub policy_no: String,
    pub renews_on: String,
    pub sum_insured: String,
    pub item_limit: String,
    pub focused: PropertyField,
    /// Whether this is Add, which is when the Unit is chosen.
    pub adding: bool,
    /// The fields that have been typed in, which is when their errors start to show.
    touched: Vec<PropertyField>,
}

/// An amount as the dialog shows it back for editing: no trailing zeros.
fn amount_text(amount: &Money) -> String {
    amount.0.normalized().to_string()
}

impl PropertyForm {
    /// Add: `unit` is the option label to pre-select (see `Shell::fiat_unit_options`).
    pub fn for_add(unit: Option<String>) -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            unit: SelectState::new(unit),
            insurer: String::new(),
            policy_no: String::new(),
            renews_on: String::new(),
            sum_insured: String::new(),
            item_limit: String::new(),
            focused: PropertyField::Name,
            adding: true,
            touched: Vec::new(),
        }
    }

    pub fn from_property(property: &Property) -> Self {
        let cover = property.cover.as_ref();
        Self {
            name: property.name.clone(),
            address: property.address.clone(),
            unit: SelectState::new(Some(property.unit.clone())),
            insurer: cover.map(|c| c.insurer.clone()).unwrap_or_default(),
            policy_no: cover.and_then(|c| c.policy_no.clone()).unwrap_or_default(),
            renews_on: cover
                .and_then(|c| c.renews_on)
                // ISO, which `parse_date` always takes back; the Locale's short form has a
                // two-digit year it refuses.
                .map(|date| date.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            sum_insured: cover
                .map(|c| amount_text(&c.sum_insured))
                .unwrap_or_default(),
            item_limit: cover
                .and_then(|c| c.item_limit.as_ref())
                .map(amount_text)
                .unwrap_or_default(),
            focused: PropertyField::Name,
            adding: false,
            touched: Vec::new(),
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

    fn text_mut(&mut self, field: PropertyField) -> Option<&mut String> {
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
            PropertyField::Name => &self.name,
            PropertyField::Address => &self.address,
            PropertyField::Insurer => &self.insurer,
            PropertyField::PolicyNo => &self.policy_no,
            PropertyField::RenewsOn => &self.renews_on,
            PropertyField::SumInsured => &self.sum_insured,
            PropertyField::ItemLimit => &self.item_limit,
            PropertyField::Unit => "",
        }
    }

    fn edited(&mut self, field: PropertyField) {
        if !self.touched.contains(&field) {
            self.touched.push(field);
        }
        // No insurer, no cover: the other four fields go with it.
        if field == PropertyField::Insurer && self.insurer.trim().is_empty() {
            self.insurer.clear();
            self.policy_no.clear();
            self.renews_on.clear();
            self.sum_insured.clear();
            self.item_limit.clear();
        }
    }

    pub fn push_char(&mut self, ch: char) {
        let field = self.focused;
        if ch.is_control() {
            return;
        }
        if let Some(text) = self.text_mut(field) {
            text.push(ch);
            self.edited(field);
        }
    }

    pub fn backspace(&mut self) {
        let field = self.focused;
        if let Some(text) = self.text_mut(field) {
            text.pop();
            self.edited(field);
        }
    }

    /// Fills the Insurer from a suggestion.
    pub fn set_insurer(&mut self, insurer: &str) {
        self.insurer = insurer.to_string();
        self.edited(PropertyField::Insurer);
    }

    /// Whether the cover section has an insurer, and so a cover to check.
    pub fn has_cover(&self) -> bool {
        !self.insurer.trim().is_empty()
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

    /// The draft the model checks, or the first thing wrong with what was typed. `unit` is the
    /// Unit code Add will use (`None` while there is no fiat Unit to pick).
    pub fn draft(
        &self,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Result<PropertyDraft, Problem> {
        let cover = if self.has_cover() {
            let renews_on =
                parse_date(&self.renews_on, today, date_style).map_err(Problem::RenewsOn)?;
            CoverDraft {
                insurer: self.insurer.clone(),
                policy_no: self.policy_no.clone(),
                renews_on,
                sum_insured: self.amount(PropertyField::SumInsured)?,
                item_limit: self.amount(PropertyField::ItemLimit)?,
            }
        } else {
            CoverDraft::default()
        };
        Ok(PropertyDraft {
            name: self.name.clone(),
            address: self.address.clone(),
            cover,
        })
    }

    /// The first problem with the draft, in screen order. `own_id` is the Property being edited.
    pub fn problem(
        &self,
        inventory: &Inventory,
        own_id: Option<u32>,
        unit: Option<&str>,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Option<Problem> {
        let draft = match self.draft(today, date_style) {
            Ok(draft) => draft,
            Err(problem) => {
                // A name problem comes first on screen, so it wins over a later field's.
                return self
                    .name_problem(inventory, own_id)
                    .or(self.unit_problem(unit))
                    .or(Some(problem));
            }
        };
        let mut scratch = inventory.clone();
        let result = match own_id {
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

    fn name_problem(&self, inventory: &Inventory, own_id: Option<u32>) -> Option<Problem> {
        let name = self.name.trim();
        if name.is_empty() {
            return Some(Problem::NameEmpty);
        }
        inventory
            .properties
            .iter()
            .any(|p| Some(p.id) != own_id && p.name.to_lowercase() == name.to_lowercase())
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

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()
    }

    fn seed() -> Inventory {
        inventory::default_inventory(today())
    }

    fn type_into(form: &mut PropertyForm, field: PropertyField, text: &str) {
        form.focus(field);
        for ch in text.chars() {
            form.push_char(ch);
        }
    }

    fn problem(form: &PropertyForm, own: Option<u32>) -> Option<Problem> {
        form.problem(&seed(), own, Some("aud"), today(), None)
    }

    #[test]
    fn a_fresh_add_has_no_name_and_shows_no_error_yet() {
        let form = PropertyForm::for_add(None);
        let found = problem(&form, None);
        assert_eq!(found, Some(Problem::NameEmpty));
        assert!(!form.shows(&found.unwrap()));
    }

    #[test]
    fn a_name_alone_is_valid_with_no_cover() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        assert_eq!(problem(&form, None), None);
    }

    #[test]
    fn a_taken_name_is_refused_ignoring_case_but_not_for_the_property_itself() {
        let inventory = seed();
        let existing = inventory.properties[0].clone();
        let mut form = PropertyForm::for_add(None);
        type_into(
            &mut form,
            PropertyField::Name,
            &existing.name.to_uppercase(),
        );
        assert_eq!(problem(&form, None), Some(Problem::NameTaken));
        assert_eq!(problem(&form, Some(existing.id)), None);
    }

    #[test]
    fn an_insurer_needs_a_sum_insured() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        assert_eq!(problem(&form, None), Some(Problem::SumMissing));
        assert!(form.shows(&Problem::SumMissing));
        type_into(&mut form, PropertyField::SumInsured, "250,000");
        assert_eq!(problem(&form, None), None);
    }

    #[test]
    fn amounts_must_be_numbers_and_positive() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "abc");
        assert_eq!(
            problem(&form, None),
            Some(Problem::Amount(PropertyField::SumInsured))
        );
        form.sum_insured = "0".into();
        assert_eq!(problem(&form, None), Some(Problem::SumNotPositive));
    }

    #[test]
    fn a_comma_is_only_a_thousands_separator() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1,5");
        assert_eq!(
            problem(&form, None),
            Some(Problem::Amount(PropertyField::SumInsured))
        );
        form.sum_insured = "1,500.50".into();
        assert_eq!(problem(&form, None), None);
    }

    #[test]
    fn the_item_limit_is_positive_and_no_more_than_the_sum() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        type_into(&mut form, PropertyField::ItemLimit, "0");
        assert_eq!(problem(&form, None), Some(Problem::LimitNotPositive));
        form.item_limit = "1000.01".into();
        assert_eq!(problem(&form, None), Some(Problem::LimitAboveSum));
        form.item_limit = "1000".into();
        assert_eq!(problem(&form, None), None);
    }

    #[test]
    fn a_bad_renewal_date_is_refused_only_with_cover() {
        crate::locale::init_for_tests();
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Name, "Beach house");
        form.renews_on = "not a date".into();
        assert_eq!(
            problem(&form, None),
            None,
            "no insurer, so no cover to check"
        );
        type_into(&mut form, PropertyField::Insurer, "NRMA");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        assert!(matches!(problem(&form, None), Some(Problem::RenewsOn(_))));
    }

    #[test]
    fn emptying_the_insurer_clears_the_other_cover_fields() {
        let mut form = PropertyForm::for_add(None);
        type_into(&mut form, PropertyField::Insurer, "AA");
        type_into(&mut form, PropertyField::PolicyNo, "P1");
        type_into(&mut form, PropertyField::SumInsured, "1000");
        form.focus(PropertyField::Insurer);
        form.backspace();
        assert!(form.has_cover());
        assert_eq!(form.policy_no, "P1");
        form.backspace();
        assert!(!form.has_cover());
        assert_eq!(form.policy_no, "");
        assert_eq!(form.sum_insured, "");
        let draft = form.draft(today(), None).unwrap();
        assert_eq!(draft.cover, CoverDraft::default());
    }

    #[test]
    fn tab_skips_the_unit_on_edit_and_wraps() {
        let inventory = seed();
        let mut edit = PropertyForm::from_property(&inventory.properties[0]);
        assert!(!edit.fields().contains(&PropertyField::Unit));
        edit.focus(PropertyField::Unit);
        assert_eq!(
            edit.focused,
            PropertyField::Name,
            "a disabled field takes no focus"
        );
        edit.cycle_focus(true);
        assert_eq!(edit.focused, PropertyField::ItemLimit);
        edit.cycle_focus(false);
        assert_eq!(edit.focused, PropertyField::Name);

        let mut add = PropertyForm::for_add(None);
        add.cycle_focus(false);
        add.cycle_focus(false);
        assert_eq!(add.focused, PropertyField::Unit);
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
        let form = PropertyForm::from_property(covered);
        let cover = covered.cover.as_ref().unwrap();
        assert_eq!(form.insurer, cover.insurer);
        let draft = form.draft(today(), None).unwrap();
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
}
