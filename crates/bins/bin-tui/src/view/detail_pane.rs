//! The detail pane a list page docks at X-Large (ADR-0050, TUI-055–057): rows down a 53-column
//! column at the View's right edge, behind a left border. Units builds its rows from `lib-units`'
//! `DetailPane` (`crate::units::view::pane`), and the Settings pages without a Presenter from
//! their fixtures (`view::settings::pane`); this file only draws them, so every pane reads alike.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::colours::Colours;

/// The pane's width in columns, its border included: the Desktop's 400px at 7.5px a column.
pub const WIDTH: u16 = 53;

/// Width of a fact's label column, as TUI-055 draws it.
const LABEL_WIDTH: usize = 18;

/// Width of an ACTIVITY line's time column, `HH:MM`, before the space that always follows it.
const TIME_WIDTH: usize = 5;

/// Between two keys of a group's key row, as the Units View's hint bar spaces them.
const KEY_SEPARATOR: &str = "  \u{b7}  ";

/// One row of the pane, top to bottom.
pub enum Row {
    Text(Line<'static>),
    /// A line with a second one right-aligned beside it.
    Split(Line<'static>, Line<'static>),
    /// A dim sentence, word-wrapped to the pane's width since a note reads poorly clipped.
    Note(String),
    Rule,
    Blank,
}

/// Draws `rows` in `area`, behind a left border. Rows past the bottom clip.
pub fn render(frame: &mut Frame<'_>, area: Rect, rows: Vec<Row>, c: &Colours) {
    let block = Block::new()
        .borders(Borders::LEFT)
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = usize::from(inner.width);
    let rows = rows.into_iter().flat_map(|row| match row {
        Row::Note(text) => wrap(&text, width)
            .into_iter()
            .map(|line| Row::Text(dim(line, c)))
            .collect(),
        row => vec![row],
    });
    for (row, y) in rows.zip(inner.y..inner.bottom()) {
        let area = Rect {
            y,
            height: 1,
            ..inner
        };
        match row {
            Row::Text(line) => frame.render_widget(Paragraph::new(line), area),
            Row::Split(left, right) => {
                let width = u16::try_from(right.width()).unwrap_or(u16::MAX);
                let [left_area, right_area] =
                    Layout::horizontal([Constraint::Min(0), Constraint::Length(width)])
                        .spacing(1)
                        .areas(area);
                frame.render_widget(Paragraph::new(left), left_area);
                frame.render_widget(
                    Paragraph::new(right).alignment(Alignment::Right),
                    right_area,
                );
            }
            Row::Note(_) | Row::Blank => {}
            Row::Rule => frame.render_widget(Block::new().borders(Borders::BOTTOM), area),
        }
    }
}

/// `text` broken at spaces into lines no wider than `width`; a word wider than that clips.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// A group's heading, upper case and dim, over a rule.
pub fn heading(rows: &mut Vec<Row>, heading: &str, c: &Colours) {
    rows.push(Row::Text(dim(lib_locale::format::upper(heading), c)));
    rows.push(Row::Rule);
}

pub fn dim(text: String, c: &Colours) -> Line<'static> {
    Line::from(Span::styled(text, c.muted()))
}

pub fn bold(text: String) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

/// A fact: its label dim in a fixed column, then its value. A monospace value needs nothing more
/// in a terminal.
pub fn fact(label: &str, value: Vec<Span<'static>>, c: &Colours) -> Row {
    let mut spans = vec![Span::styled(format!("{label:<LABEL_WIDTH$}"), c.muted())];
    spans.extend(value);
    Row::Text(Line::from(spans))
}

/// The keys under a group, each a `(chord, label)`: the chord bold and the label dim. No row when
/// `keys` is empty.
pub fn key_row(rows: &mut Vec<Row>, keys: Vec<(String, String)>, c: &Colours) {
    if keys.is_empty() {
        return;
    }
    let mut spans = Vec::new();
    for (index, (chord, label)) in keys.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(KEY_SEPARATOR, c.muted()));
        }
        spans.push(Span::styled(
            chord,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, c.muted()));
    }
    rows.push(Row::Text(Line::from(spans)));
}

/// `time` padded to the time column, then a space, so a longer time (`2:12 PM`) pushes its
/// line's text along rather than running into it.
pub fn time_column(time: &str) -> String {
    format!("{time:<TIME_WIDTH$} ")
}

/// An ACTIVITY line: its time in a dim column before its text.
pub fn timed(time: &str, text: String, c: &Colours) -> Row {
    Row::Text(Line::from(vec![
        Span::styled(time_column(time), c.muted()),
        Span::raw(text),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_time_keeps_a_space_before_its_text() {
        assert_eq!(time_column("14:20"), "14:20 ");
        assert_eq!(time_column("2:12 PM"), "2:12 PM ");
    }

    #[test]
    fn a_note_wraps_at_spaces() {
        assert_eq!(
            wrap("This session's outcomes, newest first.", 20),
            ["This session's", "outcomes, newest", "first."]
        );
        assert_eq!(wrap("", 20), Vec::<String>::new());
    }
}
