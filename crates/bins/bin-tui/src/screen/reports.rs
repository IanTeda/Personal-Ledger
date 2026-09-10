//! The Reports screen (FR.34-38, CC-TUI-011) — one screen with an internal picker across
//! every report type, per "Decide TUI screen map and navigation shape" (not a separate
//! dashboard area per report). All five reports now exist: Account Balance (FR.34),
//! Category-total (FR.35), Payee-total (FR.36), Budget-vs-actual (FR.37), and
//! Balance-check-variance (FR.38).
//!
//! `Field` unifies focus across the top-level report picker and each report's own inputs:
//! `Field::ReportKind` is always first (Left/Right there changes which report is shown, via
//! Tab to reach it). Category-total and Payee-total share one scope/date-range filter (the
//! same Unit-or-Account-plus-date-range shape, per FR.35/36) rather than each tracking its
//! own — switching between them keeps whatever scope/range is already entered, letting a
//! user compare "which Category" against "which Payee" for the same range without
//! re-entering it. Tab cycles all fields; Left/Right (or typing) acts on whichever is
//! focused.

use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

/// Which report is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportKind {
    /// FR.34: the current Balance of every Account, each expressed in its own Unit.
    AccountBalance,
    /// FR.35: the signed Transaction total per Category, scoped to a Unit or Account, over
    /// a date range.
    CategoryTotal,
    /// FR.36: the signed Transaction total per Payee, scoped to a Unit or Account, over a
    /// date range — only Payees with a matching Transaction appear.
    PayeeTotal,
    /// FR.37: every active Budget's limit vs. its actual spend for the current period —
    /// no filter, since a Budget already carries its own fixed Category/Unit scope and
    /// always tracks the current period (never a user-chosen range).
    BudgetVsActual,
    /// FR.38: every Balance Check ever recorded (a historical audit trail, not just the
    /// latest per Account) against its Account's Balance as of that Check's own date — no
    /// filter, since a Balance Check already carries its own fixed Account and date.
    BalanceCheckVariance,
}

const REPORT_KINDS: [ReportKind; 5] = [
    ReportKind::AccountBalance,
    ReportKind::CategoryTotal,
    ReportKind::PayeeTotal,
    ReportKind::BudgetVsActual,
    ReportKind::BalanceCheckVariance,
];

impl ReportKind {
    fn title(&self) -> &'static str {
        match self {
            ReportKind::AccountBalance => "Account Balance",
            ReportKind::CategoryTotal => "Category Total",
            ReportKind::PayeeTotal => "Payee Total",
            ReportKind::BudgetVsActual => "Budget vs Actual",
            ReportKind::BalanceCheckVariance => "Balance Check Variance",
        }
    }
}

/// Which report is scoped by: a Unit (every Account denominated in it) or a single Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Unit,
    Account,
}

impl ScopeKind {
    fn label(&self) -> &'static str {
        match self {
            ScopeKind::Unit => "Unit",
            ScopeKind::Account => "Account",
        }
    }
}

/// Which field currently has focus, spanning the top-level report picker and whichever
/// report is active's own inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    /// Always present, always first — Left/Right here changes `selected_report`.
    ReportKind,
    /// Category-total and Payee-total's shared scope/date filter.
    ScopeKind,
    ScopeTarget,
    DateFrom,
    DateTo,
}

/// Reports with no filter of their own — just the top-level report picker. Account Balance
/// and Budget-vs-actual both load-and-render unconditionally on `init()`.
const NO_FILTER_FIELDS: [Field; 1] = [Field::ReportKind];
const SCOPE_DATE_FIELDS: [Field; 5] = [
    Field::ReportKind,
    Field::ScopeKind,
    Field::ScopeTarget,
    Field::DateFrom,
    Field::DateTo,
];

enum AccountsStatus {
    Loading,
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

enum BudgetsStatus {
    Loading,
    Loaded(Vec<lib_database::Budgets>),
    Failed(String),
}

enum BalanceChecksStatus {
    Loading,
    Loaded(Vec<lib_database::BalanceChecks>),
    Failed(String),
}

/// One screen, an internal picker across every report type.
pub struct ReportsScreen {
    selected_report: ReportKind,
    focus: Field,
    accounts: AccountsStatus,
    units: Vec<lib_database::Units>,
    categories: Vec<lib_database::Categories>,
    payees: Vec<lib_database::Payees>,
    balances: Vec<(lib_core::RowID, lib_core::Money)>,

    // Category-total and Payee-total's shared scope/date filter.
    scope_kind: ScopeKind,
    scope_unit_id: Option<lib_core::RowID>,
    scope_account_id: Option<lib_core::RowID>,
    date_from: String,
    date_to: String,
    category_totals: Vec<(lib_core::RowID, lib_core::Money)>,
    payee_totals: Vec<(lib_core::RowID, lib_core::Money)>,

    // Budget-vs-actual (FR.37) — no filter of its own, loads unconditionally.
    budgets: BudgetsStatus,
    budget_progress: Vec<(lib_core::RowID, lib_database::BudgetProgress)>,

