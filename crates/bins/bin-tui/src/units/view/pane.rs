//! The detail pane the Units View docks at X-Large (TUI-055, DESKTOP-125): `lib-units`' `DetailPane`
//! drawn as rows down a 53-column column at the View's right edge. Its groups, facts and ACTIVITY
//! are the Presenter's; this file only lays them out.
//!
//! The pane follows the selection and has no focus, so it adds no keys. In place of the Desktop's
//! buttons each group names the keys that act on the selected row. A button no key reaches from
//! the selection is left out (Edit source on a Unit's link, where `e` edits the Unit), and a Unit's
//! PRICE SOURCE names `tab` instead, which moves to the Price Sources table.

use lib_units::keymap::{NEXT_TABLE, OPEN, SET_PRICE, TEST_SOURCE};
use lib_units::presenter::{
    AccountLine, AccountsGroup, ActivityGroup, DetailPane, Fact, MenuAction, PaneButton, PaneGroup,
    PriceGroup, SourceGroup, UnitGroup, UnitsPageEvent, UnitsRow, ValueLine, ValuesGroup,
};
use lib_viper::keymap::{DELETE, EDIT, Keymap, REACTIVATE};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
};

use crate::colours::Colours;
use crate::view::detail_pane::{self, Row, bold, dim, heading};

pub use crate::view::detail_pane::WIDTH;

/// What the pane needs beyond the Presenter's `DetailPane` to name its keys.
pub struct PaneKeys<'a> {
    pub keymap: &'a Keymap<UnitsPageEvent>,
    /// The selected row, which every key acts on.
    pub selected: Option<UnitsRow>,
    /// What `tab` moves to, the Price Sources table's heading.
    pub sources: &'a str,
}

/// Draws `pane` in `area`, behind a left border. Rows past the bottom clip.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &DetailPane,
    keys: &PaneKeys<'_>,
    c: &Colours,
) {
    let mut rows = Vec::new();
    for (index, group) in pane.groups.iter().enumerate() {
        if index > 0 {
            rows.push(Row::Blank);
        }
        match group {
            PaneGroup::Unit(group) => unit_rows(&mut rows, group, keys, c),
            PaneGroup::Source(group) => source_rows(&mut rows, group, keys, c),
            PaneGroup::Price(group) => price_rows(&mut rows, group, c),
            PaneGroup::Accounts(group) => accounts_rows(&mut rows, group, c),
            PaneGroup::Values(group) => values_rows(&mut rows, group, c),
            PaneGroup::Activity(group) => activity_rows(&mut rows, group, c),
        }
    }
    detail_pane::render(frame, area, rows, c);
}

fn fact(fact: &Fact, c: &Colours) -> Row {
    detail_pane::fact(&fact.label, vec![Span::raw(fact.value.clone())], c)
}

/// The keys under a group: each button that a key reaches from the selection, `extra` after them.
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
        .collect();
    detail_pane::key_row(rows, named, c);
}

/// The command whose key does what `event` does with `selected` selected, or `None` when no key
/// reaches it from there.
fn command(event: &UnitsPageEvent, selected: Option<UnitsRow>) -> Option<&'static str> {
    let UnitsPageEvent::Action(action, target) = event else {
        return None;
    };
    let command = match action {
        MenuAction::Prices => OPEN,
        MenuAction::EditUnit | MenuAction::EditSource => EDIT,
        MenuAction::DeleteUnit | MenuAction::DeleteSource => DELETE,
        MenuAction::ReactivateUnit => REACTIVATE,
        MenuAction::SetPrice => SET_PRICE,
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

fn price_rows(rows: &mut Vec<Row>, group: &PriceGroup, c: &Colours) {
    heading(rows, &group.heading, c);
    rows.extend(group.facts.iter().map(|f| fact(f, c)));
    if let Some(empty) = &group.empty {
        rows.push(Row::Note(empty.clone()));
    }
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
    rows.push(Row::Split(
        dim(line.detail.clone(), c),
        dim(line.age.clone().unwrap_or_default(), c),
    ));
}

/// ACTIVITY: each outcome's time in a dim column before its text, newest first, then the note.
fn activity_rows(rows: &mut Vec<Row>, group: &ActivityGroup, c: &Colours) {
    heading(rows, &group.heading, c);
    for line in &group.lines {
        rows.push(detail_pane::timed(&line.time, line.text.clone(), c));
    }
    rows.push(Row::Note(group.note.clone()));
}

#[cfg(test)]
mod tests {
    use lib_units::{COINGECKO, UnitId};

    use super::*;

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
