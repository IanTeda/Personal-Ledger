//! The detail pane a Settings list page docks at X-Large (#884, TUI-056, TUI-057, DESKTOP-141–151)
//! for the pages with no Presenter yet. Each builds a [`Pane`] for its selected row on each draw,
//! from the same fixture as its rows, and [`render`] lays it out with `view::detail_pane`, as
//! Units' pane is laid out from `lib-units`' (`crate::units::view::pane`).
//!
//! The pane follows the selection and has no focus, so it adds no keys. Under each group it names
//! the keys that act on the selected row, read from the page's key table ([`super::keys::named`]),
//! so a key the pane names is one the page takes. A Desktop button no key reaches is left out.
//!
//! Every pane but History's ends with ACTIVITY, the page's Success and Info outcomes, which then
//! raise no Toast. No page here has one yet, since each of their keys flashes "not yet built", so
//! ACTIVITY draws only its note until a page's Presenter gives it lines.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
};

use super::list::{self, Cell};
use crate::colours::Colours;
use crate::msg;
use crate::view::detail_pane::{self, Row, dim, heading};

/// One line of a group, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Line {
    /// The row's name, bold.
    Title(Cell),
    /// What the row is, dim, under its name.
    Meta(String),
    /// A dim label and its value.
    Fact(String, Cell),
    /// A cell on its own.
    Text(Cell),
    /// A cell with a second right-aligned beside it.
    Split(Cell, Cell),
    /// A dim sentence, wrapped.
    Note(String),
    /// The keys that act on the row, each a `(chord, label)`.
    Keys(Vec<(String, String)>),
}

/// One group of the pane, under its heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Group {
    pub heading: String,
    pub lines: Vec<Line>,
}

impl Group {
    pub fn new(heading: String) -> Self {
        Self {
            heading,
            lines: Vec::new(),
        }
    }

    pub fn title(self, title: impl Into<Cell>) -> Self {
        self.line(Line::Title(title.into()))
    }

    pub fn line(mut self, line: Line) -> Self {
        self.lines.push(line);
        self
    }

    pub fn meta(self, meta: String) -> Self {
        self.line(Line::Meta(meta))
    }

    pub fn fact(self, label: String, value: impl Into<Cell>) -> Self {
        self.line(Line::Fact(label, value.into()))
    }

    pub fn note(self, note: String) -> Self {
        self.line(Line::Note(note))
    }

    pub fn keys(self, keys: Vec<(String, String)>) -> Self {
        self.line(Line::Keys(keys))
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self::plain(text)
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Self::plain(text)
    }
}

/// A page's pane: its groups, then ACTIVITY unless the page is the record itself (History).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Pane {
    pub groups: Vec<Group>,
    pub activity: bool,
}

impl Pane {
    /// `groups`, then ACTIVITY.
    pub fn with_activity(groups: Vec<Group>) -> Self {
        Self {
            groups,
            activity: true,
        }
    }
}

/// Draws `pane` in `area`, behind a left border; `page` names the page in ACTIVITY's note.
pub(super) fn render(frame: &mut Frame<'_>, area: Rect, pane: &Pane, page: &str, c: &Colours) {
    let mut rows = Vec::new();
    for (index, group) in pane.groups.iter().enumerate() {
        if index > 0 {
            rows.push(Row::Blank);
        }
        heading(&mut rows, &group.heading, c);
        for line in &group.lines {
            rows.push(match line {
                Line::Title(cell) => Row::Text(ratatui::text::Line::from(
                    list::spans(cell, c)
                        .into_iter()
                        .map(|span| span.patch_style(Style::default().add_modifier(Modifier::BOLD)))
                        .collect::<Vec<_>>(),
                )),
                Line::Meta(text) => Row::Text(dim(text.clone(), c)),
                Line::Text(cell) => Row::Text(ratatui::text::Line::from(list::spans(cell, c))),
                Line::Fact(label, value) => detail_pane::fact(label, list::spans(value, c), c),
                Line::Split(left, right) => Row::Split(
                    ratatui::text::Line::from(list::spans(left, c)),
                    ratatui::text::Line::from(list::spans(right, c)),
                ),
                Line::Note(text) => Row::Note(text.clone()),
                Line::Keys(keys) => {
                    detail_pane::key_row(&mut rows, keys.clone(), c);
                    continue;
                }
            });
        }
    }
    if pane.activity {
        if !rows.is_empty() {
            rows.push(Row::Blank);
        }
        heading(&mut rows, &msg::tui_settings_pane_activity(), c);
        rows.push(Row::Note(msg::tui_settings_pane_activity_note(page)));
    }
    detail_pane::render(frame, area, rows, c);
}
