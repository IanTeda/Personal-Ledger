//! The Accounts list screen (CC-TUI-008), mirroring `UnitsListScreen`/`CategoriesListScreen`.
//! Shows `starting_balance`, not a computed running Balance — Balance is compute-on-read
//! from Transactions ("Decide the TUI app's data model and persistence layer"), and no
//! Transaction can exist until [issue #70](https://github.com/IanTeda/Personal-Ledger/issues/70)
//! lands, so the two are numerically identical right now but only one of them is honest to
//! label "Balance". The real Account Balance report is
//! [issue #75](https://github.com/IanTeda/Personal-Ledger/issues/75).

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
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

/// Lists every Account; the entry point for creating, editing, and deleting them.
pub struct AccountsListScreen {
    status: Status,
    table_state: TableState,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl AccountsListScreen {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            status: Status::Loading,
            table_state,
            pending_delete: None,
            error: None,
            action_tx: None,
        }
    }

    fn accounts(&self) -> &[lib_database::Accounts] {
        match &self.status {
            Status::Loaded(accounts) => accounts,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.accounts().get(index))
            .map(|account| account.id)
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.accounts().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> crate::Result<Vec<lib_database::Accounts>> {
        let pool = db::connect().await?;
        lib_database::Accounts::find_all(&pool)
            .await
            .map_err(crate::error::Error::from)
    }

    async fn delete(id: lib_core::RowID) -> crate::Result<()> {
        let pool = db::connect().await?;
        lib_database::Accounts::delete_by_id(id, &pool)
            .await
            .map_err(crate::error::Error::from)
    }
}

impl Default for AccountsListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for AccountsListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(accounts) => Action::AccountsLoaded(accounts),
                Err(err) => Action::AccountsLoadFailed(err.to_string()),
            };
            let _ = load_tx.send(action);
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
                                Ok(()) => Action::AccountDeleted(pending_id),
                                Err(err) => Action::AccountDeleteFailed(err.to_string()),
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
                .and_then(|index| self.accounts().get(index))
                .cloned()
                .map(|account| Action::OpenAccountDetail(Some(account))),
            KeyCode::Char('n') => Some(Action::OpenAccountDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::AccountsLoaded(accounts) => {
                self.status = Status::Loaded(accounts.clone());
                if self.table_state.selected().unwrap_or(0) >= accounts.len()
                    && !accounts.is_empty()
                {
                    self.table_state.select(Some(accounts.len() - 1));
                }
            }
            Action::AccountsLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::AccountSaved(saved) => {
                if let Status::Loaded(accounts) = &mut self.status {
                    match accounts.iter_mut().find(|a| a.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => accounts.push(saved.clone()),
                    }
                    accounts.sort_by(|a, b| a.name.cmp(&b.name));
                }
            }
            Action::AccountDeleted(id) => {
                if let Status::Loaded(accounts) = &mut self.status {
                    accounts.retain(|a| a.id != *id);
                }
            }
            Action::AccountSaveFailed(message) | Action::AccountDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Accounts"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Accounts...")
                        .block(Block::bordered().title(" Accounts ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Accounts: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Accounts ")),
                    rows[0],
                );
            }
            Status::Loaded(accounts) => {
                let header = Row::new(["Name", "Type", "Starting Balance", "Active"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = accounts.iter().map(|account| {
                    Row::new([
                        Cell::from(account.name.clone()),
                        Cell::from(account.account_type.as_str()),
                        Cell::from(account.starting_balance.to_string()),
                        Cell::from(if account.is_active { "yes" } else { "no" }),
                    ])
                });
                let widths = [
                    Constraint::Length(24),
                    Constraint::Length(14),
                    Constraint::Length(18),
                    Constraint::Length(6),
                ];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(Block::bordered().title(format!(" Accounts ({}) ", accounts.len())));
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Account? y: confirm, any other key: cancel".to_string()
        } else if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            "↑/k ↓/j: move  Enter: edit  n: new  d: delete  Esc: back".to_string()
        };
        frame.render_widget(Paragraph::new(footer), rows[1]);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn mock_account(name: &str) -> lib_database::Accounts {
        let now = chrono::Utc::now();
        lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: name.to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id: lib_core::RowID::new(),
            starting_balance: lib_core::Money::mock(),
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = AccountsListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Accounts list screen should not error");
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
        render(Status::Loaded(vec![mock_account("Everyday Spending")]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = AccountsListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenAccountDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_account() {
        let mut screen = AccountsListScreen::new();
        let account = mock_account("Everyday Spending");
        screen.update(&Action::AccountsLoaded(vec![account.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenAccountDetail(Some(account)))
        );
    }

    #[test]
    fn d_arms_delete_confirmation_and_a_non_y_key_cancels_it() {
        let mut screen = AccountsListScreen::new();
        screen.update(&Action::AccountsLoaded(vec![mock_account(
            "Everyday Spending",
        )]));

        screen.handle_key(key(KeyCode::Char('d')), InputMode::Navigation);
        assert!(screen.pending_delete.is_some());

        screen.handle_key(key(KeyCode::Char('x')), InputMode::Navigation);
        assert!(screen.pending_delete.is_none());
    }

    #[test]
    fn account_deleted_removes_it_from_the_loaded_list() {
        let mut screen = AccountsListScreen::new();
        let account = mock_account("Everyday Spending");
        screen.update(&Action::AccountsLoaded(vec![account.clone()]));
        screen.update(&Action::AccountDeleted(account.id));

        assert!(screen.accounts().is_empty());
    }

    #[test]
    fn account_saved_inserts_a_new_row_or_replaces_an_existing_one() {
        let mut screen = AccountsListScreen::new();
        let mut account = mock_account("Everyday Spending");
        screen.update(&Action::AccountsLoaded(vec![account.clone()]));

        account.name = "Updated".to_string();
        screen.update(&Action::AccountSaved(account.clone()));

        assert_eq!(screen.accounts().len(), 1);
        assert_eq!(screen.accounts()[0].name, "Updated");
    }
}
