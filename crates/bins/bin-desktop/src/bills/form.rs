//! The Add and Edit bill plan dialog's live form (`docs/ux/desktop/12-bills/README.md`'s 8c):
//! `gpui`-free and unit-tested, the same split `transactions/filter_form.rs` uses.
//!
//! The behaviour is the Desktop Bills Surface map's decisions (#365, #367):
//!
//! - The fields, in `Tab` order: Name, Category (expense leaves only), Unit (a select on Add,
//!   read-only on Edit since it locks at creation), Account (only those in the chosen Unit that
//!   take Transactions), Payee (optional), Planned amount, Fixed/Estimated, Recurrence, First Due,
//!   Ends On (skipped and hidden for a One-shot), Attention lead, and Active (Edit only).
//! - Changing the Unit keeps the Account when it is in the new Unit, and otherwise moves it to that
//!   Unit's first Account, so the draft never pairs an Account with another Unit.
//! - First Due and Ends On take the same typed dates as the Transactions filter; Ends On and
//!   Attention lead may be left empty. Each field's problem shows inline, and Save is refused
//!   until there are none.
//!
//! The form is a Dialog (`dialog_host`), so it carries what it needs from `Shell` as copied in
//! when it opens: the selects' choices ([`BillPlanSource`]), today and the date style. It
//! rebuilds its Account list itself whenever the Unit may have changed.

use bigdecimal::{BigDecimal, Signed};
use chrono::NaiveDate;
use lib_core::{CategoryTypes, DateStyle, Money};
use lib_locale::format::format_date_input;

use crate::{
    accounts::{Account, SelectKey},
    bills::{AmountKind, BillError, BillPlan, BillPlanDraft, Recurrence},
    categories::{self, Category},
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectState,
    payees::Payee,
    transactions,
    transactions::filter_form::parse_date,
};

/// The dialog's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillPlanField {
    #[default]
    Name,
    Category,
    Unit,
    Account,
    Payee,
    Amount,
    AmountKind,
    Recurrence,
    FirstDue,
    EndsOn,
    AttentionLead,
    Active,
}

impl BillPlanField {
    const ORDER: [BillPlanField; 12] = [
        Self::Name,
        Self::Category,
        Self::Unit,
        Self::Account,
        Self::Payee,
        Self::Amount,
        Self::AmountKind,
        Self::Recurrence,
        Self::FirstDue,
        Self::EndsOn,
        Self::AttentionLead,
        Self::Active,
    ];
}

/// What the selects choose from, copied in when the dialog opens. Each `*_labels` is what its
/// select shows and stores; the matching `*_ids` runs parallel to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillPlanSource {
    category_labels: Vec<String>,
    category_ids: Vec<u32>,
    /// Unit codes: every Unit some Account taking Transactions is kept in.
    units: Vec<String>,
    /// Every Account taking Transactions, as (Unit code, id, name).
    accounts: Vec<(String, u32, String)>,
    /// "none" first, then the active Payees by name.
    payee_labels: Vec<String>,
    payee_ids: Vec<Option<u32>>,
    /// [`Recurrence::ALL`]'s labels, in that order.
    recurrence_labels: Vec<String>,
    today: NaiveDate,
    date_style: Option<DateStyle>,
}

