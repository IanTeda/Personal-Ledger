//! The Payees list screen (CC-TUI-007, ADR-0012). Mirrors `UnitsListScreen`'s shape — the
//! baseline list-screen vocabulary (`Enter` open, `n` new, `d` delete, two-step `y` confirm)
//! locked in by "Decide keybinding and navigation/workflow scheme". `n`/`Enter` here open
//! `PayeeDetailScreen`, whose "edit" mode is really a rename, not a plain field update.

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
    Loaded(Vec<lib_database::Payees>),
    Failed(String),
}

/// Lists every Payee; the entry point for creating, renaming, and deleting them.
pub struct PayeesListScreen {
    status: Status,
    table_state: TableState,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl PayeesListScreen {
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

    fn payees(&self) -> &[lib_database::Payees] {
        match &self.status {
            Status::Loaded(payees) => payees,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.table_state
            .selected()
            .and_then(|index| self.payees().get(index))
            .map(|payee| payee.id)
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.payees().len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize);
        self.table_state.select(Some(next as usize));
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> lib_database::Result<Vec<lib_database::Payees>> {
        let pool = db::connect().await?;
        lib_database::Payees::find_all(&pool).await
    }

    async fn delete(id: lib_core::RowID) -> lib_database::Result<()> {
        let pool = db::connect().await?;
        lib_database::Payees::delete_by_id(id, &pool).await
    }
}

impl Default for PayeesListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for PayeesListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(payees) => Action::PayeesLoaded(payees),
                Err(err) => Action::PayeesLoadFailed(err.to_string()),
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
                                Ok(()) => Action::PayeeDeleted(pending_id),
                                Err(err) => Action::PayeeDeleteFailed(err.to_string()),
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
                .and_then(|index| self.payees().get(index))
                .cloned()
                .map(|payee| Action::OpenPayeeDetail(Some(payee))),
            KeyCode::Char('n') => Some(Action::OpenPayeeDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::PayeesLoaded(payees) => {
                self.status = Status::Loaded(payees.clone());
                if self.table_state.selected().unwrap_or(0) >= payees.len() && !payees.is_empty() {
                    self.table_state.select(Some(payees.len() - 1));
                }
            }
            Action::PayeesLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::PayeeSaved(saved) => {
                if let Status::Loaded(payees) = &mut self.status {
                    match payees.iter_mut().find(|p| p.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => payees.push(saved.clone()),
                    }
                    payees.sort_by(|a, b| a.name.cmp(&b.name));
                }
            }
            Action::PayeeDeleted(id) => {
                if let Status::Loaded(payees) = &mut self.status {
                    payees.retain(|p| p.id != *id);
                }
            }
            Action::PayeeSaveFailed(message) | Action::PayeeDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Payees"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Payees...").block(Block::bordered().title(" Payees ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Payees: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Payees ")),
                    rows[0],
                );
            }
            Status::Loaded(payees) => {
                let header = Row::new(["Name", "Active"]).style(Style::default().fg(Color::Yellow));
                let table_rows = payees.iter().map(|payee| {
                    Row::new([
                        Cell::from(payee.name.clone()),
                        Cell::from(if payee.is_active { "yes" } else { "no" }),
                    ])
                });
                let widths = [Constraint::Length(30), Constraint::Length(6)];
                let table = Table::new(table_rows, widths)
                    .header(header)
                    .row_highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
                    .block(Block::bordered().title(format!(" Payees ({}) ", payees.len())));
                frame.render_stateful_widget(table, rows[0], &mut self.table_state.clone());
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Payee? y: confirm, any other key: cancel".to_string()
        } else if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            "↑/k ↓/j: move  Enter: rename  n: new  d: delete  Esc: back".to_string()
        };
        frame.render_widget(Paragraph::new(footer), rows[1]);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

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

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = PayeesListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Payees list screen should not error");
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
        render(Status::Loaded(vec![mock_payee("Woolworths")]));
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = PayeesListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenPayeeDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_payee() {
        let mut screen = PayeesListScreen::new();
        let payee = mock_payee("Woolworths");
        screen.update(&Action::PayeesLoaded(vec![payee.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenPayeeDetail(Some(payee)))
        );
    }

    #[test]
    fn d_arms_delete_confirmation_and_a_non_y_key_cancels_it() {
        let mut screen = PayeesListScreen::new();
        screen.update(&Action::PayeesLoaded(vec![mock_payee("Woolworths")]));

        screen.handle_key(key(KeyCode::Char('d')), InputMode::Navigation);
        assert!(screen.pending_delete.is_some());

        screen.handle_key(key(KeyCode::Char('x')), InputMode::Navigation);
        assert!(screen.pending_delete.is_none());
    }

    #[test]
    fn payee_deleted_removes_it_from_the_loaded_list() {
        let mut screen = PayeesListScreen::new();
        let payee = mock_payee("Woolworths");
        screen.update(&Action::PayeesLoaded(vec![payee.clone()]));
        screen.update(&Action::PayeeDeleted(payee.id));

        assert!(screen.payees().is_empty());
    }

    #[test]
    fn payee_saved_inserts_a_new_row_or_replaces_an_existing_one() {
        let mut screen = PayeesListScreen::new();
        let mut payee = mock_payee("Woolworths");
        screen.update(&Action::PayeesLoaded(vec![payee.clone()]));

        payee.name = "Woolworths AU".to_string();
        screen.update(&Action::PayeeSaved(payee.clone()));

        assert_eq!(screen.payees().len(), 1);
        assert_eq!(screen.payees()[0].name, "Woolworths AU");
    }
}
