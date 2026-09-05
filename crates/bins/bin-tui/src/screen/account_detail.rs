//! The Accounts create/edit screen (CC-TUI-008). Unlike Units/Categories, editing an
//! existing Account is narrower than creating one — FR.13 only allows changing name, type,
//! and active status; `unit_id` and `starting_balance` are fixed at creation (FR.10) — so
//! this screen's focus cycle differs between create and edit mode rather than sharing one
//! fixed field list like `UnitDetailScreen`/`CategoryDetailScreen` do.

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
    Name,
    AccountType,
    Unit,
    StartingBalance,
    IsActive,
}

/// Create mode cycles through every field; edit mode skips `Unit`/`StartingBalance`, which
/// FR.13 doesn't allow changing after creation.
const CREATE_FIELDS: [Field; 5] = [
    Field::Name,
    Field::AccountType,
    Field::Unit,
    Field::StartingBalance,
    Field::IsActive,
];
const EDIT_FIELDS: [Field; 3] = [Field::Name, Field::AccountType, Field::IsActive];

enum UnitsStatus {
    Loading,
    Loaded(Vec<lib_database::Units>),
    Failed(String),
}

/// Create (empty) or edit (pre-filled) one Account.
pub struct AccountDetailScreen {
    editing_id: Option<lib_core::RowID>,
    name: String,
    account_type: lib_core::AccountType,
    /// The Unit's own id, not just a list index — so an edit form can still show the
    /// Account's existing Unit even if it's since fallen out of the active-Units list this
    /// screen loads for the create-mode picker.
    unit_id: Option<lib_core::RowID>,
    units: UnitsStatus,
    starting_balance: String,
    is_active: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl AccountDetailScreen {
    /// An empty form for creating a new Account.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            account_type: lib_core::AccountType::default(),
            unit_id: None,
            units: UnitsStatus::Loading,
            starting_balance: "0".to_string(),
            is_active: true,
            focus: Field::Name,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Account's fields. `Unit`/`StartingBalance` are
    /// shown but not editable (FR.13).
    pub fn new_edit(account: lib_database::Accounts) -> Self {
        Self {
            editing_id: Some(account.id),
            name: account.name,
            account_type: account.account_type,
            unit_id: Some(account.unit_id),
            units: UnitsStatus::Loading,
            starting_balance: account.starting_balance.to_string(),
            is_active: account.is_active,
            focus: Field::Name,
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

    fn cycle_account_type(&mut self, delta: isize) {
        let all = lib_core::AccountType::all();
        let current = all
            .iter()
            .position(|t| *t == self.account_type)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(all.len() as isize);
        self.account_type = all[next as usize].clone();
    }

    fn cycle_unit(&mut self, delta: isize) {
        let UnitsStatus::Loaded(units) = &self.units else {
            return;
        };
        if units.is_empty() {
            return;
        }
        let current = self
            .unit_id
            .and_then(|id| units.iter().position(|u| u.id == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(units.len() as isize);
        self.unit_id = Some(units[next as usize].id);
    }

    fn selected_unit<'a>(
        &self,
        units: &'a [lib_database::Units],
    ) -> Option<&'a lib_database::Units> {
        self.unit_id
            .and_then(|id| units.iter().find(|u| u.id == id))
    }

    /// Validates the form and, if valid, spawns the async save; stores the validation error
    /// on `self.error` when the form isn't ready to submit.
    fn save(&mut self) {
        if self.name.trim().is_empty() {
            self.error = Some("Name is required".to_string());
            return;
        }

        let is_create = self.editing_id.is_none();

        // unit_id and starting_balance are only read from the form on create; on edit
        // they're carried over unchanged from whatever was passed to `new_edit`.
        let (unit_id, starting_balance) = if is_create {
            let Some(unit_id) = self.unit_id else {
                self.error = Some("Select a Unit — create one first if none exist yet".to_string());
                return;
            };
            let starting_balance: lib_core::Money = match self.starting_balance.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    self.error =
                        Some("Starting balance must be a valid decimal amount".to_string());
                    return;
                }
            };
            (unit_id, starting_balance)
        } else {
            (
                self.unit_id
                    .expect("edit form was pre-filled with a real unit_id"),
                self.starting_balance
                    .parse()
                    .expect("edit form's starting_balance came from an already-valid Money"),
            )
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let now = chrono::Utc::now();
        let account = lib_database::Accounts {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            name: self.name.trim().to_string(),
            account_type: self.account_type.clone(),
            unit_id,
            starting_balance,
            is_active: self.is_active,
            created_on: now,
            updated_on: now,
        };

        tokio::spawn(async move {
            let result = async {
                let pool = db::connect().await?;
                if is_create {
                    account.insert(&pool).await
                } else {
                    account.update(&pool).await
                }
            }
            .await;

            let action = match result {
                Ok(saved) => Action::AccountSaved(saved),
                Err(err) => Action::AccountSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Screen for AccountDetailScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Units::find_active(&pool).await
            }
            .await;
            let action = match action {
                Ok(units) => Action::UnitsLoaded(units),
                Err(err) => Action::UnitsLoadFailed(err.to_string()),
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
            Field::Name => match key.code {
                KeyCode::Char(c) => self.name.push(c),
                KeyCode::Backspace => {
                    self.name.pop();
                }
                _ => {}
            },
            Field::AccountType => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_account_type(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_account_type(1),
                _ => {}
            },
            Field::Unit => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_unit(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_unit(1),
                _ => {}
            },
            Field::StartingBalance => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                    self.starting_balance.push(c)
                }
                KeyCode::Backspace => {
                    self.starting_balance.pop();
                }
                _ => {}
            },
            Field::IsActive => {
                if matches!(
                    key.code,
                    KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
                ) {
                    self.is_active = !self.is_active;
                }
            }
        }

        Some(Action::NoOp)
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::UnitsLoaded(units) => {
                if self.unit_id.is_none() {
                    self.unit_id = units.first().map(|u| u.id);
                }
                self.units = UnitsStatus::Loaded(units.clone());
            }
            Action::UnitsLoadFailed(message) => {
                self.units = UnitsStatus::Failed(message.clone());
            }
            Action::AccountSaveFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Account"
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
                Span::raw(format!("{label:<18}")),
                Span::styled(value, style),
            ])
        };

