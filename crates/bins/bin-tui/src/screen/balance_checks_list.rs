//! The Balance Checks list screen (CC-TUI-013), mirroring the Units/Accounts/Transactions
//! list shape. Needs Account names for its column, not just ids, so `init()` also kicks off
//! an Accounts load reusing `AccountsLoaded` — the same action Accounts' own list screen and
//! `BalanceCheckDetailScreen`'s picker already use, rather than a Balance-Check-specific
//! lookup action.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

enum Status {
    Loading,
    Loaded(Vec<lib_database::BalanceChecks>),
    Failed(String),
}

/// Lists every Balance Check; the entry point for creating, editing, and deleting them.
pub struct BalanceChecksListScreen {
    status: Status,
    accounts: Vec<lib_database::Accounts>,
    table_state: TableState,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl BalanceChecksListScreen {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            status: Status::Loading,
            accounts: Vec::new(),
            table_state,
            pending_delete: None,
            error: None,
            action_tx: None,
        }
    }

    fn balance_checks(&self) -> &[lib_database::BalanceChecks] {
        match &self.status {
            Status::Loaded(balance_checks) => balance_checks,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.balance_checks().get(index))
            .map(|balance_check| balance_check.id)
    }

    fn account_name(&self, id: lib_core::RowID) -> &str {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.balance_checks().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> crate::Result<Vec<lib_database::BalanceChecks>> {
        let pool = db::connect().await?;
        lib_database::BalanceChecks::find_all(&pool)
            .await
            .map_err(crate::error::Error::from)
    }

    async fn delete(id: lib_core::RowID) -> crate::Result<()> {
        let pool = db::connect().await?;
        lib_database::BalanceChecks::delete_by_id(id, &pool)
            .await
            .map_err(crate::error::Error::from)
    }
}

