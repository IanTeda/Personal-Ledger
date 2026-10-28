//! The in-place editor (TUI-020): `enter` on a form page's field opens it where it sits, as a
//! bordered, accent-framed box holding the field's own sub-rows, while the rest of the page dims
//! and the status line shows `INSERT`. Inside it `←→` chooses a segment, `space` toggles a
//! checkbox, typing edits text, `enter` saves, `tab` saves and moves to the next field, and `esc`
//! keeps the old value.
//!
//! The editor holds only the draft: what each field offers and what its sub-rows say are the
//! page's (`view::settings::{general,display}`), and what saving does is the
//! [`Action::SetSetting`] the page returns from [`Outcome::Save`].
//!
//! [`Action::SetSetting`]: crate::view::Action::SetSetting

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::colours::Colours;
use crate::msg;
use crate::view::settings::field::SettingField;

/// The label column inside the box: the form pages', so the longest label keeps its gap.
const LABEL_WIDTH: u16 = 22;

/// One segment a choice field offers: the value `:set` takes, and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub value: String,
    pub label: String,
}

/// What the open field holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draft {
    /// Segments, `←→` moving between them, wrapping at both ends.
    Choice {
        segments: Vec<Segment>,
        chosen: usize,
    },
    /// Free text.
    Text(String),
    /// A checkbox, `space` toggling it.
    Check(bool),
}

/// What a key did to the open field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `enter`: save this value.
    Save(String),
    /// `tab`: save this value, then select the next field.
    SaveAndNext(String),
    /// `esc`: close, keeping the old value.
    Keep,
    /// The draft changed, or the key did nothing; either way the field stays open.
    Stay,
}

/// The open field: which it is, its draft, and the old value's label for `esc keep …`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEditor {
    field: SettingField,
    draft: Draft,
    was: String,
}

impl FieldEditor {
    pub fn new(field: SettingField, draft: Draft, was: String) -> Self {
        Self { field, draft, was }
    }

    pub fn field(&self) -> SettingField {
        self.field
    }

    /// The old value's label.
    pub fn was(&self) -> &str {
        &self.was
    }

    /// The draft as `:set` would take it.
    pub fn value(&self) -> String {
        match &self.draft {
            Draft::Choice { segments, chosen } => segments
                .get(*chosen)
                .map(|segment| segment.value.clone())
                .unwrap_or_default(),
            Draft::Text(text) => text.clone(),
            Draft::Check(on) => if *on { "on" } else { "off" }.to_string(),
        }
    }

    /// The chosen segment's label, or the text, for the page's sub-rows.
    pub fn label(&self) -> String {
        match &self.draft {
            Draft::Choice { segments, chosen } => segments
                .get(*chosen)
                .map(|segment| segment.label.clone())
                .unwrap_or_default(),
            _ => self.value(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match (&mut self.draft, key.code) {
            (_, KeyCode::Enter) => return Outcome::Save(self.value()),
            (_, KeyCode::Tab) => return Outcome::SaveAndNext(self.value()),
            (_, KeyCode::Esc) => return Outcome::Keep,
            (Draft::Choice { segments, chosen }, KeyCode::Left | KeyCode::Right) => {
                let count = segments.len().max(1);
                *chosen = if key.code == KeyCode::Right {
                    (*chosen + 1) % count
                } else {
                    (*chosen + count - 1) % count
                };
            }
            (Draft::Check(on), KeyCode::Char(' ')) => *on = !*on,
            (Draft::Text(text), KeyCode::Backspace) => {
                text.pop();
            }
            (Draft::Text(text), KeyCode::Char(ch)) if !ctrl => text.push(ch),
            _ => {}
        }
        Outcome::Stay
    }

    /// The box's height around `sub_rows` rows below the field's own.
    pub fn height(sub_rows: usize) -> u16 {
        u16::try_from(sub_rows).unwrap_or(0).saturating_add(3)
    }

    /// The box over `area`: the field's label and draft, then each `(label, value)` sub-row.
    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        sub_rows: &[(String, String)],
        c: &Colours,
    ) {
        frame.render_widget(Clear, area);
        let block = Block::bordered().border_style(c.accent());
        let inner = block.inner(area).inner(Margin::new(1, 0));
        frame.render_widget(block, area);

        let rows = Layout::vertical(vec![Constraint::Length(1); sub_rows.len() + 1]).split(inner);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        if let Some(head) = rows.first() {
            let [label, draft] =
                Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
                    .areas(*head);
            frame.render_widget(
                Paragraph::new(Span::styled(self.field.label(), bold)),
                label,
            );
            frame.render_widget(Paragraph::new(self.draft_line(c)), draft);
        }
        for ((label, value), row) in sub_rows.iter().zip(rows.iter().skip(1)) {
            let [label_area, value_area] =
                Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
                    .areas(*row);
            frame.render_widget(
                Paragraph::new(Span::styled(label.as_str(), c.muted())),
                label_area,
            );
            frame.render_widget(Paragraph::new(value.as_str()), value_area);
        }
    }

    /// The draft as drawn: the segments with the chosen one reversed, the text and its cursor, or
    /// the checkbox, then a dim reminder of the key that changes it.
    fn draft_line(&self, c: &Colours) -> Line<'static> {
        let mut spans = Vec::new();
        let key = match &self.draft {
            Draft::Choice { segments, chosen } => {
                for (index, segment) in segments.iter().enumerate() {
                    if index > 0 {
                        spans.push(Span::raw("  "));
                    }
                    let style = if index == *chosen {
                        c.selection()
                    } else {
                        c.muted()
                    };
                    spans.push(Span::styled(format!(" {} ", segment.label), style));
                }
                "←→"
            }
            Draft::Text(text) => {
                spans.push(Span::raw(text.clone()));
                spans.push(Span::styled("▌", c.cursor()));
                return Line::from(spans);
            }
            Draft::Check(on) => {
                spans.push(Span::raw(if *on { "[x]" } else { "[ ]" }));
                "space"
            }
        };
        spans.push(Span::styled(format!("  · {key}"), c.muted()));
        Line::from(spans)
    }

