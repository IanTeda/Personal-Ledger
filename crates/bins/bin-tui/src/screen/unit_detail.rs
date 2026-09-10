//! The Units create/edit screen (CC-TUI-005) — one shape serves both create (empty) and
//! edit (pre-filled), per "Decide TUI screen map and navigation shape". The first screen
//! with real free-text fields, so the first to actually exercise the mode-split intent from
//! "Decide keybinding and navigation/workflow scheme": every key is consumed here while a
//! text field has focus (see the module-level note on `InputMode` below), rather than
//! falling through to `App`'s global `Esc`/`?` bindings.

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
    Code,
    Name,
    UnitKind,
    DecimalPlaces,
    IsActive,
}

const FIELDS: [Field; 5] = [
    Field::Code,
    Field::Name,
    Field::UnitKind,
    Field::DecimalPlaces,
    Field::IsActive,
];

/// Create (empty) or edit (pre-filled) one Unit.
///
/// `App`'s central `InputMode` field stays `Navigation` even while this screen is active —
/// every key that would otherwise leak to a global shortcut is consumed here directly
/// (`handle_key` always returns `Some`, never `None`, while a text field has focus), which
/// turned out to fully satisfy the mode split without needing App to track a matching
/// `Editing` state of its own. Revisit only if a cross-screen indicator (e.g. a status-bar
/// "EDITING" badge) is wanted later.
pub struct UnitDetailScreen {
    editing_id: Option<lib_core::RowID>,
    code: String,
    name: String,
    unit_kind: lib_core::UnitKind,
    decimal_places: String,
    is_active: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl UnitDetailScreen {
    /// An empty form for creating a new Unit.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            code: String::new(),
            name: String::new(),
            unit_kind: lib_core::UnitKind::default(),
            decimal_places: "2".to_string(),
            is_active: true,
            focus: Field::Code,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Unit's fields.
    pub fn new_edit(unit: lib_database::Units) -> Self {
        Self {
            editing_id: Some(unit.id),
            code: unit.code,
            name: unit.name,
            unit_kind: unit.unit_kind,
            decimal_places: unit.decimal_places.to_string(),
            is_active: unit.is_active,
            focus: Field::Code,
            error: None,
            action_tx: None,
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

    fn cycle_unit_kind(&mut self, delta: isize) {
        let all = lib_core::UnitKind::all();
        let current = all.iter().position(|k| *k == self.unit_kind).unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(all.len() as isize);
        self.unit_kind = all[next as usize].clone();
    }

    /// Validates the form and, if valid, spawns the async save; returns the validation error
    /// (also stored on `self.error`) when the form isn't ready to submit.
    fn save(&mut self) {
        if self.code.trim().is_empty() {
            self.error = Some("Code is required".to_string());
            return;
        }
        if self.name.trim().is_empty() {
            self.error = Some("Name is required".to_string());
            return;
        }
        let decimal_places: i64 = match self.decimal_places.parse() {
            Ok(value) if (0..=18).contains(&value) => value,
            _ => {
                self.error = Some("Decimal places must be a number from 0 to 18".to_string());
                return;
            }
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let now = chrono::Utc::now();
        let unit = lib_database::Units {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            code: self.code.trim().to_string(),
            name: self.name.trim().to_string(),
            unit_kind: self.unit_kind.clone(),
            decimal_places,
            is_active: self.is_active,
            created_on: now,
            updated_on: now,
        };
        let is_create = self.editing_id.is_none();

        tokio::spawn(async move {
            let result: crate::Result<_> = async {
                let pool = db::connect().await?;
                let saved = if is_create {
                    unit.insert(&pool).await?
                } else {
                    unit.update(&pool).await?
                };
                Ok(saved)
            }
            .await;

            let action = match result {
                Ok(saved) => Action::UnitSaved(saved),
                Err(err) => Action::UnitSaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Screen for UnitDetailScreen {
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
            Field::Code | Field::Name => {
                let buffer = if self.focus == Field::Code {
                    &mut self.code
                } else {
                    &mut self.name
                };
                match key.code {
                    KeyCode::Char(c) => buffer.push(c),
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    _ => {}
                }
            }
            Field::DecimalPlaces => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => self.decimal_places.push(c),
                KeyCode::Backspace => {
                    self.decimal_places.pop();
                }
                _ => {}
            },
            Field::UnitKind => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_unit_kind(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_unit_kind(1),
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

        // Every key while this screen is active is consumed here — the point of the mode
        // split: a `?` typed into the Name field must never open Help.
        Some(Action::NoOp)
    }

    fn update(&mut self, action: &Action) {
        if let Action::UnitSaveFailed(message) = action {
            self.error = Some(message.clone());
        }
    }

    fn title(&self) -> &'static str {
        "Unit"
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
            field_line("Code", self.code.clone(), Field::Code),
            field_line("Name", self.name.clone(), Field::Name),
            field_line("Kind", self.unit_kind.as_str().to_string(), Field::UnitKind),
            field_line(
                "Decimal places",
                self.decimal_places.clone(),
                Field::DecimalPlaces,
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

        if let Some(error) = &self.error {
            lines.push(Line::styled(error.clone(), Style::default().fg(Color::Red)));
            lines.push(Line::raw(""));
        }

        lines.push(Line::raw(
            "Tab/Shift+Tab: next/prev field  ←/→: change Kind/Active  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Unit "
        } else {
            " New Unit "
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

    fn render(screen: &UnitDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Unit detail screen should not error");
    }

    #[test]
    fn new_create_starts_empty() {
        let screen = UnitDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert!(screen.code.is_empty());
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_unit() {
        let now = chrono::Utc::now();
        let unit = lib_database::Units {
            id: lib_core::RowID::new(),
            code: "AUD".to_string(),
            name: "Australian Dollar".to_string(),
            unit_kind: lib_core::UnitKind::Fiat,
            decimal_places: 2,
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        let screen = UnitDetailScreen::new_edit(unit.clone());

        assert_eq!(screen.editing_id, Some(unit.id));
        assert_eq!(screen.code, "AUD");
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = UnitDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_focus_forward_and_wraps() {
        let mut screen = UnitDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Code);
        for expected in [
            Field::Name,
            Field::UnitKind,
            Field::DecimalPlaces,
            Field::IsActive,
            Field::Code,
        ] {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
            assert_eq!(screen.focus, expected);
        }
    }

    #[test]
    fn back_tab_cycles_focus_backward() {
        let mut screen = UnitDetailScreen::new_create();
        screen.handle_key(key(KeyCode::BackTab), InputMode::Navigation);
        assert_eq!(screen.focus, Field::IsActive);
    }

    #[test]
    fn typing_appends_to_the_focused_text_field() {
        let mut screen = UnitDetailScreen::new_create();
        screen.handle_key(key(KeyCode::Char('A')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('U')), InputMode::Navigation);
        assert_eq!(screen.code, "AU");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.code, "A");
    }

    #[test]
    fn a_key_that_would_be_a_global_shortcut_elsewhere_is_consumed_as_text() {
        let mut screen = UnitDetailScreen::new_create();
        let result = screen.handle_key(key(KeyCode::Char('?')), InputMode::Navigation);
        assert_eq!(
            result,
            Some(Action::NoOp),
            "must not fall through to OpenHelp"
        );
        assert_eq!(screen.code, "?");
    }

    #[test]
    fn cycling_unit_kind_wraps() {
        let mut screen = UnitDetailScreen::new_create();
        screen.focus = Field::UnitKind;
        let all = lib_core::UnitKind::all();
        for expected in all.iter().cycle().skip(1).take(all.len()) {
            screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
            assert_eq!(&screen.unit_kind, expected);
        }
    }

    #[test]
    fn toggling_is_active() {
        let mut screen = UnitDetailScreen::new_create();
        screen.focus = Field::IsActive;
        assert!(screen.is_active);
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(!screen.is_active);
    }

    #[test]
    fn save_rejects_an_empty_code() {
        let mut screen = UnitDetailScreen::new_create();
        screen.name = "Australian Dollar".to_string();
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Code is required"));
    }

    #[test]
    fn save_rejects_an_empty_name() {
        let mut screen = UnitDetailScreen::new_create();
        screen.code = "AUD".to_string();
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Name is required"));
    }

    #[test]
    fn save_rejects_an_out_of_range_decimal_places() {
        let mut screen = UnitDetailScreen::new_create();
        screen.code = "AUD".to_string();
        screen.name = "Australian Dollar".to_string();
        screen.decimal_places = "99".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }
}
