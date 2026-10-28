//! The Settings index (TUI-047): a heading over every page in index order, each with its scope
//! glyph, and the scope legend pinned to the bottom. At Medium it is a page of its own, each row
//! with a one-line description; at Large and X-Large it docks beside the page, names only
//! (TUI-050, TUI-051).

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::{SettingsPage, clip, render_heading, render_ruled_heading};
use crate::colours::Colours;
use crate::msg;

/// The page-name column, wide enough for the longest label.
const NAME_WIDTH: u16 = 16;

/// The scope-glyph column: the glyph and a gap.
const GLYPH_WIDTH: u16 = 2;

/// The docked index's width, border included: the Desktop's 232px at 7.5px a column.
pub(super) const DOCKED_WIDTH: u16 = 31;

/// The index's keys, for the hint bar. `back` is the Shell's configured back key. `enter` opens
/// the page in the index's place, or, `docked`, focuses the page already beside it.
pub(super) fn hints(back: &str, docked: bool) -> Vec<(String, String)> {
    let enter = if docked {
        msg::tui_settings_hint_focus_page()
    } else {
        msg::tui_settings_hint_open()
    };
    vec![
        ("j/k".into(), msg::tui_settings_hint_page()),
        ("enter".into(), enter),
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

/// The docked index in `area`, ruled off from the page on its right: the heading, one row per
/// page with its scope glyph, and the legend a half to a row at the bottom. `focused`, the
/// selected row is reversed and the heading's rule strong; otherwise the open page, which is the
/// selection, carries the accent bar.
pub(super) fn render_docked(
    frame: &mut Frame<'_>,
    area: Rect,
    selected: SettingsPage,
    focused: bool,
    c: &Colours,
) {
    let dock = Block::new().borders(Borders::RIGHT);
    let inner = dock.inner(area);
    frame.render_widget(dock, area);

    let pages = u16::try_from(SettingsPage::ALL.len()).unwrap_or(u16::MAX);
    let [heading, rows, _, legend] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(pages),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(inner);

    let title = Line::from(Span::styled(
        msg::tui_settings_heading(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    let rule = if focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };
    render_ruled_heading(frame, heading, title, &page_count(), rule, c);

    let row_areas =
        Layout::vertical(vec![Constraint::Length(1); SettingsPage::ALL.len()]).split(rows);
    for (page, row) in SettingsPage::ALL.into_iter().zip(row_areas.iter()) {
        let open = page == selected;
        if open && focused {
            frame.render_widget(Block::new().style(c.selection()), *row);
        }
        let [bar, name, glyph, _] = Layout::horizontal([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(*row);
        let mut name_style = Style::default();
        if open && !focused {
            frame.render_widget(Paragraph::new(Span::styled("▌", c.accent())), bar);
            name_style = name_style.add_modifier(Modifier::BOLD);
        }
        frame.render_widget(
            Paragraph::new(Span::styled(
                clip(&page.label(), usize::from(name.width)),
                name_style,
            )),
            name,
        );
        frame.render_widget(Paragraph::new(page.scope().glyph()), glyph);
    }

    let block = Block::new().borders(Borders::TOP);
    let legend_rows = block.inner(legend);
    frame.render_widget(block, legend);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(msg::tui_settings_legend_synced(), c.muted())),
            Line::from(Span::styled(
                msg::tui_settings_legend_this_device(),
                c.muted(),
            )),
        ]),
        legend_rows,
    );
}
