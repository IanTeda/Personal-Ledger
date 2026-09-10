//! The Units list screen (CC-TUI-005) — the first real entity CRUD screen built on the
//! skeleton (issue #66). Follows the baseline list-screen vocabulary "Decide keybinding and
//! navigation/workflow scheme" locked in: `Enter` open, `n` new, `d` delete (two-step `y`
//! confirm). Filter (`/`) and sort (`s`) are deliberately not implemented yet — deferred,
//! not missed, since Units lists are expected to stay short.

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
    Loaded(Vec<lib_database::Units>),
    Failed(String),
}

/// Lists every Unit; the entry point for creating, editing, and deleting them.
pub struct UnitsListScreen {
    status: Status,
    table_state: TableState,
    /// Set after a `d` press, awaiting a `y` to actually delete — any other key cancels.
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    /// Captured from `init()`, used to spawn the delete's background task the same way
    /// `init()` itself reports the initial load back.
    action_tx: Option<UnboundedSender<Action>>,
}

impl UnitsListScreen {
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

    fn units(&self) -> &[lib_database::Units] {
        match &self.status {
            Status::Loaded(units) => units,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.units().get(index))
            .map(|unit| unit.id)
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.units().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> crate::Result<Vec<lib_database::Units>> {
        let pool = db::connect().await?;
        lib_database::Units::find_all(&pool)
            .await
            .map_err(crate::error::Error::from)
    }

    async fn delete(id: lib_core::RowID) -> crate::Result<()> {
        let pool = db::connect().await?;
        lib_database::Units::delete_by_id(id, &pool)
            .await
            .map_err(crate::error::Error::from)
    }
}

impl Default for UnitsListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for UnitsListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(units) => Action::UnitsLoaded(units),
                Err(err) => Action::UnitsLoadFailed(err.to_string()),
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
                                Ok(()) => Action::UnitDeleted(pending_id),
                                Err(err) => Action::UnitDeleteFailed(err.to_string()),
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
                .and_then(|index| self.units().get(index))
                .cloned()
                .map(|unit| Action::OpenUnitDetail(Some(unit))),
            KeyCode::Char('n') => Some(Action::OpenUnitDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::UnitsLoaded(units) => {
                self.status = Status::Loaded(units.clone());
                if self.table_state.selected().unwrap_or(0) >= units.len() && !units.is_empty() {
                    self.table_state.select(Some(units.len() - 1));
                }
            }
            Action::UnitsLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::UnitSaved(saved) => {
                if let Status::Loaded(units) = &mut self.status {
                    match units.iter_mut().find(|u| u.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => units.push(saved.clone()),
                    }
                    units.sort_by(|a, b| a.code.cmp(&b.code));
                }
            }
            Action::UnitDeleted(id) => {
                if let Status::Loaded(units) = &mut self.status {
                    units.retain(|u| u.id != *id);
                }
            }
            Action::UnitSaveFailed(message) | Action::UnitDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Units"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Units...").block(Block::bordered().title(" Units ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Units: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Units ")),
                    rows[0],
                );
            }
            Status::Loaded(units) => {
                let header = Row::new(["Code", "Name", "Kind", "Decimals", "Active"])
                    .style(Style::default().fg(Color::Yellow));
                let table_rows = units.iter().map(|unit| {
                    Row::new([
                        Cell::from(unit.code.clone()),
                        Cell::from(unit.name.clone()),
                        Cell::from(unit.unit_kind.as_str()),
                        Cell::from(unit.decimal_places.to_string()),
                        Cell::from(if unit.is_active { "yes" } else { "no" }),
                    ])
                });
                let widths = [
                    Constraint::Length(10),
                    Constraint::Length(24),
                    Constraint::Length(14),
                    Constraint::Length(8),
                    Constraint::Length(6),
                ];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(Block::bordered().title(format!(" Units ({}) ", units.len())));
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Unit? y: confirm, any other key: cancel".to_string()
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

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = UnitsListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Units list screen should not error");
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
        render(Status::Loaded(vec![mock_unit("AUD")]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = UnitsListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenUnitDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_unit() {
        let mut screen = UnitsListScreen::new();
        let unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenUnitDetail(Some(unit)))
        );
    }

    #[test]
    fn d_arms_delete_confirmation_and_a_non_y_key_cancels_it() {
        let mut screen = UnitsListScreen::new();
        screen.update(&Action::UnitsLoaded(vec![mock_unit("AUD")]));

        screen.handle_key(key(KeyCode::Char('d')), InputMode::Navigation);
        assert!(screen.pending_delete.is_some());

        screen.handle_key(key(KeyCode::Char('x')), InputMode::Navigation);
        assert!(screen.pending_delete.is_none());
    }

    #[test]
    fn units_loaded_replaces_the_status() {
        let mut screen = UnitsListScreen::new();
        let unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit]));

        assert_eq!(screen.units().len(), 1);
    }

    #[test]
    fn unit_deleted_removes_it_from_the_loaded_list() {
        let mut screen = UnitsListScreen::new();
        let unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit.clone()]));
        screen.update(&Action::UnitDeleted(unit.id));

        assert!(screen.units().is_empty());
    }

    #[test]
    fn unit_saved_inserts_a_new_row_or_replaces_an_existing_one() {
        let mut screen = UnitsListScreen::new();
        let mut unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit.clone()]));

        unit.name = "Updated".to_string();
        screen.update(&Action::UnitSaved(unit.clone()));

        assert_eq!(screen.units().len(), 1);
        assert_eq!(screen.units()[0].name, "Updated");
    }

    #[test]
    fn move_selection_wraps_around() {
        let mut screen = UnitsListScreen::new();
        screen.update(&Action::UnitsLoaded(vec![
            mock_unit("AUD"),
            mock_unit("USD"),
        ]));

        screen.move_selection(-1);
        assert_eq!(screen.table_state.selected(), Some(1));
    }
}
