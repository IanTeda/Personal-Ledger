//! The Settings page picker (TUI-048): every page in index order under the index's two group
//! headings, in a popup centred in the View, the command popup's treatment, with a `•` on the open
//! page. All seventeen fit, so there is no filter, and the mode stays NORMAL. With no Ledger open
//! the this ledger pages are dim, and `enter` on one is refused by the View (TUI-059).

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::index::{Entry, entries, height, page_count, render_group_heading};
use super::{SettingsPage, render_heading};
use crate::colours::Colours;
use crate::msg;

/// The popup's width in cells, border included.
const WIDTH: u16 = 40;

/// What a key did to the picker.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Moved,
    Open(SettingsPage),
    Close,
    Ignored,
}

/// The open picker: which page its selection is on.
#[derive(Debug)]
pub(super) struct Picker {
    selected: SettingsPage,
}

impl Picker {
    /// Opens on `current`, the page showing.
    pub(super) fn new(current: SettingsPage) -> Self {
        Self { selected: current }
    }

    pub(super) fn selected(&self) -> SettingsPage {
        self.selected
    }

    /// The picker takes every key while open, as a popup does. Bare `g` reaches it, since the
    /// Shell's jump leader only arms with no popup open.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        let step = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Step::Next,
            KeyCode::Char('k') | KeyCode::Up => Step::Prev,
            KeyCode::Char('g') | KeyCode::Home => Step::First,
            KeyCode::Char('G') | KeyCode::End => Step::Last,
            KeyCode::Enter => return Outcome::Open(self.selected),
            KeyCode::Esc => return Outcome::Close,
            _ => return Outcome::Ignored,
        };
        self.selected = self.selected.step(step);
        Outcome::Moved
    }

    /// Draws the popup centred in `area`, the View, with `current` marked: a title row and its
    /// rule over the index's rows, inside the border.
    pub(super) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        current: SettingsPage,
        ledger_open: bool,
        c: &Colours,
    ) {
        let popup = centred(area, WIDTH, height() + 4);
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let [heading, rows] =
            Layout::vertical([Constraint::Length(2), Constraint::Length(height())]).areas(inner);
        let title = Line::from(Span::styled(
            msg::tui_settings_picker_title(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        // The plain count: the picker is too narrow for the no-Ledger note beside its title.
        render_heading(frame, heading, title, &page_count(true), c);

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
            let selected = page == self.selected;
            if selected {
                frame.render_widget(Block::new().style(c.selection()), *row);
            }
            let [mark, name] =
                Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).areas(*row);
            if page == current {
                frame.render_widget(Paragraph::new("•"), mark);
            }
            let style = if !ledger_open && page.needs_ledger() && !selected {
                c.muted()
            } else {
                Style::default()
            };
            frame.render_widget(Paragraph::new(Span::styled(page.label(), style)), name);
        }
    }
}

/// The picker's keys, for the hint bar while it is open. Its close key is `Esc`, as every popup's.
pub(super) fn hints(_back: &str) -> Vec<(String, String)> {
    vec![
        ("j/k".into(), msg::tui_settings_hint_page()),
        ("g/G".into(), msg::tui_settings_hint_first_last()),
        ("enter".into(), msg::tui_settings_hint_open()),
        ("esc".into(), msg::tui_settings_hint_close()),
    ]
}

/// A `width` × `height` rect centred in `area`, shrunk to fit it.
fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
