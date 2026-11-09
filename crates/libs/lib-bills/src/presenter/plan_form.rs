//! The Add and Edit bill plan Dialog (8c): one form, Edit pre-filled from the Plan.
//!
//! - The fields, in `Tab` order: Name, Category (expense leaves only), Unit (a select on Add, locked
//!   on Edit), Account (only those in the chosen Unit that take Transactions), Payee (optional),
//!   Planned amount, Fixed/Estimated, Recurrence, First Due, Ends On (hidden for a One-shot),
//!   Attention lead, and Active (Edit only).
//! - Changing the Unit keeps the Account when it is in the new Unit, and otherwise moves it to that
//!   Unit's first Account, so the draft never pairs an Account with another Unit.
//! - The amount takes a decimal and the lead whole days; First Due and Ends On take typed dates.
//!   Each field's problem shows under it, and Save waits until there are none. A refusal from the
//!   Interactor shows under the fields until the form changes.
//!
//! It takes the Dialog keymap's commands and typed keys as [`DialogInput`]s and decides what each
//! means on the focused field. On a select, `select` and `Space` open its list or pick from it and
//! `↑`/`↓` step it or move its highlight; on Fixed/Estimated, `Space`, `←` and `→` flip it; on
//! Active `Space` flips it; on a text field typing types. `select` anywhere else confirms a valid
//! form, and `submit` confirms from any field. Leaving a select commits its list's highlight.
//!
//! The choices are read when the Dialog opens, as it is modal and nothing changes them under it.

use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, CategoryId, Clock, DateStyle, Money, PayeeId, UnitId};
use lib_locale::format::{format_date_input, parse_optional_date, parse_positive_amount};
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, NEXT_FIELD, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{
    Choice, DialogFrame, Hint, SelectList, SelectMove, SelectView, TextFieldView,
};

