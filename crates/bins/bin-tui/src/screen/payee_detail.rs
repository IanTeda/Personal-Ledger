//! The Payees create/rename screen (CC-TUI-007, ADR-0012). Renaming isn't a plain field
//! update — it goes through `Payees::rename`, which preserves the prior name as a Payee
//! Alias — so `save()` calls `rename`/`set_active` independently rather than a single
//! `update`, mirroring how `TransactionDetailScreen` sequences `set_status`/`set_flagged`.

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

/// Which field currently has focus; `Tab`/`Shift+Tab` cycle through them in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    IsActive,
}

const FIELDS: [Field; 2] = [Field::Name, Field::IsActive];

/// Create (empty) or rename/toggle-active (pre-filled) one Payee.
pub struct PayeeDetailScreen {
    editing_id: Option<lib_core::RowID>,
    original: Option<lib_database::Payees>,
    name: String,
    is_active: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl PayeeDetailScreen {
    /// An empty form for creating a new Payee.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            original: None,
            name: String::new(),
            is_active: true,
            focus: Field::Name,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Payee's fields.
    pub fn new_edit(payee: lib_database::Payees) -> Self {
        Self {
            editing_id: Some(payee.id),
            name: payee.name.clone(),
            is_active: payee.is_active,
            focus: Field::Name,
            error: None,
            action_tx: None,
            original: Some(payee),
        }
    }

    fn focus_index(&self) -> usize {
        FIELDS.iter().position(|f| *f == self.focus).unwrap_or(0)
    }

    fn move_focus(&mut self, delta: isize) {
        let len = FIELDS.len() as isize;
        let next = (self.focus_index() as isize + delta).rem_euclid(len);
        self.focus = FIELDS[next as usize];
        self.error = None;
    }

    /// Validates the form and, if valid, spawns the async save; stores the validation error
    /// on `self.error` when the form isn't ready to submit.
    fn save(&mut self) {
        let name = self.name.trim().to_string();
        if name.is_empty() {
            self.error = Some("Name is required".to_string());
            return;
        }

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let editing_id = self.editing_id;
        let is_active = self.is_active;
        let original = self.original.clone();

        tokio::spawn(async move {
            let result = Self::save_payee(editing_id, name, is_active, original).await;
            let action = match result {
                Ok(saved) => Action::PayeeSaved(saved),
                Err(err) => Action::PayeeSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

    async fn save_payee(
        editing_id: Option<lib_core::RowID>,
        name: String,
        is_active: bool,
        original: Option<lib_database::Payees>,
    ) -> lib_database::Result<lib_database::Payees> {
        let pool = db::connect().await?;

        let Some(id) = editing_id else {
            let payee = lib_database::payees::PayeesBuilder::new()
                .with_name(name)
                .with_is_active_opt(Some(is_active))
                .build()?;
            return payee.insert(&pool).await;
        };

        let original = original.expect("editing an existing Payee always carries its original");
        let mut latest = original.clone();

        if !original.name.eq_ignore_ascii_case(&name) {
            latest = lib_database::Payees::rename(id, &name, &pool).await?;
        }
        if is_active != original.is_active {
            latest = lib_database::Payees::set_active(id, is_active, &pool).await?;
        }

        Ok(latest)
    }
}

impl Screen for PayeeDetailScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
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
        if let Action::PayeeSaveFailed(message) = action {
            self.error = Some(message.clone());
        }
    }

    fn title(&self) -> &'static str {
        "Payee"
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
                Span::raw(format!("{label:<10}")),
                Span::styled(value, style),
            ])
        };

        let mut lines = vec![
            field_line("Name", self.name.clone(), Field::Name),
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
                "Renaming preserves the old name so it still resolves here later.",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(Line::raw(""));
        }

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: toggle Active  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Rename Payee "
        } else {
            " New Payee "
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

    fn render(screen: &PayeeDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Payee detail screen should not error");
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
    fn new_create_starts_empty() {
        let screen = PayeeDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert!(screen.name.is_empty());
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_payee() {
        let payee = mock_payee("Woolworths");
        let screen = PayeeDetailScreen::new_edit(payee.clone());

        assert_eq!(screen.editing_id, Some(payee.id));
        assert_eq!(screen.name, "Woolworths");
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = PayeeDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_focus_forward_and_wraps() {
        let mut screen = PayeeDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Name);
        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::IsActive);
        screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_the_name_field() {
        let mut screen = PayeeDetailScreen::new_create();
        screen.handle_key(key(KeyCode::Char('K')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('m')), InputMode::Navigation);
        assert_eq!(screen.name, "Km");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.name, "K");
    }

    #[test]
    fn toggling_is_active() {
        let mut screen = PayeeDetailScreen::new_create();
        screen.focus = Field::IsActive;
        assert!(screen.is_active);
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(!screen.is_active);
    }

    #[test]
    fn save_rejects_an_empty_name() {
        let mut screen = PayeeDetailScreen::new_create();
        screen.action_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Name is required"));
    }
}