impl BillPlanSource {
    /// `none_label` heads the Payee list, and `recurrence_label` names each Recurrence. A Payee
    /// kept on the Plan being edited is listed even when inactive, so opening Edit never silently
    /// drops it (`keep_payee`).
    #[expect(
        clippy::too_many_arguments,
        reason = "each is one input copied from Shell"
    )]
    pub fn new(
        categories: &[Category],
        accounts: &[Account],
        payees: &[Payee],
        keep_payee: Option<u32>,
        none_label: String,
        recurrence_label: fn(Recurrence) -> String,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        let mut leaves: Vec<&Category> = categories
            .iter()
            .filter(|c| {
                c.category_type == CategoryTypes::Expense && categories::is_leaf(categories, c.id)
            })
            .collect();
        leaves.sort_by(|a, b| a.name.cmp(&b.name));
        let takes: Vec<&Account> = accounts
            .iter()
            .filter(|a| transactions::takes_transactions(&a.account_type))
            .collect();
        let mut units: Vec<String> = takes.iter().map(|a| a.unit.clone()).collect();
        units.sort();
        units.dedup();
        let mut listed: Vec<&Payee> = payees
            .iter()
            .filter(|p| p.is_active || Some(p.id) == keep_payee)
            .collect();
        listed.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            category_labels: leaves.iter().map(|c| c.name.clone()).collect(),
            category_ids: leaves.iter().map(|c| c.id).collect(),
            units,
            accounts: takes
                .iter()
                .map(|a| (a.unit.clone(), a.id, a.name.clone()))
                .collect(),
            payee_labels: std::iter::once(none_label)
                .chain(listed.iter().map(|p| p.name.clone()))
                .collect(),
            payee_ids: std::iter::once(None)
                .chain(listed.iter().map(|p| Some(p.id)))
                .collect(),
            recurrence_labels: Recurrence::ALL
                .iter()
                .map(|r| recurrence_label(*r))
                .collect(),
            today,
            date_style,
        }
    }

    /// The selects' options for `unit`, which scopes the Account list.
    fn options(&self, unit: Option<&str>) -> BillPlanOptions {
        let in_unit: Vec<&(String, u32, String)> = self
            .accounts
            .iter()
            .filter(|(account_unit, ..)| Some(account_unit.as_str()) == unit)
            .collect();
        BillPlanOptions {
            category_labels: self.category_labels.clone(),
            category_ids: self.category_ids.clone(),
            units: self.units.clone(),
            account_labels: in_unit.iter().map(|(_, _, name)| name.clone()).collect(),
            account_ids: in_unit.iter().map(|(_, id, _)| *id).collect(),
            payee_labels: self.payee_labels.clone(),
            payee_ids: self.payee_ids.clone(),
            recurrence_labels: self.recurrence_labels.clone(),
        }
    }
}

/// The selects' options for the form's current Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillPlanOptions {
    pub category_labels: Vec<String>,
    category_ids: Vec<u32>,
    pub units: Vec<String>,
    pub account_labels: Vec<String>,
    account_ids: Vec<u32>,
    pub payee_labels: Vec<String>,
    payee_ids: Vec<Option<u32>>,
    pub recurrence_labels: Vec<String>,
}

impl BillPlanOptions {
    fn label(labels: &[String], index: Option<usize>) -> Option<String> {
        index.and_then(|index| labels.get(index).cloned())
    }

    fn index(labels: &[String], value: Option<&str>) -> Option<usize> {
        labels
            .iter()
            .position(|label| Some(label.as_str()) == value)
    }

    fn category_label(&self, id: u32) -> Option<String> {
        Self::label(
            &self.category_labels,
            self.category_ids.iter().position(|c| *c == id),
        )
    }

    fn account_label(&self, id: u32) -> Option<String> {
        Self::label(
            &self.account_labels,
            self.account_ids.iter().position(|a| *a == id),
        )
    }

    fn payee_label(&self, id: Option<u32>) -> Option<String> {
        Self::label(
            &self.payee_labels,
            self.payee_ids.iter().position(|p| *p == id),
        )
    }

    fn recurrence_label_of(&self, recurrence: Recurrence) -> Option<String> {
        Self::label(
            &self.recurrence_labels,
            Recurrence::ALL.iter().position(|r| *r == recurrence),
        )
    }
}

/// A field's inline problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    /// Not a decimal amount, or not above zero.
    AmountNotPositive,
    /// Not a whole number of days.
    LeadNotWholeDays,
}

/// The form's inline problems. A date's is the parser's own hint, already a Message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormErrors {
    pub amount: Option<FieldError>,
    pub first_due: Option<String>,
    pub ends_on: Option<String>,
    pub ends_before_first_due: bool,
    pub lead: Option<FieldError>,
}

