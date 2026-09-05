//! The Balance Checks create/edit screen (CC-TUI-013). Mirrors `AccountDetailScreen`'s
//! create-vs-edit field split: FR.31 only allows changing date/asserted balance —
//! `account_id` is fixed at creation, so edit mode skips the Account picker entirely.

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
    Date,
    AssertedBalance,
}

/// Create mode cycles through every field; edit mode skips `Account`, which FR.31 doesn't
/// allow changing after creation.
const CREATE_FIELDS: [Field; 3] = [Field::Account, Field::Date, Field::AssertedBalance];
const EDIT_FIELDS: [Field; 2] = [Field::Date, Field::AssertedBalance];

enum AccountsStatus {
    Loading,
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

/// Create (empty) or edit (pre-filled) one Balance Check.
pub struct BalanceCheckDetailScreen {
    editing_id: Option<lib_core::RowID>,
    /// The Account's own id, not just a list index — so an edit form can still show the
    /// Balance Check's existing Account even if it's since fallen out of the active-Accounts
    /// list this screen loads for the create-mode picker.
    account_id: Option<lib_core::RowID>,
    accounts: AccountsStatus,
    date: String,
    asserted_balance: String,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl BalanceCheckDetailScreen {
    /// An empty form for creating a new Balance Check, defaulting to today's date.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            account_id: None,
            accounts: AccountsStatus::Loading,
            date: chrono::Utc::now().date_naive().to_string(),
            asserted_balance: "0".to_string(),
            focus: Field::Account,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Balance Check's fields. `Account` is shown but not
    /// editable (FR.31).
    pub fn new_edit(balance_check: lib_database::BalanceChecks) -> Self {
        Self {
            editing_id: Some(balance_check.id),
            account_id: Some(balance_check.account_id),
            accounts: AccountsStatus::Loading,
            date: balance_check.date.to_string(),
            asserted_balance: balance_check.asserted_balance.to_string(),
            focus: Field::Date,
            error: None,
            action_tx: None,
        }
    }

    fn fields(&self) -> &'static [Field] {
        if self.editing_id.is_some() {
            &EDIT_FIELDS
        } else {
            &CREATE_FIELDS
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

    /// Validates the form and, if valid, spawns the async save; stores the validation error
    /// on `self.error` when the form isn't ready to submit.
    fn save(&mut self) {
        let date: chrono::NaiveDate = match self.date.parse() {
            Ok(date) => date,
            Err(_) => {
                self.error = Some("Date must be in YYYY-MM-DD format".to_string());
                return;
            }
        };
        let asserted_balance: lib_core::Money = match self.asserted_balance.parse() {
            Ok(amount) => amount,
            Err(_) => {
                self.error = Some("Asserted balance must be a valid decimal amount".to_string());
                return;
            }
        };

        let is_create = self.editing_id.is_none();

        // account_id is only read from the form on create; on edit it's carried over
        // unchanged from whatever was passed to `new_edit`.
        let account_id = if is_create {
            let Some(account_id) = self.account_id else {
                self.error =
                    Some("Select an Account — create one first if none exist yet".to_string());
                return;
            };
            account_id
        } else {
            self.account_id
                .expect("edit form was pre-filled with a real account_id")
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let now = chrono::Utc::now();
        let balance_check = lib_database::BalanceChecks {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            account_id,
            date,
            asserted_balance,
            created_on: now,
            updated_on: now,
        };

        tokio::spawn(async move {
            let result = async {
                let pool = db::connect().await?;
                if is_create {
                    balance_check.insert(&pool).await
                } else {
                    balance_check.update(&pool).await
                }
            }
            .await;

            let action = match result {
                Ok(saved) => Action::BalanceCheckSaved(saved),
                Err(err) => Action::BalanceCheckSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Screen for BalanceCheckDetailScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Accounts::find_all_active(&pool).await
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
                self.save();
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
            Field::Date => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => self.date.push(c),
                KeyCode::Backspace => {
                    self.date.pop();
                }
                _ => {}
            },
            Field::AssertedBalance => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                    self.asserted_balance.push(c)
                }
                KeyCode::Backspace => {
                    self.asserted_balance.pop();
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
            Action::BalanceCheckSaveFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Balance Check"
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
                Span::raw(format!("{label:<16}")),
                Span::styled(value, style),
            ])
        };

        let mut lines = vec![
            field_line("Account", self.selected_account_label(), Field::Account),
            field_line("Date", self.date.clone(), Field::Date),
            field_line(
                "Asserted balance",
                self.asserted_balance.clone(),
                Field::AssertedBalance,
            ),
            Line::raw(""),
        ];

        if self.editing_id.is_some() {
            lines.push(Line::styled(
                "Account is fixed at creation and can't be changed here.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Account  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Balance Check "
        } else {
            " New Balance Check "
        };
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(title)),
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

    fn render(screen: &BalanceCheckDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Balance Check detail screen should not error");
    }

    #[test]
    fn new_create_starts_empty_with_all_three_fields_in_the_cycle() {
        let screen = BalanceCheckDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert_eq!(screen.fields(), CREATE_FIELDS);
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_balance_check_and_skips_account() {
        let account_id = lib_core::RowID::new();
        let balance_check = mock_balance_check(account_id);
        let screen = BalanceCheckDetailScreen::new_edit(balance_check.clone());

        assert_eq!(screen.editing_id, Some(balance_check.id));
        assert_eq!(screen.account_id, Some(account_id));
        assert_eq!(screen.fields(), EDIT_FIELDS);
        assert!(!EDIT_FIELDS.contains(&Field::Account));
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_through_only_the_create_fields() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Account);
        for expected in [Field::Date, Field::AssertedBalance, Field::Account] {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
            assert_eq!(screen.focus, expected);
        }
    }

    #[test]
    fn tab_skips_account_while_editing() {
        let balance_check = mock_balance_check(lib_core::RowID::new());
        let mut screen = BalanceCheckDetailScreen::new_edit(balance_check);
        assert_eq!(screen.focus, Field::Date);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::AssertedBalance);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::Date, "Account is skipped");
    }

    #[test]
    fn cycling_account_moves_through_the_loaded_active_accounts() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        let accounts = vec![mock_account("Everyday"), mock_account("Savings")];
        screen.update(&Action::AccountsLoaded(accounts.clone()));
        assert_eq!(screen.account_id, Some(accounts[0].id));

        screen.focus = Field::Account;
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.account_id, Some(accounts[1].id));

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.account_id, Some(accounts[0].id), "cycling wraps");
    }

    #[test]
    fn typing_appends_to_the_date_field() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        screen.focus = Field::Date;
        screen.date.clear();
        screen.handle_key(key(KeyCode::Char('2')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('0')), InputMode::Navigation);
        assert_eq!(screen.date, "20");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.date, "2");
    }

    #[test]
    fn save_rejects_an_invalid_date() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.date = "not-a-date".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_an_invalid_asserted_balance() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.asserted_balance = "not-a-number".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_when_no_account_is_selected() {
        let mut screen = BalanceCheckDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        // No AccountsLoaded delivered yet -- account_id stays None.
        screen.save();
        assert!(screen.error.is_some());
    }
}
