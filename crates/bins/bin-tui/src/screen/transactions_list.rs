//! The Transactions list screen (CC-TUI-009), mirroring the Units/Categories/Accounts list
//! shape. Needs Account and Category names for its columns, not just ids, so `init()` also
//! kicks off Account/Category loads reusing `AccountsLoaded`/`CategoriesLoaded` — the same
//! actions the Accounts/Categories list screens and `AccountDetailScreen`'s picker already
//! use, rather than a Transactions-specific lookup action.

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
    Loaded(Vec<lib_database::Transactions>),
    Failed(String),
}

/// Lists every Transaction; the entry point for creating, editing, and deleting them.
pub struct TransactionsListScreen {
    status: Status,
    accounts: Vec<lib_database::Accounts>,
    categories: Vec<lib_database::Categories>,
    table_state: TableState,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl TransactionsListScreen {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            status: Status::Loading,
            accounts: Vec::new(),
            categories: Vec::new(),
            table_state,
            pending_delete: None,
            error: None,
            action_tx: None,
        }
    }

    fn transactions(&self) -> &[lib_database::Transactions] {
        match &self.status {
            Status::Loaded(transactions) => transactions,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.transactions().get(index))
            .map(|transaction| transaction.id)
    }

    fn account_name(&self, id: lib_core::RowID) -> &str {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn category_name(&self, id: lib_core::RowID) -> &str {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.transactions().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> crate::Result<Vec<lib_database::Transactions>> {
        let pool = db::connect().await?;
        lib_database::Transactions::find_all(&pool)
            .await
            .map_err(crate::error::Error::from)
    }

    async fn delete(id: lib_core::RowID) -> crate::Result<()> {
        let pool = db::connect().await?;
        lib_database::Transactions::delete_by_id(id, &pool)
            .await
            .map_err(crate::error::Error::from)
    }
}

impl Default for TransactionsListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TransactionsListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let transactions_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(transactions) => Action::TransactionsLoaded(transactions),
                Err(err) => Action::TransactionsLoadFailed(err.to_string()),
            };
            let _ = transactions_tx.send(action);
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

        let categories_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Categories::find_all(&pool)
                    .await
                    .map_err(crate::error::Error::from)
            }
            .await;
            let action = match action {
                Ok(categories) => Action::CategoriesLoaded(categories),
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
            };
            let _ = categories_tx.send(action);
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
                                Ok(()) => Action::TransactionDeleted(pending_id),
                                Err(err) => Action::TransactionDeleteFailed(err.to_string()),
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
                .and_then(|index| self.transactions().get(index))
                .cloned()
                .map(|transaction| Action::OpenTransactionDetail(Some(transaction))),
            KeyCode::Char('n') => Some(Action::OpenTransactionDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::TransactionsLoaded(transactions) => {
                self.status = Status::Loaded(transactions.clone());
                if self.table_state.selected().unwrap_or(0) >= transactions.len()
                    && !transactions.is_empty()
                {
                    self.table_state.select(Some(transactions.len() - 1));
                }
            }
            Action::TransactionsLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::AccountsLoaded(accounts) => self.accounts = accounts.clone(),
            Action::CategoriesLoaded(categories) => self.categories = categories.clone(),
            Action::TransactionSaved(saved) => {
                if let Status::Loaded(transactions) = &mut self.status {
                    match transactions.iter_mut().find(|t| t.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => transactions.insert(0, saved.clone()),
                    }
                    transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
                }
            }
            Action::TransactionDeleted(id) => {
                if let Status::Loaded(transactions) = &mut self.status {
                    transactions.retain(|t| t.id != *id);
                }
            }
            Action::TransactionSaveFailed(message) | Action::TransactionDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Transactions"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Transactions...")
                        .block(Block::bordered().title(" Transactions ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Transactions: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Transactions ")),
                    rows[0],
                );
            }
            Status::Loaded(transactions) => {
                let header = Row::new(["Date", "Account", "Category", "Amount", "Status", "Flag"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = transactions.iter().map(|transaction| {
                    Row::new([
                        Cell::from(transaction.date.to_string()),
                        Cell::from(self.account_name(transaction.account_id).to_string()),
                        Cell::from(self.category_name(transaction.category_id).to_string()),
                        Cell::from(transaction.amount.to_string()),
                        Cell::from(transaction.status.as_str()),
                        Cell::from(if transaction.is_flagged { "!" } else { "" }),
                    ])
                });
                let widths = [
                    Constraint::Length(11),
                    Constraint::Length(18),
                    Constraint::Length(16),
                    Constraint::Length(12),
                    Constraint::Length(11),
                    Constraint::Length(4),
                ];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(
                        Block::bordered().title(format!(" Transactions ({}) ", transactions.len())),
                    );
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Transaction? y: confirm, any other key: cancel".to_string()
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

    fn mock_transaction() -> lib_database::Transactions {
        lib_database::Transactions {
            id: lib_core::RowID::new(),
            date: chrono::Utc::now().date_naive(),
            amount: lib_core::Money::mock(),
            category_id: lib_core::RowID::new(),
            account_id: lib_core::RowID::new(),
            payee_id: None,
            description: None,
            status: lib_core::TransactionStatus::Open,
            is_flagged: false,
            updated_on: chrono::Utc::now(),
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = TransactionsListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Transactions list screen should not error");
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
        render(Status::Loaded(vec![mock_transaction()]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = TransactionsListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenTransactionDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_transaction() {
        let mut screen = TransactionsListScreen::new();
        let transaction = mock_transaction();
        screen.update(&Action::TransactionsLoaded(vec![transaction.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenTransactionDetail(Some(transaction)))
        );
    }

    #[test]
    fn transaction_deleted_removes_it_from_the_loaded_list() {
        let mut screen = TransactionsListScreen::new();
        let transaction = mock_transaction();
        screen.update(&Action::TransactionsLoaded(vec![transaction.clone()]));
        screen.update(&Action::TransactionDeleted(transaction.id));

        assert!(screen.transactions().is_empty());
    }

    #[test]
    fn account_and_category_names_resolve_once_loaded() {
        let mut screen = TransactionsListScreen::new();
        let now = chrono::Utc::now();
        let unit_id = lib_core::RowID::new();
        let account = lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: "Everyday Spending".to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id,
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