impl FormErrors {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The dialog's live state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillPlanForm {
    pub name: TextField,
    pub category: SelectState,
    pub unit: SelectState,
    pub account: SelectState,
    pub payee: SelectState,
    pub amount: TextField,
    pub amount_kind: AmountKind,
    pub recurrence: SelectState,
    pub first_due: TextField,
    pub ends_on: TextField,
    pub attention_lead: TextField,
    pub is_active: bool,
    /// Edit locks the Unit and shows Active; Add does neither.
    pub is_edit: bool,
    pub focused: BillPlanField,
    /// What the last Save was refused for, shown until the form changes.
    pub error: Option<BillError>,
    source: BillPlanSource,
    /// The selects' options for the current Unit, rebuilt whenever the form changes.
    pub options: BillPlanOptions,
}

impl BillPlanForm {
    /// A fresh Add form: the first Unit and its first Account, Monthly, Fixed, First Due today
    /// (the word `today`), everything else empty.
    pub fn new(source: BillPlanSource) -> Self {
        let unit = source.units.first().cloned();
        let options = source.options(unit.as_deref());
        Self {
            name: TextField::default(),
            category: SelectState::new(options.category_labels.first().cloned()),
            unit: SelectState::new(unit),
            account: SelectState::new(options.account_labels.first().cloned()),
            payee: SelectState::new(options.payee_label(None)),
            amount: TextField::default(),
            amount_kind: AmountKind::Fixed,
            recurrence: SelectState::new(options.recurrence_label_of(Recurrence::Monthly)),
            first_due: TextField::new(lib_locale::msg::date_word_today()),
            ends_on: TextField::default(),
            attention_lead: TextField::default(),
            is_active: true,
            is_edit: false,
            focused: BillPlanField::Name,
            error: None,
            source,
            options,
        }
    }

    /// A form pre-filled from `plan`, for the Edit dialog. `source` must keep the Plan's Payee.
    pub fn from_plan(plan: &BillPlan, source: BillPlanSource) -> Self {
        let options = source.options(Some(&plan.unit));
        let date_style = source.date_style;
        Self {
            name: TextField::new(plan.name.clone()),
            category: SelectState::new(options.category_label(plan.category_id)),
            unit: SelectState::new(Some(plan.unit.clone())),
            account: SelectState::new(options.account_label(plan.account_id)),
            payee: SelectState::new(options.payee_label(plan.payee_id)),
            amount: TextField::new(plan.planned_amount.0.to_string()),
            amount_kind: plan.amount_kind,
            recurrence: SelectState::new(options.recurrence_label_of(plan.recurrence)),
            first_due: TextField::new(format_date_input(plan.first_due, date_style)),
            ends_on: TextField::new(
                plan.ends_on
                    .map(|date| format_date_input(date, date_style))
                    .unwrap_or_default(),
            ),
            attention_lead: TextField::new(
                plan.attention_lead
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
            ),
            is_active: plan.is_active,
            is_edit: true,
            focused: BillPlanField::Name,
            error: None,
            source,
            options,
        }
    }

    pub fn unit_code(&self) -> Option<&str> {
        self.unit.value()
    }

    pub fn recurrence(&self) -> Recurrence {
        BillPlanOptions::index(&self.options.recurrence_labels, self.recurrence.value())
            .and_then(|index| Recurrence::ALL.get(index).copied())
            .unwrap_or(Recurrence::Monthly)
    }

    /// Whether `field` is on the form: Ends On only when recurring, Active only on Edit.
    pub fn shows(&self, field: BillPlanField) -> bool {
        match field {
            BillPlanField::EndsOn => self.recurrence() != Recurrence::OneShot,
            BillPlanField::Active => self.is_edit,
            _ => true,
        }
    }

    /// Clears a stale Save error, rebuilds the options for the Unit now chosen and keeps the
    /// Account in it (a Unit change swaps the Account list).
    fn changed(&mut self) {
        self.error = None;
        self.options = self.source.options(self.unit.value());
        let in_unit = self.account.value().is_some_and(|value| {
            self.options
                .account_labels
                .iter()
                .any(|label| label == value)
        });
        if !in_unit {
            self.account = SelectState::new(self.options.account_labels.first().cloned());
        }
    }