    /// The hint bar while the field is open; `back` is the Shell's back key.
    pub fn hints(&self, back: &str) -> Vec<(String, String)> {
        let mut hints = match self.draft {
            Draft::Choice { .. } => vec![("←→".to_string(), msg::tui_settings_hint_choose())],
            Draft::Check(_) => vec![("space".to_string(), msg::tui_settings_hint_toggle())],
            Draft::Text(_) => Vec::new(),
        };
        hints.extend([
            ("enter".to_string(), msg::tui_settings_hint_save()),
            (back.to_string(), msg::tui_settings_hint_keep(&self.was)),
            ("tab".to_string(), msg::tui_settings_hint_next_field()),
        ]);
        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn choice(labels: &[&str]) -> FieldEditor {
        let segments = labels
            .iter()
            .map(|label| Segment {
                value: label.to_string(),
                label: label.to_string(),
            })
            .collect();
        FieldEditor::new(
            SettingField::DateStyle,
            Draft::Choice {
                segments,
                chosen: 0,
            },
            "a".into(),
        )
    }

    #[test]
    fn arrows_cycle_the_segments_and_enter_saves_the_chosen_one() {
        let mut editor = choice(&["a", "b", "c"]);
        assert_eq!(editor.handle_key(key(KeyCode::Left)), Outcome::Stay);
        assert_eq!(editor.value(), "c", "← wraps to the last");
        editor.handle_key(key(KeyCode::Right));
        editor.handle_key(key(KeyCode::Right));
        assert_eq!(editor.value(), "b");
        assert_eq!(
            editor.handle_key(key(KeyCode::Enter)),
            Outcome::Save("b".into())
        );
        assert_eq!(
            editor.handle_key(key(KeyCode::Tab)),
            Outcome::SaveAndNext("b".into())
        );
        assert_eq!(editor.handle_key(key(KeyCode::Esc)), Outcome::Keep);
    }

    #[test]
    fn space_toggles_a_checkbox_and_text_takes_typing() {
        let mut check = FieldEditor::new(SettingField::Toasts, Draft::Check(true), "on".into());
        check.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(check.value(), "off");

        let mut text = FieldEditor::new(SettingField::Owner, Draft::Text("Ia".into()), "Ia".into());
        text.handle_key(key(KeyCode::Backspace));
        for ch in "lex".chars() {
            text.handle_key(key(KeyCode::Char(ch)));
        }
        assert_eq!(text.value(), "Ilex");
        text.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(text.value(), "Ilex", "a control chord isn't typed");
    }

    #[test]
    fn the_hints_name_the_draft_s_key_and_the_value_esc_keeps() {
        crate::locale::init_for_tests();
        let hints: Vec<String> = choice(&["a", "b"])
            .hints("esc")
            .into_iter()
            .map(|(key, label)| format!("{key} {label}"))
            .collect();
        assert_eq!(
            hints,
            ["←→ choose", "enter save", "esc keep a", "tab next field"]
        );
    }
}