    // Balance-check-variance (FR.38) — no filter of its own, loads unconditionally.
    balance_checks: BalanceChecksStatus,
    balance_check_balances: Vec<(lib_core::RowID, lib_core::Money)>,

    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl ReportsScreen {
    pub fn new() -> Self {
        let today = chrono::Utc::now().date_naive();
        let month_start = today.with_day(1).unwrap_or(today);

        Self {
            selected_report: REPORT_KINDS[0],
            focus: Field::ReportKind,
            accounts: AccountsStatus::Loading,
            units: Vec::new(),
            categories: Vec::new(),
            payees: Vec::new(),
            balances: Vec::new(),
            scope_kind: ScopeKind::Unit,
            scope_unit_id: None,
            scope_account_id: None,
            date_from: month_start.to_string(),
            date_to: today.to_string(),
            category_totals: Vec::new(),
            budgets: BudgetsStatus::Loading,
            budget_progress: Vec::new(),
            balance_checks: BalanceChecksStatus::Loading,
            balance_check_balances: Vec::new(),
            payee_totals: Vec::new(),
            error: None,
            action_tx: None,
        }
    }

    fn fields(&self) -> &'static [Field] {
        match self.selected_report {
            ReportKind::AccountBalance
            | ReportKind::BudgetVsActual
            | ReportKind::BalanceCheckVariance => &NO_FILTER_FIELDS,
            ReportKind::CategoryTotal | ReportKind::PayeeTotal => &SCOPE_DATE_FIELDS,
        }
    }

    fn move_focus(&mut self, delta: isize) {
        let fields = self.fields();
        let len = fields.len() as isize;
        let current = fields.iter().position(|f| *f == self.focus).unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len);
        self.focus = fields[next as usize];
        self.error = None;
    }

