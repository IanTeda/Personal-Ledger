//! The Settings index page (TUI-047): a heading over every page in index order, each with its
//! scope glyph and a one-line description, and the scope legend pinned to the bottom.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::{SettingsPage, clip, render_heading};
use crate::colours::Colours;
use crate::msg;

/// The page-name column, wide enough for the longest label.
const NAME_WIDTH: u16 = 16;

/// The scope-glyph column: the glyph and a gap.
const GLYPH_WIDTH: u16 = 2;

/// The index page's keys, for the hint bar. `back` is the Shell's configured back key.
pub(super) fn hints(back: &str) -> Vec<(String, String)> {
    vec![
        ("j/k".into(), msg::tui_settings_hint_page()),
        ("enter".into(), msg::tui_settings_hint_open()),
        ("home/G".into(), msg::tui_settings_hint_first_last()),
        (back.to_string(), msg::tui_settings_hint_back()),
    ]
}

/// The heading over its strong rule, one row per page, then a `Min(0)` gap and the legend over a
/// rule, so a taller terminal leaves the space above the legend rather than below it.
pub(super) fn render(frame: &mut Frame<'_>, area: Rect, selected: SettingsPage, c: &Colours) {
    let pages = u16::try_from(SettingsPage::ALL.len()).unwrap_or(u16::MAX);
    let [heading, rows, _, legend] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(pages),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(area);

    let title = Line::from(Span::styled(
        msg::tui_settings_heading(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    render_heading(frame, heading, title, &page_count(), c);

    let row_areas =
        Layout::vertical(vec![Constraint::Length(1); SettingsPage::ALL.len()]).split(rows);
    for (page, row) in SettingsPage::ALL.into_iter().zip(row_areas.iter()) {
        render_row(frame, *row, page, page == selected, c);
    }

    let block = Block::new().borders(Borders::TOP);
    let legend_row = block.inner(legend);
    frame.render_widget(block, legend);
    frame.render_widget(
        Paragraph::new(Span::styled(msg::tui_settings_legend(), c.muted())),
        legend_row,
    );
}

/// `16 pages`, the heading's right-hand note.
pub(super) fn page_count() -> String {
    msg::tui_settings_page_count(i64::try_from(SettingsPage::ALL.len()).unwrap_or(i64::MAX))
}

/// One row: the name, its scope glyph, the description dim and clipped, and a dim `›` at the right
/// edge. The selected row is reversed across its width.
fn render_row(frame: &mut Frame<'_>, area: Rect, page: SettingsPage, selected: bool, c: &Colours) {
    if selected {
        frame.render_widget(Block::new().style(c.selection()), area);
    }
    let dim = if selected {
        Style::default()
    } else {
        c.muted()
    };
    let [name, glyph, description, chevron] = Layout::horizontal([
        Constraint::Length(NAME_WIDTH),
        Constraint::Length(GLYPH_WIDTH),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(area);
    frame.render_widget(Paragraph::new(page.label()), name);
    frame.render_widget(Paragraph::new(page.scope().glyph()), glyph);
    frame.render_widget(
        Paragraph::new(Span::styled(
            clip(&page.description(), usize::from(description.width)),
            dim,
        )),
        description,
    );
    frame.render_widget(Paragraph::new(Span::styled("›", dim)), chevron);
}
