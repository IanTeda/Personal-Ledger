//! The list-page pattern every ledger-data page reuses (TUI-033, Stage 3 of
//! `docs/ux/tui-mockups/05-settings/CLAUDE_CODE.md`): one or more [`Table`]s, each a block
//! heading with its `n add …` key over a light rule, a column header and rows whose selection is
//! reversed, then dim notes pinned to the bottom under a `Min(0)` gap.
//!
//! Actions are keys, never row buttons, so a row is only cells. A page builds its tables and notes
//! from its rows on each draw, counts included, and nothing here is stored.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_list_cursor::Step;
use lib_locale::format::upper;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::colours::Colours;
use crate::view::Action;

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
    /// Drawn in the accent, as a flag is.
    Accent,
    Bold,
    /// A one-cell colour mark (`■`) in a Tag's colour.
    Swatch(Color),
    /// Muted and struck through, as an undone History change is; it stays struck on a dim row.
    Struck,
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
    /// A reversed band across the row with a bold label and a note after it (`▸ CURRENT STATE`);
    /// never selected.
    Band { label: String, note: String },
}

/// One block of a list page.
pub(super) struct Table {
    /// The block's label, drawn upper case and dim.
    pub heading: String,
    /// The keys that add a row here and what each adds, drawn at the heading's right.
    pub add: Vec<(String, String)>,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

/// The heading, its rule and the column header above a table's rows.
const TABLE_CHROME: u16 = 3;

impl Table {
    /// The rows the table takes with every row in view.
    fn height(&self) -> u16 {
        self.height_with(0)
    }

    /// [`Self::height`] with `lead` fact lines under the rule.
    pub(super) fn height_with(&self, lead: usize) -> u16 {
        u16::try_from(self.rows.len() + lead)
            .unwrap_or(u16::MAX)
            .saturating_add(TABLE_CHROME)
    }

    /// The table in `area`, scrolled to keep the selected row in view. Unless `focused`, no row
    /// draws as selected.
    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        self.render_with(&[], frame, area, focused, c);
    }

    /// [`Self::render`] with `lead` fact lines between the rule and the column header, as Assistant
    /// access shows its server, access and approval.
    pub(super) fn render_with(
        &self,
        lead: &[(String, Cell)],
        frame: &mut Frame<'_>,
        area: Rect,
        focused: bool,
        c: &Colours,
    ) {
        let [heading, rule, facts, header, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(u16::try_from(lead.len()).unwrap_or(u16::MAX)),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        self.render_heading(frame, heading, c);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
        render_lines(frame, facts, lead, c);

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
                Row::Band { label, note } => {
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::styled(
                                upper(label),
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                            Span::raw("  "),
                            Span::raw(note.as_str()),
                        ]))
                        .style(c.selection()),
                        area,
                    );
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
                                        Tone::Struck => Tone::Struck,
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
        let mut spans = Vec::new();
        for (key, action) in &self.add {
            if !spans.is_empty() {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(
                key.as_str(),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(action.as_str(), c.muted()));
        }
        if !spans.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
                area,
            );
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
        Tone::Accent if selected => Span::styled(text.to_string(), c.accent_selection()),
        Tone::Accent => Span::styled(text.to_string(), c.accent()),
        Tone::Bold => Span::styled(
            text.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Tone::Swatch(colour) => Span::styled(text.to_string(), Style::default().fg(colour)),
        Tone::Struck => Span::styled(
            text.to_string(),
            c.muted().add_modifier(Modifier::CROSSED_OUT),
        ),
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

/// The box under the tables naming the selected row's detail (`selected · ANZ`): each line a
/// dim label and its value.
pub(super) struct Detail {
    pub title: String,
    pub lines: Vec<(String, Cell)>,
}

/// The label column of a [`Detail`].
const DETAIL_LABEL: u16 = 18;

impl Detail {
    fn height(&self) -> u16 {
        u16::try_from(self.lines.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let block = Block::new()
            .borders(Borders::ALL)
            .title(Span::styled(format!(" {} ", self.title), c.muted()));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        render_lines(frame, inner, &self.lines, c);
    }
}

/// `lines` down `area`, one a row: each a dim label and its value.
fn render_lines(frame: &mut Frame<'_>, area: Rect, lines: &[(String, Cell)], c: &Colours) {
    for (index, (label, value)) in lines.iter().enumerate() {
        let row = Rect {
            y: area.y + u16::try_from(index).unwrap_or(0),
            height: 1,
            ..area
        };
        let [label_area, value_area] =
            Layout::horizontal([Constraint::Length(DETAIL_LABEL), Constraint::Min(0)]).areas(row);
        frame.render_widget(
            Paragraph::new(Span::styled(label.as_str(), c.muted())),
            label_area,
        );
        let spans: Vec<Span<'static>> = value
            .0
            .iter()
            .map(|(text, tone)| span(text, *tone, false, c))
            .collect();
        frame.render_widget(Paragraph::new(Line::from(spans)), value_area);
    }
}

/// A list page's body in `area`: each table at its full height with a blank row between, then
/// the `detail` box of the selected row, then the `notes`, dim, pinned to the bottom under a
/// `Min(0)` gap. A table that doesn't fit is cut to the room left and scrolls to its selection.
/// Unless `focused`, no row draws as selected.
pub(super) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    tables: &[Table],
    detail: Option<&Detail>,
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
    let detail = detail.filter(|_| focused);
    if let Some(detail) = detail {
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(detail.height()));
    }
    constraints.push(Constraint::Min(0));
    constraints.push(Constraint::Length(
        u16::try_from(notes.len()).unwrap_or(u16::MAX),
    ));
    let areas = Layout::vertical(constraints).split(area);
    for (index, table) in tables.iter().enumerate() {
        table.render(frame, areas[index * 2], focused, c);
    }
    if let Some(detail) = detail {
        detail.render(frame, areas[tables.len() * 2], c);
    }
    if let Some(notes_area) = areas.last() {
        let lines: Vec<Line<'_>> = notes
            .iter()
            .map(|note| Line::from(Span::styled(note.as_str(), c.muted())))
            .collect();
        frame.render_widget(Paragraph::new(lines), *notes_area);
    }
}

/// The cursor step a plain `j`/`k`/`Home`/`G` key asks for; `None` for any other key.
pub(super) fn motion(key: KeyEvent) -> Option<Step> {
    if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
        return None;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Step::Next),
        KeyCode::Char('k') | KeyCode::Up => Some(Step::Prev),
        KeyCode::Home => Some(Step::First),
        KeyCode::Char('G') | KeyCode::End => Some(Step::Last),
        _ => None,
    }
}

/// A ledger-data list page Settings hosts beside Units and Accounts.
pub(super) trait Page {
    /// `None` when the page leaves `key` to Settings.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;
    /// The heading's note, counts derived from the rows.
    fn scope_note(&self) -> String;
    fn hints(&self) -> Vec<(String, String)>;
    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours);
}
