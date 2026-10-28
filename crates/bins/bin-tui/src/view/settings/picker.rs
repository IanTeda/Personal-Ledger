//! The Settings page picker (TUI-048): every page in index order in a popup centred in the View,
//! the command popup's treatment, with a `•` on the open page. All sixteen fit, so there is no
//! filter, and the mode stays NORMAL.

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::{SettingsPage, index::page_count, render_heading};
use crate::colours::Colours;
use crate::msg;

/// The popup's size in cells, border included: a title row and its rule over the sixteen pages.
const WIDTH: u16 = 40;
const HEIGHT: u16 = 20;

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

    /// Draws the popup centred in `area`, the View, with `current` marked.
    pub(super) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        current: SettingsPage,
        c: &Colours,
    ) {
        let popup = centred(area, WIDTH, HEIGHT);
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let pages = u16::try_from(SettingsPage::ALL.len()).unwrap_or(u16::MAX);
        let [heading, rows] =
            Layout::vertical([Constraint::Length(2), Constraint::Length(pages)]).areas(inner);
        let title = Line::from(Span::styled(
            msg::tui_settings_picker_title(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        render_heading(frame, heading, title, &page_count(), c);

        let row_areas =
            Layout::vertical(vec![Constraint::Length(1); SettingsPage::ALL.len()]).split(rows);
        for (page, row) in SettingsPage::ALL.into_iter().zip(row_areas.iter()) {
            if page == self.selected {
                frame.render_widget(Block::new().style(c.selection()), *row);
            }
            let [mark, name, glyph] = Layout::horizontal([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(*row);
            if page == current {
                frame.render_widget(Paragraph::new("•"), mark);
            }
            frame.render_widget(Paragraph::new(page.label()), name);
            frame.render_widget(Paragraph::new(page.scope().glyph()), glyph);
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
