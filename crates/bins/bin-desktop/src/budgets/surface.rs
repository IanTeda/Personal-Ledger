//! The Budgets surface's own state: its tabs, the Switcher, Manage budgets, Category detail and
//! the open dialog.

use crate::{
    accounts,
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    period::Period,
};

use super::form::BudgetForm;
use super::limit_form::{LimitForm, StopForm};
use super::plan::FillSource;
use super::store::{BudgetError, Budgets};

/// Which tab of the Budgets surface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetsTab {
    #[default]
    Progress,
    Plan,
    History,
}

impl BudgetsTab {
    /// `tab` cycles forward, wrapping.
    pub fn next(self) -> Self {
        match self {
            BudgetsTab::Progress => BudgetsTab::Plan,
            BudgetsTab::Plan => BudgetsTab::History,
            BudgetsTab::History => BudgetsTab::Progress,
        }
    }

    /// `1`/`2`/`3` pick a tab directly.
    pub fn from_digit(digit: char) -> Option<Self> {
        match digit {
            '1' => Some(BudgetsTab::Progress),
            '2' => Some(BudgetsTab::Plan),
            '3' => Some(BudgetsTab::History),
            _ => None,
        }
    }
}

/// What confirming the Switcher does: open the highlighted Budget, or start a new one (`n`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitcherRequest {
    #[default]
    Choose,
    New,
}

/// The Switcher popover's state (11b). The list takes `j`/`k`/`n` until `/` hands the keys to the
/// search field. The Budgets it lists are copied in as it opens, so its keys never need the
/// ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Switcher {
    pub query: TextField,
    pub searching: bool,
    /// Index into [`Self::ids`] for the query.
    pub selected: usize,
    /// Every Budget's id and name: active ones in creation order, then archived ones.
    entries: Vec<(u32, String)>,
    pub request: SwitcherRequest,
}

impl Switcher {
    /// Opens with `current` highlighted.
    pub fn new(budgets: &Budgets, current: u32) -> Self {
        let entries: Vec<(u32, String)> = budgets
            .active()
            .chain(budgets.archived())
            .map(|budget| (budget.id, budget.name.clone()))
            .collect();
        Self {
            selected: entries
                .iter()
                .position(|(id, _)| *id == current)
                .unwrap_or(0),
            entries,
            ..Self::default()
        }
    }

    /// The rows for the query, as [`switcher_ids`] gives them.
    pub fn ids(&self) -> Vec<u32> {
        let query = self.query.text().trim().to_lowercase();
        self.entries
            .iter()
            .filter(|(_, name)| query.is_empty() || name.to_lowercase().contains(&query))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Moves the highlight, stopping at either end of the matching rows.
    pub fn step(&mut self, forward: bool) {
        let last = self.ids().len().saturating_sub(1);
        self.selected = if forward {
            (self.selected + 1).min(last)
        } else {
            self.selected.min(last).saturating_sub(1)
        };
    }

    /// The highlighted Budget, if any row matches.
    pub fn chosen(&self) -> Option<u32> {
        let ids = self.ids();
        ids.get(self.selected.min(ids.len().saturating_sub(1)))
            .copied()
    }

    pub fn type_char(&mut self, c: char) {
        if !c.is_control() {
            self.query.push(c);
            self.selected = 0;
        }
    }

    pub fn backspace(&mut self) {
        self.query.backspace();
        self.selected = 0;
    }
}

impl Dialog for Switcher {
    /// With the list focused `j`/`k` move, `n` starts a new Budget and `/` hands the keys to the
    /// search field; while searching, every character narrows the list and only the arrows move.
    /// `Enter` opens the highlighted row.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let searching = self.searching;
        match key {
            DialogKey::Down => self.step(true),
            DialogKey::Up => self.step(false),
            DialogKey::Backspace if searching => self.backspace(),
            DialogKey::Char('j') if !searching => self.step(true),
            DialogKey::Char('k') if !searching => self.step(false),
            DialogKey::Char('/') if !searching => self.searching = true,
            DialogKey::Char('n') if !searching => {
                self.request = SwitcherRequest::New;
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Char(ch) if searching => self.type_char(ch),
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.request == SwitcherRequest::New || self.chosen().is_some()
    }

    /// The first `Esc` gives the keys back to the list before the popover closes.
    fn close_open_select(&mut self) -> bool {
        std::mem::take(&mut self.searching)
    }
}

/// The Switcher's rows: active Budgets in creation order, then archived ones, keeping those whose
/// name contains `query` whatever its case.
pub fn switcher_ids(budgets: &Budgets, query: &str) -> Vec<u32> {
    let query = query.trim().to_lowercase();
    budgets
        .active()
        .chain(budgets.archived())
        .filter(|budget| query.is_empty() || budget.name.to_lowercase().contains(&query))
        .map(|budget| budget.id)
        .collect()
}

/// What a Manage budgets row's action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAction {
    Open,
    Edit,
    Duplicate,
    SetDefault,
    Archive,
    Restore,
}

/// What a key in Manage budgets asks `Shell` to do once it confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageRequest {
    New,
    Action(u32, ManageAction),
}