impl Default for BalanceChecksListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for BalanceChecksListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(balance_checks) => Action::BalanceChecksLoaded(balance_checks),
                Err(err) => Action::BalanceChecksLoadFailed(err.to_string()),
            };
            let _ = load_tx.send(action);
        });

        let accounts_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Accounts::find_all(&pool)
                    .await
                    .map_err(crate::error::Error::from)
            }
            .await;
            let action = match action {
                Ok(accounts) => Action::AccountsLoaded(accounts),
                Err(err) => Action::AccountsLoadFailed(err.to_string()),
            };
            let _ = accounts_tx.send(action);
        });

        self.action_tx = Some(action_tx);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        if let Some(pending_id) = self.pending_delete {
            self.pending_delete = None;
            return match key.code {
                KeyCode::Char('y') => {
                    if let Some(action_tx) = self.action_tx.clone() {
                        tokio::spawn(async move {
                            let action = match Self::delete(pending_id).await {
                                Ok(()) => Action::BalanceCheckDeleted(pending_id),
                                Err(err) => Action::BalanceCheckDeleteFailed(err.to_string()),
                            };
                            let _ = action_tx.send(action);
                        });
                    }
                    Some(Action::NoOp)
                }
                _ => Some(Action::NoOp),
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Some(Action::NoOp)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Some(Action::NoOp)
            }
            KeyCode::Enter => self
                .table_state
                .selected()
                .and_then(|index| self.balance_checks().get(index))
                .cloned()
                .map(|balance_check| Action::OpenBalanceCheckDetail(Some(balance_check))),
            KeyCode::Char('n') => Some(Action::OpenBalanceCheckDetail(None)),
            KeyCode::Char('i') => Some(Action::OpenCsvImport),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::BalanceChecksLoaded(balance_checks) => {
                self.status = Status::Loaded(balance_checks.clone());
                if self.table_state.selected().unwrap_or(0) >= balance_checks.len()
                    && !balance_checks.is_empty()
                {
                    self.table_state.select(Some(balance_checks.len() - 1));
                }
            }
            Action::BalanceChecksLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::AccountsLoaded(accounts) => self.accounts = accounts.clone(),
            Action::BalanceCheckSaved(saved) => {
                if let Status::Loaded(balance_checks) = &mut self.status {
                    match balance_checks.iter_mut().find(|b| b.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => balance_checks.insert(0, saved.clone()),
                    }
                    balance_checks.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
                }
            }
            Action::BalanceCheckDeleted(id) => {
                if let Status::Loaded(balance_checks) = &mut self.status {
                    balance_checks.retain(|b| b.id != *id);
                }
            }
            Action::BalanceChecksImported(imported) => {
                if let Status::Loaded(balance_checks) = &mut self.status {
                    for saved in imported {
                        match balance_checks.iter_mut().find(|b| b.id == saved.id) {
                            Some(existing) => *existing = saved.clone(),
                            None => balance_checks.push(saved.clone()),
                        }
                    }
                    balance_checks.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
                }
            }
            Action::BalanceCheckSaveFailed(message)
            | Action::BalanceCheckDeleteFailed(message)
            | Action::BalanceChecksImportFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Balance Checks"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Balance Checks...")
                        .block(Block::bordered().title(" Balance Checks ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Balance Checks: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Balance Checks ")),
                    rows[0],
                );
            }
            Status::Loaded(balance_checks) => {
                let header = Row::new(["Date", "Account", "Asserted balance"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = balance_checks.iter().map(|balance_check| {
                    Row::new([
                        Cell::from(balance_check.date.to_string()),
                        Cell::from(self.account_name(balance_check.account_id).to_string()),
                        Cell::from(balance_check.asserted_balance.to_string()),
                    ])
                });
                let widths = [
                    Constraint::Length(11),
                    Constraint::Length(20),
                    Constraint::Length(16),
                ];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(
                        Block::bordered()
                            .title(format!(" Balance Checks ({}) ", balance_checks.len())),
                    );
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Balance Check? y: confirm, any other key: cancel".to_string()
        } else if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            "↑/k ↓/j: move  Enter: edit  n: new  i: import CSV  d: delete  Esc: back".to_string()
        };
        frame.render_widget(Paragraph::new(footer), rows[1]);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn mock_balance_check(account_id: lib_core::RowID) -> lib_database::BalanceChecks {
        let now = chrono::Utc::now();
        lib_database::BalanceChecks {
            id: lib_core::RowID::new(),
            account_id,
            date: now.date_naive(),
            asserted_balance: lib_core::Money::mock(),
            created_on: now,
            updated_on: now,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = BalanceChecksListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Balance Checks list screen should not error");
    }

    #[test]
    fn renders_loading_without_panicking() {
        render(Status::Loading);
    }

    #[test]
    fn renders_failed_without_panicking() {
        render(Status::Failed("connection refused".to_string()));
    }

    #[test]
    fn renders_loaded_without_panicking() {
        render(Status::Loaded(vec![mock_balance_check(
            lib_core::RowID::new(),
        )]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = BalanceChecksListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenBalanceCheckDetail(None))
        );
    }

    #[test]
    fn i_opens_the_csv_import_screen() {
        let mut screen = BalanceChecksListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('i')), InputMode::Navigation),
            Some(Action::OpenCsvImport)
        );
    }

    #[test]
    fn balance_checks_imported_appends_to_the_loaded_list() {
        let mut screen = BalanceChecksListScreen::new();
        let existing = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksLoaded(vec![existing.clone()]));

        let imported = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksImported(vec![imported.clone()]));

        assert_eq!(screen.balance_checks().len(), 2);
        assert!(screen.balance_checks().iter().any(|b| b.id == imported.id));
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_balance_check() {
        let mut screen = BalanceChecksListScreen::new();
        let balance_check = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksLoaded(vec![balance_check.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenBalanceCheckDetail(Some(balance_check)))
        );
    }

    #[test]
    fn balance_check_deleted_removes_it_from_the_loaded_list() {
        let mut screen = BalanceChecksListScreen::new();
        let balance_check = mock_balance_check(lib_core::RowID::new());
        screen.update(&Action::BalanceChecksLoaded(vec![balance_check.clone()]));
        screen.update(&Action::BalanceCheckDeleted(balance_check.id));

        assert!(screen.balance_checks().is_empty());
    }

    #[test]
    fn account_name_resolves_once_loaded() {
        let mut screen = BalanceChecksListScreen::new();
        let now = chrono::Utc::now();
        let account = lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: "Everyday Spending".to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id: lib_core::RowID::new(),
            starting_balance: lib_core::Money::mock(),
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        screen.update(&Action::AccountsLoaded(vec![account.clone()]));

        assert_eq!(screen.account_name(account.id), "Everyday Spending");
        assert_eq!(screen.account_name(lib_core::RowID::new()), "(unknown)");
    }
}
