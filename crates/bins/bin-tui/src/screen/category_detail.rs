//! The Categories create/edit screen (CC-TUI-006) — one shape for create and edit, mirroring
//! `UnitDetailScreen` (issue #67): every key is consumed here while a field has focus, so a
//! `?` typed into the Description field never leaks out as the global Help binding.

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
    Description,
    UrlSlug,
    CategoryType,
    Color,
    Icon,
    IsActive,
}

const FIELDS: [Field; 8] = [
    Field::Code,
    Field::Name,
    Field::Description,
    Field::UrlSlug,
    Field::CategoryType,
    Field::Color,
    Field::Icon,
    Field::IsActive,
];

/// Create (empty) or edit (pre-filled) one Category.
pub struct CategoryDetailScreen {
    editing_id: Option<lib_core::RowID>,
    code: String,
    name: String,
    description: String,
    url_slug: String,
    category_type: lib_core::CategoryTypes,
    color: String,
    icon: String,
    is_active: bool,
    focus: Field,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl CategoryDetailScreen {
    /// An empty form for creating a new Category.
    pub fn new_create() -> Self {
        Self {
            editing_id: None,
            code: String::new(),
            name: String::new(),
            description: String::new(),
            url_slug: String::new(),
            category_type: lib_core::CategoryTypes::default(),
            color: String::new(),
            icon: String::new(),
            is_active: true,
            focus: Field::Code,
            error: None,
            action_tx: None,
        }
    }

    /// A form pre-filled with an existing Category's fields.
    pub fn new_edit(category: lib_database::Categories) -> Self {
        Self {
            editing_id: Some(category.id),
            code: category.code,
            name: category.name,
            description: category.description.unwrap_or_default(),
            url_slug: category
                .url_slug
                .map(|slug| slug.as_str().to_string())
                .unwrap_or_default(),
            category_type: category.category_type,
            color: category.color.map(|c| c.to_string()).unwrap_or_default(),
            icon: category.icon.unwrap_or_default(),
            is_active: category.is_active,
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

    fn cycle_category_type(&mut self, delta: isize) {
        let all = lib_core::CategoryTypes::all();
        let current = all
            .iter()
            .position(|t| *t == self.category_type)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(all.len() as isize);
        self.category_type = all[next as usize].clone();
    }

    /// Validates the form and, if valid, spawns the async save; stores the validation error
    /// on `self.error` when the form isn't ready to submit.
    fn save(&mut self) {
        if self.code.trim().is_empty() {
            self.error = Some("Code is required".to_string());
            return;
        }
        if self.name.trim().is_empty() {
            self.error = Some("Name is required".to_string());
            return;
        }

        let url_slug = if self.url_slug.trim().is_empty() {
            None
        } else {
            match lib_core::UrlSlug::parse(self.url_slug.trim()) {
                Ok(slug) => Some(slug),
                Err(err) => {
                    self.error = Some(format!("Invalid URL slug: {err}"));
                    return;
                }
            }
        };

        let color = if self.color.trim().is_empty() {
            None
        } else {
            match lib_core::HexColor::parse(self.color.trim()) {
                Ok(color) => Some(color),
                Err(err) => {
                    self.error = Some(format!("Invalid colour: {err}"));
                    return;
                }
            }
        };

        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };

        let now = chrono::Utc::now();
        let description = if self.description.trim().is_empty() {
            None
        } else {
            Some(self.description.trim().to_string())
        };
        let icon = if self.icon.trim().is_empty() {
            None
        } else {
            Some(self.icon.trim().to_string())
        };

        let category = lib_database::Categories {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run for a brand-new (create-mode) row.
            #[allow(clippy::unwrap_or_default)]
            id: self.editing_id.unwrap_or_else(lib_core::RowID::new),
            code: self.code.trim().to_string(),
            name: self.name.trim().to_string(),
            description,
            url_slug,
            category_type: self.category_type.clone(),
            color,
            icon,
            is_active: self.is_active,
            created_on: now,
            updated_on: now,
        };
        let is_create = self.editing_id.is_none();

        tokio::spawn(async move {
            let result: crate::Result<_> = async {
                let pool = db::connect().await?;
                let saved = if is_create {
                    category.insert(&pool).await?
                } else {
                    category.update(&pool).await?
                };
                Ok(saved)
            }
            .await;

            let action = match result {
                Ok(saved) => Action::CategorySaved(saved),
                Err(err) => Action::CategorySaveFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Screen for CategoryDetailScreen {
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
            Field::Code
            | Field::Name
            | Field::Description
            | Field::UrlSlug
            | Field::Color
            | Field::Icon => {
                let buffer = match self.focus {
                    Field::Code => &mut self.code,
                    Field::Name => &mut self.name,
                    Field::Description => &mut self.description,
                    Field::UrlSlug => &mut self.url_slug,
                    Field::Color => &mut self.color,
                    Field::Icon => &mut self.icon,
                    _ => unreachable!(),
                };
                match key.code {
                    KeyCode::Char(c) => buffer.push(c),
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    _ => {}
                }
            }
            Field::CategoryType => match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.cycle_category_type(-1),
                KeyCode::Right | KeyCode::Char('l') => self.cycle_category_type(1),
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
        if let Action::CategorySaveFailed(message) = action {
            self.error = Some(message.clone());
        }
    }

    fn title(&self) -> &'static str {
        "Category"
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
            field_line("Description", self.description.clone(), Field::Description),
            field_line("URL Slug", self.url_slug.clone(), Field::UrlSlug),
            field_line(
                "Type",
                self.category_type.as_str().to_string(),
                Field::CategoryType,
            ),
            field_line("Colour", self.color.clone(), Field::Color),
            field_line("Icon", self.icon.clone(), Field::Icon),
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
            "Tab/Shift+Tab: next/prev field  ←/→: change Type/Active  Enter: save  Esc: cancel",
        ));

        let title = if self.editing_id.is_some() {
            " Edit Category "
        } else {
            " New Category "
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

    fn render(screen: &CategoryDetailScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Category detail screen should not error");
    }

    #[test]
    fn new_create_starts_empty() {
        let screen = CategoryDetailScreen::new_create();
        assert!(screen.editing_id.is_none());
        assert!(screen.code.is_empty());
        render(&screen);
    }

    #[test]
    fn new_edit_pre_fills_from_an_existing_category() {
        let now = chrono::Utc::now();
        let category = lib_database::Categories {
            id: lib_core::RowID::new(),
            code: "FOO.BAR.BAZ".to_string(),
            name: "Groceries".to_string(),
            description: None,
            url_slug: None,
            category_type: lib_core::CategoryTypes::Expense,
            color: None,
            icon: None,
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        let screen = CategoryDetailScreen::new_edit(category.clone());

        assert_eq!(screen.editing_id, Some(category.id));
        assert_eq!(screen.code, "FOO.BAR.BAZ");
        render(&screen);
    }

    #[test]
    fn esc_backs_out_without_saving() {
        let mut screen = CategoryDetailScreen::new_create();
        assert_eq!(
            screen.handle_key(key(KeyCode::Esc), InputMode::Navigation),
            Some(Action::Back)
        );
    }

    #[test]
    fn tab_cycles_focus_forward_and_wraps() {
        let mut screen = CategoryDetailScreen::new_create();
        assert_eq!(screen.focus, Field::Code);
        for _ in 0..FIELDS.len() {
            screen.handle_key(key(KeyCode::Tab), InputMode::Navigation);
        }
        assert_eq!(
            screen.focus,
            Field::Code,
            "a full cycle returns to the start"
        );
    }

    #[test]
    fn typing_appends_to_the_focused_text_field() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.handle_key(key(KeyCode::Char('F')), InputMode::Navigation);
        screen.handle_key(key(KeyCode::Char('O')), InputMode::Navigation);
        assert_eq!(screen.code, "FO");

        screen.handle_key(key(KeyCode::Backspace), InputMode::Navigation);
        assert_eq!(screen.code, "F");
    }

    #[test]
    fn a_key_that_would_be_a_global_shortcut_elsewhere_is_consumed_as_text() {
        let mut screen = CategoryDetailScreen::new_create();
        let result = screen.handle_key(key(KeyCode::Char('?')), InputMode::Navigation);
        assert_eq!(result, Some(Action::NoOp));
        assert_eq!(screen.code, "?");
    }

    #[test]
    fn cycling_category_type_wraps() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.focus = Field::CategoryType;
        let all = lib_core::CategoryTypes::all();
        let start = all
            .iter()
            .position(|t| *t == screen.category_type)
            .expect("default category type is one of `all()`");

        for expected in all.iter().cycle().skip(start + 1).take(all.len()) {
            screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
            assert_eq!(&screen.category_type, expected);
        }
    }

    #[test]
    fn toggling_is_active() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.focus = Field::IsActive;
        assert!(screen.is_active);
        screen.handle_key(key(KeyCode::Char(' ')), InputMode::Navigation);
        assert!(!screen.is_active);
    }

    #[test]
    fn save_rejects_an_empty_code() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.name = "Groceries".to_string();
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Code is required"));
    }

    #[test]
    fn save_rejects_an_empty_name() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.code = "FOO.BAR.BAZ".to_string();
        screen.save();
        assert_eq!(screen.error.as_deref(), Some("Name is required"));
    }

    #[test]
    fn save_rejects_an_invalid_colour() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.code = "FOO.BAR.BAZ".to_string();
        screen.name = "Groceries".to_string();
        screen.color = "not-a-colour".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_rejects_a_url_slug_that_cleans_to_empty() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.code = "FOO.BAR.BAZ".to_string();
        screen.name = "Groceries".to_string();
        // `UrlSlug::parse` normalises input (spaces/case/punctuation) rather than rejecting
        // it outright -- it only errors when nothing alphanumeric survives the cleanup.
        screen.url_slug = "!!!".to_string();
        screen.save();
        assert!(screen.error.is_some());
    }

    #[test]
    fn save_normalises_a_messy_but_non_empty_url_slug_instead_of_rejecting_it() {
        let mut screen = CategoryDetailScreen::new_create();
        screen.code = "FOO.BAR.BAZ".to_string();
        screen.name = "Groceries".to_string();
        screen.url_slug = "Hello World!".to_string();
        screen.save();
        assert!(screen.error.is_none());
    }
}
