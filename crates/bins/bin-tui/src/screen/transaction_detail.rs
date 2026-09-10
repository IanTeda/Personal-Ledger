//! The Transactions create/edit screen (CC-TUI-009). Unlike Accounts' create-vs-edit field
//! split, here the Tab-cycle narrows based on the Transaction's *current, live* status in
//! the form: once it's Reconciled, only Status and Flagged stay editable (FR.19's
//! Reconciled-lock) — but cycling Status away from Reconciled in the same session
//! immediately reopens the other fields, letting a user un-reconcile and fix a mistake in
//! one submission. `save()` relies on `lib_database::Transactions::update`'s own
//! current-status check as the actual authority; the field-cycle restriction here is a UX
//! guard, not the only enforcement.

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
    Date,
    Amount,
    Category,
    Account,
    Payee,
    Description,
    Status,
    IsFlagged,
}

const FULL_FIELDS: [Field; 8] = [
    Field::Date,
    Field::Amount,
    Field::Category,
    Field::Account,
    Field::Payee,
    Field::Description,
    Field::Status,
    Field::IsFlagged,
];
/// While Reconciled, only Status/Flagged stay in the cycle (FR.19).
const LOCKED_FIELDS: [Field; 2] = [Field::Status, Field::IsFlagged];

enum PickerStatus<T> {
    Loading,
    Loaded(Vec<T>),
    Failed(String),
}

