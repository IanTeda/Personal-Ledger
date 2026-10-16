//! The live form behind **New budget** (11c in `docs/ux/desktop-mockups/14-budgets-v2/README.md`) and its
//! edit mode: `gpui`-free and unit-tested, the same split `bills/form.rs` uses.
//!
//! The behaviour is the Desktop Budgets Surface map's settled decision (#400):
//!
//! - All four methods are drawn but only Category limits can be chosen, so the Method grid is
//!   never a focus stop.
//! - **Accounts on budget** offers the Accounts that take Transactions in the chosen Unit, all
//!   preselected; changing the Unit re-selects them.
//! - **Start from** is Empty, Copy categories from the Budget on show (offered while that Budget
//!   is in the chosen Unit) or Last 3 months' spending. Each writes Onward from the current month.
//! - Edit mode locks Unit and Method and has no Start from: it is the one place to rename a
//!   Budget or change its Accounts.

use lib_core::UnitId;
use lib_units::UnitRow;

use crate::{
    budgets::{Budget, BudgetError, NewBudget, StartFrom},
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectKey,
    form::select::SelectState,
    transactions,
};
use lib_accounts::Account;

/// The form's focus stops, in `Tab` order. Edit mode skips Unit and Start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    Name,
    Unit,
    Accounts,
    StartFrom,
}

/// Start from's three cells, in the control's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartChoice {
    #[default]
    Empty,
    Copy,
    LastThreeMonths,
}

pub const START_CHOICES: [StartChoice; 3] = [
    StartChoice::Empty,
    StartChoice::Copy,
    StartChoice::LastThreeMonths,
];

/// The Unit select's options and the Accounts offered for the form's Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOptions {
    /// Unit codes: every Unit some Account taking Transactions is kept in.
    pub units: Vec<String>,
    /// The Accounts taking Transactions in the form's Unit: id and name.
    pub accounts: Vec<(u32, String)>,
}

/// An Account that takes Transactions, as the form needs it: id, name and Unit.
type Candidate = (u32, String, String);

fn unit_code_of(units: &[UnitRow], id: UnitId) -> String {
    units
        .iter()
        .find(|unit| unit.id == id)
        .map_or_else(String::new, |unit| unit.code.clone())
}

fn candidates(accounts: &[Account], units: &[UnitRow]) -> Vec<Candidate> {
    accounts
        .iter()
        .filter(|a| transactions::takes_transactions(&a.account_type))
        .map(|a| (a.id, a.name.clone(), unit_code_of(units, a.unit)))
        .collect()
}

fn unit_ids(units: &[UnitRow]) -> Vec<(String, UnitId)> {
    units
        .iter()
        .map(|unit| (unit.code.clone(), unit.id))
        .collect()
}

impl BudgetOptions {
    pub fn new(accounts: &[Account], units: &[UnitRow], unit: Option<&str>) -> Self {
        Self::of(&candidates(accounts, units), unit)
    }

    fn of(takes: &[Candidate], unit: Option<&str>) -> Self {
        let mut units: Vec<String> = takes.iter().map(|(_, _, unit)| unit.clone()).collect();
        units.sort();
        units.dedup();
        Self {
            units,
            accounts: takes
                .iter()
                .filter(|(_, _, each)| Some(each.as_str()) == unit)
                .map(|(id, name, _)| (*id, name.clone()))
                .collect(),
        }
    }

    fn account_ids(&self) -> Vec<u32> {
        self.accounts.iter().map(|(id, _)| *id).collect()
    }
}