    fn select_mut(&mut self, field: BillPlanField) -> Option<&mut SelectState> {
        match field {
            BillPlanField::Category => Some(&mut self.category),
            BillPlanField::Unit if !self.is_edit => Some(&mut self.unit),
            BillPlanField::Account => Some(&mut self.account),
            BillPlanField::Payee => Some(&mut self.payee),
            BillPlanField::Recurrence => Some(&mut self.recurrence),
            _ => None,
        }
    }

    fn select_options(field: BillPlanField, options: &BillPlanOptions) -> &[String] {
        match field {
            BillPlanField::Category => &options.category_labels,
            BillPlanField::Unit => &options.units,
            BillPlanField::Account => &options.account_labels,
            BillPlanField::Payee => &options.payee_labels,
            BillPlanField::Recurrence => &options.recurrence_labels,
            _ => &[],
        }
    }

    fn close_selects(&mut self) {
        for state in [
            &mut self.category,
            &mut self.unit,
            &mut self.account,
            &mut self.payee,
            &mut self.recurrence,
        ] {
            state.cancel();
        }
    }

    /// Whether the focused field is a select that takes keys (the Unit's is locked on Edit).
    fn on_select(&self) -> bool {
        matches!(
            self.focused,
            BillPlanField::Category
                | BillPlanField::Account
                | BillPlanField::Payee
                | BillPlanField::Recurrence
        ) || (self.focused == BillPlanField::Unit && !self.is_edit)
    }