/// Create (empty) or edit (pre-filled) one Transaction.
pub struct TransactionDetailScreen {
    editing_id: Option<lib_core::RowID>,
    /// The as-loaded Transaction, used to diff against on save so only the fields that
    /// actually changed get written — `set_status`/`set_flagged`/`update` are three
    /// independent calls, not one.
    original: Option<lib_database::Transactions>,
    date: String,
    amount: String,
    category_id: Option<lib_core::RowID>,
    categories: PickerStatus<lib_database::Categories>,
    account_id: Option<lib_core::RowID>,
    accounts: PickerStatus<lib_database::Accounts>,
    /// The typed Payee name — resolved to a `payee_id` at save time via
    /// `Payees::resolve_or_create`, reusing an existing (or aliased) Payee or auto-creating
    /// a new one. Unlike Category/Account, Payee has no fixed Left/Right-cyclable set, so
    /// this stays free text with suggestions rather than a picker over `payees`.
    payee: String,
    payees: PickerStatus<lib_database::Payees>,
    payee_aliases: Vec<lib_database::PayeeAliases>,
    /// Which suggestion in `payee_suggestions()`'s current result is highlighted.
    payee_suggestion_index: usize,
    description: String,
    status: lib_core::TransactionStatus,
    is_flagged: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl TransactionDetailScreen {
    /// An empty form for creating a new Transaction, defaulting to today's date.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            original: None,
            date: chrono::Utc::now().date_naive().to_string(),
            amount: "0".to_string(),
            category_id: None,
            categories: PickerStatus::Loading,
            account_id: None,
            accounts: PickerStatus::Loading,
            payee: String::new(),
            payees: PickerStatus::Loading,
            payee_aliases: Vec::new(),
            payee_suggestion_index: 0,
            description: String::new(),
            status: lib_core::TransactionStatus::default(),
            is_flagged: false,
            focus: Field::Date,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Transaction's fields. When the Transaction is
    /// already Reconciled, focus starts on `Status` rather than `Date` — `Date` isn't in
    /// the locked field cycle, so starting there would let a stray keystroke edit it.
    pub fn new_edit(transaction: lib_database::Transactions) -> Self {
        let mut screen = Self {
            editing_id: Some(transaction.id),
            date: transaction.date.to_string(),
            amount: transaction.amount.to_string(),
            category_id: Some(transaction.category_id),
            categories: PickerStatus::Loading,
            account_id: Some(transaction.account_id),
            accounts: PickerStatus::Loading,
            // Filled in once Payees load (`update`'s `PayeesLoaded` arm) — the name isn't
            // known from `transaction.payee_id` alone.
            payee: String::new(),
            payees: PickerStatus::Loading,
            payee_aliases: Vec::new(),
            payee_suggestion_index: 0,
            description: transaction.description.clone().unwrap_or_default(),
            status: transaction.status.clone(),
            is_flagged: transaction.is_flagged,
            focus: Field::Date,
            error: None,
            action_tx: None,
            original: Some(transaction),
        };
        screen.focus = screen.fields()[0];
        screen
    }

    fn fields(&self) -> &'static [Field] {
        if self.editing_id.is_some() && self.status == lib_core::TransactionStatus::Reconciled {
            &LOCKED_FIELDS
        } else {
            &FULL_FIELDS
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

    fn cycle_status(&mut self, delta: isize) {
        let all = lib_core::TransactionStatus::all();
        let current = all.iter().position(|s| *s == self.status).unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(all.len() as isize);
        self.status = all[next as usize].clone();
        // Cycling off Reconciled may have just widened `fields()`; cycling back onto it may
        // have just narrowed it out from under the current focus.
        if !self.fields().contains(&self.focus) {
            self.focus = Field::Status;
        }
    }

    fn cycle_category(&mut self, delta: isize) {
        let PickerStatus::Loaded(categories) = &self.categories else {
            return;
        };
        if categories.is_empty() {
            return;
        }
        let current = self
            .category_id
            .and_then(|id| categories.iter().position(|c| c.id == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(categories.len() as isize);
        self.category_id = Some(categories[next as usize].id);
    }

    fn cycle_account(&mut self, delta: isize) {
        let PickerStatus::Loaded(accounts) = &self.accounts else {
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

    fn selected_category_label(&self) -> String {
        match &self.categories {
            PickerStatus::Loading => "Loading Categories...".to_string(),
            PickerStatus::Failed(message) => format!("Failed to load Categories: {message}"),
            PickerStatus::Loaded(categories) if categories.is_empty() => {
                "No Categories yet — create one first".to_string()
            }
            PickerStatus::Loaded(categories) => self
                .category_id
                .and_then(|id| categories.iter().find(|c| c.id == id))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "(none selected)".to_string()),
        }
    }

    fn selected_account_label(&self) -> String {
        match &self.accounts {
            PickerStatus::Loading => "Loading Accounts...".to_string(),
            PickerStatus::Failed(message) => format!("Failed to load Accounts: {message}"),
            PickerStatus::Loaded(accounts) if accounts.is_empty() => {
                "No Accounts yet — create one first".to_string()
            }
            PickerStatus::Loaded(accounts) => self
                .account_id
                .and_then(|id| accounts.iter().find(|a| a.id == id))
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "(none selected)".to_string()),
        }
    }

    /// Payees whose name case-insensitively starts with the typed buffer, plus any Payee
    /// reached via an alias pattern match, deduplicated and capped at 5 — an empty buffer
    /// shows the first 5 alphabetically instead.
    fn payee_suggestions(&self) -> Vec<&lib_database::Payees> {
        let PickerStatus::Loaded(payees) = &self.payees else {
            return Vec::new();
        };

        let buffer = self.payee.trim();
        let mut matches: Vec<&lib_database::Payees> = if buffer.is_empty() {
            payees.iter().collect()
        } else {
            let lower = buffer.to_lowercase();
            let mut matches: Vec<&lib_database::Payees> = payees
                .iter()
                .filter(|payee| payee.name.to_lowercase().starts_with(&lower))
                .collect();

            for alias in &self.payee_aliases {
                let is_match = regex::Regex::new(&alias.pattern)
                    .map(|regex| regex.is_match(buffer))
                    .unwrap_or(false);
                if is_match
                    && let Some(payee) = payees.iter().find(|payee| payee.id == alias.payee_id)
                    && !matches.iter().any(|existing| existing.id == payee.id)
                {
                    matches.push(payee);
                }
            }
            matches
        };

        matches.sort_by(|a, b| a.name.cmp(&b.name));
        matches.truncate(5);
        matches
    }

    fn move_payee_suggestion(&mut self, delta: isize) {
        let len = self.payee_suggestions().len();
        if len == 0 {
            return;
        }
        let next = (self.payee_suggestion_index as isize + delta).rem_euclid(len as isize);
        self.payee_suggestion_index = next as usize;
    }

    fn accept_payee_suggestion(&mut self) {
        let name = self
            .payee_suggestions()
            .get(self.payee_suggestion_index)
            .map(|payee| payee.name.clone());
        if let Some(name) = name {
            self.payee = name;
        }
        self.payee_suggestion_index = 0;
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
        let amount: lib_core::Money = match self.amount.parse() {
            Ok(amount) => amount,
            Err(_) => {
                self.error = Some("Amount must be a valid decimal amount".to_string());
                return;
            }
        };
        let Some(category_id) = self.category_id else {
            self.error = Some("Select a Category — create one first if none exist yet".to_string());
            return;
        };
        let Some(account_id) = self.account_id else {
            self.error = Some("Select an Account — create one first if none exist yet".to_string());
            return;
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let payee_name = self.payee.trim().to_string();
        let description = if self.description.trim().is_empty() {
            None
        } else {
            Some(self.description.trim().to_string())
        };

        let original = self.original.clone();
        let form = lib_database::Transactions {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            date,
            amount,
            category_id,
            account_id,
            // Resolved from `payee_name` inside `save_transaction`, once a pool is
            // available — `Payees::resolve_or_create` is async.
            payee_id: None,
            description,
            status: self.status.clone(),
            is_flagged: self.is_flagged,
            updated_on: chrono::Utc::now(),
        };

        tokio::spawn(async move {
            let result = Self::save_transaction(form, payee_name, original).await;
            let action = match result {
                Ok(saved) => Action::TransactionSaved(saved),
                Err(err) => Action::TransactionSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    /// Create: one plain insert. Edit: up to three independent calls in order (status,
    /// flagged, then the rest) — status changes first so un-reconciling and editing other
    /// fields in the same submission is naturally allowed by `update`'s own current-status
    /// check, without this screen needing to duplicate that rule.
    async fn save_transaction(
        mut form: lib_database::Transactions,
        payee_name: String,
        original: Option<lib_database::Transactions>,
    ) -> lib_database::Result<lib_database::Transactions> {
        let pool = db::connect().await?;

        form.payee_id = if payee_name.is_empty() {
            None
        } else {
            Some(
                lib_database::Payees::resolve_or_create(&payee_name, &pool)
                    .await?
                    .id,
            )
        };

        let Some(original) = original else {
            return form.insert(&pool).await;
        };

        let mut latest = original.clone();

        if form.status != original.status {
            latest =
                lib_database::Transactions::set_status(form.id, form.status.clone(), &pool).await?;
        }
        if form.is_flagged != original.is_flagged {
            latest =
                lib_database::Transactions::set_flagged(form.id, form.is_flagged, &pool).await?;
        }

        let other_fields_changed = form.date != original.date
            || form.amount != original.amount
            || form.category_id != original.category_id
            || form.account_id != original.account_id
            || form.payee_id != original.payee_id
            || form.description != original.description;
        if other_fields_changed {
            latest = form.update(&pool).await?;
        }

        Ok(latest)
    }
}

impl Screen for TransactionDetailScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
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

        let payees_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Payees::find_all_active(&pool).await
            }
            .await;
            let action = match action {
                Ok(payees) => Action::PayeesLoaded(payees),
                Err(err) => Action::PayeesLoadFailed(err.to_string()),
            };
            let _ = payees_tx.send(action);
        });

        let payee_aliases_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::PayeeAliases::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(aliases) => Action::PayeeAliasesLoaded(aliases),
                Err(err) => Action::PayeeAliasesLoadFailed(err.to_string()),
            };
            let _ = payee_aliases_tx.send(action);
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
            Field::Date => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => self.date.push(c),
                KeyCode::Backspace => {
                    self.date.pop();
                }
                _ => {}
            },
            Field::Amount => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                    self.amount.push(c)
                }
                KeyCode::Backspace => {
                    self.amount.pop();
                }
                _ => {}
            },
            Field::Category => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_category(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_category(1),
                _ => {}
            },
            Field::Account => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_account(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_account(1),
                _ => {}
            },
            Field::Payee => match key.code {
                KeyCode::Down => self.move_payee_suggestion(1),
                KeyCode::Up => self.move_payee_suggestion(-1),
                KeyCode::Right => self.accept_payee_suggestion(),
                KeyCode::Char(c) => {
                    self.payee.push(c);
                    self.payee_suggestion_index = 0;
                }
                KeyCode::Backspace => {
                    self.payee.pop();
                    self.payee_suggestion_index = 0;
                }
                _ => {}
            },
            Field::Description => match key.code {
                KeyCode::Char(c) => self.description.push(c),
                KeyCode::Backspace => {
                    self.description.pop();
                }
                _ => {}
            },
            Field::Status => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_status(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_status(1),
                _ => {}
            },
            Field::IsFlagged => {
                if matches!(
                    key.code,
                    KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
                ) {
                    self.is_flagged = !self.is_flagged;
                }
            }
        }

