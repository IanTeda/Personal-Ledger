//! The Categories list screen (CC-TUI-006) — retires the feasibility cycle's live-Categories
//! demo (FC-TUI-005), replacing its single-purpose "prove real SQLite works" screen with the
//! real CRUD entry point, mirroring `UnitsListScreen`'s shape (issue #67). Follows the same
//! baseline vocabulary: `Enter` open, `n` new, `d` delete (two-step `y` confirm); filter/sort
//! deferred, not missed.

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
    Loaded(Vec<lib_database::Categories>),
    Failed(String),
}

/// Lists every Category; the entry point for creating, editing, and deleting them.
pub struct CategoriesListScreen {
    status: Status,
    table_state: TableState,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl CategoriesListScreen {
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

    fn categories(&self) -> &[lib_database::Categories] {
        match &self.status {
            Status::Loaded(categories) => categories,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.categories().get(index))
            .map(|category| category.id)
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.categories().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> lib_database::DatabaseResult<Vec<lib_database::Categories>> {
        let pool = db::connect().await?;
        lib_database::Categories::find_all(&pool).await
    }

    async fn delete(id: lib_core::RowID) -> lib_database::DatabaseResult<()> {
        let pool = db::connect().await?;
        lib_database::Categories::delete_by_id(id, &pool).await
    }
}

impl Default for CategoriesListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for CategoriesListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(categories) => Action::CategoriesLoaded(categories),
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
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
                                Ok(()) => Action::CategoryDeleted(pending_id),
                                Err(err) => Action::CategoryDeleteFailed(err.to_string()),
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
                .and_then(|index| self.categories().get(index))
                .cloned()
                .map(|category| Action::OpenCategoryDetail(Some(category))),
            KeyCode::Char('n') => Some(Action::OpenCategoryDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::CategoriesLoaded(categories) => {
                self.status = Status::Loaded(categories.clone());
                if self.table_state.selected().unwrap_or(0) >= categories.len()
                    && !categories.is_empty()
                {
                    self.table_state.select(Some(categories.len() - 1));
                }
            }
            Action::CategoriesLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::CategorySaved(saved) => {
                if let Status::Loaded(categories) = &mut self.status {
                    match categories.iter_mut().find(|c| c.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => categories.insert(0, saved.clone()),
                    }
                }
            }
            Action::CategoryDeleted(id) => {
                if let Status::Loaded(categories) = &mut self.status {
                    categories.retain(|c| c.id != *id);
                }
            }
            Action::CategorySaveFailed(message) | Action::CategoryDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Categories"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Categories...")
                        .block(Block::bordered().title(" Categories ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Categories: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Categories ")),
                    rows[0],
                );
            }
            Status::Loaded(categories) => {
                let header = Row::new(["Code", "Name", "Type", "Active"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = categories.iter().map(|category| {
                    Row::new([
                        Cell::from(category.code.clone()),
                        Cell::from(category.name.clone()),
                        Cell::from(category.category_type.as_str()),
                        Cell::from(if category.is_active { "yes" } else { "no" }),
                    ])
                });
                let widths = [
                    Constraint::Length(14),
                    Constraint::Length(24),
                    Constraint::Length(10),
                    Constraint::Length(6),
                ];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(Block::bordered().title(format!(" Categories ({}) ", categories.len())));
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Category? y: confirm, any other key: cancel".to_string()
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

    fn mock_category(code: &str) -> lib_database::Categories {
        let now = chrono::Utc::now();
        lib_database::Categories {
            id: lib_core::RowID::new(),
            code: code.to_string(),
            name: format!("{code} Name"),
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

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = CategoriesListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Categories list screen should not error");
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
        render(Status::Loaded(vec![mock_category("FOO.BAR.BAZ")]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = CategoriesListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenCategoryDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_category() {
        let mut screen = CategoriesListScreen::new();
        let category = mock_category("FOO.BAR.BAZ");
        screen.update(&Action::CategoriesLoaded(vec![category.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenCategoryDetail(Some(category)))
        );
    }

    #[test]
    fn d_arms_delete_confirmation_and_a_non_y_key_cancels_it() {
        let mut screen = CategoriesListScreen::new();
        screen.update(&Action::CategoriesLoaded(vec![mock_category(
            "FOO.BAR.BAZ",
        )]));

        screen.handle_key(key(KeyCode::Char('d')), InputMode::Navigation);
        assert!(screen.pending_delete.is_some());

        screen.handle_key(key(KeyCode::Char('x')), InputMode::Navigation);
        assert!(screen.pending_delete.is_none());
    }

    #[test]
    fn category_deleted_removes_it_from_the_loaded_list() {
        let mut screen = CategoriesListScreen::new();
        let category = mock_category("FOO.BAR.BAZ");
        screen.update(&Action::CategoriesLoaded(vec![category.clone()]));
        screen.update(&Action::CategoryDeleted(category.id));

        assert!(screen.categories().is_empty());
    }

    #[test]
    fn category_saved_inserts_a_new_row_or_replaces_an_existing_one() {
        let mut screen = CategoriesListScreen::new();
        let mut category = mock_category("FOO.BAR.BAZ");
        screen.update(&Action::CategoriesLoaded(vec![category.clone()]));

        category.name = "Updated".to_string();
        screen.update(&Action::CategorySaved(category.clone()));

        assert_eq!(screen.categories().len(), 1);
        assert_eq!(screen.categories()[0].name, "Updated");
    }
}
