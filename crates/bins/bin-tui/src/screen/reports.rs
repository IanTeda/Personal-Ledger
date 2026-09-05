//! The Reports screen (FR.34-38, CC-TUI-011) — one screen with an internal picker across
//! every report type, per "Decide TUI screen map and navigation shape" (not a separate
//! dashboard area per report). Only the Account Balance report (FR.34) exists so far;
//! tickets #76-79 each add their own [`ReportKind`] variant and rendering branch here.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

/// Which report is currently selected. `#76-79` will each add a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportKind {
    /// FR.34: the current Balance of every Account, each expressed in its own Unit.
    AccountBalance,
}

const REPORT_KINDS: [ReportKind; 1] = [ReportKind::AccountBalance];

impl ReportKind {
    fn title(&self) -> &'static str {
        match self {
            ReportKind::AccountBalance => "Account Balance",
        }
    }
}

enum AccountsStatus {
    Loading,
    Loaded(Vec<lib_database::Accounts>),
    Failed(String),
}

/// One screen, an internal picker across every report type.
pub struct ReportsScreen {
    selected_report: ReportKind,
    accounts: AccountsStatus,
    units: Vec<lib_database::Units>,
    balances: Vec<(lib_core::RowID, lib_core::Money)>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl ReportsScreen {
    pub fn new() -> Self {
        Self {
            selected_report: REPORT_KINDS[0],
            accounts: AccountsStatus::Loading,
            units: Vec::new(),
            balances: Vec::new(),
            error: None,
            action_tx: None,
        }
    }

    fn cycle_report(&mut self, delta: isize) {
        let len = REPORT_KINDS.len() as isize;
        let current = REPORT_KINDS
            .iter()
            .position(|k| *k == self.selected_report)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len);
        self.selected_report = REPORT_KINDS[next as usize];
    }

    fn unit_code(&self, id: lib_core::RowID) -> &str {
        self.units
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.code.as_str())
            .unwrap_or("?")
    }

    fn balance_for(&self, id: lib_core::RowID) -> Option<&lib_core::Money> {
        self.balances
            .iter()
            .find(|(aid, _)| *aid == id)
            .map(|(_, balance)| balance)
    }
}

impl Default for ReportsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for ReportsScreen {
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

