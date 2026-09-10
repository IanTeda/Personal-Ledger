//! The Balance Checks CSV import screen (FR.33, CC-TUI-012) — reached from the Balance
//! Checks list screen (`i`), not its own dashboard area, per "Decide TUI screen map and
//! navigation shape". A one-shot action, not a persisted-entity form: pick the target
//! Account and a file path, then import. The whole file is validated and inserted
//! atomically by `lib_database::BalanceChecks::import_csv` — a malformed row aborts the
//! entire import, nothing is committed.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

/// Which field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Account,
    FilePath,
}

const FIELDS: [Field; 2] = [Field::Account, Field::FilePath];

enum AccountsStatus {
    Loading,
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

/// Imports Balance Checks for one Account from a CSV file.
pub struct CsvImportScreen {
    account_id: Option<lib_core::RowID>,
    accounts: AccountsStatus,
    file_path: String,
    focus: Field,
    error: Option<String>,
    /// Set after a successful import — shown until the user backs out.
    success_message: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl CsvImportScreen {
    pub fn new() -> Self {
        Self {
            account_id: None,
            accounts: AccountsStatus::Loading,
            file_path: String::new(),
            focus: Field::Account,
            error: None,
            success_message: None,
            action_tx: None,
        }
    }

    fn move_focus(&mut self, delta: isize) {
        let len = FIELDS.len() as isize;
        let current = FIELDS.iter().position(|f| *f == self.focus).unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len);
        self.focus = FIELDS[next as usize];
        self.error = None;
    }

    fn cycle_account(&mut self, delta: isize) {
        let AccountsStatus::Loaded(accounts) = &self.accounts else {
            return;
        };
        if accounts.is_empty() {
            return;
        }
        let current = self
            .account_id
            .and_then(|id| accounts.iter().position(|a| a.id == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(accounts.len() as isize);
        self.account_id = Some(accounts[next as usize].id);
    }

    fn selected_account_label(&self) -> String {
        match &self.accounts {
            AccountsStatus::Loading => "Loading Accounts...".to_string(),
            AccountsStatus::Failed(message) => format!("Failed to load Accounts: {message}"),
            AccountsStatus::Loaded(accounts) if accounts.is_empty() => {
                "No Accounts yet — create one first".to_string()
            }
            AccountsStatus::Loaded(accounts) => self
                .account_id
                .and_then(|id| accounts.iter().find(|a| a.id == id))
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "(none selected)".to_string()),
        }
    }

    /// Validates the form and, if valid, spawns the async import; stores the validation
    /// error on `self.error` when the form isn't ready to submit.
    fn import(&mut self) {
        let Some(account_id) = self.account_id else {
            self.error = Some("Select an Account — create one first if none exist yet".to_string());
            return;
        };
        let path = self.file_path.trim().to_string();
        if path.is_empty() {
            self.error = Some("Enter a file path".to_string());
            return;
        }

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        self.success_message = None;
        tokio::spawn(async move {
            let result = Self::run_import(account_id, path).await;
            let action = match result {
                Ok(imported) => Action::BalanceChecksImported(imported),
                Err(err) => Action::BalanceChecksImportFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    async fn run_import(
        account_id: lib_core::RowID,
        path: String,
    ) -> crate::Result<Vec<lib_database::BalanceChecks>> {
        let pool = db::connect().await?;
        let file = std::fs::File::open(&path).map_err(|err| {
            lib_database::Error::CsvImport(format!("could not open {path}: {err}"))
        })?;
        lib_database::BalanceChecks::import_csv(account_id, file, &pool)
            .await
            .map_err(crate::error::Error::from)
    }
}

impl Default for CsvImportScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for CsvImportScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Accounts::find_all_active(&pool)
                    .await
                    .map_err(crate::error::Error::from)
            }
            .await;
            let action = match action {
                Ok(accounts) => Action::AccountsLoaded(accounts),
                Err(err) => Action::AccountsLoadFailed(err.to_string()),
            };
            let _ = load_tx.send(action);
        });
        self.action_tx = Some(action_tx);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        match key.code {
            KeyCode::Esc => return Some(Action::Back),
            KeyCode::Tab => {
                self.move_focus(1);
                return Some(Action::NoOp);
            }
            KeyCode::BackTab => {
                self.move_focus(-1);
                return Some(Action::NoOp);
            }
            KeyCode::Enter => {
                self.import();
                return Some(Action::NoOp);
            }
            _ => {}
        }

        match self.focus {
            Field::Account => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_account(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_account(1),
                _ => {}
            },
            Field::FilePath => match key.code {
                KeyCode::Char(c) => self.file_path.push(c),
                KeyCode::Backspace => {
                    self.file_path.pop();
                }
                _ => {}
            },
        }

        Some(Action::NoOp)
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::AccountsLoaded(accounts) => {
                if self.account_id.is_none() {
                    self.account_id = accounts.first().map(|a| a.id);
                }
                self.accounts = AccountsStatus::Loaded(accounts.clone());
            }
            Action::AccountsLoadFailed(message) => {
                self.accounts = AccountsStatus::Failed(message.clone());
            }
            Action::BalanceChecksImported(imported) => {
                self.error = None;
                self.success_message = Some(format!(
                    "Imported {} Balance Check{}",
                    imported.len(),
                    if imported.len() == 1 { "" } else { "s" }
                ));
            }
            Action::BalanceChecksImportFailed(message) => {
                self.success_message = None;
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Import Balance Checks"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let field_line = |label: &str, value: String, field: Field| {
            let style = if self.focus == field {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::raw(format!("{label:<12}")),
                Span::styled(value, style),
            ])
        };