        Some(Action::NoOp)
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::CategoriesLoaded(categories) => {
                if self.category_id.is_none() {
                    self.category_id = categories.first().map(|c| c.id);
                }
                self.categories = PickerStatus::Loaded(categories.clone());
            }
            Action::CategoriesLoadFailed(message) => {
                self.categories = PickerStatus::Failed(message.clone());
            }
            Action::AccountsLoaded(accounts) => {
                if self.account_id.is_none() {
                    self.account_id = accounts.first().map(|a| a.id);
                }
                self.accounts = PickerStatus::Loaded(accounts.clone());
            }
            Action::AccountsLoadFailed(message) => {
                self.accounts = PickerStatus::Failed(message.clone());
            }
            Action::PayeesLoaded(payees) => {
                if self.payee.is_empty()
                    && let Some(payee_id) = self.original.as_ref().and_then(|t| t.payee_id)
                    && let Some(payee) = payees.iter().find(|p| p.id == payee_id)
                {
                    self.payee = payee.name.clone();
                }
                self.payees = PickerStatus::Loaded(payees.clone());
            }
            Action::PayeesLoadFailed(message) => {
                self.payees = PickerStatus::Failed(message.clone());
            }
            Action::PayeeAliasesLoaded(aliases) => {
                self.payee_aliases = aliases.clone();
            }
            // Non-critical: suggestions just fall back to name-prefix matching only.
            Action::PayeeAliasesLoadFailed(_) => {}
            Action::TransactionSaveFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Transaction"
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
                Span::raw(format!("{label:<14}")),
                Span::styled(value, style),
            ])
        };

        let mut lines = vec![
            field_line("Date", self.date.clone(), Field::Date),
            field_line("Amount", self.amount.clone(), Field::Amount),
            field_line("Category", self.selected_category_label(), Field::Category),
            field_line("Account", self.selected_account_label(), Field::Account),
            field_line("Payee", self.payee.clone(), Field::Payee),
            field_line("Description", self.description.clone(), Field::Description),
            field_line("Status", self.status.as_str().to_string(), Field::Status),
            field_line(
                "Flagged",
                if self.is_flagged {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
                Field::IsFlagged,
            ),
            Line::raw(""),
        ];

        if self.focus == Field::Payee {
            let suggestions = self.payee_suggestions();
            if !suggestions.is_empty() {
                let text = suggestions
                    .iter()
                    .enumerate()
                    .map(|(index, payee)| {
                        if index == self.payee_suggestion_index {
                            format!("[{}]", payee.name)
                        } else {
                            payee.name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                lines.push(Line::styled(
                    format!("  Payee suggestions (↓/↑ highlight, → accept): {text}"),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::raw(""));
            }
        }

        if self.editing_id.is_some() && self.status == lib_core::TransactionStatus::Reconciled {
            lines.push(Line::styled(
                "Reconciled — only Status/Flagged can change until this moves back to Open or Cleared.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Category/Account/Status/Flagged  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Transaction "
        } else {
            " New Transaction "
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

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn mock_transaction(status: lib_core::TransactionStatus) -> lib_database::Transactions {
        lib_database::Transactions {
            id: lib_core::RowID::new(),
            date: chrono::Utc::now().date_naive(),
            amount: lib_core::Money::mock(),
            category_id: lib_core::RowID::new(),
            account_id: lib_core::RowID::new(),
            payee_id: None,
            description: Some("Groceries".to_string()),
            status,
            is_flagged: false,
            updated_on: chrono::Utc::now(),
        }
    }

    #[test]
    fn new_create_starts_empty_with_all_eight_fields_in_the_cycle() {
        let screen = TransactionDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert_eq!(screen.status, lib_core::TransactionStatus::Open);
        assert!(!screen.amount.is_empty());
        assert_eq!(screen.fields().len(), 8);
        assert_eq!(screen.focus, Field::Date);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_transaction() {
        let transaction = mock_transaction(lib_core::TransactionStatus::Open);
        let screen = TransactionDetailScreen::new_edit(transaction.clone());

        assert_eq!(screen.editing_id, Some(transaction.id));
        assert_eq!(screen.date, transaction.date.to_string());
        assert_eq!(screen.description, "Groceries");
        assert_eq!(screen.fields().len(), 8);
    }

    fn mock_payee(name: &str) -> lib_database::Payees {
        let now = chrono::Utc::now();
        lib_database::Payees {
            id: lib_core::RowID::new(),
            name: name.to_string(),
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    #[test]
    fn payees_loaded_resolves_the_original_transactions_payee_name() {
        let mut transaction = mock_transaction(lib_core::TransactionStatus::Open);
        let payee = mock_payee("Woolworths");
        transaction.payee_id = Some(payee.id);
        let mut screen = TransactionDetailScreen::new_edit(transaction);

        screen.update(&Action::PayeesLoaded(vec![payee]));

        assert_eq!(screen.payee, "Woolworths");
    }

    #[test]
    fn payee_suggestions_prefix_matches_case_insensitively() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.update(&Action::PayeesLoaded(vec![
            mock_payee("Woolworths"),
            mock_payee("Kmart"),
        ]));
        screen.payee = "wool".to_string();

        let suggestions: Vec<&str> = screen
            .payee_suggestions()
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        assert_eq!(suggestions, vec!["Woolworths"]);
    }

    #[test]
    fn payee_suggestions_include_an_alias_match() {
        let mut screen = TransactionDetailScreen::new_create();
        let renamed = mock_payee("Kmart AU");
        screen.update(&Action::PayeesLoaded(vec![renamed.clone()]));
        screen.update(&Action::PayeeAliasesLoaded(vec![
            lib_database::PayeeAliases {
                id: lib_core::RowID::new(),
                payee_id: renamed.id,
                pattern: "(?i)^Kmart$".to_string(),
            },
        ]));
        screen.payee = "Kmart".to_string();

        let suggestions: Vec<&str> = screen
            .payee_suggestions()
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        assert_eq!(suggestions, vec!["Kmart AU"]);
    }

    #[test]
    fn accept_payee_suggestion_fills_the_buffer() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.focus = Field::Payee;
        screen.update(&Action::PayeesLoaded(vec![mock_payee("Woolworths")]));
        screen.payee = "wool".to_string();

        screen.accept_payee_suggestion();

        assert_eq!(screen.payee, "Woolworths");
    }

    #[test]
    fn editing_a_reconciled_transaction_locks_every_field_but_status_and_flagged() {
        let transaction = mock_transaction(lib_core::TransactionStatus::Reconciled);
        let screen = TransactionDetailScreen::new_edit(transaction);

        assert_eq!(screen.fields(), &LOCKED_FIELDS);
        assert_eq!(screen.focus, Field::Status);
    }

    #[test]
    fn a_new_transaction_is_never_locked_even_if_status_starts_as_reconciled() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.status = lib_core::TransactionStatus::Reconciled;

        assert_eq!(screen.fields(), &FULL_FIELDS);
    }

    #[test]
    fn tab_cycles_through_all_eight_fields_and_wraps() {
        let mut screen = TransactionDetailScreen::new_create();
        for _ in 0..8 {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        }
        assert_eq!(screen.focus, Field::Date);
    }

    #[test]
    fn cycling_status_to_reconciled_narrows_the_cycle_and_moves_focus_off_a_now_locked_field() {
        let transaction = mock_transaction(lib_core::TransactionStatus::Open);
        let mut screen = TransactionDetailScreen::new_edit(transaction);
        screen.focus = Field::Payee;

        // Cycle Open -> Cleared -> Reconciled.
        screen.status = lib_core::TransactionStatus::Cleared;
        screen.cycle_status(1);

        assert_eq!(screen.status, lib_core::TransactionStatus::Reconciled);
        assert_eq!(screen.focus, Field::Status);
    }

    #[test]
    fn cycling_status_away_from_reconciled_reopens_the_full_cycle() {
        let transaction = mock_transaction(lib_core::TransactionStatus::Reconciled);
        let mut screen = TransactionDetailScreen::new_edit(transaction);

        screen.cycle_status(-1);

        assert_eq!(screen.status, lib_core::TransactionStatus::Cleared);
        assert_eq!(screen.fields(), &FULL_FIELDS);
    }

    #[test]
    fn typing_appends_to_the_focused_text_field() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.focus = Field::Payee;
        screen.handle_key(key(KeyCode::Char('X')), InputMode::Navigation);
        assert_eq!(screen.payee, "X");
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = TransactionDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn save_rejects_an_invalid_date() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.date = "not-a-date".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_an_invalid_amount() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.amount = "not-a-number".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_when_no_category_is_selected() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.amount = "10".to_string();
        screen.account_id = Some(lib_core::RowID::new());
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_when_no_account_is_selected() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.amount = "10".to_string();
        screen.category_id = Some(lib_core::RowID::new());
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn toggling_flagged_flips_the_boolean() {
        let mut screen = TransactionDetailScreen::new_create();
        screen.focus = Field::IsFlagged;
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(screen.is_flagged);
    }
}
