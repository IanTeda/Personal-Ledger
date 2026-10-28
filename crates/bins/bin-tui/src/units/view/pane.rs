//! The detail pane the Units View docks at X-Large (TUI-055, DESKTOP-125): `lib-units`' `DetailPane`
//! drawn as rows down a 53-column column at the View's right edge. Its groups, facts and ACTIVITY
//! are the Presenter's; this file only lays them out.
//!
//! The pane follows the selection and has no focus, so it adds no keys. In place of the Desktop's
//! buttons each group names the keys that act on the selected row. A button no key reaches from
//! the selection is left out (Edit source on a Unit's link, where `e` edits the Unit), and a Unit's
//! PRICE SOURCE names `tab` instead, which moves to the Price Sources table.

use lib_locale::format::upper;
use lib_units::keymap::{NEXT_TABLE, TEST_SOURCE};
use lib_units::presenter::{
    AccountLine, AccountsGroup, ActivityGroup, DetailPane, Fact, MenuAction, PaneButton, PaneGroup,
    SourceGroup, UnitGroup, UnitsPageEvent, UnitsRow, ValueLine, ValuesGroup,
};
use lib_viper::keymap::{DELETE, EDIT, Keymap, REACTIVATE};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use super::units::HINT_SEPARATOR;
use crate::colours::Colours;

/// The pane's width in columns, its border included: the Desktop's 400px at 7.5px a column.
pub const WIDTH: u16 = 53;

/// Width of a fact's label column, as TUI-055 draws it.
const LABEL_WIDTH: usize = 18;

/// Width of an ACTIVITY line's time column: `HH:MM` and a space.
const TIME_WIDTH: usize = 6;

/// What the pane needs beyond the Presenter's `DetailPane` to name its keys.
pub struct PaneKeys<'a> {
    pub keymap: &'a Keymap<UnitsPageEvent>,
    /// The selected row, which every key acts on.
    pub selected: Option<UnitsRow>,
    /// What `tab` moves to, the Price Sources table's heading.
    pub sources: &'a str,
}

/// One row of the pane, top to bottom.
enum Row {
    Text(Line<'static>),
    /// A line with a second one right-aligned beside it.
    Split(Line<'static>, Line<'static>),
    /// A dim sentence, word-wrapped to the pane's width since a note reads poorly clipped.
    Note(String),
    Rule,
    Blank,
}

/// Draws `pane` in `area`, behind a left border. Rows past the bottom clip.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &DetailPane,
    keys: &PaneKeys<'_>,
    c: &Colours,
) {
    let block = Block::new()
        .borders(Borders::LEFT)
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut rows = Vec::new();
    for (index, group) in pane.groups.iter().enumerate() {
        if index > 0 {
            rows.push(Row::Blank);
        }
        match group {
            PaneGroup::Unit(group) => unit_rows(&mut rows, group, keys, c),
            PaneGroup::Source(group) => source_rows(&mut rows, group, keys, c),
            PaneGroup::Accounts(group) => accounts_rows(&mut rows, group, c),
            PaneGroup::Values(group) => values_rows(&mut rows, group, c),
            PaneGroup::Activity(group) => activity_rows(&mut rows, group, c),
        }
    }

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
            Row::Note(_) => {}
            Row::Rule => frame.render_widget(Block::new().borders(Borders::BOTTOM), area),
            Row::Blank => {}
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

fn heading(rows: &mut Vec<Row>, heading: &str, c: &Colours) {
    rows.push(Row::Text(dim(upper(heading), c)));
    rows.push(Row::Rule);
}

fn dim(text: String, c: &Colours) -> Line<'static> {
    Line::from(Span::styled(text, c.muted()))
}

fn bold(text: String) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

/// A fact: its label dim in a fixed column, then its value. A monospace value needs nothing more
/// in a terminal.
fn fact(fact: &Fact, c: &Colours) -> Row {
    Row::Text(Line::from(vec![
        Span::styled(format!("{:<LABEL_WIDTH$}", fact.label), c.muted()),
        Span::raw(fact.value.clone()),
    ]))
}

/// The keys under a group: each button's chord bold and its label dim, `extra` after them. No row
/// when nothing is left.
fn key_row(
    rows: &mut Vec<Row>,
    buttons: &[PaneButton],
    extra: Option<(String, String)>,
    keys: &PaneKeys<'_>,
    c: &Colours,
) {
    let named = buttons
        .iter()
        .filter_map(|button| {
            let chord = keys.keymap.chord(command(&button.event, keys.selected)?)?;
            Some((chord.to_string(), button.label.clone()))
        })
        .chain(extra)
        .filter(|(chord, _)| !chord.is_empty())
        .collect::<Vec<_>>();
    if named.is_empty() {
        return;
    }
    let mut spans = Vec::new();
    for (index, (chord, label)) in named.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(HINT_SEPARATOR, c.muted()));
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

/// The command whose key does what `event` does with `selected` selected, or `None` when no key
/// reaches it from there.
fn command(event: &UnitsPageEvent, selected: Option<UnitsRow>) -> Option<&'static str> {
    let UnitsPageEvent::Action(action, target) = event else {
        return None;
    };
    let command = match action {
        MenuAction::EditUnit | MenuAction::EditSource => EDIT,
        MenuAction::DeleteUnit | MenuAction::DeleteSource => DELETE,
        MenuAction::ReactivateUnit => REACTIVATE,
        // On a Unit `t` tests the source its link names, so it reaches the linked group's Test.
        MenuAction::TestSource => return Some(TEST_SOURCE),
    };
    (selected == Some(*target)).then_some(command)
}