        let mut lines = vec![
            field_line("Account", self.selected_account_label(), Field::Account),
            field_line("File path", self.file_path.clone(), Field::FilePath),
            Line::raw(""),
            Line::styled(
                "CSV must have a header row with \"date\" and \"balance\" columns (any order).",
                Style::default().fg(Color::DarkGray),
            ),
            Line::styled(
                "Dates are YYYY-MM-DD. A malformed row aborts the whole import.",
                Style::default().fg(Color::DarkGray),
            ),
            Line::raw(""),
        ];

        if let Some(success) = &self.success_message {
            lines.push(Line::styled(
                success.clone(),
                Style::default().fg(Color::Green),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Account  Enter: import  Esc: back",
        ));

        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(" Import Balance Checks ")),
            area,
        );
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

    fn render(screen: &CsvImportScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the CSV import screen should not error");
    }

    #[test]
    fn new_starts_empty() {
        let screen = CsvImportScreen::new();
        assert!(screen.account_id.is_none());
        assert!(screen.file_path.is_empty());
        render(&screen);
    }

    #[test]
    fn esc_backs_out() {
        let mut screen = CsvImportScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_between_the_two_fields() {
        let mut screen = CsvImportScreen::new();
        assert_eq!(screen.focus, Field::Account);
        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::FilePath);
        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::Account);
    }

    #[test]
    fn typing_appends_to_the_file_path_field() {
        let mut screen = CsvImportScreen::new();
        screen.focus = Field::FilePath;
        screen.handle_key(key(KeyCode::Char('/')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('a')), InputMode::Navigation);
        assert_eq!(screen.file_path, "/a");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.file_path, "/");
    }

    #[test]
    fn cycling_account_moves_through_the_loaded_active_accounts() {
        let mut screen = CsvImportScreen::new();
        let accounts = vec![mock_account("Everyday"), mock_account("Savings")];
        screen.update(&Action::AccountsLoaded(accounts.clone()));
        assert_eq!(screen.account_id, Some(accounts[0].id));

        screen.focus = Field::Account;
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.account_id, Some(accounts[1].id));
    }

    #[test]
    fn import_rejects_when_no_account_is_selected() {
        let mut screen = CsvImportScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.file_path = "/tmp/some.csv".to_string();
        screen.import();
        assert!(screen.error.is_some());
    }

    #[test]
    fn import_rejects_an_empty_file_path() {
        let mut screen = CsvImportScreen::new();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.update(&Action::AccountsLoaded(vec![mock_account("Everyday")]));
        screen.import();
        assert!(screen.error.is_some());
    }

    #[test]
    fn balance_checks_imported_sets_a_success_message() {
        let mut screen = CsvImportScreen::new();
        let now = chrono::Utc::now();
        screen.update(&Action::BalanceChecksImported(vec![
            lib_database::BalanceChecks {
                id: lib_core::RowID::new(),
                account_id: lib_core::RowID::new(),
                date: now.date_naive(),
                asserted_balance: lib_core::Money::mock(),
                created_on: now,
                updated_on: now,
            },
        ]));
        assert_eq!(
            screen.success_message.as_deref(),
            Some("Imported 1 Balance Check")
        );
    }

    #[test]
    fn balance_checks_import_failed_sets_an_error() {
        let mut screen = CsvImportScreen::new();
        screen.update(&Action::BalanceChecksImportFailed("boom".to_string()));
        assert_eq!(screen.error.as_deref(), Some("boom"));
    }
}
