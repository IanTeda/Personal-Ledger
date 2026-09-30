//! The live form behind **New budget** (11c in `docs/ux/desktop/Budgets_v2/README.md`) and its
//! edit mode: `gpui`-free and unit-tested, the same split `bill_form.rs` uses.
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

use crate::{
    accounts::{Account, SelectKey},
    budgets::{Budget, BudgetError, NewBudget, StartFrom},
    select::SelectState,
    transactions,
};

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

impl BudgetOptions {
    pub fn new(accounts: &[Account], unit: Option<&str>) -> Self {
        let takes: Vec<&Account> = accounts
            .iter()
            .filter(|a| transactions::takes_transactions(&a.account_type))
            .collect();
        let mut units: Vec<String> = takes.iter().map(|a| a.unit.clone()).collect();
        units.sort();
        units.dedup();
        Self {
            units,
            accounts: takes
                .iter()
                .filter(|a| Some(a.unit.as_str()) == unit)
                .map(|a| (a.id, a.name.clone()))
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
    pub name: String,
    pub unit: SelectState,
    /// The on-budget Accounts, in no particular order.
    pub account_ids: Vec<u32>,
    /// The keyboard cursor among the Account chips.
    pub account_cursor: usize,
    pub start_from: StartChoice,
    /// The Budget "Copy categories from" names: the one on show when the dialog opened, with its
    /// Unit.
    pub copy_source: Option<(u32, String)>,
    pub focused: BudgetField,
    /// A refused save, shown until the next change.
    pub error: Option<BudgetError>,
}

impl BudgetForm {
    /// A new Budget in `source`'s Unit (or the first Unit on offer), every Account preselected.
    pub fn new(accounts: &[Account], source: Option<&Budget>) -> Self {
        let units = BudgetOptions::new(accounts, None).units;
        let unit = source
            .map(|budget| budget.unit.clone())
            .filter(|unit| units.contains(unit))
            .or_else(|| units.first().cloned());
        let options = BudgetOptions::new(accounts, unit.as_deref());
        Self {
            editing: None,
            name: String::new(),
            unit: SelectState::new(unit),
            account_ids: options.account_ids(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: source.map(|budget| (budget.id, budget.unit.clone())),
            focused: BudgetField::Name,
            error: None,
        }
    }

    /// Edit mode, prefilled from `budget`.
    pub fn from_budget(budget: &Budget) -> Self {
        Self {
            editing: Some(budget.id),
            name: budget.name.clone(),
            unit: SelectState::new(Some(budget.unit.clone())),
            account_ids: budget.account_ids.clone(),
            account_cursor: 0,
            start_from: StartChoice::Empty,
            copy_source: None,
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

    /// Closes the Unit list, for the first `Esc`. Returns whether it was open.
    pub fn close_open_select(&mut self) -> bool {
        let open = self.unit.is_open();
        self.unit.cancel();
        open
    }

    pub fn push_char(&mut self, ch: char) {
        if self.focused == BudgetField::Name && !ch.is_control() {
            self.name.push(ch);
            self.error = None;
        }
    }

    pub fn backspace(&mut self) {
        if self.focused == BudgetField::Name {
            self.name.pop();
            self.error = None;
        }
    }

    /// A key on the Unit select. Returns whether it is focused (and so took the key). The caller
    /// follows a change with [`Self::sync_unit`].
    pub fn handle_select_key(&mut self, key: SelectKey, options: &BudgetOptions) -> bool {
        if self.focused != BudgetField::Unit || self.is_edit() {
            return false;
        }
        let list = &options.units;
        match (key, self.unit.is_open()) {
            (SelectKey::Up, true) => self.unit.move_highlight(list, -1),
            (SelectKey::Down, true) => self.unit.move_highlight(list, 1),
            (SelectKey::Up, false) => self.unit.step(list, -1),
            (SelectKey::Down, false) => self.unit.step(list, 1),
            (SelectKey::Activate, true) => self.unit.commit(list),
            (SelectKey::Activate, false) => self.unit.open(list),
        }
        self.error = None;
        true
    }

    /// A click on the Unit field: focuses it and toggles its list.
    pub fn click_unit(&mut self, options: &BudgetOptions) {
        if self.is_edit() {
            return;
        }
        self.focus(BudgetField::Unit);
        if self.unit.is_open() {
            self.unit.cancel();
        } else {
            self.unit.open(&options.units);
        }
    }

    /// A click on row `index` of the open Unit list.
    pub fn choose_unit(&mut self, index: usize, options: &BudgetOptions) {
        if !self.is_edit() {
            self.unit.choose(&options.units, index);
            self.error = None;
        }
    }

    /// After a Unit change: when the selection holds an Account outside the Unit's list, every
    /// Account in the Unit is selected again, and Copy falls back to Empty once it no longer
    /// applies. `options` is built for the form's current Unit.
    pub fn sync_unit(&mut self, options: &BudgetOptions) {
        let offered = options.account_ids();
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
    pub fn step(&mut self, forward: bool, options: &BudgetOptions) -> bool {
        match self.focused {
            BudgetField::Accounts => {
                let last = options.accounts.len().saturating_sub(1);
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
    pub fn toggle_cursor_account(&mut self, options: &BudgetOptions) {
        if let Some((id, _)) = options.accounts.get(self.account_cursor) {
            self.toggle_account(*id, options);
        }
    }

    /// A click on an Account chip.
    pub fn toggle_account(&mut self, id: u32, options: &BudgetOptions) {
        self.focus(BudgetField::Accounts);
        if let Some(at) = options.accounts.iter().position(|(each, _)| *each == id) {
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
        !self.name.trim().is_empty() && self.unit_code().is_some() && !self.account_ids.is_empty()
    }

    /// What confirm submits, or `None` while the form is incomplete.
    pub fn draft(&self) -> Option<BudgetDraft> {
        if !self.is_valid() {
            return None;
        }
        let name = self.name.trim().to_string();
        Some(match self.editing {
            Some(id) => BudgetDraft::Edit {
                id,
                name,
                account_ids: self.account_ids.clone(),
            },
            None => BudgetDraft::Create(NewBudget {
                name,
                unit: self.unit_code()?.to_string(),
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        accounts::default_accounts,
        budgets::{self, Budgets},
        categories::default_categories,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    }

    fn seeded() -> (Vec<Account>, Budgets) {
        let accounts = default_accounts();
        let budgets = budgets::default_budgets(&accounts, &default_categories(), today());
        (accounts, budgets)
    }

    fn personal(budgets: &Budgets) -> &Budget {
        budgets.get(budgets::PERSONAL_SPENDING_ID).unwrap()
    }

    fn options(accounts: &[Account], form: &BudgetForm) -> BudgetOptions {
        BudgetOptions::new(accounts, form.unit_code())
    }

    #[test]
    fn a_new_form_takes_the_source_unit_and_preselects_its_accounts() {
        let (accounts, budgets) = seeded();
        let form = BudgetForm::new(&accounts, Some(personal(&budgets)));
        let options = options(&accounts, &form);
        assert_eq!(form.unit_code(), Some(personal(&budgets).unit.as_str()));
        assert!(!options.accounts.is_empty());
        let mut selected = form.account_ids.clone();
        selected.sort_unstable();
        let mut offered = options.account_ids();
        offered.sort_unstable();
        assert_eq!(selected, offered);
        for (id, _) in &options.accounts {
            let account = accounts.iter().find(|a| a.id == *id).unwrap();
            assert!(transactions::takes_transactions(&account.account_type));
            assert_eq!(Some(account.unit.as_str()), form.unit_code());
        }
        assert!(form.can_copy());
        assert!(!form.is_valid(), "no name yet");
    }

    #[test]
    fn the_draft_carries_the_name_accounts_and_start_from() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(&accounts, Some(personal(&budgets)));
        let options = options(&accounts, &form);
        for ch in "  Holiday ".chars() {
            form.push_char(ch);
        }
        form.focus(BudgetField::StartFrom);
        form.step(true, &options);
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

        form.step(true, &options);
        form.step(true, &options);
        assert_eq!(form.start_from, StartChoice::LastThreeMonths);
        let Some(BudgetDraft::Create(new)) = form.draft() else {
            panic!("a complete form drafts a new Budget");
        };
        assert_eq!(new.start_from, StartFrom::LastThreeMonths);
    }

    #[test]
    fn accounts_toggle_by_cursor_and_an_empty_set_is_invalid() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(&accounts, Some(personal(&budgets)));
        let options = options(&accounts, &form);
        form.name = "Holiday".to_string();
        form.focus(BudgetField::Accounts);
        let first = options.accounts[0].0;
        form.toggle_cursor_account(&options);
        assert!(!form.account_ids.contains(&first));
        form.toggle_cursor_account(&options);
        assert!(form.account_ids.contains(&first));

        form.step(false, &options);
        assert_eq!(form.account_cursor, 0, "stops at the first chip");
        for _ in 0..options.accounts.len() + 2 {
            form.step(true, &options);
        }
        assert_eq!(form.account_cursor, options.accounts.len() - 1);

        for (id, _) in &options.accounts {
            if form.account_ids.contains(id) {
                form.toggle_account(*id, &options);
            }
        }
        assert!(!form.is_valid());
        assert_eq!(form.draft(), None);
    }

    #[test]
    fn changing_the_unit_reselects_accounts_and_drops_copy() {
        let (accounts, budgets) = seeded();
        let mut form = BudgetForm::new(&accounts, Some(personal(&budgets)));
        let all = BudgetOptions::new(&accounts, None);
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
        form.choose_unit(at, &all);
        let options = options(&accounts, &form);
        form.sync_unit(&options);
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
        let mut form = BudgetForm::from_budget(budget);
        let options = options(&accounts, &form);
        assert!(form.is_edit());
        assert_eq!(form.name, budget.name);
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
        assert!(!form.handle_select_key(SelectKey::Down, &options));
        form.click_unit(&options);
        assert!(!form.unit.is_open());

        form.focus(BudgetField::Name);
        form.backspace();
        form.push_char('!');
        let Some(BudgetDraft::Edit { id, name, .. }) = form.draft() else {
            panic!("edit mode drafts an edit");
        };
        assert_eq!(id, budget.id);
        assert!(name.ends_with('!'));
    }
}