    fn cycle_report(&mut self, delta: isize) {
        let len = REPORT_KINDS.len() as isize;
        let current = REPORT_KINDS
            .iter()
            .position(|k| *k == self.selected_report)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len);
        self.selected_report = REPORT_KINDS[next as usize];
        if !self.fields().contains(&self.focus) {
            self.focus = Field::ReportKind;
        }
    }

    fn toggle_scope_kind(&mut self) {
        self.scope_kind = match self.scope_kind {
            ScopeKind::Unit => ScopeKind::Account,
            ScopeKind::Account => ScopeKind::Unit,
        };
    }

    fn cycle_scope_target(&mut self, delta: isize) {
        match self.scope_kind {
            ScopeKind::Unit => {
                if self.units.is_empty() {
                    return;
                }
                let current = self
                    .scope_unit_id
                    .and_then(|id| self.units.iter().position(|u| u.id == id))
                    .unwrap_or(0) as isize;
                let next = (current + delta).rem_euclid(self.units.len() as isize);
                self.scope_unit_id = Some(self.units[next as usize].id);
            }
            ScopeKind::Account => {
                let AccountsStatus::Loaded(accounts) = &self.accounts else {
                    return;
                };
                if accounts.is_empty() {
                    return;
                }
                let current = self
                    .scope_account_id
                    .and_then(|id| accounts.iter().position(|a| a.id == id))
                    .unwrap_or(0) as isize;
                let next = (current + delta).rem_euclid(accounts.len() as isize);
                self.scope_account_id = Some(accounts[next as usize].id);
            }
        }
    }

    fn scope_target_label(&self) -> String {
        match self.scope_kind {
            ScopeKind::Unit => self
                .scope_unit_id
                .map(|id| self.unit_code(id).to_string())
                .unwrap_or_else(|| "(none selected)".to_string()),
            ScopeKind::Account => {
                let AccountsStatus::Loaded(accounts) = &self.accounts else {
                    return "Loading Accounts...".to_string();
                };
                self.scope_account_id
                    .and_then(|id| accounts.iter().find(|a| a.id == id))
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "(none selected)".to_string())
            }
        }
    }

    fn unit_code(&self, id: lib_core::RowID) -> &str {
        self.units
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.code.as_str())
            .unwrap_or("?")
    }

    fn category_name(&self, id: lib_core::RowID) -> &str {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn payee_name(&self, id: lib_core::RowID) -> &str {
        self.payees
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn account(&self, id: lib_core::RowID) -> Option<&lib_database::Accounts> {
        match &self.accounts {
            AccountsStatus::Loaded(accounts) => accounts.iter().find(|a| a.id == id),
            _ => None,
        }
    }

    fn account_name(&self, id: lib_core::RowID) -> &str {
        self.account(id)
            .map(|a| a.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn account_unit_code(&self, account_id: lib_core::RowID) -> &str {
        self.account(account_id)
            .map(|a| self.unit_code(a.unit_id))
            .unwrap_or("?")
    }

    fn balance_for(&self, id: lib_core::RowID) -> Option<&lib_core::Money> {
        self.balances
            .iter()
            .find(|(aid, _)| *aid == id)
            .map(|(_, balance)| balance)
    }

    fn budget_progress_for(&self, id: lib_core::RowID) -> Option<&lib_database::BudgetProgress> {
        self.budget_progress
            .iter()
            .find(|(bid, _)| *bid == id)
            .map(|(_, progress)| progress)
    }

    /// Computes progress for every given Budget, in order, against one connection — mirrors
    /// `BudgetsListScreen::spawn_progress_load`.
    fn spawn_budget_progress_load(&self, budgets: Vec<lib_database::Budgets>) {
        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                let mut results = Vec::with_capacity(budgets.len());
                for budget in budgets {
                    let progress = budget.current_progress(&pool).await?;
                    results.push((budget.id, progress));
                }
                Ok::<_, lib_database::Error>(results)
            }
            .await;
            let action = match action {
                Ok(progress) => Action::BudgetProgressLoaded(progress),
                Err(err) => Action::BudgetProgressLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    fn balance_check_balance_for(&self, id: lib_core::RowID) -> Option<&lib_core::Money> {
        self.balance_check_balances
            .iter()
            .find(|(bid, _)| *bid == id)
            .map(|(_, balance)| balance)
    }

    /// Computes each given Balance Check's Account Balance as of that Check's own date, in
    /// order, against one connection.
    fn spawn_balance_check_balances_load(&self, checks: Vec<lib_database::BalanceChecks>) {
        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                let mut results = Vec::with_capacity(checks.len());
                for check in checks {
                    let account = lib_database::Accounts::find_by_id(check.account_id, &pool)
                        .await?
                        .ok_or_else(|| {
                            lib_database::Error::NotFound(format!(
                                "Account {} not found for Balance Check {}",
                                check.account_id, check.id
                            ))
                        })?;
                    let balance = account.balance_as_of(check.date, &pool).await?;
                    results.push((check.id, balance));
                }
                Ok::<_, lib_database::Error>(results)
            }
            .await;
            let action = match action {
                Ok(balances) => Action::BalanceCheckBalancesLoaded(balances),
                Err(err) => Action::BalanceCheckBalancesLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    /// Parses the shared scope/date filter, storing a validation error on `self.error` (and
    /// returning `None`) when the form isn't ready to submit.
    fn parse_filter(
        &mut self,
    ) -> Option<(
        chrono::NaiveDate,
        chrono::NaiveDate,
        lib_database::TransactionScope,
    )> {
        let from: chrono::NaiveDate = match self.date_from.parse() {
            Ok(date) => date,
            Err(_) => {
                self.error = Some("From date must be in YYYY-MM-DD format".to_string());
                return None;
            }
        };
        let to: chrono::NaiveDate = match self.date_to.parse() {
            Ok(date) => date,
            Err(_) => {
                self.error = Some("To date must be in YYYY-MM-DD format".to_string());
                return None;
            }
        };

        let scope = match self.scope_kind {
            ScopeKind::Unit => match self.scope_unit_id {
                Some(id) => lib_database::TransactionScope::Unit(id),
                None => {
                    self.error = Some("Select a Unit".to_string());
                    return None;
                }
            },
            ScopeKind::Account => match self.scope_account_id {
                Some(id) => lib_database::TransactionScope::Account(id),
                None => {
                    self.error = Some("Select an Account".to_string());
                    return None;
                }
            },
        };

        Some((from, to, scope))
    }

    /// Runs the Category-total report with the currently entered scope/date range.
    fn run_category_total_report(&mut self) {
        let Some((from, to, scope)) = self.parse_filter() else {
            return;
        };
        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Categories::totals(scope, from, to, &pool).await
            }
            .await;
            let action = match action {
                Ok(totals) => Action::CategoryTotalsLoaded(totals),
                Err(err) => Action::CategoryTotalsLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    /// Runs the Payee-total report with the currently entered scope/date range.
    fn run_payee_total_report(&mut self) {
        let Some((from, to, scope)) = self.parse_filter() else {
            return;
        };
        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Payees::totals(scope, from, to, &pool).await
            }
            .await;
            let action = match action {
                Ok(totals) => Action::PayeeTotalsLoaded(totals),
                Err(err) => Action::PayeeTotalsLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Default for ReportsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for ReportsScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let accounts_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Accounts::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(accounts) => Action::AccountsLoaded(accounts),
                Err(err) => Action::AccountsLoadFailed(err.to_string()),
            };
            let _ = accounts_tx.send(action);
        });

        let units_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Units::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(units) => Action::UnitsLoaded(units),
                Err(err) => Action::UnitsLoadFailed(err.to_string()),
            };
            let _ = units_tx.send(action);
        });

        let categories_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Categories::find_all_active(&pool).await
            }
            .await;
            let action = match action {
                Ok(categories) => Action::CategoriesLoaded(categories),
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
            };
            let _ = categories_tx.send(action);
        });

        let payees_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Payees::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(payees) => Action::PayeesLoaded(payees),
                Err(err) => Action::PayeesLoadFailed(err.to_string()),
            };
            let _ = payees_tx.send(action);
        });

        let balances_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                let accounts = lib_database::Accounts::find_all(&pool).await?;
                let mut balances = Vec::with_capacity(accounts.len());
                for account in accounts {
                    let balance = account.balance(&pool).await?;
                    balances.push((account.id, balance));
                }
                Ok::<_, lib_database::Error>(balances)
            }
            .await;
            let action = match action {
                Ok(balances) => Action::AccountBalancesLoaded(balances),
                Err(err) => Action::AccountBalancesLoadFailed(err.to_string()),
            };
            let _ = balances_tx.send(action);
        });

        let budgets_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Budgets::find_all_active(&pool).await
            }
            .await;
            let action = match action {
                Ok(budgets) => Action::BudgetsLoaded(budgets),
                Err(err) => Action::BudgetsLoadFailed(err.to_string()),
            };
            let _ = budgets_tx.send(action);
        });

        let balance_checks_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::BalanceChecks::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(checks) => Action::BalanceChecksLoaded(checks),
                Err(err) => Action::BalanceChecksLoadFailed(err.to_string()),
            };
            let _ = balance_checks_tx.send(action);
        });

        self.action_tx = Some(action_tx);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        match key.code {
            KeyCode::Tab => {
                self.move_focus(1);
                return Some(Action::NoOp);
            }
            KeyCode::BackTab => {
                self.move_focus(-1);
                return Some(Action::NoOp);
            }
            KeyCode::Enter if self.selected_report == ReportKind::CategoryTotal => {
                self.run_category_total_report();
                return Some(Action::NoOp);
            }
            KeyCode::Enter if self.selected_report == ReportKind::PayeeTotal => {
                self.run_payee_total_report();
                return Some(Action::NoOp);
            }
            _ => {}
        }

        match self.focus {
            Field::ReportKind => match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    self.cycle_report(-1);
                    Some(Action::NoOp)
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.cycle_report(1);
                    Some(Action::NoOp)
                }
                _ => None,
            },
            Field::ScopeKind => {
                if matches!(
                    key.code,
                    KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l')
                ) {
                    self.toggle_scope_kind();
                }
                Some(Action::NoOp)
            }
            Field::ScopeTarget => {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => self.cycle_scope_target(-1),
                    KeyCode::Right | KeyCode::Char('l') => self.cycle_scope_target(1),
                    _ => {}
                }
                Some(Action::NoOp)
            }
            Field::DateFrom => {
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => self.date_from.push(c),
                    KeyCode::Backspace => {
                        self.date_from.pop();
                    }
                    _ => {}
                }
                Some(Action::NoOp)
            }
            Field::DateTo => {
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => self.date_to.push(c),
                    KeyCode::Backspace => {
                        self.date_to.pop();
                    }
                    _ => {}
                }
                Some(Action::NoOp)
            }
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::AccountsLoaded(accounts) => {
                if self.scope_account_id.is_none() {
                    self.scope_account_id = accounts.first().map(|a| a.id);
                }
                self.accounts = AccountsStatus::Loaded(accounts.clone());
            }
            Action::AccountsLoadFailed(message) => {
                self.accounts = AccountsStatus::Failed(message.clone());
            }
            Action::UnitsLoaded(units) => {
                if self.scope_unit_id.is_none() {
                    self.scope_unit_id = units.first().map(|u| u.id);
                }
                self.units = units.clone();
            }
            Action::CategoriesLoaded(categories) => self.categories = categories.clone(),
            Action::PayeesLoaded(payees) => self.payees = payees.clone(),
            Action::AccountBalancesLoaded(balances) => {
                for (id, balance) in balances {
                    match self.balances.iter_mut().find(|(aid, _)| aid == id) {
                        Some((_, existing)) => *existing = balance.clone(),
                        None => self.balances.push((*id, balance.clone())),
                    }
                }
            }
            Action::AccountBalancesLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::CategoryTotalsLoaded(totals) => {
                self.error = None;
                self.category_totals = totals.clone();
            }
            Action::CategoryTotalsLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::PayeeTotalsLoaded(totals) => {
                self.error = None;
                self.payee_totals = totals.clone();
            }
            Action::PayeeTotalsLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::PayeesLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::BudgetsLoaded(budgets) => {
                self.budgets = BudgetsStatus::Loaded(budgets.clone());
                self.spawn_budget_progress_load(budgets.clone());
            }
            Action::BudgetsLoadFailed(message) => {
                self.budgets = BudgetsStatus::Failed(message.clone());
            }
            Action::BudgetProgressLoaded(progress) => {
                for (id, new_progress) in progress {
                    match self.budget_progress.iter_mut().find(|(pid, _)| pid == id) {
                        Some((_, existing)) => *existing = new_progress.clone(),
                        None => self.budget_progress.push((*id, new_progress.clone())),
                    }
                }
            }
            Action::BudgetProgressLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::BalanceChecksLoaded(checks) => {
                self.balance_checks = BalanceChecksStatus::Loaded(checks.clone());
                self.spawn_balance_check_balances_load(checks.clone());
            }
            Action::BalanceChecksLoadFailed(message) => {
                self.balance_checks = BalanceChecksStatus::Failed(message.clone());
            }
            Action::BalanceCheckBalancesLoaded(balances) => {
                for (id, balance) in balances {
                    match self
                        .balance_check_balances
                        .iter_mut()
                        .find(|(bid, _)| bid == id)
                    {
                        Some((_, existing)) => *existing = balance.clone(),
                        None => self.balance_check_balances.push((*id, balance.clone())),
                    }
                }
            }
            Action::BalanceCheckBalancesLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Reports"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let picker_style = if self.focus == Field::ReportKind {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let picker_text = Line::from(Span::styled(
            format!("← {} →", self.selected_report.title()),
            picker_style,
        ));
        frame.render_widget(
            Paragraph::new(picker_text).block(Block::bordered().title(" Report ")),
            rows[0],
        );

        match self.selected_report {
            ReportKind::AccountBalance => self.view_account_balance(frame, rows[1]),
            ReportKind::CategoryTotal => self.view_scope_date_report(
                frame,
                rows[1],
                " Category Total ",
                "Category",
                &self.category_totals,
                |screen, id| screen.category_name(id),
            ),
            ReportKind::PayeeTotal => self.view_scope_date_report(
                frame,
                rows[1],
                " Payee Total ",
                "Payee",
                &self.payee_totals,
                |screen, id| screen.payee_name(id),
            ),
            ReportKind::BudgetVsActual => self.view_budget_vs_actual(frame, rows[1]),
            ReportKind::BalanceCheckVariance => self.view_balance_check_variance(frame, rows[1]),
        }

        let footer = if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            match self.selected_report {
                ReportKind::AccountBalance
                | ReportKind::BudgetVsActual
                | ReportKind::BalanceCheckVariance => {
                    "Tab: focus  ←/h →/l: change report  Esc: back".to_string()
                }
                ReportKind::CategoryTotal | ReportKind::PayeeTotal => {
                    "Tab: next field  ←/h →/l: change value  Enter: run report  Esc: back"
                        .to_string()
                }
            }
        };
        frame.render_widget(Paragraph::new(footer), rows[2]);
    }
}

