//! The dashboard — the navigation hub "Decide TUI screen map and navigation shape" locked
//! in. Every entity/report area is a push/pop drill-in reached from here; a live snapshot
//! (Account balances, once Accounts exist — see issue #69) will eventually live above the
//! menu, but starts honestly empty rather than faked from unrelated data.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
};

use crate::{
    action::{Action, InputMode},
    screen::Screen,
};

/// One row of the dashboard's drill-in menu.
struct Area {
    name: &'static str,
    /// `None` while nothing built yet plugs into this row; `Some(action)` once a real
    /// build ticket lands and wires it up (see each row's ticket reference below).
    action: Option<Action>,
    /// Shown when the row isn't wired up yet and the user tries to open it anyway.
    not_yet_built_hint: &'static str,
}

/// The fixed drill-in areas per the decided screen map — order matches how they'll be
/// built out, not alphabetical.
fn areas() -> Vec<Area> {
    vec![
        Area {
            name: "Units",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #67",
        },
        Area {
            name: "Categories",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #68",
        },
        Area {
            name: "Accounts",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #69",
        },
        Area {
            name: "Transactions",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #70",
        },
        Area {
            name: "Balance Checks",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #72",
        },
        Area {
            name: "Budgeting",
            action: None,
            not_yet_built_hint: "Not built yet — see issue #73",
        },
        Area {
            name: "Reports",
            action: None,
            not_yet_built_hint: "Not built yet — see issues #75-79",
        },
        Area {
            name: "Settings",
            action: Some(Action::OpenSettings),
            not_yet_built_hint: "",
        },
    ]
}

/// The navigation hub every other screen is reached from.
pub struct DashboardScreen {
    areas: Vec<Area>,
    selected: usize,
    /// Set when the user tries to open a not-yet-built area; cleared on the next move.
    status: Option<&'static str>,
}

impl DashboardScreen {
    pub fn new() -> Self {
        Self {
            areas: areas(),
            selected: 0,
            status: None,
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.areas.len() as isize;
        let next = (self.selected as isize + delta).rem_euclid(len);
        self.selected = next as usize;
        self.status = None;
    }
}

impl Default for DashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for DashboardScreen {
    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                None
            }
            KeyCode::Enter => {
                let area = &self.areas[self.selected];
                match &area.action {
                    Some(action) => Some(action.clone()),
                    None => {
                        self.status = Some(area.not_yet_built_hint);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    fn update(&mut self, _action: &Action) {}

    fn title(&self) -> &'static str {
        "Dashboard"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let snapshot =
            Paragraph::new("No Accounts yet — build Units then Accounts to see balances here.")
                .block(Block::bordered().title(" Accounts "));
        frame.render_widget(snapshot, rows[0]);

        let items: Vec<ListItem> = self
            .areas
            .iter()
            .enumerate()
            .map(|(index, area)| {
                let style = if index == self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if area.action.is_none() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(format!(" {} ", area.name), style)))
            })
            .collect();
        frame.render_widget(
            List::new(items).block(Block::bordered().title(" Areas ")),
            rows[1],
        );

        let footer_text = self
            .status
            .map(str::to_string)
            .unwrap_or_else(|| "↑/k ↓/j: move  Enter: open  ?: help  Ctrl+C: quit".to_string());
        frame.render_widget(Paragraph::new(footer_text), rows[2]);
    }
}