/// What the dialog submits.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetDraft {
    Create(NewBudget),
    Edit {
        id: u32,
        name: String,
        account_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetForm {
    /// The Budget being edited; `None` while creating one.
    pub editing: Option<u32>,
    pub name: TextField,
    pub unit: SelectState,
    /// The on-budget Accounts, in no particular order.
    pub account_ids: Vec<u32>,
    /// The keyboard cursor among the Account chips.
    pub account_cursor: usize,
    pub start_from: StartChoice,
    /// The Budget "Copy categories from" names: the one on show when the dialog opened, with its
    /// Unit.
    pub copy_source: Option<(u32, String)>,
    /// The Accounts that take Transactions, copied in as the Dialog opens: the Unit select's
    /// options and the Accounts on offer are worked out from them as the Unit changes.
    takes_transactions: Vec<Candidate>,
    /// Every Unit's code and id, copied in as the Dialog opens: the Unit select stores the code,
    /// the Budget the id.
    unit_ids: Vec<(String, UnitId)>,
    pub focused: BudgetField,
    /// A refused save, shown until the next change.
    pub error: Option<BudgetError>,
}

impl BudgetForm {
    /// A new Budget in `source`'s Unit (or the first Unit on offer), every Account preselected.
    pub fn new(accounts: &[Account], unit_rows: &[UnitRow], source: Option<&Budget>) -> Self {
        let units = BudgetOptions::new(accounts, unit_rows, None).units;
        let unit = source
            .map(|budget| unit_code_of(unit_rows, budget.unit))
            .filter(|unit| units.contains(unit))
            .or_else(|| units.first().cloned());
        let options = BudgetOptions::new(accounts, unit_rows, unit.as_deref());
        Self {
            editing: None,
            name: TextField::default(),
            unit: SelectState::new(unit),
            account_ids: options.account_ids(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: source.map(|budget| (budget.id, unit_code_of(unit_rows, budget.unit))),
            takes_transactions: candidates(accounts, unit_rows),
            unit_ids: unit_ids(unit_rows),
            focused: BudgetField::Name,
            error: None,
        }
    }

    /// Edit mode, prefilled from `budget`.
    pub fn from_budget(budget: &Budget, accounts: &[Account], unit_rows: &[UnitRow]) -> Self {
        Self {
            editing: Some(budget.id),
            name: TextField::new(budget.name.clone()),
            unit: SelectState::new(Some(unit_code_of(unit_rows, budget.unit))),
            account_ids: budget.account_ids.clone(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: None,
            takes_transactions: candidates(accounts, unit_rows),
            unit_ids: unit_ids(unit_rows),
            focused: BudgetField::Name,
            error: None,
        }
    }

    pub fn is_edit(&self) -> bool {
        self.editing.is_some()
    }

    pub fn unit_code(&self) -> Option<&str> {
        self.unit.value()
    }

    fn unit_id(&self) -> Option<UnitId> {
        let code = self.unit_code()?;
        self.unit_ids
            .iter()
            .find(|(unit, _)| unit == code)
            .map(|(_, id)| *id)
    }

    /// The Unit select's options and the Accounts on offer for the form's Unit.
    pub fn options(&self) -> BudgetOptions {
        BudgetOptions::of(&self.takes_transactions, self.unit_code())
    }

    /// Whether Copy can be chosen: there is a source Budget and it is in the form's Unit.
    pub fn can_copy(&self) -> bool {
        self.copy_source
            .as_ref()
            .is_some_and(|(_, unit)| Some(unit.as_str()) == self.unit_code())
    }

    fn stops(&self) -> Vec<BudgetField> {
        if self.is_edit() {
            vec![BudgetField::Name, BudgetField::Accounts]
        } else {
            vec![
                BudgetField::Name,
                BudgetField::Unit,
                BudgetField::Accounts,
                BudgetField::StartFrom,
            ]
        }
    }

    pub fn focus(&mut self, field: BudgetField) {
        if !self.stops().contains(&field) {
            return;
        }
        if field != BudgetField::Unit {
            self.unit.cancel();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift+Tab`, wrapping.
    pub fn cycle_focus(&mut self, back: bool) {
        let stops = self.stops();
        let at = stops.iter().position(|f| *f == self.focused).unwrap_or(0);
        let next = if back {
            (at + stops.len() - 1) % stops.len()
        } else {
            (at + 1) % stops.len()
        };
        self.unit.cancel();
        self.focused = stops[next];
    }

    /// A key on the Unit select. Returns whether it is focused (and so took the key); a change
    /// re-selects the Accounts for the new Unit.
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        if self.focused != BudgetField::Unit || self.is_edit() {
            return false;
        }
        let list = &self.options().units;
        match (key, self.unit.is_open()) {
            (SelectKey::Up, true) => self.unit.move_highlight(list, -1),
            (SelectKey::Down, true) => self.unit.move_highlight(list, 1),
            (SelectKey::Up, false) => self.unit.step(list, -1),
            (SelectKey::Down, false) => self.unit.step(list, 1),
            (SelectKey::Activate, true) => self.unit.commit(list),
            (SelectKey::Activate, false) => self.unit.open(list),
        }
        self.error = None;
        self.sync_unit();
        true
    }

    /// A click on the Unit field: focuses it and toggles its list.
    pub fn click_unit(&mut self) {
        if self.is_edit() {
            return;
        }
        self.focus(BudgetField::Unit);
        if self.unit.is_open() {
            self.unit.cancel();
        } else {
            self.unit.open(&self.options().units);
        }
    }

    /// A click on row `index` of the open Unit list.
    pub fn choose_unit(&mut self, index: usize) {
        if !self.is_edit() {
            self.unit.choose(&self.options().units, index);
            self.error = None;
            self.sync_unit();
        }
    }

    /// After a Unit change: when the selection holds an Account outside the Unit's list, every
    /// Account in the Unit is selected again, and Copy falls back to Empty once it no longer
    /// applies.
    fn sync_unit(&mut self) {
        let offered = self.options().account_ids();
        if self.account_ids.iter().any(|id| !offered.contains(id)) {
            self.account_ids = offered;
            self.account_cursor = 0;
        }
        if self.start_from == StartChoice::Copy && !self.can_copy() {
            self.start_from = StartChoice::Empty;
        }
    }

    /// `left`/`right` on the Accounts chips or Start from, stopping at either end. Returns
    /// whether one of them is focused.
    pub fn step(&mut self, forward: bool) -> bool {
        match self.focused {
            BudgetField::Accounts => {
                let last = self.options().accounts.len().saturating_sub(1);
                self.account_cursor = if forward {
                    (self.account_cursor + 1).min(last)
                } else {
                    self.account_cursor.min(last).saturating_sub(1)
                };
            }
            BudgetField::StartFrom => {
                let choices: Vec<StartChoice> = START_CHOICES
                    .into_iter()
                    .filter(|choice| *choice != StartChoice::Copy || self.can_copy())
                    .collect();
                let at = choices
                    .iter()
                    .position(|c| *c == self.start_from)
                    .unwrap_or(0);
                let next = if forward {
                    (at + 1).min(choices.len() - 1)
                } else {
                    at.saturating_sub(1)
                };
                self.start_from = choices[next];
                self.error = None;
            }
            _ => return false,
        }
        true
    }

    /// `space` on the Accounts chips: toggles the one under the cursor.
    pub fn toggle_cursor_account(&mut self) {
        if let Some((id, _)) = self.options().accounts.get(self.account_cursor) {
            self.toggle_account(*id);
        }
    }

    /// A click on an Account chip.
    pub fn toggle_account(&mut self, id: u32) {
        self.focus(BudgetField::Accounts);
        if let Some(at) = self
            .options()
            .accounts
            .iter()
            .position(|(each, _)| *each == id)
        {
            self.account_cursor = at;
        }
        if let Some(at) = self.account_ids.iter().position(|each| *each == id) {
            self.account_ids.remove(at);
        } else {
            self.account_ids.push(id);
        }
        self.error = None;
    }

    /// A click on a Start from cell; Copy is ignored while it doesn't apply.
    pub fn set_start_from(&mut self, choice: StartChoice) {
        if self.is_edit() || (choice == StartChoice::Copy && !self.can_copy()) {
            return;
        }
        self.focus(BudgetField::StartFrom);
        self.start_from = choice;
        self.error = None;
    }

    /// A Budget needs a name, a Unit and at least one on-budget Account.
    pub fn is_valid(&self) -> bool {
        !self.name.is_blank() && self.unit_code().is_some() && !self.account_ids.is_empty()
    }

    /// What confirm submits, or `None` while the form is incomplete.
    pub fn draft(&self) -> Option<BudgetDraft> {
        if !self.is_valid() {
            return None;
        }
        let name = self.name.text().trim().to_string();
        Some(match self.editing {
            Some(id) => BudgetDraft::Edit {
                id,
                name,
                account_ids: self.account_ids.clone(),
            },
            None => BudgetDraft::Create(NewBudget {
                name,
                unit: self.unit_id()?,
                account_ids: self.account_ids.clone(),
                start_from: match (self.start_from, &self.copy_source) {
                    (StartChoice::Copy, Some((source, _))) if self.can_copy() => {
                        StartFrom::Copy(*source)
                    }
                    (StartChoice::LastThreeMonths, _) => StartFrom::LastThreeMonths,
                    _ => StartFrom::Empty,
                },
            }),
        })
    }
}

impl Dialog for BudgetForm {
    /// `Shift-Tab` goes back a field. On the Unit select `↑`/`↓` step or move the highlight,
    /// `Space` opens or commits its list and so does `Enter` while the list is open (otherwise
    /// `Enter` confirms). `←`/`→` walk the Account chips and Start from; `Space` toggles the
    /// chip under the cursor. The Unit and Start from take no text. Any change clears a stale
    /// Save error.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let focused = self.focused;
        let on_name = focused == BudgetField::Name;
        match key {
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Up if focused == BudgetField::Unit => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down if focused == BudgetField::Unit => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::Char(' ') if focused == BudgetField::Unit => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Enter if self.unit.is_open() => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Char(' ') if focused == BudgetField::Accounts => {
                self.toggle_cursor_account();
            }
            DialogKey::Left | DialogKey::Right if !on_name => {
                self.step(key == DialogKey::Right);
            }
            DialogKey::Char(_) | DialogKey::Backspace if on_name => {
                self.error = None;
                return None;
            }
            DialogKey::Char(_) | DialogKey::Backspace => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        (self.focused == BudgetField::Name).then_some(&mut self.name)
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        BudgetForm::is_valid(self)
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.unit.is_open();
        self.unit.cancel();
        open
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::budgets::{self, Budgets};
    use lib_accounts::default_accounts;
    use lib_categories::default_categories;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    }

    fn seeded() -> (Vec<Account>, Budgets) {
        let accounts = default_accounts();
        let budgets = budgets::default_budgets(&accounts, &default_categories(), today());
        (accounts, budgets)
    }

    fn unit_code(id: UnitId) -> &'static str {
        match id {
            UnitId::SEED_AUD => "aud",
            UnitId::SEED_BTC => "btc",
            _ => "vas",
        }
    }

    fn personal(budgets: &Budgets) -> &Budget {
        budgets.get(budgets::PERSONAL_SPENDING_ID).unwrap()
    }

    #[test]
    fn a_new_form_takes_the_source_unit_and_preselects_its_accounts() {
        let (accounts, budgets) = seeded();
        let form = BudgetForm::new(
            &accounts,
            &lib_units::default_units(),
            Some(personal(&budgets)),
        );
        let options = form.options();
        assert_eq!(form.unit_code(), Some(unit_code(personal(&budgets).unit)));
        assert!(!options.accounts.is_empty());
        let mut selected = form.account_ids.clone();
        selected.sort_unstable();
        let mut offered = options.account_ids();
        offered.sort_unstable();
        assert_eq!(selected, offered);
        for (id, _) in &options.accounts {
            let account = accounts.iter().find(|a| a.id == *id).unwrap();
            assert!(transactions::takes_transactions(&account.account_type));
            assert_eq!(Some(unit_code(account.unit)), form.unit_code());
        }
        assert!(form.can_copy());
        assert!(!form.is_valid(), "no name yet");
    }

    #[test]
    fn the_draft_carries_the_name_accounts_and_start_from() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(
            &accounts,
            &lib_units::default_units(),
            Some(personal(&budgets)),
        );
        form.name = TextField::new("  Holiday ");
        form.focus(BudgetField::StartFrom);
        form.step(true);
        assert_eq!(form.start_from, StartChoice::Copy);
        let Some(BudgetDraft::Create(new)) = form.draft() else {
            panic!("a complete form drafts a new Budget");
        };
        assert_eq!(new.name, "Holiday");
        assert_eq!(
            new.start_from,
            StartFrom::Copy(budgets::PERSONAL_SPENDING_ID)
        );
        assert_eq!(new.account_ids, form.account_ids);

        form.step(true);
        form.step(true);
        assert_eq!(form.start_from, StartChoice::LastThreeMonths);
        let Some(BudgetDraft::Create(new)) = form.draft() else {
            panic!("a complete form drafts a new Budget");
        };
        assert_eq!(new.start_from, StartFrom::LastThreeMonths);
    }

    #[test]
    fn accounts_toggle_by_cursor_and_an_empty_set_is_invalid() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(
            &accounts,
            &lib_units::default_units(),
            Some(personal(&budgets)),
        );
        let options = form.options();
        form.name = TextField::new("Holiday");
        form.focus(BudgetField::Accounts);
        let first = options.accounts[0].0;
        form.toggle_cursor_account();
        assert!(!form.account_ids.contains(&first));
        form.toggle_cursor_account();
        assert!(form.account_ids.contains(&first));

        form.step(false);
        assert_eq!(form.account_cursor, 0, "stops at the first chip");
        for _ in 0..options.accounts.len() + 2 {
            form.step(true);
        }
        assert_eq!(form.account_cursor, options.accounts.len() - 1);

        for (id, _) in &options.accounts {
            if form.account_ids.contains(id) {
                form.toggle_account(*id);
            }
        }
        assert!(!form.is_valid());
        assert_eq!(form.draft(), None);
    }

    #[test]
    fn changing_the_unit_reselects_accounts_and_drops_copy() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(
            &accounts,
            &lib_units::default_units(),
            Some(personal(&budgets)),
        );
        let all = BudgetOptions::new(&accounts, &lib_units::default_units(), None);
        let Some(other) = all
            .units
            .iter()
            .find(|unit| Some(unit.as_str()) != form.unit_code())
            .cloned()
        else {
            // The seed keeps every Transaction Account in one Unit: nothing to switch to.
            return;
        };
        form.set_start_from(StartChoice::Copy);
        form.focus(BudgetField::Unit);
        let at = all.units.iter().position(|unit| *unit == other).unwrap();
        form.choose_unit(at);
        let options = form.options();
        assert_eq!(form.unit_code(), Some(other.as_str()));
        let mut selected = form.account_ids.clone();
        selected.sort_unstable();
        let mut offered = options.account_ids();
        offered.sort_unstable();
        assert_eq!(selected, offered);
        assert!(!form.can_copy());
        assert_eq!(form.start_from, StartChoice::Empty);
        form.set_start_from(StartChoice::Copy);
        assert_eq!(form.start_from, StartChoice::Empty);
    }

    #[test]
    fn edit_mode_locks_the_unit_and_drafts_a_rename() {
        let (accounts, budgets) = seeded();
        let budget = personal(&budgets);
        let mut form = BudgetForm::from_budget(budget, &accounts, &lib_units::default_units());
        assert!(form.is_edit());
        assert_eq!(form.name.text(), budget.name);
        assert_eq!(form.account_ids, budget.account_ids);

        for _ in 0..4 {
            form.cycle_focus(false);
            assert!(matches!(
                form.focused,
                BudgetField::Name | BudgetField::Accounts
            ));
        }
        form.focus(BudgetField::Unit);
        assert_ne!(form.focused, BudgetField::Unit);
        assert!(!form.handle_select_key(SelectKey::Down));
        form.click_unit();
        assert!(!form.unit.is_open());

        form.focus(BudgetField::Name);
        form.name.backspace();
        form.name.push('!');
        let Some(BudgetDraft::Edit { id, name, .. }) = form.draft() else {
            panic!("edit mode drafts an edit");
        };
        assert_eq!(id, budget.id);
        assert!(name.ends_with('!'));
    }

    use crate::budgets::BudgetsDialog;
    use crate::chrome::dialog_host::type_text;
    use crate::chrome::dialog_host::{DialogKey, DialogOutcome, OpenDialog, handle_key};

    fn budgets_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    }

    fn budgets_world() -> (Vec<Account>, Vec<lib_categories::Category>, Budgets) {
        let accounts = default_accounts();
        let categories = default_categories();
        let budgets = budgets::default_budgets(&accounts, &categories, budgets_today());
        (accounts, categories, budgets)
    }

    fn budgets_dialog(dialog: BudgetsDialog) -> OpenDialog {
        OpenDialog::Budgets(Box::new(dialog))
    }

    fn budget_form_of(dialog: &OpenDialog) -> &BudgetForm {
        match dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::Budget(form) => form,
                other => panic!("expected the Budget form, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        }
    }

    fn new_budget() -> OpenDialog {
        let (accounts, _, budgets) = budgets_world();
        let source = budgets.get(budgets::PERSONAL_SPENDING_ID);
        budgets_dialog(BudgetsDialog::Budget(BudgetForm::new(
            &accounts,
            &lib_units::default_units(),
            source,
        )))
    }

    #[test]
    fn budget_form_types_into_the_name_only_and_tab_moves_on() {
        let mut dialog = new_budget();
        type_text(&mut dialog, "Holiday");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(budget_form_of(&dialog).name.text(), "Holida");

        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(budget_form_of(&dialog).focused, BudgetField::Unit);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Handled,
            "the Unit select swallows text"
        );
        assert_eq!(budget_form_of(&dialog).name.text(), "Holida");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(budget_form_of(&dialog).focused, BudgetField::Name);
    }

    #[test]
    fn budget_form_confirms_only_with_a_name_and_a_select_takes_enter_while_open() {
        let mut dialog = new_budget();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Holiday");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(budget_form_of(&dialog).unit.is_open());
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled,
            "Enter commits the open list"
        );
        assert!(!budget_form_of(&dialog).unit.is_open());
    }

    #[test]
    fn budget_form_first_esc_closes_only_the_open_unit_list() {
        let mut dialog = new_budget();
        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
    }

    fn limit_options(count: usize) -> budgets::limit_form::LimitOptions {
        let (_, categories, mut budgets) = budgets_world();
        let today = budgets_today();
        // The seed budgets every Expense leaf, so two are Stopped for the picker to offer.
        for name in ["Household", "Water"] {
            let id = lib_categories::find_by_name(&categories, name).unwrap();
            budgets
                .stop(
                    budgets::PERSONAL_SPENDING_ID,
                    &categories,
                    id,
                    crate::period::Period::of(today),
                    today,
                )
                .unwrap();
        }
        let budget = budgets.get(budgets::PERSONAL_SPENDING_ID).unwrap();
        budgets::limit_form::LimitOptions::new(
            budget,
            &categories,
            crate::period::Period::of(today),
            count,
            |month, _| format!("{}-{}", month.year, month.month),
        )
    }

    fn limit_form_of(dialog: &OpenDialog) -> &budgets::limit_form::LimitForm {
        match dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::EditLimit(form) => form,
                other => panic!("expected Edit budget, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        }
    }

    fn limit_picker() -> OpenDialog {
        budgets_dialog(BudgetsDialog::EditLimit(
            budgets::limit_form::LimitForm::pick(
                None,
                &limit_options(budgets::limit_form::START_MONTHS),
            ),
        ))
    }

    #[test]
    fn limit_form_amount_takes_a_decimal_only_and_tab_moves_on() {
        let mut dialog = limit_picker();
        // The picker opens on its Category select, which takes no text.
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('5')),
            DialogOutcome::Handled
        );
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            limit_form_of(&dialog).focused,
            budgets::limit_form::LimitField::Amount
        );
        assert_eq!(limit_form_of(&dialog).amount.text(), "");

        type_text_filtered(&mut dialog, "12.5.0x");
        assert_eq!(limit_form_of(&dialog).amount.text(), "12.50");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(limit_form_of(&dialog).amount.text(), "12.5");
    }

    /// Types `text`, letting a Dialog refuse characters it doesn't take.
    fn type_text_filtered(dialog: &mut OpenDialog, text: &str) {
        for ch in text.chars() {
            handle_key(dialog, DialogKey::Char(ch));
        }
    }

    #[test]
    fn limit_form_confirms_only_with_a_category_and_an_amount() {
        let mut dialog = limit_picker();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled,
            "nothing to confirm yet"
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select(), "Space opened the Category list");
        handle_key(&mut dialog, DialogKey::Down);
        handle_key(&mut dialog, DialogKey::Tab);
        assert!(!dialog.is_valid(), "no amount yet");
        type_text(&mut dialog, "80");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn stop_form_steps_months_and_the_first_esc_closes_its_list() {
        let options = limit_options(budgets::limit_form::STOP_MONTHS);
        let form = budgets::limit_form::StopForm::new(
            1,
            crate::period::Period::of(budgets_today()),
            &options,
        );
        let mut dialog = budgets_dialog(BudgetsDialog::Stop(form));
        handle_key(&mut dialog, DialogKey::Char('j'));
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    fn switcher_of(dialog: &OpenDialog) -> &budgets::Switcher {
        match dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::Switcher(switcher) => switcher,
                other => panic!("expected the Switcher, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        }
    }

    fn switcher() -> OpenDialog {
        let (_, _, budgets) = budgets_world();
        budgets_dialog(BudgetsDialog::Switcher(budgets::Switcher::new(
            &budgets,
            budgets::PERSONAL_SPENDING_ID,
        )))
    }

    #[test]
    fn switcher_search_takes_text_until_esc_gives_the_keys_back_to_the_list() {
        let mut dialog = switcher();
        // With the list focused, `x` is nobody's key.
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Ignored
        );
        handle_key(&mut dialog, DialogKey::Char('/'));
        assert!(switcher_of(&dialog).searching);
        type_text(&mut dialog, "per");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(switcher_of(&dialog).query.text(), "pe");
        assert_eq!(
            switcher_of(&dialog).chosen(),
            Some(budgets::PERSONAL_SPENDING_ID)
        );

        assert!(
            dialog.close_open_select(),
            "the first Esc leaves the search"
        );
        assert!(!switcher_of(&dialog).searching);
        assert!(!dialog.close_open_select(), "the second closes the popover");
    }

    #[test]
    fn switcher_enter_opens_the_highlight_only_while_a_row_matches_and_n_asks_for_a_new_budget() {
        let mut dialog = switcher();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        handle_key(&mut dialog, DialogKey::Char('/'));
        type_text(&mut dialog, "no such budget");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled,
            "no row, so nothing to open"
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );

        let mut dialog = switcher();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('n')),
            DialogOutcome::Confirm
        );
        assert_eq!(switcher_of(&dialog).request, budgets::SwitcherRequest::New);
    }

    fn manage() -> OpenDialog {
        let (_, _, budgets) = budgets_world();
        budgets_dialog(BudgetsDialog::Manage(budgets::Manage::new(
            &budgets,
            budgets::PERSONAL_SPENDING_ID,
        )))
    }

    fn manage_request(dialog: &OpenDialog) -> Option<budgets::ManageRequest> {
        match dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::Manage(manage) => manage.request,
                other => panic!("expected Manage, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        }
    }

    #[test]
    fn manage_keys_move_the_cursor_or_ask_shell_for_an_action_on_the_row() {
        use crate::budgets::{ManageAction, ManageRequest, PERSONAL_SPENDING_ID};
        let mut dialog = manage();
        assert!(!dialog.is_valid(), "nothing asked yet");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        assert_eq!(
            manage_request(&dialog),
            Some(ManageRequest::Action(
                PERSONAL_SPENDING_ID,
                ManageAction::Open
            ))
        );

        let mut dialog = manage();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Confirm
        );
        assert_eq!(
            manage_request(&dialog),
            Some(ManageRequest::Action(
                PERSONAL_SPENDING_ID,
                ManageAction::Archive
            ))
        );

        let mut dialog = manage();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('n')),
            DialogOutcome::Confirm
        );
        assert_eq!(manage_request(&dialog), Some(ManageRequest::New));

        let mut dialog = manage();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('q')),
            DialogOutcome::Ignored
        );
    }

    #[test]
    fn category_detail_moves_its_cursor_and_hands_off_on_t_enter_or_e() {
        use crate::budgets::{Detail, DetailRequest};
        let month = crate::period::Period::of(budgets_today());
        let mut dialog = budgets_dialog(BudgetsDialog::CategoryDetail(Detail::new(1, month, 3)));
        handle_key(&mut dialog, DialogKey::Char('j'));
        handle_key(&mut dialog, DialogKey::Down);
        handle_key(&mut dialog, DialogKey::Down);
        handle_key(&mut dialog, DialogKey::Char('k'));
        let selected = match &dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::CategoryDetail(detail) => detail.selected,
                other => panic!("expected the detail, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        };
        assert_eq!(selected, 1, "stops at the last listed row, then steps back");

        assert!(!dialog.is_valid());
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('e')),
            DialogOutcome::Confirm
        );
        let request = match &dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::CategoryDetail(detail) => detail.request,
                _ => None,
            },
            _ => None,
        };
        assert_eq!(request, Some(DetailRequest::Edit));
    }

    #[test]
    fn fill_picks_its_source_with_j_k_and_confirms_on_enter() {
        use crate::budgets::FillSource;
        let month = crate::period::Period::of(budgets_today());
        let mut dialog = budgets_dialog(BudgetsDialog::Fill {
            month,
            source: FillSource::PreviousMonth,
        });
        handle_key(&mut dialog, DialogKey::Char('j'));
        let source = |dialog: &OpenDialog| match dialog {
            OpenDialog::Budgets(inner) => match &**inner {
                BudgetsDialog::Fill { source, .. } => *source,
                other => panic!("expected Fill, got {other:?}"),
            },
            other => panic!("expected Budgets, got {other:?}"),
        };
        assert_eq!(source(&dialog), FillSource::Average);
        handle_key(&mut dialog, DialogKey::Char('k'));
        assert_eq!(source(&dialog), FillSource::PreviousMonth);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    // -- Documents ------------------------------------------------------------------------------
}
