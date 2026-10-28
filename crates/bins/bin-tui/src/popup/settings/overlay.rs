//! The frame Settings' Edit overlays share (TUI-035): Sync Server's Edit ([`super::sync_server`])
//! and Backup's Set up Git backup ([`super::git_backup`]). Each is its own popup, with its own
//! fields, keys and test; this module draws them alike and holds the test step.
//!
//! The test gates Save: Save stays dim until a test of the values showing passes, and any change
//! sends it back to untested, so a value that was never tested can't be saved. A failed test
//! names its cause in plain words under the fields.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::colours::Colours;
use crate::msg;

/// The overlay's width, and the most rows a test's detail wraps to.
const WIDTH: u16 = 76;
const DETAIL_ROWS: u16 = 2;

/// What the last test of the values showing found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Tested {
    /// No test since the last change: Save stays dim.
    #[default]
    Untested,
    Passed {
        title: String,
        detail: String,
    },
    /// The cause in plain words: what went wrong, then what to check.
    Failed {
        title: String,
        detail: String,
    },
}

impl Tested {
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

/// A field's value as the overlay draws it.
pub enum Value {
    Text(String),
    /// A secret, drawn as one `•` per character typed: never its text.
    Secret(usize),
    /// One of `options`, `←→` stepping through them.
    Choice {
        options: Vec<String>,
        selected: usize,
    },
    /// Text after a fixed prefix drawn as a dim chip, which typing never reaches.
    Prefixed {
        prefix: String,
        text: String,
    },
}

/// One field: its label over its value, with an optional dim note under them.
pub struct Field {
    pub label: String,
    pub value: Value,
    pub note: Option<String>,
}

/// What an overlay draws.
pub struct Overlay<'a> {
    pub title: String,
    /// The command it answers to, dim at the title's right.
    pub tag: Option<&'static str>,
    pub fields: Vec<Field>,
    pub focus: usize,
    pub tested: &'a Tested,
    /// The line under the test, saying what saving does.
    pub save_note: String,
    pub save_label: String,
}

/// What a key means to an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Cancel,
    Save,
    Test,
    Next,
    Prev,
    /// `←` (`-1`) or `→` (`1`) on a choice.
    Step(isize),
    /// Anything else: typing into a text field.
    Edit(KeyEvent),
}

impl Key {
    pub fn of(key: KeyEvent) -> Self {
        match key.code {
            KeyCode::Esc => Self::Cancel,
            KeyCode::Enter => Self::Save,
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => Self::Test,
            KeyCode::Tab => Self::Next,
            KeyCode::BackTab => Self::Prev,
            KeyCode::Left => Self::Step(-1),
            KeyCode::Right => Self::Step(1),
            _ => Self::Edit(key),
        }
    }
}

/// What a key did to an overlay, for the page that hosts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Cancel,
    /// `enter` after a passing test.
    Save,
}