impl ReportsScreen {
    fn view_account_balance(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        match &self.accounts {
            AccountsStatus::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Accounts...")
                        .block(Block::bordered().title(" Account Balance ")),
                    area,
                );
            }
            AccountsStatus::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Accounts: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Account Balance ")),
                    area,
                );
            }
            AccountsStatus::Loaded(accounts) => {
                let mut sorted: Vec<&lib_database::Accounts> = accounts.iter().collect();
                sorted.sort_by(|a, b| {
                    self.unit_code(a.unit_id)
                        .cmp(self.unit_code(b.unit_id))
                        .then(a.name.cmp(&b.name))
                });

                let header = Row::new(["Account", "Unit", "Balance"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = sorted.iter().map(|account| {
                    let style = if account.is_active {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let balance_text = self
                        .balance_for(account.id)
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "…".to_string());
                    Row::new([
                        Cell::from(account.name.clone()),
                        Cell::from(self.unit_code(account.unit_id).to_string()),
                        Cell::from(balance_text),
                    ])
                    .style(style)
                });
                let widths = [
                    Constraint::Length(24),
                    Constraint::Length(8),
                    Constraint::Length(16),
                ];
                let table = Table::new(table_rows, widths).header(header).block(
                    Block::bordered().title(format!(" Account Balance ({}) ", accounts.len())),
                );
                frame.render_widget(table, area);
            }
        }
    }

    /// Shared rendering for Category-total and Payee-total: both are the same shape (a
    /// scope/date filter, then a two-column name/total table), differing only in the title,
    /// the entity-name column header, the results, and how to resolve an id to a name.
    fn view_scope_date_report(
        &self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        title: &str,
        name_column: &str,
        results: &[(lib_core::RowID, lib_core::Money)],
        name_of: impl Fn(&Self, lib_core::RowID) -> &str,
    ) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(area);

        let field_spans = |label: &str, value: String, field: Field| -> Vec<Span<'static>> {
            let style = if self.focus == field {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            vec![
                Span::raw(format!("{label:<8}")),
                Span::styled(value, style),
                Span::raw("   "),
            ]
        };

        let mut scope_line = field_spans(
            "Scope",
            self.scope_kind.label().to_string(),
            Field::ScopeKind,
        );
        scope_line.extend(field_spans(
            "Target",
            self.scope_target_label(),
            Field::ScopeTarget,
        ));

        let mut date_line = field_spans("From", self.date_from.clone(), Field::DateFrom);
        date_line.extend(field_spans("To", self.date_to.clone(), Field::DateTo));

        let inputs = Paragraph::new(vec![Line::from(scope_line), Line::from(date_line)])
            .block(Block::bordered().title(title.to_string()));
        frame.render_widget(inputs, sections[0]);

        let mut sorted = results.to_vec();
        sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        let header = Row::new([name_column, "Total"]).style(Style::default().fg(Color::Yellow));
        let table_rows = sorted.iter().map(|(id, total)| {
            Row::new([
                Cell::from(name_of(self, *id).to_string()),
                Cell::from(total.to_string()),
            ])
        });
        let widths = [Constraint::Length(24), Constraint::Length(16)];
        let table = Table::new(table_rows, widths)
            .header(header)
            .block(Block::bordered().title(format!(" Results ({}) ", sorted.len())));
        frame.render_widget(table, sections[1]);
    }

    /// FR.37: every active Budget's limit vs. its current-period spend, most urgent (closest
    /// to or over its limit) first. No filter to enter — a Budget already carries its own
    /// fixed Category/Unit scope and always tracks the current period.
    fn view_budget_vs_actual(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        match &self.budgets {
            BudgetsStatus::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Budgets...")
                        .block(Block::bordered().title(" Budget vs Actual ")),
                    area,
                );
            }
            BudgetsStatus::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Budgets: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Budget vs Actual ")),
                    area,
                );
            }
            BudgetsStatus::Loaded(budgets) => {
                let mut sorted: Vec<&lib_database::Budgets> = budgets.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fraction = self
                        .budget_progress_for(a.id)
                        .map(|p| p.spend_fraction())
                        .unwrap_or(0.0);
                    let b_fraction = self
                        .budget_progress_for(b.id)
                        .map(|p| p.spend_fraction())
                        .unwrap_or(0.0);
                    b_fraction
                        .partial_cmp(&a_fraction)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let header = Row::new(["Category", "Unit", "Limit", "Spent", "Remaining"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = sorted.iter().map(|budget| {
                    let progress = self.budget_progress_for(budget.id);
                    let spent = progress
                        .map(|p| p.spend.to_string())
                        .unwrap_or_else(|| "…".to_string());
                    let remaining = progress
                        .map(|p| lib_core::Money::from(&p.limit_amount.0 - &p.spend.0).to_string())
                        .unwrap_or_else(|| "…".to_string());
                    let style = if progress.map(|p| p.is_ahead_of_pace()).unwrap_or(false) {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default()
                    };
                    Row::new([
                        Cell::from(self.category_name(budget.category_id).to_string()),
                        Cell::from(self.unit_code(budget.unit_id).to_string()),
                        Cell::from(budget.limit_amount.to_string()),
                        Cell::from(spent),
                        Cell::from(remaining),
                    ])
                    .style(style)
                });
                let widths = [
                    Constraint::Length(20),
                    Constraint::Length(6),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(12),
                ];
                let table = Table::new(table_rows, widths).header(header).block(
                    Block::bordered().title(format!(" Budget vs Actual ({}) ", budgets.len())),
                );
                frame.render_widget(table, area);
            }
        }
    }

    /// FR.38: every Balance Check ever recorded (a historical audit trail, not just the
    /// latest per Account) against its Account's Balance as of that Check's own date —
    /// variance = asserted − computed, per FR.38's own wording — sorted by the size of the
    /// discrepancy, biggest first. No filter to enter — a Balance Check already carries its
    /// own fixed Account and date.
    fn view_balance_check_variance(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        match &self.balance_checks {
            BalanceChecksStatus::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Balance Checks...")
                        .block(Block::bordered().title(" Balance Check Variance ")),
                    area,
                );
            }
            BalanceChecksStatus::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Balance Checks: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Balance Check Variance ")),
                    area,
                );
            }
            BalanceChecksStatus::Loaded(checks) => {
                let variance_for = |check: &lib_database::BalanceChecks| {
                    self.balance_check_balance_for(check.id)
                        .map(|computed| &check.asserted_balance.0 - &computed.0)
                };

                let mut sorted: Vec<&lib_database::BalanceChecks> = checks.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_magnitude = variance_for(a).map(|v| &v * &v);
                    let b_magnitude = variance_for(b).map(|v| &v * &v);
                    b_magnitude.cmp(&a_magnitude)
                });

                let header = Row::new([
                    "Account", "Unit", "Date", "Asserted", "Computed", "Variance",
                ])
                .style(Style::default().fg(Color::Yellow));
                let table_rows = sorted.iter().map(|check| {
                    let computed = self.balance_check_balance_for(check.id);
                    let computed_text = computed
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "…".to_string());
                    let variance_text = variance_for(check)
                        .map(|v| lib_core::Money::from(v).to_string())
                        .unwrap_or_else(|| "…".to_string());
                    let is_mismatch = variance_for(check).map(|v| v != 0).unwrap_or(false);
                    let style = if is_mismatch {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default()
                    };
                    Row::new([
                        Cell::from(self.account_name(check.account_id).to_string()),
                        Cell::from(self.account_unit_code(check.account_id).to_string()),
                        Cell::from(check.date.to_string()),
                        Cell::from(check.asserted_balance.to_string()),
                        Cell::from(computed_text),
                        Cell::from(variance_text),
                    ])
                    .style(style)
                });
                let widths = [
                    Constraint::Length(18),
                    Constraint::Length(6),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(12),
                ];
                let table = Table::new(table_rows, widths).header(header).block(
                    Block::bordered().title(format!(" Balance Check Variance ({}) ", checks.len())),
                );
                frame.render_widget(table, area);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn mock_unit(code: &str) -> lib_database::Units {
        let now = chrono::Utc::now();
        lib_database::Units {
            id: lib_core::RowID::new(),
            code: code.to_string(),
            name: format!("{code} Name"),
            unit_kind: lib_core::UnitKind::Fiat,
            decimal_places: 2,
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn mock_account(
        name: &str,
        unit_id: lib_core::RowID,
        is_active: bool,
    ) -> lib_database::Accounts {
        let now = chrono::Utc::now();
        lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: name.to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id,
            starting_balance: lib_core::Money::mock(),
            is_active,
            created_on: now,
            updated_on: now,
        }
    }

    fn render(screen: &ReportsScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Reports screen should not error");
    }

    #[test]
    fn new_starts_on_the_account_balance_report_loading() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.selected_report, ReportKind::AccountBalance);
        render(&screen);
    }

    #[test]
    fn new_defaults_the_category_total_date_range_to_the_current_month() {
        let screen = ReportsScreen::new();
        let today = chrono::Utc::now().date_naive();
        assert_eq!(screen.date_to, today.to_string());
        assert!(screen.date_from.ends_with("-01"));
    }

    #[test]
    fn renders_loaded_accounts_including_inactive_ones() {
        let mut screen = ReportsScreen::new();
        let unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit.clone()]));
        screen.update(&Action::AccountsLoaded(vec![
            mock_account("Everyday", unit.id, true),
            mock_account("Old Account", unit.id, false),
        ]));
        render(&screen);
    }

    #[test]
    fn cycling_report_moves_through_every_kind_and_back() {
        let mut screen = ReportsScreen::new();
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::CategoryTotal);
        render(&screen);

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::PayeeTotal);
        render(&screen);

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::BudgetVsActual);
        render(&screen);

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::BalanceCheckVariance);
        render(&screen);

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::AccountBalance, "wraps");
    }

    #[test]
    fn account_balances_loaded_populates_the_lookup() {
        let mut screen = ReportsScreen::new();
        let account_id = lib_core::RowID::new();
        screen.update(&Action::AccountBalancesLoaded(vec![(
            account_id,
            "42".parse().unwrap(),
        )]));

        assert_eq!(screen.balance_for(account_id), Some(&"42".parse().unwrap()));
    }

    #[test]
    fn unit_code_falls_back_when_unknown() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.unit_code(lib_core::RowID::new()), "?");
    }

    #[test]
    fn tab_reaches_category_total_fields_and_wraps() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::CategoryTotal;
        for expected in [
            Field::ScopeKind,
            Field::ScopeTarget,
            Field::DateFrom,
            Field::DateTo,
            Field::ReportKind,
        ] {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
            assert_eq!(screen.focus, expected);
        }
    }

    #[test]
    fn toggling_scope_kind_switches_between_unit_and_account() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::CategoryTotal;
        screen.focus = Field::ScopeKind;
        assert_eq!(screen.scope_kind, ScopeKind::Unit);

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.scope_kind, ScopeKind::Account);
    }

    #[test]
    fn typing_appends_to_the_date_from_field() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::CategoryTotal;
        screen.focus = Field::DateFrom;
        screen.date_from.clear();
        screen.handle_key(key(KeyCode::Char('2')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('0')), InputMode::Navigation);
        assert_eq!(screen.date_from, "20");
    }

    #[test]
    fn run_report_rejects_an_invalid_date() {
        let mut screen = ReportsScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.date_from = "not-a-date".to_string();
        screen.run_category_total_report();
        assert!(screen.error.is_some());
    }

    #[test]
    fn run_report_rejects_when_no_unit_is_selected() {
        let mut screen = ReportsScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.scope_unit_id = None;
        screen.run_category_total_report();
        assert!(screen.error.is_some());
    }

    #[test]
    fn category_totals_loaded_populates_results() {
        let mut screen = ReportsScreen::new();
        let category_id = lib_core::RowID::new();
        screen.update(&Action::CategoryTotalsLoaded(vec![(
            category_id,
            "10".parse().unwrap(),
        )]));

        assert_eq!(screen.category_totals.len(), 1);
        assert_eq!(screen.category_totals[0].0, category_id);
    }

    #[test]
    fn category_name_falls_back_when_unknown() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.category_name(lib_core::RowID::new()), "(unknown)");
    }

    #[test]
    fn payee_totals_loaded_populates_results() {
        let mut screen = ReportsScreen::new();
        let payee_id = lib_core::RowID::new();
        screen.update(&Action::PayeeTotalsLoaded(vec![(
            payee_id,
            "10".parse().unwrap(),
        )]));

        assert_eq!(screen.payee_totals.len(), 1);
        assert_eq!(screen.payee_totals[0].0, payee_id);
    }

    #[test]
    fn payee_name_falls_back_when_unknown() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.payee_name(lib_core::RowID::new()), "(unknown)");
    }

    #[test]
    fn run_payee_total_report_rejects_an_invalid_date() {
        let mut screen = ReportsScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.date_from = "not-a-date".to_string();
        screen.run_payee_total_report();
        assert!(screen.error.is_some());
    }

    #[test]
    fn run_payee_total_report_rejects_when_no_unit_is_selected() {
        let mut screen = ReportsScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.scope_unit_id = None;
        screen.run_payee_total_report();
        assert!(screen.error.is_some());
    }

    #[test]
    fn renders_the_payee_total_report_without_panicking() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::PayeeTotal;
        let payee_id = lib_core::RowID::new();
        screen.update(&Action::PayeeTotalsLoaded(vec![(
            payee_id,
            "10".parse().unwrap(),
        )]));
        render(&screen);
    }

    fn mock_budget(
        category_id: lib_core::RowID,
        unit_id: lib_core::RowID,
    ) -> lib_database::Budgets {
        let now = chrono::Utc::now();
        lib_database::Budgets {
            id: lib_core::RowID::new(),
            category_id,
            unit_id,
            limit_amount: lib_core::Money::mock(),
            period: lib_core::BudgetPeriod::Monthly,
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn mock_progress(
        spend: &str,
        limit: &str,
        today_fraction: f64,
    ) -> lib_database::BudgetProgress {
        lib_database::BudgetProgress {
            spend: spend.parse().unwrap(),
            limit_amount: limit.parse().unwrap(),
            period_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            period_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            today_fraction,
        }
    }

    #[test]
    fn budgets_loaded_populates_results() {
        let mut screen = ReportsScreen::new();
        let budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        screen.update(&Action::BudgetsLoaded(vec![budget.clone()]));

        match &screen.budgets {
            BudgetsStatus::Loaded(budgets) => assert_eq!(budgets, &vec![budget]),
            _ => panic!("expected Loaded"),
        }
    }

    #[test]
    fn budgets_load_failed_sets_a_failed_status() {
        let mut screen = ReportsScreen::new();
        screen.update(&Action::BudgetsLoadFailed("connection refused".to_string()));

        assert!(matches!(screen.budgets, BudgetsStatus::Failed(_)));
    }

    #[test]
    fn budget_progress_loaded_populates_the_lookup() {
        let mut screen = ReportsScreen::new();
        let budget_id = lib_core::RowID::new();
        let progress = mock_progress("50", "100", 0.5);
        screen.update(&Action::BudgetProgressLoaded(vec![(
            budget_id,
            progress.clone(),
        )]));

        assert_eq!(screen.budget_progress_for(budget_id), Some(&progress));
    }

    #[test]
    fn renders_the_budget_vs_actual_report_sorted_by_urgency_without_panicking() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::BudgetVsActual;

        let over_budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        let under_budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        screen.update(&Action::BudgetsLoaded(vec![
            under_budget.clone(),
            over_budget.clone(),
        ]));
        screen.update(&Action::BudgetProgressLoaded(vec![
            (under_budget.id, mock_progress("10", "100", 0.5)),
            (over_budget.id, mock_progress("150", "100", 0.5)),
        ]));

        assert!(
            screen
                .budget_progress_for(over_budget.id)
                .unwrap()
                .is_ahead_of_pace()
        );
        render(&screen);
    }

    #[test]
    fn renders_the_budget_vs_actual_report_loading_and_failed_without_panicking() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::BudgetVsActual;
        render(&screen);

        screen.update(&Action::BudgetsLoadFailed("connection refused".to_string()));
        render(&screen);
    }

    fn mock_balance_check(account_id: lib_core::RowID) -> lib_database::BalanceChecks {
        let now = chrono::Utc::now();
        lib_database::BalanceChecks {
            id: lib_core::RowID::new(),
            account_id,
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            asserted_balance: "100".parse().unwrap(),
            created_on: now,
            updated_on: now,
        }
    }

    #[test]
    fn balance_checks_loaded_populates_results() {
        let mut screen = ReportsScreen::new();
        let check = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksLoaded(vec![check.clone()]));

        match &screen.balance_checks {
            BalanceChecksStatus::Loaded(checks) => assert_eq!(checks, &vec![check]),
            _ => panic!("expected Loaded"),
        }
    }

    #[test]
    fn balance_checks_load_failed_sets_a_failed_status() {
        let mut screen = ReportsScreen::new();
        screen.update(&Action::BalanceChecksLoadFailed(
            "connection refused".to_string(),
        ));

        assert!(matches!(
            screen.balance_checks,
            BalanceChecksStatus::Failed(_)
        ));
    }

    #[test]
    fn balance_check_balances_loaded_populates_the_lookup() {
        let mut screen = ReportsScreen::new();
        let check_id = lib_core::RowID::new();
        screen.update(&Action::BalanceCheckBalancesLoaded(vec![(
            check_id,
            "95".parse().unwrap(),
        )]));

        assert_eq!(
            screen.balance_check_balance_for(check_id),
            Some(&"95".parse().unwrap())
        );
    }

    #[test]
    fn renders_the_balance_check_variance_report_sorted_by_magnitude_without_panicking() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::BalanceCheckVariance;

        let mismatched = mock_balance_check(lib_core::RowID::new());
        let matched = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksLoaded(vec![
            matched.clone(),
            mismatched.clone(),
        ]));
        screen.update(&Action::BalanceCheckBalancesLoaded(vec![
            (matched.id, "100".parse().unwrap()),
            (mismatched.id, "40".parse().unwrap()),
        ]));

        render(&screen);
    }

    #[test]
    fn renders_the_balance_check_variance_report_loading_and_failed_without_panicking() {
        let mut screen = ReportsScreen::new();
        screen.selected_report = ReportKind::BalanceCheckVariance;
        render(&screen);

        screen.update(&Action::BalanceChecksLoadFailed(
            "connection refused".to_string(),
        ));
        render(&screen);
    }
}