    /// Moves focus to `field`, closing any open list.
    pub fn focus(&mut self, field: BillPlanField) {
        if field != self.focused {
            self.close_selects();
        }
        self.focused = field;
        self.changed();
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next shown field.
    pub fn cycle_focus(&mut self, backward: bool) {
        let field = self.focused;
        let options = self.options.clone();
        if let Some(state) = self.select_mut(field) {
            state.commit(Self::select_options(field, &options));
        }
        let order = BillPlanField::ORDER;
        let mut index = order.iter().position(|f| *f == field).unwrap_or(0);
        for _ in 0..order.len() {
            index = if backward {
                (index + order.len() - 1) % order.len()
            } else {
                (index + 1) % order.len()
            };
            if self.shows(order[index]) {
                break;
            }
        }
        self.focused = order[index];
        self.changed();
    }

    /// A key on the focused select. Returns whether a select is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        let field = self.focused;
        let options = self.options.clone();
        let list = Self::select_options(field, &options);
        let Some(state) = self.select_mut(field) else {
            return false;
        };
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        self.changed();
        true
    }

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: BillPlanField) {
        self.focus(field);
        let options = self.options.clone();
        let list = Self::select_options(field, &options);
        if let Some(state) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
        self.changed();
    }

    /// A click on row `index` of `field`'s open list.
    pub fn choose(&mut self, field: BillPlanField, index: usize) {
        let options = self.options.clone();
        let list = Self::select_options(field, &options);
        if let Some(state) = self.select_mut(field) {
            state.choose(list, index);
        }
        self.changed();
    }

    /// A click on the Fixed/Estimated segments.
    pub fn set_amount_kind(&mut self, kind: AmountKind) {
        self.focus(BillPlanField::AmountKind);
        self.amount_kind = kind;
    }

    /// `Space` or a click on the focused toggle: Fixed/Estimated or Active.
    pub fn toggle(&mut self) {
        match self.focused {
            BillPlanField::AmountKind => {
                self.amount_kind = match self.amount_kind {
                    AmountKind::Fixed => AmountKind::Estimated,
                    AmountKind::Estimated => AmountKind::Fixed,
                };
            }
            BillPlanField::Active if self.is_edit => self.is_active = !self.is_active,
            _ => {}
        }
        self.changed();
    }

    fn amount_money(&self) -> Option<Money> {
        self.amount
            .text()
            .trim()
            .parse::<BigDecimal>()
            .ok()
            .filter(|amount| amount.is_positive())
            .map(Money)
    }

    fn lead(&self) -> Result<Option<u32>, FieldError> {
        match self.attention_lead.text().trim() {
            "" => Ok(None),
            text => text
                .parse()
                .map(Some)
                .map_err(|_| FieldError::LeadNotWholeDays),
        }
    }

    /// Every field's problem. An empty amount is only kept from saving, not reported.
    pub fn errors(&self) -> FormErrors {
        let (today, date_style) = (self.source.today, self.source.date_style);
        let first_due = parse_date(self.first_due.text(), today, date_style);
        let recurring = self.recurrence() != Recurrence::OneShot;
        let ends_on = if recurring {
            parse_date(self.ends_on.text(), today, date_style)
        } else {
            Ok(None)
        };
        let ends_before_first_due = matches!(
            (&first_due, &ends_on),
            (Ok(Some(first)), Ok(Some(ends))) if ends < first
        );
        FormErrors {
            amount: (!self.amount.is_blank() && self.amount_money().is_none())
                .then_some(FieldError::AmountNotPositive),
            first_due: first_due.err(),
            ends_on: ends_on.err(),
            ends_before_first_due,
            lead: self.lead().err(),
        }
    }

    /// What Save submits, or `None` while a field is empty or has a problem. An empty First Due
    /// counts as missing.
    pub fn draft(&self) -> Option<BillPlanDraft> {
        let (today, date_style) = (self.source.today, self.source.date_style);
        if self.name.is_blank() || !self.errors().is_empty() {
            return None;
        }
        let options = &self.options;
        let recurrence = self.recurrence();
        let category = BillPlanOptions::index(&options.category_labels, self.category.value())?;
        let account = BillPlanOptions::index(&options.account_labels, self.account.value())?;
        let payee = BillPlanOptions::index(&options.payee_labels, self.payee.value())?;
        Some(BillPlanDraft {
            name: self.name.text().to_string(),
            category_id: options.category_ids[category],
            unit: self.unit.value()?.to_string(),
            account_id: options.account_ids[account],
            payee_id: options.payee_ids[payee],
            planned_amount: self.amount_money()?,
            amount_kind: self.amount_kind,
            recurrence,
            first_due: parse_date(self.first_due.text(), today, date_style).ok()??,
            ends_on: if recurrence == Recurrence::OneShot {
                None
            } else {
                parse_date(self.ends_on.text(), today, date_style).ok()?
            },
            attention_lead: self.lead().ok()?,
        })
    }
}