/// `index` moved `delta` along `len` options, stopping at both ends.
pub fn stepped(index: usize, len: usize, delta: isize) -> usize {
    index
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

impl Overlay<'_> {
    /// Draws the overlay centred across `area` and a sixth of the way down, or as high as it
    /// must to fit.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let field_rows: u16 = self.fields.iter().map(field_height).sum();
        // Title and rule, the fields, a blank, the test, the save note, then the footer and its
        // rule, inside the border.
        let height = (2 + field_rows + 1 + 1 + DETAIL_ROWS + 1 + 2 + 2).min(area.height);
        let width = WIDTH.min(area.width);
        let popup = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: (area.y + (area.height / 6).max(1)).min(area.y + area.height.saturating_sub(height)),
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut constraints = vec![Constraint::Length(1), Constraint::Length(1)];
        constraints.extend(
            self.fields
                .iter()
                .map(|field| Constraint::Length(field_height(field))),
        );
        constraints.extend([
            Constraint::Length(1),
            Constraint::Length(1 + DETAIL_ROWS),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let rows = Layout::vertical(constraints).split(inner);
        let last = rows.len() - 1;

        self.render_title(frame, rows[0], c);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        for (index, field) in self.fields.iter().enumerate() {
            render_field(frame, rows[2 + index], field, index == self.focus, c);
        }
        let after = 2 + self.fields.len();
        render_tested(frame, rows[after + 1], self.tested, c);
        frame.render_widget(
            Paragraph::new(Span::styled(self.save_note.as_str(), c.muted())),
            rows[after + 2],
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[last - 1]);
        self.render_footer(frame, rows[last], c);
    }

    fn render_title(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let tag = self.tag.unwrap_or_default();
        let [title, tag_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(u16::try_from(tag.chars().count()).unwrap_or(u16::MAX)),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(Span::styled(
                self.title.as_str(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            title,
        );
        frame.render_widget(Paragraph::new(Span::styled(tag, c.muted())), tag_area);
    }

    /// The overlay's keys, `enter` dim while Save is locked, and the test's state at the right.
    fn render_footer(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let save_open = self.tested.passed();
        let mut spans = Vec::new();
        for (key, label, open) in [
            ("^t", msg::tui_settings_overlay_hint_test(), true),
            ("tab", msg::tui_settings_overlay_hint_next(), true),
            ("enter", self.save_label.clone(), save_open),
            ("esc", msg::tui_settings_overlay_hint_cancel(), true),
        ] {
            let (key_style, label_style) = if open {
                (bold, Style::default())
            } else {
                (c.muted(), c.muted())
            };
            spans.push(Span::styled(key, key_style));
            spans.push(Span::styled(format!(" {label}  "), label_style));
        }
        let state = match self.tested {
            Tested::Untested => msg::tui_settings_overlay_untested(),
            Tested::Passed { .. } => msg::tui_settings_overlay_passed(),
            Tested::Failed { .. } => msg::tui_settings_overlay_failed(),
        };
        let [keys, state_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(u16::try_from(state.chars().count()).unwrap_or(u16::MAX)),
        ])
        .areas(area);
        frame.render_widget(Paragraph::new(Line::from(spans)), keys);
        frame.render_widget(Paragraph::new(Span::styled(state, c.muted())), state_area);
    }
}

fn field_height(field: &Field) -> u16 {
    if field.note.is_some() { 3 } else { 2 }
}

fn render_field(frame: &mut Frame<'_>, area: Rect, field: &Field, focused: bool, c: &Colours) {
    let [label, value, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    let label_style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        c.muted()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(field.label.as_str(), label_style)),
        label,
    );
    frame.render_widget(
        Paragraph::new(value_line(&field.value, focused, value.width, c)),
        value,
    );
    if let Some(text) = &field.note {
        frame.render_widget(Paragraph::new(Span::styled(text.as_str(), c.muted())), note);
    }
}

/// A value's line, `width` cells at most: a long text shows its end, where the cursor is.
fn value_line(value: &Value, focused: bool, width: u16, c: &Colours) -> Line<'static> {
    let cursor = || Span::styled("\u{258c}", c.cursor());
    let tail = |text: &str, room: usize| {
        let skip = text.chars().count().saturating_sub(room);
        text.chars().skip(skip).collect::<String>()
    };
    let room = usize::from(width).saturating_sub(1);
    let mut spans = match value {
        Value::Text(text) => vec![Span::raw(tail(text, room))],
        Value::Secret(len) => vec![Span::raw("•".repeat((*len).min(room)))],
        Value::Choice { options, selected } => {
            let mut spans = Vec::new();
            for (index, option) in options.iter().enumerate() {
                let style = match (index == *selected, focused) {
                    (true, true) => c.selection(),
                    (true, false) => Style::default().add_modifier(Modifier::BOLD),
                    (false, _) => c.muted(),
                };
                spans.push(Span::styled(format!(" {option} "), style));
                spans.push(Span::raw(" "));
            }
            return Line::from(spans);
        }
        Value::Prefixed { prefix, text } => {
            let room = room.saturating_sub(prefix.chars().count() + 1);
            vec![
                Span::styled(prefix.clone(), c.muted()),
                Span::raw(" "),
                Span::raw(tail(text, room)),
            ]
        }
    };
    if focused {
        spans.push(cursor());
    }
    Line::from(spans)
}

/// The test's result, indented under the fields: its cause or what passed, then the detail; a
/// dim prompt before any test.
fn render_tested(frame: &mut Frame<'_>, area: Rect, tested: &Tested, c: &Colours) {
    let area = Rect {
        x: area.x + 2,
        width: area.width.saturating_sub(2),
        ..area
    };
    let (mark, title, detail) = match tested {
        Tested::Untested => {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    msg::tui_settings_overlay_untested_note(),
                    c.muted(),
                )),
                area,
            );
            return;
        }
        Tested::Passed { title, detail } => (Span::styled("✓ ", c.accent()), title, detail),
        Tested::Failed { title, detail } => (Span::styled("■ ", c.negative()), title, detail),
    };
    let [title_area, detail_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![mark, Span::raw(title.clone())])),
        title_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(detail.as_str(), c.muted())).wrap(Wrap { trim: true }),
        detail_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_the_overlay_grammar() {
        let key = |code, modifiers| Key::of(KeyEvent::new(code, modifiers));
        assert_eq!(key(KeyCode::Esc, KeyModifiers::NONE), Key::Cancel);
        assert_eq!(key(KeyCode::Enter, KeyModifiers::NONE), Key::Save);
        assert_eq!(key(KeyCode::Char('t'), KeyModifiers::CONTROL), Key::Test);
        assert!(matches!(
            key(KeyCode::Char('t'), KeyModifiers::NONE),
            Key::Edit(_)
        ));
        assert_eq!(key(KeyCode::Tab, KeyModifiers::NONE), Key::Next);
        assert_eq!(key(KeyCode::BackTab, KeyModifiers::SHIFT), Key::Prev);
        assert_eq!(key(KeyCode::Left, KeyModifiers::NONE), Key::Step(-1));
    }

    #[test]
    fn stepped_stops_at_both_ends() {
        assert_eq!(stepped(0, 3, -1), 0);
        assert_eq!(stepped(1, 3, 1), 2);
        assert_eq!(stepped(2, 3, 1), 2);
    }
}