use super::{cycle, recurrence_label};
use crate::interactor::{AddBillPlan, EditBillPlan};
use crate::protocols::{BillsOutcome, SharedPorts};
use crate::{AmountKind, BillPlanDraft, Error, Recurrence, msg};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanField {
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

impl PlanField {
    const ORDER: [Self; 12] = [
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanFormEvent {
    /// A Dialog keymap command or a typed key, meaning what the focused field makes it.
    Input(DialogInput),
    /// A text field's whole new text, as its View's input holds it. The amount keeps only a
    /// decimal's characters and the lead only digits.
    Name(String),
    Amount(String),
    FirstDue(String),
    EndsOn(String),
    Lead(String),
    Category(CategoryId),
    /// Ignored on Edit, where the Unit is locked.
    Unit(UnitId),
    Account(AccountId),
    Payee(Option<PayeeId>),
    Recurrence(Recurrence),
    /// A click on a Fixed/Estimated segment, which focuses it.
    AmountKind(AmountKind),
    /// A click on Active, which focuses it.
    ToggleActive,
    /// A click on a select's field: focuses it and opens or closes its list.
    ToggleList(PlanField),
    /// Closes an open list without picking: the host's `back`.
    CloseList,
    /// A click on a field. A hidden field can't take focus, and an open list closes.
    Focus(PlanField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    pub category: SelectView<CategoryId>,
    pub unit: SelectView<UnitId>,
    /// The Unit's code, upper-cased, while Edit locks it.
    pub unit_locked: Option<String>,
    pub account: SelectView<AccountId>,
    /// Shown in place of the Account select while no Account takes the Unit.
    pub no_accounts: Option<String>,
    pub payee: SelectView<Option<PayeeId>>,
    pub amount: TextFieldView,
    pub amount_error: Option<String>,
    pub amount_kind: SelectView<AmountKind>,
    pub recurrence: SelectView<Recurrence>,
    pub first_due: TextFieldView,
    pub first_due_error: Option<String>,
    /// Hidden for a One-shot.
    pub ends_on: Option<TextFieldView>,
    pub ends_on_error: Option<String>,
    pub lead: TextFieldView,
    /// What the lead is for, beside its label.
    pub lead_explainer: String,
    pub lead_error: Option<String>,
    /// The Active checkbox, Edit only.
    pub active: Option<ActiveView>,
    /// The fields on screen, in `Tab` order.
    pub fields: Vec<PlanField>,
    /// The focused select's list: open, and its highlight.
    pub list: SelectList,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveView {
    pub label: String,
    pub checked: bool,
    pub focused: bool,
}

/// What the selects choose from, read when the Dialog opens.
struct Choices {
    categories: Vec<Choice<CategoryId>>,
    units: Vec<Choice<UnitId>>,
    /// Every Account taking Transactions, with its Unit.
    accounts: Vec<(UnitId, Choice<AccountId>)>,
    payees: Vec<Choice<Option<PayeeId>>>,
}

pub struct PlanFormPresenter {
    ports: SharedPorts,
    /// The Dialog keymap's chords, which the legend shows.
    keys: DialogKeys,
    /// The Plan an Edit saves, and its stored name, which the title keeps while Name is retyped.
    editing: Option<(BillPlanId, String)>,
    /// What the dates are judged on: `today`, and the day a save supersedes from.
    clock: Clock,
    date_style: Option<DateStyle>,
    choices: Choices,
    name: String,
    category: Option<CategoryId>,
    unit: Option<UnitId>,
    account: Option<AccountId>,
    payee: Option<PayeeId>,
    amount: String,
    amount_kind: AmountKind,
    recurrence: Recurrence,
    first_due: String,
    ends_on: String,
    lead: String,
    is_active: bool,
    focused: PlanField,
    /// The focused select's list; it closes whenever focus leaves.
    list: SelectList,
    refusal: Option<String>,
}

impl PlanFormPresenter {
    /// A fresh Add form: the first Unit and its first Account, Monthly, Fixed, First Due today
    /// (the word `today`), everything else empty. Dates are judged on the day `clock` reads and
    /// typed in `date_style`; `keys` are the Dialog keymap's chords, which its hints show.
    pub fn add(
        ports: SharedPorts,
        clock: Clock,
        date_style: Option<DateStyle>,
        keys: DialogKeys,
    ) -> Self {
        let choices = Self::choices(&ports, None);
        let unit = choices.units.first().map(|choice| choice.value);
        let mut form = Self {
            ports,
            keys,
            editing: None,
            clock,
            date_style,
            category: choices.categories.first().map(|choice| choice.value),
            unit,
            account: None,
            payee: None,
            choices,
            name: String::new(),
            amount: String::new(),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::Monthly,
            first_due: lib_locale::msg::date_word_today(),
            ends_on: String::new(),
            lead: String::new(),
            is_active: true,
            focused: PlanField::Name,
            list: SelectList::default(),
            refusal: None,
        };
        form.keep_account_in_unit();
        form
    }

    /// The Edit form pre-filled from Plan `id`, or `None` when it is gone. The Plan's Payee stays
    /// listed even if it has since been deactivated, so opening Edit never drops it.
    pub fn edit(
        ports: SharedPorts,
        id: BillPlanId,
        clock: Clock,
        date_style: Option<DateStyle>,
        keys: DialogKeys,
    ) -> Option<Self> {
        let plan = ports.borrow().bills().bill_plan(id)?;
        let choices = Self::choices(&ports, plan.payee_id);
        Some(Self {
            ports,
            keys,
            editing: Some((id, plan.name.clone())),
            clock,
            date_style,
            category: choices
                .categories
                .iter()
                .any(|c| c.value == plan.category_id)
                .then_some(plan.category_id),
            unit: Some(plan.unit),
            account: choices
                .accounts
                .iter()
                .any(|(unit, c)| *unit == plan.unit && c.value == plan.account_id)
                .then_some(plan.account_id),
            payee: plan.payee_id,
            choices,
            name: plan.name,
            amount: plan.planned_amount.0.to_string(),
            amount_kind: plan.amount_kind,
            recurrence: plan.recurrence,
            first_due: format_date_input(plan.first_due, date_style),
            ends_on: plan
                .ends_on
                .map(|date| format_date_input(date, date_style))
                .unwrap_or_default(),
            lead: plan
                .attention_lead
                .map(|days| days.to_string())
                .unwrap_or_default(),
            is_active: plan.is_active,
            focused: PlanField::Name,
            list: SelectList::default(),
            refusal: None,
        })
    }

    fn choices(ports: &SharedPorts, keep_payee: Option<PayeeId>) -> Choices {
        let ports = ports.borrow();
        let all_categories = ports.categories().bill_categories();
        let mut categories: Vec<Choice<CategoryId>> = all_categories
            .iter()
            .filter(|c| c.is_expense && c.is_leaf)
            .map(|c| Choice {
                value: c.id,
                label: c.name.clone(),
            })
            .collect();
        categories.sort_by(|a, b| a.label.cmp(&b.label));
        let units = ports.units();
        let code = |id: UnitId| units.unit_code(id).unwrap_or_default();
        let accounts: Vec<(UnitId, Choice<AccountId>)> = ports
            .accounts()
            .bill_accounts()
            .into_iter()
            .filter(|a| a.takes_transactions)
            .map(|a| {
                (
                    a.unit,
                    Choice {
                        value: a.id,
                        label: a.name,
                    },
                )
            })
            .collect();
        let mut unit_pairs: Vec<(String, UnitId)> = accounts
            .iter()
            .map(|(unit, _)| (code(*unit), *unit))
            .collect();
        unit_pairs.sort();
        unit_pairs.dedup();
        let mut payees: Vec<_> = ports
            .payees()
            .bill_payees()
            .into_iter()
            .filter(|p| p.is_active || Some(p.id) == keep_payee)
            .collect();
        payees.sort_by(|a, b| a.name.cmp(&b.name));
        Choices {
            categories,
            units: unit_pairs
                .into_iter()
                .map(|(label, value)| Choice { value, label })
                .collect(),
            accounts,
            payees: std::iter::once(Choice {
                value: None,
                label: msg::bills_plan_payee_none(),
            })
            .chain(payees.into_iter().map(|p| Choice {
                value: Some(p.id),
                label: p.name,
            }))
            .collect(),
        }
    }

    /// What `input` means on the focused field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<BillsOutcome>) {
        match (input, self.focused) {
            (DialogInput::NextField, _) => self.move_focus(false),
            (DialogInput::PreviousField, _) => self.move_focus(true),
            (DialogInput::Submit, _) => self.confirm(router),
            (_, field) if self.is_select(field) => self.list_input(input),
            (
                DialogInput::Left | DialogInput::Right | DialogInput::Char(' '),
                PlanField::AmountKind,
            ) => {
                self.amount_kind = match self.amount_kind {
                    AmountKind::Fixed => AmountKind::Estimated,
                    AmountKind::Estimated => AmountKind::Fixed,
                };
            }
            (DialogInput::Char(' '), PlanField::Active) => self.toggle_active(),
            (DialogInput::Select, _) => self.confirm(router),
            (DialogInput::Char(ch), field) => {
                if let Some(text) = self.text_mut(field) {
                    text.push(ch);
                    self.filter(field);
                }
            }
            (DialogInput::Backspace, field) => {
                if let Some(text) = self.text_mut(field) {
                    text.pop();
                }
            }
            (
                DialogInput::SelectNext
                | DialogInput::SelectPrevious
                | DialogInput::Left
                | DialogInput::Right,
                _,
            ) => {}
        }
    }

    /// Moves focus to the next (or, `backward`, previous) field shown, wrapping. Leaving a select
    /// commits an open list's highlight, as a click elsewhere would.
    fn move_focus(&mut self, backward: bool) {
        self.commit_list();
        self.focused = cycle(&PlanField::ORDER, self.focused, backward, |field| {
            self.shows(field)
        });
    }

    /// Focuses `field` if it is shown, closing an open list on another select.
    fn focus(&mut self, field: PlanField) {
        if self.focused != field {
            self.list.close();
        }
        if self.shows(field) {
            self.focused = field;
        }
    }

    /// Whether `field` is a select that takes list input: the Unit's is locked on Edit.
    fn is_select(&self, field: PlanField) -> bool {
        match field {
            PlanField::Category | PlanField::Account | PlanField::Payee | PlanField::Recurrence => {
                true
            }
            PlanField::Unit => self.editing.is_none(),
            _ => false,
        }
    }

    /// The text `field` types into, `None` for a select or a toggle.
    fn text_mut(&mut self, field: PlanField) -> Option<&mut String> {
        match field {
            PlanField::Name => Some(&mut self.name),
            PlanField::Amount => Some(&mut self.amount),
            PlanField::FirstDue => Some(&mut self.first_due),
            PlanField::EndsOn if self.shows(PlanField::EndsOn) => Some(&mut self.ends_on),
            PlanField::AttentionLead => Some(&mut self.lead),
            _ => None,
        }
    }

    /// Keeps only the characters `field` takes after a typed one: the amount a decimal's, the lead
    /// digits.
    fn filter(&mut self, field: PlanField) {
        match field {
            PlanField::Amount => self.amount = decimal_characters(&self.amount),
            PlanField::AttentionLead => self.lead.retain(|ch| ch.is_ascii_digit()),
            _ => {}
        }
    }

    /// Opens the focused select's list, or closes it when open.
    fn toggle_list(&mut self) {
        if self.list.is_open() {
            self.list.close();
        } else {
            self.list_input(DialogInput::Select);
        }
    }

    /// Commits an open list's highlight and closes it.
    fn commit_list(&mut self) {
        if self.list.is_open() {
            self.list_input(DialogInput::Select);
        }
        self.list.close();
    }

    /// `input` on the focused select's list, applying a value it picks.
    fn list_input(&mut self, input: DialogInput) {
        match self.focused {
            PlanField::Category => {
                let moved =
                    self.list
                        .input(input, &self.choices.categories, self.category.as_ref());
                if let SelectMove::Chose(id) = moved {
                    self.set_category(id);
                }
            }
            PlanField::Unit => {
                let moved = self
                    .list
                    .input(input, &self.choices.units, self.unit.as_ref());
                if let SelectMove::Chose(id) = moved {
                    self.set_unit(id);
                }
            }
            PlanField::Account => {
                let accounts: Vec<Choice<AccountId>> = self.accounts_in_unit().cloned().collect();
                let moved = self.list.input(input, &accounts, self.account.as_ref());
                if let SelectMove::Chose(id) = moved {
                    self.set_account(id);
                }
            }
            PlanField::Payee => {
                let moved = self
                    .list
                    .input(input, &self.choices.payees, Some(&self.payee));
                if let SelectMove::Chose(id) = moved {
                    self.set_payee(id);
                }
            }
            PlanField::Recurrence => {
                let options = recurrence_choices();
                let moved = self.list.input(input, &options, Some(&self.recurrence));
                if let SelectMove::Chose(recurrence) = moved {
                    self.set_recurrence(recurrence);
                }
            }
            _ => {}
        }
    }

    fn set_category(&mut self, id: CategoryId) {
        if self.choices.categories.iter().any(|c| c.value == id) {
            self.category = Some(id);
        }
    }

    fn set_unit(&mut self, id: UnitId) {
        if self.editing.is_none() && self.choices.units.iter().any(|c| c.value == id) {
            self.unit = Some(id);
            self.keep_account_in_unit();
        }
    }

    fn set_account(&mut self, id: AccountId) {
        if self.accounts_in_unit().any(|c| c.value == id) {
            self.account = Some(id);
        }
    }

    fn set_payee(&mut self, id: Option<PayeeId>) {
        if self.choices.payees.iter().any(|c| c.value == id) {
            self.payee = id;
        }
    }

    fn set_recurrence(&mut self, recurrence: Recurrence) {
        self.recurrence = recurrence;
        if !self.shows(self.focused) {
            self.focused = PlanField::Recurrence;
        }
    }

    /// Flips Active, which Edit alone has, and focuses it.
    fn toggle_active(&mut self) {
        if self.editing.is_some() {
            self.list.close();
            self.focused = PlanField::Active;
            self.is_active = !self.is_active;
        }
    }

    fn accounts_in_unit(&self) -> impl Iterator<Item = &Choice<AccountId>> {
        self.choices
            .accounts
            .iter()
            .filter(|(unit, _)| Some(*unit) == self.unit)
            .map(|(_, choice)| choice)
    }

    /// Keeps the Account when it is in the Unit now chosen, else moves it to that Unit's first.
    fn keep_account_in_unit(&mut self) {
        let kept = self
            .accounts_in_unit()
            .any(|c| Some(c.value) == self.account);
        if !kept {
            let first = self.accounts_in_unit().next().map(|c| c.value);
            self.account = first;
        }
    }

    /// Whether `field` is on the form: Ends On only when recurring, Active only on Edit.
    fn shows(&self, field: PlanField) -> bool {
        match field {
            PlanField::EndsOn => self.recurrence != Recurrence::OneShot,
            PlanField::Active => self.editing.is_some(),
            _ => true,
        }
    }

    fn parse(&self, text: &str) -> Result<Option<NaiveDate>, String> {
        parse_optional_date(text, self.clock.today(), self.date_style)
    }

    fn amount_money(&self) -> Option<Money> {
        parse_positive_amount(&self.amount)
    }

    fn lead_days(&self) -> Result<Option<u32>, ()> {
        match self.lead.trim() {
            "" => Ok(None),
            text => text.parse().map(Some).map_err(|_| ()),
        }
    }

    /// Each field's problem: the amount's, First Due's, Ends On's and the lead's. An empty amount
    /// only waits, unreported.
    fn errors(&self) -> FieldErrors {
        let first_due = self.parse(&self.first_due);
        let ends_on = if self.shows(PlanField::EndsOn) {
            self.parse(&self.ends_on)
        } else {
            Ok(None)
        };
        let ends_before_first_due = matches!(
            (&first_due, &ends_on),
            (Ok(Some(first)), Ok(Some(ends))) if ends < first
        );
        FieldErrors {
            amount: (!self.amount.trim().is_empty() && self.amount_money().is_none())
                .then(msg::bills_plan_error_amount),
            first_due: first_due.err(),
            ends_on: ends_on.err().or_else(|| {
                ends_before_first_due.then(msg::bills_plan_error_ends_before_first_due)
            }),
            lead: self.lead_days().is_err().then(msg::bills_plan_error_lead),
        }
    }

    /// What Save submits, or `None` while a field is empty or has a problem. An empty First Due
    /// counts as missing.
    fn draft(&self) -> Option<BillPlanDraft> {
        if self.name.trim().is_empty() || !self.errors().is_empty() {
            return None;
        }
        Some(BillPlanDraft {
            name: self.name.clone(),
            category_id: self.category?,
            unit: self.unit?,
            account_id: self.account?,
            payee_id: self.payee,
            planned_amount: self.amount_money()?,
            amount_kind: self.amount_kind,
            recurrence: self.recurrence,
            first_due: self.parse(&self.first_due).ok()??,
            ends_on: if self.shows(PlanField::EndsOn) {
                self.parse(&self.ends_on).ok()?
            } else {
                None
            },
            attention_lead: self.lead_days().ok()?,
        })
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<BillsOutcome>) {
        let Some(draft) = self.draft() else {
            return;
        };
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            match self.editing {
                None => AddBillPlan {
                    draft,
                    today: self.clock.today(),
                }
                .run(ports.bills, ports.accounts, ports.categories),
                Some((id, _)) => EditBillPlan {
                    id,
                    draft,
                    is_active: self.is_active,
                    today: self.clock.today(),
                }
                .run(ports.bills, ports.accounts, ports.categories)
                .map(|()| id),
            }
        };
        match saved {
            Ok(id) => router.close(Some(BillsOutcome::PlanSaved(id))),
            Err(refusal) => self.refusal = Some(refusal_text(&refusal)),
        }
    }
}

/// The Recurrences the select offers, in the order `Recurrence::ALL` lists them.
fn recurrence_choices() -> Vec<Choice<Recurrence>> {
    Recurrence::ALL
        .into_iter()
        .map(|value| Choice {
            value,
            label: recurrence_label(value),
        })
        .collect()
}

fn select_view<T>(
    label: String,
    options: Vec<Choice<T>>,
    chosen: Option<T>,
    focused: bool,
) -> SelectView<T> {
    SelectView {
        label,
        options,
        chosen,
        focused,
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct FieldErrors {
    amount: Option<String>,
    first_due: Option<String>,
    ends_on: Option<String>,
    lead: Option<String>,
}

impl FieldErrors {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The characters a decimal amount takes: digits, and one point.
fn decimal_characters(text: &str) -> String {
    let mut kept = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || (ch == '.' && !kept.contains('.')) {
            kept.push(ch);
        }
    }
    kept
}

/// A refused Save. The form's own checks catch most of these first; what's left is a choice the
/// selects couldn't fill (no expense Category, no Account in the Unit) or a Plan gone under it.
fn refusal_text(refusal: &Error) -> String {
    match refusal {
        Error::NameRequired => msg::bills_plan_error_name(),
        Error::CategoryNotExpenseLeaf => msg::bills_plan_error_category(),
        Error::AccountRequired | Error::UnitMismatch => msg::bills_plan_error_account(),
        Error::AmountNotPositive => msg::bills_plan_error_amount(),
        Error::EndsBeforeFirstDue => msg::bills_plan_error_ends_before_first_due(),
        Error::PlanNotFound => msg::bills_plan_error_gone(),
        // Not reachable from the form: One-shot hides Ends On, Edit locks the Unit, and the rest
        // come from settlement.
        Error::UnitLocked(_)
        | Error::EndsOnForOneShot
        | Error::EntryNotFound
        | Error::EntryNotOpen
        | Error::NotACandidate
        | Error::NotPaid
        | Error::NotSkipped => msg::bills_plan_error_other(),
    }
}

impl Presenter for PlanFormPresenter {
    type Event = PlanFormEvent;
    type ViewModel = PlanFormViewModel;
    type Router<'r> = dyn DialogRouter<BillsOutcome> + 'r;

    fn handle(&mut self, event: PlanFormEvent, router: &mut Self::Router<'_>) {
        if event != PlanFormEvent::Confirm {
            self.refusal = None;
        }
        match event {
            PlanFormEvent::Input(input) => self.input(input, router),
            PlanFormEvent::Name(text) => self.name = text,
            PlanFormEvent::Amount(text) => self.amount = decimal_characters(&text),
            PlanFormEvent::FirstDue(text) => self.first_due = text,
            PlanFormEvent::EndsOn(text) => self.ends_on = text,
            PlanFormEvent::Lead(text) => {
                self.lead = text.chars().filter(char::is_ascii_digit).collect();
            }
            PlanFormEvent::Category(id) => {
                self.list.close();
                self.set_category(id);
            }
            PlanFormEvent::Unit(id) => {
                self.list.close();
                self.set_unit(id);
            }
            PlanFormEvent::Account(id) => {
                self.list.close();
                self.set_account(id);
            }
            PlanFormEvent::Payee(id) => {
                self.list.close();
                self.set_payee(id);
            }
            PlanFormEvent::Recurrence(recurrence) => {
                self.list.close();
                self.set_recurrence(recurrence);
            }
            PlanFormEvent::AmountKind(kind) => {
                self.focused = PlanField::AmountKind;
                self.amount_kind = kind;
            }
            PlanFormEvent::ToggleActive => self.toggle_active(),
            PlanFormEvent::ToggleList(field) => {
                if self.is_select(field) {
                    // Only the clicked select's list opens; a second click closes it.
                    if self.focused == field && self.list.is_open() {
                        self.list.close();
                    } else {
                        self.focus(field);
                        self.toggle_list();
                    }
                }
            }
            PlanFormEvent::CloseList => {
                self.list.close();
            }
            PlanFormEvent::Focus(field) => {
                self.list.close();
                if self.shows(field) {
                    self.focused = field;
                }
            }
            PlanFormEvent::Confirm => self.confirm(router),
            PlanFormEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> PlanFormViewModel {
        let text = |label: String, text: &str, placeholder: String, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let select = |field| self.focused == field;
        let errors = self.errors();
        let (title, submit) = match &self.editing {
            Some((_, name)) => (
                msg::bills_plan_edit_title(name),
                msg::bills_plan_edit_submit(),
            ),
            None => (msg::bills_plan_add_title(), msg::bills_plan_add_submit()),
        };
        let accounts: Vec<Choice<AccountId>> = self.accounts_in_unit().cloned().collect();
        PlanFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.draft().is_some(),
                destructive: false,
                hints: Vec::new(),
            },
            name: text(
                lib_locale::msg::column_name(),
                &self.name,
                msg::bills_plan_name_placeholder(),
                PlanField::Name,
            ),
            category: select_view(
                msg::bills_plan_field_category(),
                self.choices.categories.clone(),
                self.category,
                select(PlanField::Category),
            ),
            unit: select_view(
                lib_locale::msg::column_unit(),
                self.choices.units.clone(),
                self.unit,
                select(PlanField::Unit),
            ),
            unit_locked: self.editing.as_ref().map(|_| {
                self.choices
                    .units
                    .iter()
                    .find(|c| Some(c.value) == self.unit)
                    .map(|c| c.label.to_uppercase())
                    .unwrap_or_default()
            }),
            no_accounts: accounts.is_empty().then(msg::bills_plan_no_accounts),
            account: select_view(
                msg::bills_column_account(),
                accounts,
                self.account,
                select(PlanField::Account),
            ),
            payee: select_view(
                msg::bills_plan_field_payee(),
                self.choices.payees.clone(),
                Some(self.payee),
                select(PlanField::Payee),
            ),
            amount: text(
                msg::bills_plan_field_amount(),
                &self.amount,
                msg::bills_plan_amount_placeholder(),
                PlanField::Amount,
            ),
            amount_error: errors.amount,
            amount_kind: select_view(
                msg::bills_plan_field_amount_kind(),
                [AmountKind::Fixed, AmountKind::Estimated]
                    .into_iter()
                    .map(|value| Choice {
                        value,
                        label: match value {
                            AmountKind::Fixed => msg::bills_amount_fixed(),
                            AmountKind::Estimated => msg::bills_amount_estimated(),
                        },
                    })
                    .collect(),
                Some(self.amount_kind),
                select(PlanField::AmountKind),
            ),
            recurrence: select_view(
                msg::bills_plan_field_recurrence(),
                recurrence_choices(),
                Some(self.recurrence),
                select(PlanField::Recurrence),
            ),
            first_due: text(
                msg::bills_plan_field_first_due(),
                &self.first_due,
                lib_locale::msg::date_word_today(),
                PlanField::FirstDue,
            ),
            first_due_error: errors.first_due,
            ends_on: self.shows(PlanField::EndsOn).then(|| {
                text(
                    msg::bills_plan_field_ends_on(),
                    &self.ends_on,
                    msg::bills_plan_ends_on_placeholder(),
                    PlanField::EndsOn,
                )
            }),
            ends_on_error: errors.ends_on,
            lead: text(
                msg::bills_plan_field_lead(),
                &self.lead,
                msg::bills_plan_lead_placeholder(),
                PlanField::AttentionLead,
            ),
            lead_explainer: msg::bills_plan_lead_explainer(),
            lead_error: errors.lead,
            active: self.editing.as_ref().map(|_| ActiveView {
                label: msg::bills_plan_active(),
                checked: self.is_active,
                focused: self.focused == PlanField::Active,
            }),
            fields: PlanField::ORDER
                .into_iter()
                .filter(|field| self.shows(*field))
                .collect(),
            list: self.list,
            refusal: self.refusal.clone(),
            hints: vec![
                self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
                self.keys
                    .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
                self.keys
                    .hint(NEXT_FIELD, lib_locale::msg::dialog_hint_next_field()),
            ],
        }
    }
}

impl DialogPresenter for PlanFormPresenter {
    type Outcome = BillsOutcome;

    fn input_event(input: DialogInput) -> PlanFormEvent {
        PlanFormEvent::Input(input)
    }

    fn confirm_event() -> PlanFormEvent {
        PlanFormEvent::Confirm
    }

    fn cancel_event() -> PlanFormEvent {
        PlanFormEvent::Cancel
    }

    /// Every field filled and none with a problem.
    fn can_confirm(&self) -> bool {
        self.draft().is_some()
    }

    fn close_open_select(&mut self) -> bool {
        self.list.close()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, Routed};
    use crate::presenter::test_support::{sample, today};

    const TAB: PlanFormEvent = PlanFormEvent::Input(DialogInput::NextField);
    const SHIFT_TAB: PlanFormEvent = PlanFormEvent::Input(DialogInput::PreviousField);
    const SELECT: PlanFormEvent = PlanFormEvent::Input(DialogInput::Select);
    const SUBMIT: PlanFormEvent = PlanFormEvent::Input(DialogInput::Submit);
    const DOWN: PlanFormEvent = PlanFormEvent::Input(DialogInput::SelectNext);
    const UP: PlanFormEvent = PlanFormEvent::Input(DialogInput::SelectPrevious);
    const SPACE: PlanFormEvent = PlanFormEvent::Input(DialogInput::Char(' '));
    const BACKSPACE: PlanFormEvent = PlanFormEvent::Input(DialogInput::Backspace);

    fn typing(text: &str) -> Vec<PlanFormEvent> {
        text.chars()
            .map(|ch| PlanFormEvent::Input(DialogInput::Char(ch)))
            .collect()
    }

    struct Form {
        ports: Rc<RefCell<InMemoryPorts>>,
        presenter: PlanFormPresenter,
        router: RecordingRouter,
    }

    impl Form {
        fn add() -> Self {
            let (ports, shared) = sample();
            Self {
                ports,
                presenter: PlanFormPresenter::add(
                    shared,
                    Clock::fixed(today()),
                    None,
                    DialogKeys::default(),
                ),
                router: RecordingRouter::default(),
            }
        }

        fn edit(name: &str) -> Self {
            let (ports, shared) = sample();
            let id = ports
                .borrow()
                .bills
                .plans
                .iter()
                .find(|plan| plan.name == name)
                .map(|plan| plan.id)
                .expect("a sample Plan");
            Self {
                ports,
                presenter: PlanFormPresenter::edit(
                    shared,
                    id,
                    Clock::fixed(today()),
                    None,
                    DialogKeys::default(),
                )
                .expect("the Plan is there"),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: PlanFormEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn send_all(&mut self, events: impl IntoIterator<Item = PlanFormEvent>) {
            for event in events {
                self.send(event);
            }
        }

        fn view(&self) -> PlanFormViewModel {
            self.presenter.view_model()
        }
    }

    #[test]
    fn a_new_form_starts_monthly_fixed_due_today_in_the_first_unit() {
        let form = Form::add();
        let view = form.view();
        assert_eq!(view.frame.title, "Add bill plan");
        assert_eq!(view.unit.chosen, view.unit.options.first().map(|c| c.value));
        assert_eq!(
            view.account.chosen,
            view.account.options.first().map(|c| c.value)
        );
        assert_eq!(view.recurrence.chosen, Some(Recurrence::Monthly));
        assert_eq!(view.amount_kind.chosen, Some(AmountKind::Fixed));
        assert_eq!(view.payee.chosen, Some(None));
        assert_eq!(view.first_due.text, "today");
        assert!(view.active.is_none(), "Active is Edit only");
        assert!(!view.frame.can_confirm, "no name or amount yet");
    }

    #[test]
    fn the_category_select_lists_only_expense_leaves() {
        let form = Form::add();
        let categories = form.ports.borrow().reads.categories.clone();
        let options = form.view().category.options;
        assert!(!options.is_empty());
        for choice in options {
            let category = categories
                .iter()
                .find(|c| c.id == choice.value)
                .expect("listed");
            assert!(category.is_expense && category.is_leaf);
        }
    }

    #[test]
    fn a_filled_add_form_saves_the_plan_and_its_schedule() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Name("Water".into()));
        form.send(PlanFormEvent::Amount("90.50".into()));
        form.send(PlanFormEvent::Lead("3".into()));
        assert!(form.view().frame.can_confirm);

        form.send(PlanFormEvent::Confirm);

        let ports = form.ports.borrow();
        let plan = ports
            .bills
            .plans
            .iter()
            .find(|plan| plan.name == "Water")
            .expect("saved");
        assert_eq!(plan.first_due, today());
        assert_eq!(plan.attention_lead, Some(3));
        assert_eq!(plan.payee_id, None);
        assert!(ports.bills.entries.iter().any(|e| e.plan_id == plan.id));
        assert_eq!(
            form.router.routed,
            [Routed::Closed(Some(BillsOutcome::PlanSaved(plan.id)))]
        );
    }

    #[test]
    fn the_amount_takes_a_decimal_and_the_lead_digits_only() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Amount("1a2.3.4-".into()));
        form.send(PlanFormEvent::Lead("x5.".into()));
        let view = form.view();
        assert_eq!(view.amount.text, "12.34");
        assert_eq!(view.lead.text, "5");
    }

    #[test]
    fn field_problems_show_under_their_fields_and_hold_save() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Name("Water".into()));
        form.send(PlanFormEvent::Amount("0".into()));
        assert_eq!(
            form.view().amount_error.as_deref(),
            Some("Enter an amount greater than zero.")
        );
        form.send(PlanFormEvent::Amount("12".into()));
        form.send(PlanFormEvent::FirstDue("2026-10-01".into()));
        form.send(PlanFormEvent::EndsOn("2026-09-01".into()));
        assert_eq!(
            form.view().ends_on_error.as_deref(),
            Some("Ends on can't be before first due.")
        );
        assert!(!form.view().frame.can_confirm);
        form.send(PlanFormEvent::EndsOn("not a date".into()));
        assert!(form.view().ends_on_error.is_some());
        form.send(PlanFormEvent::EndsOn(String::new()));
        assert!(form.view().frame.can_confirm);
    }

    #[test]
    fn a_one_shot_hides_ends_on_from_the_form_and_tab() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Focus(PlanField::EndsOn));
        form.send(PlanFormEvent::Recurrence(Recurrence::OneShot));

        let view = form.view();
        assert!(view.ends_on.is_none());
        assert!(!view.fields.contains(&PlanField::EndsOn));
        assert!(view.recurrence.focused, "focus leaves the hidden field");
        form.send(PlanFormEvent::Focus(PlanField::FirstDue));
        form.send(TAB);
        assert!(form.view().lead.focused);
        form.send(TAB);
        assert!(form.view().name.focused, "Active is Edit only");
    }

    #[test]
    fn changing_the_unit_moves_the_account_into_it() {
        let mut form = Form::add();
        let view = form.view();
        let Some(other) = view
            .unit
            .options
            .iter()
            .find(|c| Some(c.value) != view.unit.chosen)
        else {
            return;
        };

        form.send(PlanFormEvent::Unit(other.value));

        let view = form.view();
        assert_eq!(view.unit.chosen, Some(other.value));
        assert_eq!(
            view.account.chosen,
            view.account.options.first().map(|c| c.value)
        );
    }

    #[test]
    fn edit_prefills_locks_the_unit_and_saves_the_activation() {
        let mut form = Form::edit("Telstra Internet");
        let view = form.view();
        assert_eq!(view.frame.title, "Edit bill plan \u{2014} Telstra Internet");
        assert_eq!(view.frame.submit, "Save");
        assert_eq!(view.unit_locked.as_deref(), Some("AUD"));
        assert_eq!(view.name.text, "Telstra Internet");
        assert!(view.frame.can_confirm, "a stored Plan is a valid draft");
        let unit = view.unit.chosen;
        if let Some(other) = view.unit.options.iter().find(|c| Some(c.value) != unit) {
            form.send(PlanFormEvent::Unit(other.value));
            assert_eq!(form.view().unit.chosen, unit, "the Unit is locked");
        }

        form.send(PlanFormEvent::Name("Telstra NBN".into()));
        form.send(PlanFormEvent::ToggleActive);
        assert_eq!(
            form.view().frame.title,
            "Edit bill plan \u{2014} Telstra Internet",
            "the title keeps the stored name"
        );
        form.send(PlanFormEvent::Confirm);

        let ports = form.ports.borrow();
        let plan = ports
            .bills
            .plans
            .iter()
            .find(|plan| plan.name == "Telstra NBN")
            .expect("saved");
        assert!(!plan.is_active);
    }

    #[test]
    fn a_plan_gone_under_the_form_is_refused_with_its_reason() {
        let mut form = Form::edit("Telstra Internet");
        form.ports
            .borrow_mut()
            .bills
            .plans
            .retain(|plan| plan.name != "Telstra Internet");

        form.send(PlanFormEvent::Confirm);

        assert_eq!(
            form.view().refusal.as_deref(),
            Some("This bill plan no longer exists.")
        );
        assert!(form.router.routed.is_empty(), "it stays open");
        form.send(PlanFormEvent::Name("Telstra".into()));
        assert_eq!(form.view().refusal, None, "a change clears it");
    }

    #[test]
    fn cancel_closes_with_nothing_written() {
        let mut form = Form::add();
        let before = form.ports.borrow().bills.clone();
        form.send(PlanFormEvent::Name("Water".into()));

        form.send(PlanFormEvent::Cancel);

        assert_eq!(form.router.routed, [Routed::Closed(None)]);
        assert_eq!(form.ports.borrow().bills, before);
    }

    #[test]
    fn tab_and_shift_tab_move_focus_skipping_hidden_fields_and_wrapping() {
        let mut form = Form::add();
        form.send(SHIFT_TAB);
        assert!(
            form.view().lead.focused,
            "Active is Edit only, so back wraps to the lead"
        );
        form.send(TAB);
        assert!(form.view().name.focused);
        form.send(TAB);
        assert!(form.view().category.focused);

        let mut edit = Form::edit("Telstra Internet");
        edit.send(SHIFT_TAB);
        assert!(edit.view().active.is_some_and(|active| active.focused));
    }

    #[test]
    fn typing_fills_the_focused_text_field_and_backspace_takes_a_character_back() {
        let mut form = Form::add();
        form.send_all(typing("Water"));
        form.send(BACKSPACE);
        assert_eq!(form.view().name.text, "Wate");

        form.send_all([TAB, TAB, TAB, TAB, TAB]);
        assert!(form.view().amount.focused);
        form.send_all(typing("1a2.3.4-"));
        assert_eq!(
            form.view().amount.text,
            "12.34",
            "only a decimal's characters"
        );

        form.send(SHIFT_TAB);
        form.send(SHIFT_TAB);
        assert!(form.view().account.focused);
        form.send_all(typing("zz"));
        assert!(
            form.view().account.chosen.is_some(),
            "a select takes no text"
        );
        assert_eq!(form.view().name.text, "Wate");
    }

    #[test]
    fn select_opens_a_list_the_arrows_move_and_select_picks() {
        let mut form = Form::add();
        form.send(TAB);
        assert!(form.view().category.focused);
        let first = form.view().category.chosen;
        let options = form.view().category.options.clone();
        assert!(options.len() > 2, "the sample has several expense leaves");

        form.send(SELECT);
        assert_eq!(form.view().list.highlight(), Some(0));
        form.send(DOWN);
        assert_eq!(form.view().list.highlight(), Some(1));
        assert_eq!(form.view().category.chosen, first, "moving only highlights");
        form.send(SELECT);
        assert!(!form.view().list.is_open());
        assert_eq!(form.view().category.chosen, Some(options[1].value));
        assert!(form.router.routed.is_empty(), "picking is not confirming");
    }

    #[test]
    fn the_arrows_step_a_closed_select_and_space_opens_it() {
        let mut form = Form::add();
        form.send(TAB);
        let options = form.view().category.options.clone();

        form.send(UP);
        assert_eq!(
            form.view().category.chosen,
            options.last().map(|c| c.value),
            "up from the first wraps to the last"
        );
        form.send(SPACE);
        assert!(form.view().list.is_open());
    }

    #[test]
    fn tab_commits_an_open_lists_highlight_and_moves_on() {
        let mut form = Form::add();
        form.send_all([TAB, SELECT, DOWN]);
        let target = form.view().category.options[1].value;

        form.send(TAB);

        let view = form.view();
        assert_eq!(view.category.chosen, Some(target));
        assert!(!view.list.is_open());
        assert!(view.unit.focused);
    }

    #[test]
    fn close_list_leaves_the_highlight_unchosen() {
        let mut form = Form::add();
        form.send_all([TAB, SELECT, DOWN]);
        let chosen = form.view().category.chosen;

        form.send(PlanFormEvent::CloseList);

        assert!(!form.view().list.is_open());
        assert_eq!(form.view().category.chosen, chosen);
    }

    #[test]
    fn space_left_and_right_flip_fixed_and_estimated() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Focus(PlanField::AmountKind));
        form.send(SPACE);
        assert_eq!(form.view().amount_kind.chosen, Some(AmountKind::Estimated));
        form.send(PlanFormEvent::Input(DialogInput::Left));
        assert_eq!(form.view().amount_kind.chosen, Some(AmountKind::Fixed));
        form.send(PlanFormEvent::Input(DialogInput::Right));
        assert_eq!(form.view().amount_kind.chosen, Some(AmountKind::Estimated));
    }

    #[test]
    fn space_flips_active_on_edit() {
        let mut form = Form::edit("Telstra Internet");
        form.send(SHIFT_TAB);
        let before = form.view().active.expect("Edit has Active").checked;

        form.send(SPACE);

        assert_eq!(
            form.view().active.map(|active| active.checked),
            Some(!before)
        );
    }

    #[test]
    fn select_on_a_text_field_confirms_only_a_valid_form_and_submit_confirms_from_a_select() {
        let mut form = Form::add();
        form.send(SELECT);
        assert!(form.router.routed.is_empty(), "no name or amount yet");

        form.send_all(typing("Water"));
        form.send(PlanFormEvent::Amount("90".into()));
        form.send(TAB);
        form.send(SUBMIT);

        assert!(
            matches!(
                form.router.routed.as_slice(),
                [Routed::Closed(Some(BillsOutcome::PlanSaved(_)))]
            ),
            "{:?}",
            form.router.routed
        );
    }

    #[test]
    fn select_on_a_valid_text_field_confirms() {
        let mut form = Form::add();
        form.send(PlanFormEvent::Name("Water".into()));
        form.send(PlanFormEvent::Amount("90".into()));

        form.send(SELECT);

        assert!(matches!(
            form.router.routed.as_slice(),
            [Routed::Closed(Some(BillsOutcome::PlanSaved(_)))]
        ));
    }

    #[test]
    fn a_click_toggles_one_selects_list_and_a_second_click_closes_it() {
        let mut form = Form::add();
        form.send(PlanFormEvent::ToggleList(PlanField::Payee));
        assert!(form.view().payee.focused);
        assert!(form.view().list.is_open());

        form.send(PlanFormEvent::ToggleList(PlanField::Recurrence));
        assert!(form.view().recurrence.focused);
        assert!(
            form.view().list.is_open(),
            "the other list opened in its place"
        );

        form.send(PlanFormEvent::ToggleList(PlanField::Recurrence));
        assert!(!form.view().list.is_open());

        form.send(PlanFormEvent::ToggleList(PlanField::Name));
        assert!(form.view().recurrence.focused, "a text field has no list");
    }

    #[test]
    fn a_locked_unit_takes_no_list() {
        let mut form = Form::edit("Telstra Internet");
        form.send(PlanFormEvent::ToggleList(PlanField::Unit));
        assert!(!form.view().list.is_open());
    }
}