        let unit_value = match &self.units {
            UnitsStatus::Loading => "Loading Units...".to_string(),
            UnitsStatus::Failed(message) => format!("Failed to load Units: {message}"),
            UnitsStatus::Loaded(units) if units.is_empty() => {
                "No Units yet — create one first".to_string()
            }
            UnitsStatus::Loaded(units) => self
                .selected_unit(units)
                .map(|u| format!("{} — {}", u.code, u.name))
                .unwrap_or_else(|| "(none selected)".to_string()),
        };

        let mut lines = vec![
            field_line("Name", self.name.clone(), Field::Name),
            field_line(
                "Type",
                self.account_type.as_str().to_string(),
                Field::AccountType,
            ),
            field_line("Unit", unit_value, Field::Unit),
            field_line(
                "Starting balance",
                self.starting_balance.clone(),
                Field::StartingBalance,
            ),
            field_line(
                "Active",
                if self.is_active {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
                Field::IsActive,
            ),
            Line::raw(""),
        ];

        if self.editing_id.is_some() {
            lines.push(Line::styled(
                "Unit and starting balance are fixed at creation and can't be changed here.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Type/Unit/Active  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Account "
        } else {
            " New Account "
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

    fn mock_account(unit_id: lib_core::RowID) -> lib_database::Accounts {
        let now = chrono::Utc::now();
        lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: "Everyday Spending".to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id,
            starting_balance: lib_core::Money::mock(),
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn render(screen: &AccountDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Account detail screen should not error");
    }

    #[test]
    fn new_create_starts_empty_with_all_five_fields_in_the_cycle() {
        let screen = AccountDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert_eq!(screen.fields(), CREATE_FIELDS);
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_account_and_skips_unit_and_balance() {
        let unit_id = lib_core::RowID::new();
        let account = mock_account(unit_id);
        let screen = AccountDetailScreen::new_edit(account.clone());

        assert_eq!(screen.editing_id, Some(account.id));
        assert_eq!(screen.name, "Everyday Spending");
        assert_eq!(screen.unit_id, Some(unit_id));
        assert_eq!(screen.fields(), EDIT_FIELDS);
        assert!(!EDIT_FIELDS.contains(&Field::Unit));
        assert!(!EDIT_FIELDS.contains(&Field::StartingBalance));
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = AccountDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_through_only_the_create_fields() {
        let mut screen = AccountDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Name);
        for expected in [
            Field::AccountType,
            Field::Unit,
            Field::StartingBalance,
            Field::IsActive,
            Field::Name,
        ] {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
            assert_eq!(screen.focus, expected);
        }
    }

    #[test]
    fn tab_skips_unit_and_balance_while_editing() {
        let account = mock_account(lib_core::RowID::new());
        let mut screen = AccountDetailScreen::new_edit(account);
        assert_eq!(screen.focus, Field::Name);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::AccountType);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(
            screen.focus,
            Field::IsActive,
            "Unit/StartingBalance are skipped"
        );

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_the_name_field() {
        let mut screen = AccountDetailScreen::new_create();
        screen.handle_key(key(KeyCode::Char('A')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('B')), InputMode::Navigation);
        assert_eq!(screen.name, "AB");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.name, "A");
    }

    #[test]
    fn a_key_that_would_be_a_global_shortcut_elsewhere_is_consumed_as_text() {
        let mut screen = AccountDetailScreen::new_create();
        let result = screen.handle_key(key(KeyCode::Char('?')), InputMode::Navigation);
        assert_eq!(result, Some(Action::NoOp));
        assert_eq!(screen.name, "?");
    }

    #[test]
    fn cycling_account_type_wraps() {
        let mut screen = AccountDetailScreen::new_create();
        screen.focus = Field::AccountType;
        let all = lib_core::AccountType::all();
        for expected in all.iter().cycle().skip(1).take(all.len()) {
            screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
            assert_eq!(&screen.account_type, expected);
        }
    }

    #[test]
    fn cycling_unit_moves_through_the_loaded_active_units() {
        let mut screen = AccountDetailScreen::new_create();
        let units = vec![mock_unit("AUD"), mock_unit("USD")];
        screen.update(&Action::UnitsLoaded(units.clone()));
        assert_eq!(screen.unit_id, Some(units[0].id));

        screen.focus = Field::Unit;
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.unit_id, Some(units[1].id));

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.unit_id, Some(units[0].id), "cycling wraps");
    }

    #[test]
    fn toggling_is_active() {
        let mut screen = AccountDetailScreen::new_create();
        screen.focus = Field::IsActive;
        assert!(screen.is_active);
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(!screen.is_active);
    }

    #[test]
    fn save_rejects_an_empty_name() {
        let mut screen = AccountDetailScreen::new_create();
        screen.update(&Action::UnitsLoaded(vec![mock_unit("AUD")]));
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Name is required"));
    }

    #[test]
    fn save_rejects_when_no_unit_is_selected() {
        let mut screen = AccountDetailScreen::new_create();
        screen.name = "Everyday Spending".to_string();
        // No UnitsLoaded delivered yet -- unit_id stays None.
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_an_invalid_starting_balance() {
        let mut screen = AccountDetailScreen::new_create();
        screen.name = "Everyday Spending".to_string();
        screen.update(&Action::UnitsLoaded(vec![mock_unit("AUD")]));
        screen.starting_balance = "not-a-number".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }
}
