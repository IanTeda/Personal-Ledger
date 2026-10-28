//! The list-page pattern every ledger-data page reuses (TUI-033, Stage 3 of
//! `docs/ux/tui-mockups/05-settings/CLAUDE_CODE.md`): one or more [`Table`]s, each a block
//! heading with its `n add …` key over a light rule, a column header and rows whose selection is
//! reversed, then dim notes pinned to the bottom under a `Min(0)` gap.
//!
//! Actions are keys, never row buttons, so a row is only cells. A page builds its tables and notes
//! from its rows on each draw, counts included, and nothing here is stored.

use lib_locale::format::upper;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::colours::Colours;

/// How a span is drawn. Resolved against the row's selection, so a muted or negative cell stays
/// readable on the reversed row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Plain,
    Muted,
    /// A negative amount, drawn in the accent.
    Negative,
    /// An outlined chip (`[base]`), in the accent when `true`.
    Chip(bool),
}

/// One cell: its spans, left to right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Cell(pub Vec<(String, Tone)>);

impl Cell {
    pub fn plain(text: impl Into<String>) -> Self {
        Self(vec![(text.into(), Tone::Plain)])
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self(vec![(text.into(), Tone::Muted)])
    }

    pub fn tone(text: impl Into<String>, tone: Tone) -> Self {
        Self(vec![(text.into(), tone)])
    }
}

/// A column: its header and width, and whether its cells sit right, as amounts do.
pub(super) struct Column {
    pub label: String,
    pub width: Constraint,
    pub right: bool,
}

impl Column {
    pub fn left(label: String, width: Constraint) -> Self {
        Self {
            label,
            width,
            right: false,
        }
    }

    pub fn right(label: String, width: Constraint) -> Self {
        Self {
            label,
            width,
            right: true,
        }
    }
}

/// A row of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Row {
    /// A dim group label (`BANK · 3`) over the rows of one group; never selected.
    Group(String),
    Item {
        cells: Vec<Cell>,
        selected: bool,
        /// Drawn muted throughout, as an inactive Unit is.
        dim: bool,
    },
}

/// One block of a list page.
pub(super) struct Table {
    /// The block's label, drawn upper case and dim.
    pub heading: String,
    /// The key that adds a row here and what it adds, drawn at the heading's right.
    pub add: Option<(String, String)>,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

/// The heading, its rule and the column header above a table's rows.
const TABLE_CHROME: u16 = 3;

impl Table {
    /// The rows the table takes with every row in view.
    fn height(&self) -> u16 {
        u16::try_from(self.rows.len())
            .unwrap_or(u16::MAX)
            .saturating_add(TABLE_CHROME)
    }

    /// The table in `area`, scrolled to keep the selected row in view. Unless `focused`, no row
    /// draws as selected.
    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let [heading, rule, header, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        self.render_heading(frame, heading, c);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);

        let widths: Vec<Constraint> = self.columns.iter().map(|column| column.width).collect();
        let columns = Layout::horizontal(widths.clone()).spacing(1).split(header);
        for (column, cell) in self.columns.iter().zip(columns.iter()) {
            let label = Paragraph::new(Span::styled(upper(&column.label), c.muted()));
            frame.render_widget(align(label, column.right), *cell);
        }

        let visible = usize::from(body.height);
        let selected = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Item { selected: true, .. }));
        let offset = match selected {
            Some(index) if visible > 0 && index >= visible => index + 1 - visible,
            _ => 0,
        };
        for (index, row) in self.rows.iter().skip(offset).take(visible).enumerate() {
            let area = Rect {
                y: body.y + u16::try_from(index).unwrap_or(0),
                height: 1,
                ..body
            };
            match row {
                Row::Group(label) => {
                    frame.render_widget(Paragraph::new(Span::styled(upper(label), c.muted())), area)
                }
                Row::Item {
                    cells,
                    selected,
                    dim,
                } => {
                    let selected = focused && *selected;
                    if selected {
                        frame.render_widget(Block::new().style(c.selection()), area);
                    }
                    let areas = Layout::horizontal(widths.clone()).spacing(1).split(area);
                    for ((cell, column), cell_area) in
                        cells.iter().zip(&self.columns).zip(areas.iter())
                    {
                        let line = Line::from(
                            cell.0
                                .iter()
                                .map(|(text, tone)| {
                                    let tone = match tone {
                                        Tone::Chip(_) if *dim => Tone::Chip(false),
                                        _ if *dim => Tone::Muted,
                                        _ => *tone,
                                    };
                                    span(text, tone, selected, c)
                                })
                                .collect::<Vec<_>>(),
                        );
                        frame.render_widget(align(Paragraph::new(line), column.right), *cell_area);
                    }
                }
            }
        }
    }

    /// The heading row: the label dim on the left, the add key bold and its action dim on the
    /// right.
    fn render_heading(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        frame.render_widget(
            Paragraph::new(Span::styled(upper(&self.heading), c.muted())),
            area,
        );
        if let Some((key, action)) = &self.add {
            let line = Line::from(vec![
                Span::styled(key.as_str(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(action.as_str(), c.muted()),
            ]);
            frame.render_widget(Paragraph::new(line).alignment(Alignment::Right), area);
        }
    }
}

/// A span in `tone`, on the reversed row when `selected`.
fn span(text: &str, tone: Tone, selected: bool, c: &Colours) -> Span<'static> {
    match tone {
        Tone::Plain => Span::raw(text.to_string()),
        Tone::Muted if selected => Span::raw(text.to_string()),
        Tone::Muted => Span::styled(text.to_string(), c.muted()),
        Tone::Negative if selected => Span::styled(text.to_string(), c.negative_on_selection()),
        Tone::Negative => Span::styled(text.to_string(), c.negative()),
        Tone::Chip(accent) => {
            let style = match (accent, selected) {
                (true, true) => c.accent_selection(),
                (true, false) => c.accent(),
                (false, _) => Style::default(),
            };
            Span::styled(format!("[{text}]"), style)
        }
    }
}

fn align(paragraph: Paragraph<'_>, right: bool) -> Paragraph<'_> {
    if right {
        paragraph.alignment(Alignment::Right)
    } else {
        paragraph
    }
}

/// A list page's body in `area`: each table at its full height with a blank row between, then
/// the `notes`, dim, pinned to the bottom under a `Min(0)` gap. A table that doesn't fit is cut to
/// the room left and scrolls to its selection. Unless `focused`, no row draws as selected.
pub(super) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    tables: &[Table],
    notes: &[String],
    focused: bool,
    c: &Colours,
) {
    let mut constraints = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        if index > 0 {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(table.height()));
    }
    constraints.push(Constraint::Min(0));
    constraints.push(Constraint::Length(
        u16::try_from(notes.len()).unwrap_or(u16::MAX),
    ));
    let areas = Layout::vertical(constraints).split(area);
    for (index, table) in tables.iter().enumerate() {
        table.render(frame, areas[index * 2], focused, c);
    }
    if let Some(notes_area) = areas.last() {
        let lines: Vec<Line<'_>> = notes
            .iter()
            .map(|note| Line::from(Span::styled(note.as_str(), c.muted())))
            .collect();
        frame.render_widget(Paragraph::new(lines), *notes_area);
    }
}