impl Dialog for BillPlanForm {
    /// `Shift-Tab` goes back a field. On a select `↑`/`↓` step or move the highlight, and `Space`
    /// and `Enter` open or commit its list; `Space` and `←`/`→` flip Fixed/Estimated and `Space`
    /// flips Active. The amount takes a decimal and the lead digits only; the selects and toggles
    /// take no text. Any key clears a stale Save error.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.error = None;
        let focused = self.focused;
        match key {
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Up if self.on_select() => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down if self.on_select() => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::Char(' ') | DialogKey::Enter if self.on_select() => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Left | DialogKey::Right if focused == BillPlanField::AmountKind => {
                self.toggle();
            }
            DialogKey::Char(' ')
                if matches!(focused, BillPlanField::AmountKind | BillPlanField::Active) =>
            {
                self.toggle();
            }
            DialogKey::Char(ch) => match focused {
                BillPlanField::Amount
                    if !(ch.is_ascii_digit()
                        || (ch == '.' && !self.amount.text().contains('.'))) => {}
                BillPlanField::AttentionLead if !ch.is_ascii_digit() => {}
                BillPlanField::Name
                | BillPlanField::Amount
                | BillPlanField::FirstDue
                | BillPlanField::EndsOn
                | BillPlanField::AttentionLead => return None,
                _ => {}
            },
            DialogKey::Backspace => match focused {
                BillPlanField::Name
                | BillPlanField::Amount
                | BillPlanField::FirstDue
                | BillPlanField::EndsOn
                | BillPlanField::AttentionLead => return None,
                _ => {}
            },
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            BillPlanField::Name => Some(&mut self.name),
            BillPlanField::Amount => Some(&mut self.amount),
            BillPlanField::FirstDue => Some(&mut self.first_due),
            BillPlanField::EndsOn => Some(&mut self.ends_on),
            BillPlanField::AttentionLead => Some(&mut self.attention_lead),
            _ => None,
        }
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        self.draft().is_some()
    }

    fn close_open_select(&mut self) -> bool {
        let open = [
            &self.category,
            &self.unit,
            &self.account,
            &self.payee,
            &self.recurrence,
        ]
        .iter()
        .any(|state| state.is_open());
        self.close_selects();
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts, bills, categories::default_categories,
        chrome::dialog_host::handle_key, payees::default_payees, tags::default_tags,
        transactions::default_transactions,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn label(recurrence: Recurrence) -> String {
        format!("{recurrence:?}")
    }

    struct Stubs {
        accounts: Vec<Account>,
        categories: Vec<Category>,
        payees: Vec<Payee>,
    }

    fn stubs() -> Stubs {
        Stubs {
            accounts: default_accounts(),
            categories: default_categories(),
            payees: default_payees(),
        }
    }

    impl Stubs {
        fn source(&self, keep_payee: Option<u32>) -> BillPlanSource {
            BillPlanSource::new(
                &self.categories,
                &self.accounts,
                &self.payees,
                keep_payee,
                "none".to_string(),
                label,
                today(),
                None,
            )
        }

        fn add_form(&self) -> BillPlanForm {
            BillPlanForm::new(self.source(None))
        }
    }

    fn type_into(form: &mut BillPlanForm, field: BillPlanField, text: &str) {
        form.focus(field);
        for ch in text.chars() {
            handle_key(form, DialogKey::Char(ch));
        }
    }

    #[test]
    fn a_new_form_starts_monthly_fixed_due_today_in_the_first_unit() {
        let stubs = stubs();
        let form = stubs.add_form();
        assert_eq!(
            form.unit_code(),
            form.options.units.first().map(String::as_str)
        );
        assert_eq!(
            form.account.value(),
            form.options.account_labels.first().map(String::as_str)
        );
        assert_eq!(form.recurrence(), Recurrence::Monthly);
        assert_eq!(form.amount_kind, AmountKind::Fixed);
        assert_eq!(form.payee.value(), Some("none"));
        assert!(form.draft().is_none(), "no name or amount yet");
    }

    #[test]
    fn category_lists_only_expense_leaves() {
        let stubs = stubs();
        let options = stubs.add_form().options;
        for id in &options.category_ids {
            let category = stubs.categories.iter().find(|c| c.id == *id).unwrap();
            assert_eq!(category.category_type, CategoryTypes::Expense);
            assert!(categories::is_leaf(&stubs.categories, *id));
        }
        assert!(!options.category_ids.is_empty());
    }

    #[test]
    fn a_filled_add_form_saves_a_plan() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        type_into(&mut form, BillPlanField::Name, "Water");
        type_into(&mut form, BillPlanField::Amount, "90.50");
        type_into(&mut form, BillPlanField::AttentionLead, "3");
        let draft = form.draft().unwrap();
        assert_eq!(draft.first_due, today());
        assert_eq!(draft.attention_lead, Some(3));
        assert_eq!(draft.payee_id, None);

        let (mut plans, mut entries) = (Vec::new(), Vec::new());
        let id = bills::insert_plan(
            &mut plans,
            &mut entries,
            &draft,
            &stubs.categories,
            &stubs.accounts,
            today(),
        )
        .unwrap();
        assert_eq!(bills::get(&plans, id).unwrap().name, "Water");
        assert!(!entries.is_empty(), "saving generates the Schedule");
    }

    #[test]
    fn changing_the_unit_moves_the_account_into_it() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        let first = form.account.value().map(str::to_string);
        let Some(other) = form
            .options
            .units
            .iter()
            .find(|u| Some(u.as_str()) != form.unit_code())
        else {
            return;
        };
        form.unit = SelectState::new(Some(other.clone()));
        form.changed();
        assert_ne!(form.account.value().map(str::to_string), first);
        assert_eq!(
            form.account.value(),
            form.options.account_labels.first().map(String::as_str)
        );
    }

    #[test]
    fn tab_skips_ends_on_for_a_one_shot_and_active_on_add() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        form.recurrence = SelectState::new(Some(label(Recurrence::OneShot)));
        form.focus(BillPlanField::FirstDue);
        form.cycle_focus(false);
        assert_eq!(form.focused, BillPlanField::AttentionLead);
        form.cycle_focus(false);
        assert_eq!(form.focused, BillPlanField::Name, "Active is Edit only");
        form.cycle_focus(true);
        assert_eq!(form.focused, BillPlanField::AttentionLead);
    }

    #[test]
    fn a_one_shot_ignores_a_typed_ends_on() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        type_into(&mut form, BillPlanField::Name, "Rego");
        type_into(&mut form, BillPlanField::Amount, "800");
        type_into(&mut form, BillPlanField::EndsOn, "nonsense");
        form.recurrence = SelectState::new(Some(label(Recurrence::OneShot)));
        assert_eq!(form.draft().unwrap().ends_on, None);
    }

    #[test]
    fn field_problems_show_inline_and_block_save() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        type_into(&mut form, BillPlanField::Name, "Water");
        type_into(&mut form, BillPlanField::Amount, "0");
        assert_eq!(form.errors().amount, Some(FieldError::AmountNotPositive));
        form.amount = TextField::new("12");
        form.first_due = TextField::new("2026-10-01");
        form.ends_on = TextField::new("2026-09-01");
        assert!(form.errors().ends_before_first_due);
        assert!(form.draft().is_none());
        form.ends_on = TextField::new("not a date");
        assert!(form.errors().ends_on.is_some());
        form.ends_on = TextField::default();
        assert!(form.errors().is_empty());
        assert!(form.draft().is_some());
    }

    #[test]
    fn amount_takes_a_decimal_and_lead_digits_only() {
        let stubs = stubs();
        let mut form = stubs.add_form();
        type_into(&mut form, BillPlanField::Amount, "1a2.3.4-");
        assert_eq!(form.amount.text(), "12.34");
        type_into(&mut form, BillPlanField::AttentionLead, "x5.");
        assert_eq!(form.attention_lead.text(), "5");
    }

    #[test]
    fn edit_prefills_from_the_plan_locks_the_unit_and_round_trips() {
        let stubs = stubs();
        let mut transactions = default_transactions(
            &stubs.accounts,
            &stubs.categories,
            &stubs.payees,
            &default_tags(),
            today(),
        );
        let seed = bills::default_bills(
            &stubs.accounts,
            &stubs.categories,
            &stubs.payees,
            &mut transactions,
            today(),
        );
        let plan = seed
            .plans
            .iter()
            .find(|p| p.attention_lead.is_some())
            .unwrap();
        let mut form = BillPlanForm::from_plan(plan, stubs.source(plan.payee_id));
        assert!(form.is_edit);
        form.focus(BillPlanField::Unit);
        assert!(
            !form.handle_select_key(SelectKey::Down),
            "the Unit is locked"
        );

        let draft = form.draft().unwrap();
        assert_eq!(draft.name, plan.name);
        assert_eq!(draft.category_id, plan.category_id);
        assert_eq!(draft.account_id, plan.account_id);
        assert_eq!(draft.payee_id, plan.payee_id);
        assert_eq!(draft.planned_amount, plan.planned_amount);
        assert_eq!(draft.recurrence, plan.recurrence);
        assert_eq!(draft.first_due, plan.first_due);
        assert_eq!(draft.ends_on, plan.ends_on);
        assert_eq!(draft.attention_lead, plan.attention_lead);

        form.focus(BillPlanField::Active);
        form.toggle();
        assert!(!form.is_active);
    }
}