        let units_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Units::find_all(&pool).await
            }
            .await;
            let action = match action {
                Ok(units) => Action::UnitsLoaded(units),
                Err(err) => Action::UnitsLoadFailed(err.to_string()),
            };
            let _ = units_tx.send(action);
        });

        let balances_tx = action_tx.clone();
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
            let _ = balances_tx.send(action);
        });

        self.action_tx = Some(action_tx);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.cycle_report(-1);
                Some(Action::NoOp)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.cycle_report(1);
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::AccountsLoaded(accounts) => {
                self.accounts = AccountsStatus::Loaded(accounts.clone());
            }
            Action::AccountsLoadFailed(message) => {
                self.accounts = AccountsStatus::Failed(message.clone());
            }
            Action::UnitsLoaded(units) => self.units = units.clone(),
            Action::AccountBalancesLoaded(balances) => {
                for (id, balance) in balances {
                    match self.balances.iter_mut().find(|(aid, _)| aid == id) {
                        Some((_, existing)) => *existing = balance.clone(),
                        None => self.balances.push((*id, balance.clone())),
                    }
                }
            }
            Action::AccountBalancesLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Reports"
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

        let picker_text = format!("← {} →", self.selected_report.title());
        frame.render_widget(
            Paragraph::new(picker_text).block(Block::bordered().title(" Report ")),
            rows[0],
        );

        match self.selected_report {
            ReportKind::AccountBalance => match &self.accounts {
                AccountsStatus::Loading => {
                    frame.render_widget(
                        Paragraph::new("Loading Accounts...")
                            .block(Block::bordered().title(" Account Balance ")),
                        rows[1],
                    );
                }
                AccountsStatus::Failed(message) => {
                    frame.render_widget(
                        Paragraph::new(format!("Failed to load Accounts: {message}"))
                            .style(Style::default().fg(Color::Red))
                            .block(Block::bordered().title(" Account Balance ")),
                        rows[1],
                    );
                }
                AccountsStatus::Loaded(accounts) => {
                    let mut sorted: Vec<&lib_database::Accounts> = accounts.iter().collect();
                    sorted.sort_by(|a, b| {
                        self.unit_code(a.unit_id)
                            .cmp(self.unit_code(b.unit_id))
                            .then(a.name.cmp(&b.name))
                    });

                    let header = Row::new(["Account", "Unit", "Balance"])
                        .style(Style::default().fg(Color::Yellow));
                    let table_rows = sorted.iter().map(|account| {
                        let style = if account.is_active {
                            Style::default()
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        let balance_text = self
                            .balance_for(account.id)
                            .map(|b| b.to_string())
                            .unwrap_or_else(|| "…".to_string());
                        Row::new([
                            Cell::from(account.name.clone()),
                            Cell::from(self.unit_code(account.unit_id).to_string()),
                            Cell::from(balance_text),
                        ])
                        .style(style)
                    });
                    let widths = [
                        Constraint::Length(24),
                        Constraint::Length(8),
                        Constraint::Length(16),
                    ];
                    let table = Table::new(table_rows, widths).header(header).block(
                        Block::bordered().title(format!(" Account Balance ({}) ", accounts.len())),
                    );
                    frame.render_widget(table, rows[1]);
                }
            },
        }

        let footer = if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            "←/h →/l: change report  Esc: back".to_string()
        };
        frame.render_widget(Paragraph::new(footer), rows[2]);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn mock_unit(code: &str) -> lib_database::Units {
        let now = chrono::Utc::now();
        lib_database::Units {
            id: lib_core::RowID::new(),
            code: code.to_string(),
            name: format!("{code} Name"),
            unit_kind: lib_core::UnitKind::Fiat,
            decimal_places: 2,
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn mock_account(
        name: &str,
        unit_id: lib_core::RowID,
        is_active: bool,
    ) -> lib_database::Accounts {
        let now = chrono::Utc::now();
        lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: name.to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id,
            starting_balance: lib_core::Money::mock(),
            is_active,
            created_on: now,
            updated_on: now,
        }
    }

    fn render(screen: &ReportsScreen) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Reports screen should not error");
    }

    #[test]
    fn new_starts_on_the_account_balance_report_loading() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.selected_report, ReportKind::AccountBalance);
        render(&screen);
    }

    #[test]
    fn renders_loaded_accounts_including_inactive_ones() {
        let mut screen = ReportsScreen::new();
        let unit = mock_unit("AUD");
        screen.update(&Action::UnitsLoaded(vec![unit.clone()]));
        screen.update(&Action::AccountsLoaded(vec![
            mock_account("Everyday", unit.id, true),
            mock_account("Old Account", unit.id, false),
        ]));
        render(&screen);
    }

    #[test]
    fn cycling_report_wraps_with_only_one_kind() {
        let mut screen = ReportsScreen::new();
        screen.handle_key(key(KeyCode::Right), InputMode::Navigation);
        assert_eq!(screen.selected_report, ReportKind::AccountBalance);
    }

    #[test]
    fn account_balances_loaded_populates_the_lookup() {
        let mut screen = ReportsScreen::new();
        let account_id = lib_core::RowID::new();
        screen.update(&Action::AccountBalancesLoaded(vec![(
            account_id,
            "42".parse().unwrap(),
        )]));

        assert_eq!(screen.balance_for(account_id), Some(&"42".parse().unwrap()));
    }

    #[test]
    fn unit_code_falls_back_when_unknown() {
        let screen = ReportsScreen::new();
        assert_eq!(screen.unit_code(lib_core::RowID::new()), "?");
    }
}