/// 11f: the row cursor over the Switcher's rows, and the last refused action. The rows' ids and
/// whether each is archived are copied in as it opens and refreshed after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manage {
    pub selected: usize,
    pub error: Option<BudgetError>,
    rows: Vec<(u32, bool)>,
    pub request: Option<ManageRequest>,
}

impl Manage {
    /// Opens with Budget `current` under the cursor.
    pub fn new(budgets: &Budgets, current: u32) -> Self {
        let mut manage = Self {
            selected: 0,
            error: None,
            rows: Vec::new(),
            request: None,
        };
        manage.refresh(budgets, current);
        manage
    }

    /// Rebuilds the rows and follows `id` to where it now sits. Archiving or restoring moves a
    /// row between the two sections.
    pub fn refresh(&mut self, budgets: &Budgets, id: u32) {
        self.rows = budgets
            .active()
            .map(|budget| (budget.id, false))
            .chain(budgets.archived().map(|budget| (budget.id, true)))
            .collect();
        self.selected = self
            .rows
            .iter()
            .position(|(each, _)| *each == id)
            .unwrap_or(0);
    }
}

impl Dialog for Manage {
    /// `j`/`k` move, `enter` opens, `n` starts a new Budget, `e` edits, `d` duplicates, `*` sets
    /// the default and `x` archives or restores. Each but the moves is a request `Shell` runs.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let last = self.rows.len().checked_sub(1)?;
        let at = self.selected.min(last);
        let (id, archived) = self.rows[at];
        let action = match key {
            DialogKey::Down | DialogKey::Char('j') => {
                self.selected = (at + 1).min(last);
                return Some(DialogOutcome::Handled);
            }
            DialogKey::Up | DialogKey::Char('k') => {
                self.selected = at.saturating_sub(1);
                return Some(DialogOutcome::Handled);
            }
            DialogKey::Char('n') => {
                self.request = Some(ManageRequest::New);
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Enter => ManageAction::Open,
            DialogKey::Char('e') => ManageAction::Edit,
            DialogKey::Char('d') => ManageAction::Duplicate,
            DialogKey::Char('*') => ManageAction::SetDefault,
            DialogKey::Char('x') if archived => ManageAction::Restore,
            DialogKey::Char('x') => ManageAction::Archive,
            _ => return None,
        };
        self.request = Some(ManageRequest::Action(id, action));
        Some(DialogOutcome::Confirm)
    }

    fn is_valid(&self) -> bool {
        self.request.is_some()
    }
}

/// What a key on 9d asks `Shell` to do once it confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailRequest {
    /// `t` or `enter`: hand off to Transactions.
    Transactions,
    /// `e`: Edit budget.
    Edit,
}

/// 9d for a Category (a leaf or a parent's rollup) and month: the Transaction row cursor, over
/// `listed` rows counted as it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub category_id: u32,
    pub month: Period,
    pub selected: usize,
    pub listed: usize,
    pub request: Option<DetailRequest>,
}

impl Detail {
    pub fn new(category_id: u32, month: Period, listed: usize) -> Self {
        Self {
            category_id,
            month,
            selected: 0,
            listed,
            request: None,
        }
    }
}

impl Dialog for Detail {
    /// `j`/`k` move the cursor, `t` or `enter` hand off to Transactions and `e` opens Edit budget.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Down | DialogKey::Char('j') => {
                self.selected = accounts::step_selection(self.selected, self.listed, 1);
            }
            DialogKey::Up | DialogKey::Char('k') => {
                self.selected = accounts::step_selection(self.selected, self.listed, -1);
            }
            DialogKey::Enter | DialogKey::Char('t') => {
                self.request = Some(DetailRequest::Transactions);
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Char('e') => {
                self.request = Some(DetailRequest::Edit);
                return Some(DialogOutcome::Confirm);
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.request.is_some()
    }
}

/// The open Budgets dialog. Each screen's ticket adds its form to its variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetsDialog {
    /// 11b, the popover under the title.
    Switcher(Switcher),
    /// 11c creating a Budget, or its edit mode (the form names the Budget).
    Budget(BudgetForm),
    /// 11f.
    Manage(Manage),
    /// 9d.
    CategoryDetail(Detail),
    /// 9e: editing a Category's amount, or picking an unbudgeted Category to budget.
    EditLimit(LimitForm),
    /// 9f, filling this month from the chosen source.
    Fill { month: Period, source: FillSource },
    /// 9g for a Category.
    Stop(StopForm),
}

impl BudgetsDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Switcher(dialog) => Some(dialog),
            Self::Budget(dialog) => Some(dialog),
            Self::Manage(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Switcher(dialog) => Some(dialog),
            Self::Budget(dialog) => Some(dialog),
            Self::Manage(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }
}

impl Dialog for BudgetsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        if let Self::Fill { source, .. } = self {
            // `j`/`k` pick the source; `enter` fills.
            let all = FillSource::ALL;
            let at = all.iter().position(|each| each == source).unwrap_or(0);
            match key {
                DialogKey::Down | DialogKey::Char('j') => {
                    *source = all[(at + 1).min(all.len() - 1)]
                }
                DialogKey::Up | DialogKey::Char('k') => *source = all[at.saturating_sub(1)],
                _ => return None,
            }
            return Some(DialogOutcome::Handled);
        }
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(dialog) = self.inner_mut() {
            dialog.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }
}