fn unit_rows(rows: &mut Vec<Row>, group: &UnitGroup, keys: &PaneKeys<'_>, c: &Colours) {
    heading(rows, &group.heading, c);
    rows.push(Row::Text(bold(group.title.clone())));
    rows.push(Row::Text(dim(group.meta.clone(), c)));
    rows.extend(group.facts.iter().map(|f| fact(f, c)));
    key_row(rows, &group.buttons, None, keys, c);
}

fn source_rows(rows: &mut Vec<Row>, group: &SourceGroup, keys: &PaneKeys<'_>, c: &Colours) {
    heading(rows, &group.heading, c);
    if let Some(title) = &group.title {
        rows.push(Row::Text(bold(title.clone())));
    }
    if let Some(meta) = &group.meta {
        rows.push(Row::Text(dim(meta.clone(), c)));
    }
    rows.extend(group.facts.iter().map(|f| fact(f, c)));
    if let Some(note) = &group.note {
        rows.push(Row::Note(note.clone()));
    }
    // A group with no title is a selected Unit's link, whose source `tab` reaches.
    let to_sources = group.title.is_none().then(|| {
        let chord = keys.keymap.chord(NEXT_TABLE).unwrap_or_default();
        (chord.to_string(), keys.sources.to_string())
    });
    key_row(rows, &group.buttons, to_sources, keys, c);
}

fn accounts_rows(rows: &mut Vec<Row>, group: &AccountsGroup, c: &Colours) {
    heading(rows, &group.heading, c);
    for line in &group.rows {
        account_rows(rows, line, c);
    }
    if let Some(total) = &group.total {
        rows.push(Row::Split(
            bold(total.name.clone()),
            bold(total.balance.clone()),
        ));
        if let Some(value) = &total.value {
            rows.push(Row::Split(Line::default(), bold(value.clone())));
        }
    }
    if let Some(empty) = &group.empty {
        rows.push(Row::Note(empty.clone()));
    }
}

/// An Account: its name over its type, its balance over that balance's value.
fn account_rows(rows: &mut Vec<Row>, line: &AccountLine, c: &Colours) {
    rows.push(Row::Split(
        Line::from(line.name.clone()),
        Line::from(line.balance.clone()),
    ));
    rows.push(Row::Split(
        dim(line.kind.clone(), c),
        dim(line.value.clone().unwrap_or_default(), c),
    ));
}

fn values_rows(rows: &mut Vec<Row>, group: &ValuesGroup, c: &Colours) {
    heading(rows, &group.heading, c);
    for line in &group.rows {
        value_rows(rows, line, c);
    }
    if let Some(note) = &group.note {
        rows.push(Row::Note(note.clone()));
    }
    if let Some(empty) = &group.empty {
        rows.push(Row::Note(empty.clone()));
    }
}

/// A Unit the source prices: `code · name` over its symbol and quote, its price over its age.
fn value_rows(rows: &mut Vec<Row>, line: &ValueLine, c: &Colours) {
    rows.push(Row::Split(
        Line::from(format!("{} \u{b7} {}", line.code, line.name)),
        Line::from(line.price.clone()),
    ));
    let detail = line
        .symbol
        .iter()
        .map(|(label, symbol)| format!("{label} {symbol}"))
        .chain([line.quoted_in.clone()])
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    rows.push(Row::Split(
        dim(detail, c),
        dim(line.age.clone().unwrap_or_default(), c),
    ));
}

/// ACTIVITY: each outcome's time in a dim column before its text, newest first, then the note.
fn activity_rows(rows: &mut Vec<Row>, group: &ActivityGroup, c: &Colours) {
    heading(rows, &group.heading, c);
    for line in &group.lines {
        rows.push(Row::Text(Line::from(vec![
            Span::styled(format!("{:<TIME_WIDTH$}", line.time), c.muted()),
            Span::raw(line.text.clone()),
        ])));
    }
    rows.push(Row::Note(group.note.clone()));
}

#[cfg(test)]
mod tests {
    use lib_units::{COINGECKO, UnitId};

    use super::*;

    #[test]
    fn a_note_wraps_at_spaces() {
        assert_eq!(
            wrap("This session's outcomes, newest first.", 20),
            ["This session's", "outcomes, newest", "first."]
        );
        assert_eq!(wrap("", 20), Vec::<String>::new());
    }

    #[test]
    fn a_button_names_the_key_that_does_it_from_the_selection() {
        let btc = UnitsRow::Unit(UnitId::SEED_BTC);
        let source = UnitsRow::Source(COINGECKO);
        let action = |action, row| UnitsPageEvent::Action(action, row);

        assert_eq!(
            command(&action(MenuAction::EditUnit, btc), Some(btc)),
            Some(EDIT)
        );
        assert_eq!(
            command(&action(MenuAction::DeleteSource, source), Some(source)),
            Some(DELETE)
        );
        assert_eq!(
            command(&action(MenuAction::TestSource, source), Some(btc)),
            Some(TEST_SOURCE),
            "t on a Unit tests its linked source"
        );
        assert_eq!(
            command(&action(MenuAction::EditSource, source), Some(btc)),
            None,
            "e on a Unit edits the Unit, not its source"
        );
        assert_eq!(command(&UnitsPageEvent::NewPriceSource, Some(btc)), None);
    }
}
