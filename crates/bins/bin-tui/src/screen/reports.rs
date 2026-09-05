//! The Reports screen (FR.34-38, CC-TUI-011) — one screen with an internal picker across
//! every report type, per "Decide TUI screen map and navigation shape" (not a separate
//! dashboard area per report). Account Balance (FR.34) and Category-total (FR.35) exist so
//! far; tickets #77-79 each add their own [`ReportKind`] variant and rendering branch here.
//!
//! `Field` unifies focus across the top-level report picker and each report's own inputs:
//! `Field::ReportKind` is always first (Left/Right there changes which report is shown, via
//! Tab to reach it), and a report with its own inputs (like Category-total) adds its fields
//! after it — Tab cycles between all of them, Left/Right (or typing) acts on whichever is
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

/// Which report is currently selected. `#77-79` will each add a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportKind {
    /// FR.34: the current Balance of every Account, each expressed in its own Unit.
    AccountBalance,
    /// FR.35: the signed Transaction total per Category, scoped to a Unit or Account, over
    /// a date range.
    CategoryTotal,
}

const REPORT_KINDS: [ReportKind; 2] = [ReportKind::AccountBalance, ReportKind::CategoryTotal];

impl ReportKind {
    fn title(&self) -> &'static str {
        match self {
            ReportKind::AccountBalance => "Account Balance",
            ReportKind::CategoryTotal => "Category Total",
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
    /// Category-total only.
    ScopeKind,
    /// Category-total only.
    ScopeTarget,
    /// Category-total only.
    DateFrom,
    /// Category-total only.
    DateTo,
}

const ACCOUNT_BALANCE_FIELDS: [Field; 1] = [Field::ReportKind];
const CATEGORY_TOTAL_FIELDS: [Field; 5] = [
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

/// One screen, an internal picker across every report type.
pub struct ReportsScreen {
    selected_report: ReportKind,
    focus: Field,
    accounts: AccountsStatus,
    units: Vec<lib_database::Units>,
    categories: Vec<lib_database::Categories>,
    balances: Vec<(lib_core::RowID, lib_core::Money)>,

    // Category-total report state.
    scope_kind: ScopeKind,
    scope_unit_id: Option<lib_core::RowID>,
    scope_account_id: Option<lib_core::RowID>,
    date_from: String,
    date_to: String,
    category_totals: Vec<(lib_core::RowID, lib_core::Money)>,

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
            balances: Vec::new(),
            scope_kind: ScopeKind::Unit,
            scope_unit_id: None,
            scope_account_id: None,
            date_from: month_start.to_string(),
            date_to: today.to_string(),
            category_totals: Vec::new(),
            error: None,
            action_tx: None,
        }
    }

    fn fields(&self) -> &'static [Field] {
        match self.selected_report {
            ReportKind::AccountBalance => &ACCOUNT_BALANCE_FIELDS,
            ReportKind::CategoryTotal => &CATEGORY_TOTAL_FIELDS,
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

    fn balance_for(&self, id: lib_core::RowID) -> Option<&lib_core::Money> {
        self.balances
            .iter()
            .find(|(aid, _)| *aid == id)
            .map(|(_, balance)| balance)
    }

    /// Runs the Category-total report with the currently entered scope/date range; stores a
    /// validation error on `self.error` when the form isn't ready to submit.
    fn run_category_total_report(&mut self) {
        let from: chrono::NaiveDate = match self.date_from.parse() {
            Ok(date) => date,
            Err(_) => {
                self.error = Some("From date must be in YYYY-MM-DD format".to_string());
                return;
            }
        };
        let to: chrono::NaiveDate = match self.date_to.parse() {
            Ok(date) => date,
            Err(_) => {
                self.error = Some("To date must be in YYYY-MM-DD format".to_string());
                return;
            }
        };

        let scope = match self.scope_kind {
            ScopeKind::Unit => match self.scope_unit_id {
                Some(id) => lib_database::CategoryTotalScope::Unit(id),
                None => {
                    self.error = Some("Select a Unit".to_string());
                    return;
                }
            },
            ScopeKind::Account => match self.scope_account_id {
                Some(id) => lib_database::CategoryTotalScope::Account(id),
                None => {
                    self.error = Some("Select an Account".to_string());
                    return;
                }
            },
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
                Ok::<_, lib_database::DatabaseError>(balances)
            }
            .await;
            let action = match action {
                Ok(balances) => Action::AccountBalancesLoaded(balances),
                Err(err) => Action::AccountBalancesLoadFailed(err.to_string()),
            };
            let _ = balances_tx.send(action);
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
            ReportKind::CategoryTotal => self.view_category_total(frame, rows[1]),
        }

        let footer = if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            match self.selected_report {
                ReportKind::AccountBalance => {
                    "Tab: focus  ←/h →/l: change report  Esc: back".to_string()
                }
                ReportKind::CategoryTotal => {
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

    fn view_category_total(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
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
            .block(Block::bordered().title(" Category Total "));
        frame.render_widget(inputs, sections[0]);

        let mut sorted = self.category_totals.clone();
        sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        let header = Row::new(["Category", "Total"]).style(Style::default().fg(Color::Yellow));
        let table_rows = sorted.iter().map(|(id, total)| {
            Row::new([
                Cell::from(self.category_name(*id).to_string()),
                Cell::from(total.to_string()),
            ])
        });
        let widths = [Constraint::Length(24), Constraint::Length(16)];
        let table = Table::new(table_rows, widths)
            .header(header)
            .block(Block::bordered().title(format!(" Results ({}) ", sorted.len())));
        frame.render_widget(table, sections[1]);
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
    fn cycling_report_moves_to_category_total_and_back() {
        let mut screen = ReportsScreen::new();
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::CategoryTotal);
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
}
