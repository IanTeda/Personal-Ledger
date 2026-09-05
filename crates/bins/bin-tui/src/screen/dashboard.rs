//! The dashboard — the navigation hub "Decide TUI screen map and navigation shape" locked
//! in. Every entity/report area is a push/pop drill-in reached from here; a live snapshot of
//! Accounts' current Balances lives above the menu, reusing `AccountsListScreen`'s own
//! `AccountsLoaded`/`AccountsLoadFailed` actions rather than inventing dashboard-specific
//! ones — it's the same "list every Account" query either screen needs. Each Balance is
//! computed via `Accounts::balance` (FR.34) and delivered via `AccountBalancesLoaded`, the
//! same action the Reports screen's Account Balance report uses.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

enum Snapshot {
    Loading,
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

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
            action: Some(Action::OpenUnits),
            not_yet_built_hint: "",
        },
        Area {
            name: "Categories",
            action: Some(Action::OpenCategories),
            not_yet_built_hint: "",
        },
        Area {
            name: "Accounts",
            action: Some(Action::OpenAccounts),
            not_yet_built_hint: "",
        },
        Area {
            name: "Transactions",
            action: Some(Action::OpenTransactions),
            not_yet_built_hint: "",
        },
        Area {
            name: "Payees",
            action: Some(Action::OpenPayees),
            not_yet_built_hint: "",
        },
        Area {
            name: "Balance Checks",
            action: Some(Action::OpenBalanceChecks),
            not_yet_built_hint: "",
        },
        Area {
            name: "Budgeting",
            action: Some(Action::OpenBudgets),
            not_yet_built_hint: "",
        },
        Area {
            name: "Reports",
            action: Some(Action::OpenReports),
            not_yet_built_hint: "",
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
    snapshot: Snapshot,
    balances: Vec<(lib_core::RowID, lib_core::Money)>,
}

impl DashboardScreen {
    pub fn new() -> Self {
        Self {
            areas: areas(),
            selected: 0,
            status: None,
            snapshot: Snapshot::Loading,
            balances: Vec::new(),
        }
    }

    fn balance_for(&self, id: lib_core::RowID) -> Option<&lib_core::Money> {
        self.balances
            .iter()
            .find(|(aid, _)| *aid == id)
            .map(|(_, balance)| balance)
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
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let accounts_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Accounts::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(accounts) => Action::AccountsLoaded(accounts),
                Err(err) => Action::AccountsLoadFailed(err.to_string()),
            };
            let _ = accounts_tx.send(action);
        });

        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                let accounts = lib_database::Accounts::find_all(&pool).await?;
                let mut balances = Vec::with_capacity(accounts.len());
                for account in accounts {
                    let balance = account.balance(&pool).await?;
                    balances.push((account.id, balance));
                }
                Ok::<_, lib_database::DatabaseError>(balances)
            }
            .await;
            let action = match action {
                Ok(balances) => Action::AccountBalancesLoaded(balances),
                Err(err) => Action::AccountBalancesLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }

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

    fn update(&mut self, action: &Action) {
        match action {
            Action::AccountsLoaded(accounts) => self.snapshot = Snapshot::Loaded(accounts.clone()),
            Action::AccountsLoadFailed(message) => {
                self.snapshot = Snapshot::Failed(message.clone())
            }
            Action::AccountBalancesLoaded(balances) => {
                for (id, balance) in balances {
                    match self.balances.iter_mut().find(|(aid, _)| aid == id) {
                        Some((_, existing)) => *existing = balance.clone(),
                        None => self.balances.push((*id, balance.clone())),
                    }
                }
            }
            _ => {}
        }
    }

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

        let snapshot_text = match &self.snapshot {
            Snapshot::Loading => "Loading Accounts...".to_string(),
            Snapshot::Failed(message) => format!("Failed to load Accounts: {message}"),
            Snapshot::Loaded(accounts) if accounts.is_empty() => {
                "No Accounts yet — create one to see its balance here.".to_string()
            }
            Snapshot::Loaded(accounts) => accounts
                .iter()
                .map(|account| {
                    let balance = self
                        .balance_for(account.id)
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "…".to_string());
                    format!("{}: {balance}", account.name)
                })
                .collect::<Vec<_>>()
                .join("   "),
        };
        let snapshot = Paragraph::new(snapshot_text).block(Block::bordered().title(" Accounts "));
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
