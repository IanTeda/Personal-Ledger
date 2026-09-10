//! The Budgets create/edit screen (CC-TUI-010). Mirrors `AccountDetailScreen`'s create-vs-
//! edit field split: FR.26 only allows changing limit amount/period/active status —
//! `category_id`/`unit_id` are fixed at creation, so edit mode skips both pickers. Only
//! Expense-type Categories are offered (`Budgets::insert` also enforces this — see
//! `CONTEXT.md`'s Budget entry).

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
    Category,
    Unit,
    LimitAmount,
    Period,
    IsActive,
}

/// Create mode cycles through every field; edit mode skips `Category`/`Unit`, which FR.26
/// doesn't allow changing after creation.
const CREATE_FIELDS: [Field; 5] = [
    Field::Category,
    Field::Unit,
    Field::LimitAmount,
    Field::Period,
    Field::IsActive,
];
const EDIT_FIELDS: [Field; 3] = [Field::LimitAmount, Field::Period, Field::IsActive];

enum CategoriesStatus {
    Loading,
    Loaded(Vec<lib_database::Categories>),
    Failed(String),
}

enum UnitsStatus {
    Loading,
    Loaded(Vec<lib_database::Units>),
    Failed(String),
}

/// Create (empty) or edit (pre-filled) one Budget.
pub struct BudgetDetailScreen {
    editing_id: Option<lib_core::RowID>,
    /// The Category's own id, not just a list index — so an edit form can still show the
    /// Budget's existing Category even if it's since fallen out of the active-Expense-
    /// Categories list this screen loads for the create-mode picker.
    category_id: Option<lib_core::RowID>,
    categories: CategoriesStatus,
    unit_id: Option<lib_core::RowID>,
    units: UnitsStatus,
    limit_amount: String,
    period: lib_core::BudgetPeriod,
    is_active: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl BudgetDetailScreen {
    /// An empty form for creating a new Budget.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            category_id: None,
            categories: CategoriesStatus::Loading,
            unit_id: None,
            units: UnitsStatus::Loading,
            limit_amount: "0".to_string(),
            period: lib_core::BudgetPeriod::default(),
            is_active: true,
            focus: Field::Category,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Budget's fields. `Category`/`Unit` are shown but
    /// not editable (FR.26).
    pub fn new_edit(budget: lib_database::Budgets) -> Self {
        Self {
            editing_id: Some(budget.id),
            category_id: Some(budget.category_id),
            categories: CategoriesStatus::Loading,
            unit_id: Some(budget.unit_id),
            units: UnitsStatus::Loading,
            limit_amount: budget.limit_amount.to_string(),
            period: budget.period,
            is_active: budget.is_active,
            focus: Field::LimitAmount,
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

    fn cycle_category(&mut self, delta: isize) {
        let CategoriesStatus::Loaded(categories) = &self.categories else {
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

    fn cycle_period(&mut self, delta: isize) {
        let all = lib_core::BudgetPeriod::all();
        let current = all.iter().position(|p| *p == self.period).unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(all.len() as isize);
        self.period = all[next as usize].clone();
    }

    fn selected_category_label(&self) -> String {
        match &self.categories {
            CategoriesStatus::Loading => "Loading Categories...".to_string(),
            CategoriesStatus::Failed(message) => format!("Failed to load Categories: {message}"),
            CategoriesStatus::Loaded(categories) if categories.is_empty() => {
                "No Expense Categories yet — create one first".to_string()
            }
            CategoriesStatus::Loaded(categories) => self
                .category_id
                .and_then(|id| categories.iter().find(|c| c.id == id))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "(none selected)".to_string()),
        }
    }

    fn selected_unit_label(&self) -> String {
        match &self.units {
            UnitsStatus::Loading => "Loading Units...".to_string(),
            UnitsStatus::Failed(message) => format!("Failed to load Units: {message}"),
            UnitsStatus::Loaded(units) if units.is_empty() => {
                "No Units yet — create one first".to_string()
            }
            UnitsStatus::Loaded(units) => self
                .unit_id
                .and_then(|id| units.iter().find(|u| u.id == id))
                .map(|u| format!("{} — {}", u.code, u.name))
                .unwrap_or_else(|| "(none selected)".to_string()),
        }
    }

    /// Validates the form and, if valid, spawns the async save; stores the validation error
    /// on `self.error` when the form isn't ready to submit.
    fn save(&mut self) {
        let limit_amount: lib_core::Money = match self.limit_amount.parse() {
            Ok(amount) => amount,
            Err(_) => {
                self.error = Some("Limit amount must be a valid decimal amount".to_string());
                return;
            }
        };

        let is_create = self.editing_id.is_none();

        // category_id/unit_id are only read from the form on create; on edit they're
        // carried over unchanged from whatever was passed to `new_edit`.
        let (category_id, unit_id) = if is_create {
            let Some(category_id) = self.category_id else {
                self.error = Some(
                    "Select an Expense Category — create one first if none exist yet".to_string(),
                );
                return;
            };
            let Some(unit_id) = self.unit_id else {
                self.error = Some("Select a Unit — create one first if none exist yet".to_string());
                return;
            };
            (category_id, unit_id)
        } else {
            (
                self.category_id
                    .expect("edit form was pre-filled with a real category_id"),
                self.unit_id
                    .expect("edit form was pre-filled with a real unit_id"),
            )
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let now = chrono::Utc::now();
        let budget = lib_database::Budgets {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            category_id,
            unit_id,
            limit_amount,
            period: self.period.clone(),
            is_active: self.is_active,
            created_on: now,
            updated_on: now,
        };

        tokio::spawn(async move {
            let result: crate::Result<_> = async {
                let pool = db::connect().await?;
                let saved = if is_create {
                    budget.insert(&pool).await?
                } else {
                    budget.update(&pool).await?
                };
                Ok(saved)
            }
            .await;

            let action = match result {
                Ok(saved) => Action::BudgetSaved(saved),
                Err(err) => Action::BudgetSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Screen for BudgetDetailScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let categories_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Categories::find_all_active(&pool)
                    .await
                    .map_err(crate::error::Error::from)
            }
            .await;
            let action = match action {
                Ok(categories) => {
                    let expense_only: Vec<_> = categories
                        .into_iter()
                        .filter(|c| c.category_type == lib_core::CategoryTypes::Expense)
                        .collect();
                    Action::CategoriesLoaded(expense_only)
                }
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
            };
            let _ = categories_tx.send(action);
        });

        let units_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Units::find_active(&pool)
                    .await
                    .map_err(crate::error::Error::from)
            }
            .await;
            let action = match action {
                Ok(units) => Action::UnitsLoaded(units),
                Err(err) => Action::UnitsLoadFailed(err.to_string()),
            };
            let _ = units_tx.send(action);
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
            Field::Category => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_category(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_category(1),
                _ => {}
            },
            Field::Unit => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_unit(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_unit(1),
                _ => {}
            },
            Field::LimitAmount => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => self.limit_amount.push(c),
                KeyCode::Backspace => {
                    self.limit_amount.pop();
                }
                _ => {}
            },
            Field::Period => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_period(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_period(1),
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
            Action::CategoriesLoaded(categories) => {
                if self.category_id.is_none() {
                    self.category_id = categories.first().map(|c| c.id);
                }
                self.categories = CategoriesStatus::Loaded(categories.clone());
            }
            Action::CategoriesLoadFailed(message) => {
                self.categories = CategoriesStatus::Failed(message.clone());
            }
            Action::UnitsLoaded(units) => {
                if self.unit_id.is_none() {
                    self.unit_id = units.first().map(|u| u.id);
                }
                self.units = UnitsStatus::Loaded(units.clone());
            }
            Action::UnitsLoadFailed(message) => {
                self.units = UnitsStatus::Failed(message.clone());
            }
            Action::BudgetSaveFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Budget"
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
            field_line("Category", self.selected_category_label(), Field::Category),
            field_line("Unit", self.selected_unit_label(), Field::Unit),
            field_line(
                "Limit amount",
                self.limit_amount.clone(),
                Field::LimitAmount,
            ),
            field_line("Period", self.period.as_str().to_string(), Field::Period),
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
                "Category and Unit are fixed at creation and can't be changed here.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Category/Unit/Period/Active  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Budget "
        } else {
            " New Budget "
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

    fn mock_expense_category(name: &str) -> lib_database::Categories {
        let now = chrono::Utc::now();
        lib_database::Categories {
            id: lib_core::RowID::new(),
            code: format!("{name}.001").to_uppercase(),
            name: name.to_string(),
            description: None,
            url_slug: None,
            category_type: lib_core::CategoryTypes::Expense,
            color: None,
            icon: None,
            is_active: true,
            created_on: now,
            updated_on: now,
        }
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

    fn render(screen: &BudgetDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Budget detail screen should not error");
    }

    #[test]
    fn new_create_starts_empty_with_all_five_fields_in_the_cycle() {
        let screen = BudgetDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert_eq!(screen.fields(), CREATE_FIELDS);
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_budget_and_skips_category_and_unit() {
        let category_id = lib_core::RowID::new();
        let unit_id = lib_core::RowID::new();
        let budget = mock_budget(category_id, unit_id);
        let screen = BudgetDetailScreen::new_edit(budget.clone());

        assert_eq!(screen.editing_id, Some(budget.id));
        assert_eq!(screen.category_id, Some(category_id));
        assert_eq!(screen.fields(), EDIT_FIELDS);
        assert!(!EDIT_FIELDS.contains(&Field::Category));
        assert!(!EDIT_FIELDS.contains(&Field::Unit));
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = BudgetDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_through_only_the_create_fields() {
        let mut screen = BudgetDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Category);
        for expected in [
            Field::Unit,
            Field::LimitAmount,
            Field::Period,
            Field::IsActive,
            Field::Category,
        ] {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
            assert_eq!(screen.focus, expected);
        }
    }

    #[test]
    fn tab_skips_category_and_unit_while_editing() {
        let budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        let mut screen = BudgetDetailScreen::new_edit(budget);
        assert_eq!(screen.focus, Field::LimitAmount);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::Period);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::IsActive);

        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(
            screen.focus,
            Field::LimitAmount,
            "Category/Unit are skipped"
        );
    }

    #[test]
    fn cycling_category_moves_through_the_loaded_active_expense_categories() {
        let mut screen = BudgetDetailScreen::new_create();
        let categories = vec![
            mock_expense_category("Groceries"),
            mock_expense_category("Fuel"),
        ];
        screen.update(&Action::CategoriesLoaded(categories.clone()));
        assert_eq!(screen.category_id, Some(categories[0].id));

        screen.focus = Field::Category;
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.category_id, Some(categories[1].id));

        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.category_id, Some(categories[0].id), "cycling wraps");
    }

    #[test]
    fn cycling_period_wraps() {
        let mut screen = BudgetDetailScreen::new_create();
        screen.focus = Field::Period;
        let all = lib_core::BudgetPeriod::all();
        // Don't assume the default is all()[0] -- it isn't (Monthly is index 1). Compute the
        // actual starting index instead.
        let start_index = all.iter().position(|p| *p == screen.period).unwrap();
        for expected in all.iter().cycle().skip(start_index + 1).take(all.len()) {
            screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
            assert_eq!(&screen.period, expected);
        }
    }

    #[test]
    fn toggling_is_active() {
        let mut screen = BudgetDetailScreen::new_create();
        screen.focus = Field::IsActive;
        assert!(screen.is_active);
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(!screen.is_active);
    }

    #[test]
    fn save_rejects_an_invalid_limit_amount() {
        let mut screen = BudgetDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.limit_amount = "not-a-number".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_when_no_category_is_selected() {
        let mut screen = BudgetDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.update(&Action::UnitsLoaded(vec![mock_unit("AUD")]));
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_when_no_unit_is_selected() {
        let mut screen = BudgetDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.update(&Action::CategoriesLoaded(vec![mock_expense_category(
            "Groceries",
        )]));
        screen.save();
        assert!(screen.error.is_some());
    }
}
