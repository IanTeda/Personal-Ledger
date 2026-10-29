//! The Settings index (TUI-047): a heading over every page in index order, under the this device
//! and this ledger group headings (ADR-0051). At Medium it is a page of its own, each row with a
//! one-line description; at Large and X-Large it docks beside the page, names only (TUI-050,
//! TUI-051). With no Ledger open the this ledger rows are drawn dim (TUI-059).

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::{Group, SettingsPage, clip, render_heading, render_ruled_heading};
use crate::colours::Colours;
use crate::msg;

/// The page-name column, wide enough for the longest label.
const NAME_WIDTH: u16 = 16;

/// One row of the index, the docked index or the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    /// A group's heading, dim over the rest of its row's rule.
    Heading(Group),
    /// The blank row that ends a group before the next one's heading.
    Gap,
    Page(SettingsPage),
}

/// Every row in order: each group's heading, then its pages, a gap between the groups.
pub(super) fn entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut group = None;
    for page in SettingsPage::ALL {
        if group != Some(page.group()) {
            if group.is_some() {
                entries.push(Entry::Gap);
            }
            entries.push(Entry::Heading(page.group()));
            group = Some(page.group());
        }
        entries.push(Entry::Page(page));
    }
    entries
}

/// The rows [`entries`] takes, as a height.
pub(super) fn height() -> u16 {
    u16::try_from(entries().len()).unwrap_or(u16::MAX)
}

/// A group's heading: its name dim, then a light rule to the row's end.
pub(super) fn render_group_heading(frame: &mut Frame<'_>, area: Rect, group: Group, c: &Colours) {
    let name = group.heading();
    let rule_width = usize::from(area.width).saturating_sub(name.chars().count() + 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{name} {}", "\u{2500}".repeat(rule_width)),
            c.muted(),
        )),
        area,
    );
}

/// The docked index's width, border included: the Desktop's 232px at 7.5px a column.
pub(super) const DOCKED_WIDTH: u16 = 31;

/// The index's keys, for the hint bar. `back` is the Shell's configured back key. `enter` opens
/// the page in the index's place, or, `docked`, focuses the page already beside it. With no
/// `ledger_open`, the bar names the two ways to open one in place of first/last (TUI-059).
pub(super) fn hints(back: &str, docked: bool, ledger_open: bool) -> Vec<(String, String)> {
    let enter = if docked {
        msg::tui_settings_hint_focus_page()
    } else {
        msg::tui_settings_hint_open()
    };
    let mut hints = vec![
        ("j/k".into(), msg::tui_settings_hint_page()),
        ("enter".into(), enter),
    ];
    if ledger_open {
        hints.push(("home/G".into(), msg::tui_settings_hint_first_last()));
    } else {
        hints.extend([
            (
                crate::shell::OPEN_COMMAND.into(),
                msg::tui_settings_hint_open_a_ledger(),
            ),
            (
                crate::shell::NEW_COMMAND.into(),
                msg::tui_settings_hint_start_one(),
            ),
        ]);
    }
    hints.push((back.to_string(), msg::tui_settings_hint_back()));
    hints
}

/// The heading over its strong rule, then every row of [`entries`]; a taller terminal leaves the
/// space below them.
pub(super) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    selected: SettingsPage,
    ledger_open: bool,
    c: &Colours,
) {
    let [heading, rows] =
        Layout::vertical([Constraint::Length(2), Constraint::Length(height())]).areas(area);

    let title = Line::from(Span::styled(
        msg::tui_settings_heading(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    render_heading(frame, heading, title, &page_count(ledger_open), c);

    let entries = entries();
    let row_areas = Layout::vertical(vec![Constraint::Length(1); entries.len()]).split(rows);
    for (entry, row) in entries.into_iter().zip(row_areas.iter()) {
        match entry {
            Entry::Heading(group) => render_group_heading(frame, *row, group, c),
            Entry::Gap => {}
            Entry::Page(page) => {
                let dim = !ledger_open && page.needs_ledger();
                render_row(frame, *row, page, page == selected, dim, c);
            }
        }
    }
}

/// `17 pages`, the heading's right-hand note, which with no `ledger_open` also counts the pages
/// that work without one.
pub(super) fn page_count(ledger_open: bool) -> String {
    let pages =
        msg::tui_settings_page_count(i64::try_from(SettingsPage::ALL.len()).unwrap_or(i64::MAX));
    if ledger_open {
        return pages;
    }
    let without = SettingsPage::ALL
        .iter()
        .filter(|page| !page.needs_ledger())
        .count();
    msg::tui_settings_page_count_without_ledger(
        &pages,
        &crate::format::count(i64::try_from(without).unwrap_or(i64::MAX)),
    )
}

/// One row: the name, the description dim and clipped, and a dim `›` at the right edge. The
/// selected row is reversed across its width; a `dim` row, refused for want of a Ledger, dims its
/// name too.
fn render_row(
    frame: &mut Frame<'_>,
    area: Rect,
    page: SettingsPage,
    selected: bool,
    dim_name: bool,
    c: &Colours,
) {
    if selected {
        frame.render_widget(Block::new().style(c.selection()), area);
    }
    let dim = if selected {
        Style::default()
    } else {
        c.muted()
    };
    let name_style = if dim_name { dim } else { Style::default() };
    let [name, description, chevron] = Layout::horizontal([
        Constraint::Length(NAME_WIDTH),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(area);
    frame.render_widget(Paragraph::new(Span::styled(page.label(), name_style)), name);
    frame.render_widget(
        Paragraph::new(Span::styled(
            clip(&page.description(), usize::from(description.width)),
            dim,
        )),
        description,
    );
    frame.render_widget(Paragraph::new(Span::styled("›", dim)), chevron);
}

/// The docked index in `area`, ruled off from the page on its right: the heading, then every row
/// of [`entries`], names only. `focused`, the selected row is reversed and the heading's rule
/// strong; otherwise the open page, which is the selection, carries the accent bar. With no
/// `ledger_open` the this ledger names are dim.
pub(super) fn render_docked(
    frame: &mut Frame<'_>,
    area: Rect,
    selected: SettingsPage,
    focused: bool,
    ledger_open: bool,
    c: &Colours,
) {
    let dock = Block::new().borders(Borders::RIGHT);
    let inner = dock.inner(area);
    frame.render_widget(dock, area);

    let [heading, rows] =
        Layout::vertical([Constraint::Length(2), Constraint::Length(height())]).areas(inner);

    let title = Line::from(Span::styled(
        msg::tui_settings_heading(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    let rule = if focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };
    render_ruled_heading(frame, heading, title, &page_count(ledger_open), rule, c);

    let entries = entries();
    let row_areas = Layout::vertical(vec![Constraint::Length(1); entries.len()]).split(rows);
    for (entry, row) in entries.into_iter().zip(row_areas.iter()) {
        let page = match entry {
            Entry::Heading(group) => {
                render_group_heading(frame, *row, group, c);
                continue;
            }
            Entry::Gap => continue,
            Entry::Page(page) => page,
        };
        let open = page == selected;
        if open && focused {
            frame.render_widget(Block::new().style(c.selection()), *row);
        }
        let [bar, name, _] = Layout::horizontal([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(*row);
        let mut name_style = if !ledger_open && page.needs_ledger() && !(open && focused) {
            c.muted()
        } else {
            Style::default()
        };
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
    }
}
