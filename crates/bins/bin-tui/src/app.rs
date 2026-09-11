//! The top-level application: owns terminal lifecycle, the async event loop, and the
//! navigation stack — the "Elm" half of ADR-0003's hybrid architecture
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`). The Dashboard is the base of
//! the stack ("Decide TUI screen map and navigation shape"); every other screen is pushed
//! on top of it and popped back off, replacing the feasibility cycle's flat Tab-cycling.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
};
use tokio::sync::mpsc;

use crate::{
    Result,
    action::{Action, InputMode},
    event::{Event, EventHandler},
    screen::{
        Screen, account_detail::AccountDetailScreen, accounts_list::AccountsListScreen,
        balance_check_detail::BalanceCheckDetailScreen,
        balance_checks_list::BalanceChecksListScreen, budget_detail::BudgetDetailScreen,
        budgets_list::BudgetsListScreen, csv_import::CsvImportScreen, dashboard::DashboardScreen,
        help::HelpScreen, payee_detail::PayeeDetailScreen, payees_list::PayeesListScreen,
        reports::ReportsScreen, settings::SettingsScreen,
        transaction_detail::TransactionDetailScreen, transactions_list::TransactionsListScreen,
        unit_detail::UnitDetailScreen, units_list::UnitsListScreen,
    },
    tui::Tui,
};

/// How often an [`Action::Tick`] fires in the absence of input.
const TICK_RATE: Duration = Duration::from_millis(250);

/// Owns terminal lifecycle and the navigation stack, and drives the async event loop.
pub struct App {
    /// The navigation stack: index 0 is always the Dashboard; the last entry is the active,
    /// rendered-and-keyed screen. `Esc` pops it; popping the Dashboard itself quits instead.
    stack: Vec<Box<dyn Screen>>,
    /// Whether raw keys are currently interpreted as navigation shortcuts or literal text
    /// entry — see [`InputMode`]. No screen in this skeleton switches it yet; the field
    /// exists so `Screen::handle_key`'s signature is load-bearing from the start.
    mode: InputMode,
    should_quit: bool,
    /// Where a screen's `init()` (e.g. a background load) reports results back as an
    /// [`Action`].
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    /// Creates the app with the Dashboard as the sole, base screen.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut dashboard: Box<dyn Screen> = Box::new(DashboardScreen::new());
        dashboard.init(action_tx.clone());

        Self {
            stack: vec![dashboard],
            mode: InputMode::Navigation,
            should_quit: false,
            action_tx,
            action_rx,
        }
    }

    /// Runs the app until the user quits.
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.draw(frame))?;

        loop {
            let action = tokio::select! {
                event = events.next() => match event.and_then(|event| self.map_event(event)) {
                    Some(action) => action,
                    None => continue,
                },
                Some(action) = self.action_rx.recv() => action,
            };
            self.update(action);
            if self.should_quit {
                break;
            }
            tui.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    /// Translates a raw terminal event into an [`Action`]: the active screen gets first
    /// refusal via `handle_key` before falling back to the small, truly-global key set.
    fn map_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => {
                if is_hard_quit(key) {
                    return Some(Action::Quit);
                }

                let mode = self.mode;
                if let Some(action) = self
                    .stack
                    .last_mut()
                    .expect("navigation stack always has the Dashboard as its base")
                    .handle_key(key, mode)
                {
                    return Some(action);
                }

                match key.code {
                    KeyCode::Esc => Some(Action::Back),
                    KeyCode::Char('?') => Some(Action::OpenHelp),
                    _ => None,
                }
            }
            // `App` is disconnected from `main.rs` (ADR-0013) and never handles resize; this
            // arm exists only so the shared `Event` enum stays exhaustive here.
            Event::Resize => None,
        }
    }

    /// Applies an [`Action`] to application and navigation-stack state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Back => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                } else {
                    // Esc at the Dashboard, the base of the stack, quits the app.
                    self.should_quit = true;
                }
            }
            Action::OpenSettings => self.push(Box::new(SettingsScreen::new())),
            Action::OpenHelp => {
                let already_on_help = self
                    .stack
                    .last()
                    .map(|screen| screen.title() == "Help")
                    .unwrap_or(false);
                if !already_on_help {
                    self.push(Box::new(HelpScreen::new()));
                }
            }
            Action::OpenUnits => self.push(Box::new(UnitsListScreen::new())),
            Action::OpenUnitDetail(None) => self.push(Box::new(UnitDetailScreen::new_create())),
            Action::OpenUnitDetail(Some(unit)) => {
                self.push(Box::new(UnitDetailScreen::new_edit(unit)))
            }
            Action::OpenAccounts => self.push(Box::new(AccountsListScreen::new())),
            Action::OpenAccountDetail(None) => {
                self.push(Box::new(AccountDetailScreen::new_create()))
            }
            Action::OpenAccountDetail(Some(account)) => {
                self.push(Box::new(AccountDetailScreen::new_edit(account)))
            }
            Action::OpenTransactions => self.push(Box::new(TransactionsListScreen::new())),
            Action::OpenTransactionDetail(None) => {
                self.push(Box::new(TransactionDetailScreen::new_create()))
            }
            Action::OpenTransactionDetail(Some(transaction)) => {
                self.push(Box::new(TransactionDetailScreen::new_edit(transaction)))
            }
            Action::OpenPayees => self.push(Box::new(PayeesListScreen::new())),
            Action::OpenPayeeDetail(None) => self.push(Box::new(PayeeDetailScreen::new_create())),
            Action::OpenPayeeDetail(Some(payee)) => {
                self.push(Box::new(PayeeDetailScreen::new_edit(payee)))
            }
            Action::OpenBalanceChecks => self.push(Box::new(BalanceChecksListScreen::new())),
            Action::OpenBalanceCheckDetail(None) => {
                self.push(Box::new(BalanceCheckDetailScreen::new_create()))
            }
            Action::OpenBalanceCheckDetail(Some(balance_check)) => {
                self.push(Box::new(BalanceCheckDetailScreen::new_edit(balance_check)))
            }
            Action::OpenCsvImport => self.push(Box::new(CsvImportScreen::new())),
            Action::OpenReports => self.push(Box::new(ReportsScreen::new())),
            Action::OpenBudgets => self.push(Box::new(BudgetsListScreen::new())),
            Action::OpenBudgetDetail(None) => self.push(Box::new(BudgetDetailScreen::new_create())),
            Action::OpenBudgetDetail(Some(budget)) => {
                self.push(Box::new(BudgetDetailScreen::new_edit(budget)))
            }
            Action::NoOp => {}
            Action::Tick => self
                .stack
                .last_mut()
                .expect("non-empty stack")
                .update(&action),
            // A background load or delete can finish while any screen is active, and other
            // screens ignore it via their `update`'s default `_ => {}` arm. `AccountsLoaded`/
            // `AccountsLoadFailed` also feed the Dashboard's own live snapshot, not just an
            // Accounts list screen if one happens to be on the stack too.
            Action::CategoriesLoaded(_)
            | Action::CategoriesLoadFailed(_)
            | Action::UnitsLoaded(_)
            | Action::UnitsLoadFailed(_)
            | Action::UnitDeleted(_)
            | Action::UnitDeleteFailed(_)
            | Action::AccountsLoaded(_)
            | Action::AccountsLoadFailed(_)
            | Action::AccountDeleted(_)
            | Action::AccountDeleteFailed(_)
            | Action::TransactionsLoaded(_)
            | Action::TransactionsLoadFailed(_)
            | Action::TransactionDeleted(_)
            | Action::TransactionDeleteFailed(_)
            | Action::PayeesLoaded(_)
            | Action::PayeesLoadFailed(_)
            | Action::PayeeDeleted(_)
            | Action::PayeeDeleteFailed(_)
            | Action::PayeeAliasesLoaded(_)
            | Action::PayeeAliasesLoadFailed(_)
            | Action::BalanceChecksLoaded(_)
            | Action::BalanceChecksLoadFailed(_)
            | Action::BalanceCheckDeleted(_)
            | Action::BalanceCheckDeleteFailed(_)
            | Action::BalanceChecksImported(_)
            | Action::BalanceChecksImportFailed(_)
            | Action::BudgetsLoaded(_)
            | Action::BudgetsLoadFailed(_)
            | Action::BudgetProgressLoaded(_)
            | Action::BudgetProgressLoadFailed(_)
            | Action::BudgetDeleted(_)
            | Action::BudgetDeleteFailed(_)
            | Action::AccountBalancesLoaded(_)
            | Action::AccountBalancesLoadFailed(_)
            | Action::CategoryTotalsLoaded(_)
            | Action::CategoryTotalsLoadFailed(_)
            | Action::PayeeTotalsLoaded(_)
            | Action::PayeeTotalsLoadFailed(_)
            | Action::BalanceCheckBalancesLoaded(_)
            | Action::BalanceCheckBalancesLoadFailed(_) => {
                self.broadcast(&action);
            }
            // A successful save returns to whichever list screen the detail screen was
            // pushed from, after that list has absorbed the new/updated row.
            Action::UnitSaved(_) => self.broadcast_and_pop_detail(&action, "Unit"),
            Action::AccountSaved(_) => self.broadcast_and_pop_detail(&action, "Account"),
            Action::TransactionSaved(_) => self.broadcast_and_pop_detail(&action, "Transaction"),
            Action::PayeeSaved(_) => self.broadcast_and_pop_detail(&action, "Payee"),
            Action::BalanceCheckSaved(_) => self.broadcast_and_pop_detail(&action, "Balance Check"),
            Action::BudgetSaved(_) => self.broadcast_and_pop_detail(&action, "Budget"),
            // A failed save stays on the detail screen so the user can fix and retry.
            Action::UnitSaveFailed(_)
            | Action::AccountSaveFailed(_)
            | Action::TransactionSaveFailed(_)
            | Action::PayeeSaveFailed(_)
            | Action::BalanceCheckSaveFailed(_)
            | Action::BudgetSaveFailed(_) => {
                if let Some(screen) = self.stack.last_mut() {
                    screen.update(&action);
                }
            }
        }
    }

    /// Pushes a screen onto the navigation stack, giving it a chance to kick off any
    /// background work via `init()`.
    fn push(&mut self, mut screen: Box<dyn Screen>) {
        screen.init(self.action_tx.clone());
        self.stack.push(screen);
    }

    /// Delivers an action to every screen on the stack.
    fn broadcast(&mut self, action: &Action) {
        for screen in &mut self.stack {
            screen.update(action);
        }
    }

    /// Broadcasts a successful save, then pops the stack if a detail screen with the given
    /// title is on top — returning to whichever list screen pushed it.
    fn broadcast_and_pop_detail(&mut self, action: &Action, detail_title: &str) {
        self.broadcast(action);
        if self.stack.len() > 1 && self.stack.last().unwrap().title() == detail_title {
            self.stack.pop();
        }
    }

    /// Renders a breadcrumb of the navigation stack and the active (top) screen.
    fn draw(&self, frame: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(frame.area());

        let breadcrumb = self
            .stack
            .iter()
            .map(|screen| screen.title())
            .collect::<Vec<_>>()
            .join(" > ");
        frame.render_widget(
            Paragraph::new(Line::from(breadcrumb)).style(Style::default().fg(Color::Gray)),
            rows[0],
        );

        self.stack
            .last()
            .expect("navigation stack always has the Dashboard as its base")
            .view(frame, rows[1]);
    }
}

/// `Ctrl+C` — the one key that always quits immediately, regardless of the active screen or
/// input mode.
fn is_hard_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    // `App::new()` pushes the Dashboard, whose own `init()` now spawns a background Accounts
    // load via `tokio::spawn` (for its live snapshot) — every test that constructs an `App`
    // needs an active runtime, hence `#[tokio::test]` throughout this module.
    #[tokio::test]
    async fn renders_the_dashboard_without_panicking() {
        let app = App::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| app.draw(frame))
            .expect("drawing the dashboard should not error");
    }

    #[tokio::test]
    async fn esc_at_the_dashboard_quits() {
        let mut app = App::new();
        app.update(Action::Back);
        assert!(app.should_quit);
        assert_eq!(app.stack.len(), 1, "the Dashboard is never popped");
    }

    #[tokio::test]
    async fn opening_and_backing_out_of_settings_returns_to_the_dashboard() {
        let mut app = App::new();
        app.update(Action::OpenSettings);
        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Settings");

        app.update(Action::Back);
        assert_eq!(app.stack.len(), 1);
        assert!(!app.should_quit);
    }

    #[tokio::test]
    async fn help_does_not_stack_on_itself() {
        let mut app = App::new();
        app.update(Action::OpenHelp);
        app.update(Action::OpenHelp);
        assert_eq!(
            app.stack.len(),
            2,
            "a second OpenHelp while on Help is a no-op"
        );
    }

    #[test]
    fn ctrl_c_is_recognised_as_a_hard_quit() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_hard_quit(ctrl_c));

        let plain_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!is_hard_quit(plain_c));
    }

    #[tokio::test]
    async fn open_units_pushes_the_units_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenUnits);
        assert_eq!(app.stack.last().unwrap().title(), "Units");
    }

    #[tokio::test]
    async fn open_unit_detail_pushes_the_unit_screen() {
        let mut app = App::new();
        app.update(Action::OpenUnitDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Unit");
    }

    #[tokio::test]
    async fn unit_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenUnits);
        app.update(Action::OpenUnitDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let unit = lib_database::Units {
            id: lib_core::RowID::new(),
            code: "AUD".to_string(),
            name: "Australian Dollar".to_string(),
            unit_kind: lib_core::UnitKind::Fiat,
            decimal_places: 2,
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        app.update(Action::UnitSaved(unit));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Units");
    }

    #[tokio::test]
    async fn no_op_leaves_the_stack_untouched() {
        let mut app = App::new();
        app.update(Action::NoOp);
        assert_eq!(app.stack.len(), 1);
    }

    #[tokio::test]
    async fn open_accounts_pushes_the_accounts_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenAccounts);
        assert_eq!(app.stack.last().unwrap().title(), "Accounts");
    }

    #[tokio::test]
    async fn open_account_detail_pushes_the_account_screen() {
        let mut app = App::new();
        app.update(Action::OpenAccountDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Account");
    }

    #[tokio::test]
    async fn account_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenAccounts);
        app.update(Action::OpenAccountDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let account = lib_database::Accounts {
            id: lib_core::RowID::new(),
            name: "Everyday Spending".to_string(),
            account_type: lib_core::AccountType::Cash,
            unit_id: lib_core::RowID::new(),
            starting_balance: lib_core::Money::mock(),
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        app.update(Action::AccountSaved(account));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Accounts");
    }

    #[tokio::test]
    async fn open_transactions_pushes_the_transactions_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenTransactions);
        assert_eq!(app.stack.last().unwrap().title(), "Transactions");
    }

    #[tokio::test]
    async fn open_transaction_detail_pushes_the_transaction_screen() {
        let mut app = App::new();
        app.update(Action::OpenTransactionDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Transaction");
    }

    #[tokio::test]
    async fn transaction_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenTransactions);
        app.update(Action::OpenTransactionDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let transaction = lib_database::Transactions {
            id: lib_core::RowID::new(),
            date: now.date_naive(),
            amount: lib_core::Money::mock(),
            category_id: lib_core::RowID::new(),
            account_id: lib_core::RowID::new(),
            payee_id: None,
            description: None,
            status: lib_core::TransactionStatus::Open,
            is_flagged: false,
            updated_on: now,
        };
        app.update(Action::TransactionSaved(transaction));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Transactions");
    }

    #[tokio::test]
    async fn open_payees_pushes_the_payees_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenPayees);
        assert_eq!(app.stack.last().unwrap().title(), "Payees");
    }

    #[tokio::test]
    async fn open_payee_detail_pushes_the_payee_screen() {
        let mut app = App::new();
        app.update(Action::OpenPayeeDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Payee");
    }

    #[tokio::test]
    async fn payee_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenPayees);
        app.update(Action::OpenPayeeDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let payee = lib_database::Payees {
            id: lib_core::RowID::new(),
            name: "Woolworths".to_string(),
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        app.update(Action::PayeeSaved(payee));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Payees");
    }

    #[tokio::test]
    async fn open_balance_checks_pushes_the_balance_checks_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenBalanceChecks);
        assert_eq!(app.stack.last().unwrap().title(), "Balance Checks");
    }

    #[tokio::test]
    async fn open_balance_check_detail_pushes_the_balance_check_screen() {
        let mut app = App::new();
        app.update(Action::OpenBalanceCheckDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Balance Check");
    }

    #[tokio::test]
    async fn balance_check_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenBalanceChecks);
        app.update(Action::OpenBalanceCheckDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let balance_check = lib_database::BalanceChecks {
            id: lib_core::RowID::new(),
            account_id: lib_core::RowID::new(),
            date: now.date_naive(),
            asserted_balance: lib_core::Money::mock(),
            created_on: now,
            updated_on: now,
        };
        app.update(Action::BalanceCheckSaved(balance_check));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Balance Checks");
    }

    #[tokio::test]
    async fn open_budgets_pushes_the_budgets_list_screen() {
        let mut app = App::new();
        app.update(Action::OpenBudgets);
        assert_eq!(app.stack.last().unwrap().title(), "Budgets");
    }

    #[tokio::test]
    async fn open_budget_detail_pushes_the_budget_screen() {
        let mut app = App::new();
        app.update(Action::OpenBudgetDetail(None));
        assert_eq!(app.stack.last().unwrap().title(), "Budget");
    }

    #[tokio::test]
    async fn budget_saved_pops_back_from_the_detail_screen() {
        let mut app = App::new();
        app.update(Action::OpenBudgets);
        app.update(Action::OpenBudgetDetail(None));
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let budget = lib_database::Budgets {
            id: lib_core::RowID::new(),
            category_id: lib_core::RowID::new(),
            unit_id: lib_core::RowID::new(),
            limit_amount: lib_core::Money::mock(),
            period: lib_core::BudgetPeriod::Monthly,
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        app.update(Action::BudgetSaved(budget));

        assert_eq!(app.stack.len(), 2);
        assert_eq!(app.stack.last().unwrap().title(), "Budgets");
    }

    #[tokio::test]
    async fn open_csv_import_pushes_the_csv_import_screen() {
        let mut app = App::new();
        app.update(Action::OpenCsvImport);
        assert_eq!(app.stack.last().unwrap().title(), "Import Balance Checks");
    }

    #[tokio::test]
    async fn balance_checks_imported_does_not_pop_the_import_screen() {
        let mut app = App::new();
        app.update(Action::OpenBalanceChecks);
        app.update(Action::OpenCsvImport);
        assert_eq!(app.stack.len(), 3);

        let now = chrono::Utc::now();
        let balance_check = lib_database::BalanceChecks {
            id: lib_core::RowID::new(),
            account_id: lib_core::RowID::new(),
            date: now.date_naive(),
            asserted_balance: lib_core::Money::mock(),
            created_on: now,
            updated_on: now,
        };
        app.update(Action::BalanceChecksImported(vec![balance_check]));

        // Unlike a single-entity save, a CSV import stays on its screen so the user can see
        // the success message (or run another import) rather than being popped immediately.
        assert_eq!(app.stack.len(), 3);
        assert_eq!(app.stack.last().unwrap().title(), "Import Balance Checks");
    }

    #[tokio::test]
    async fn open_reports_pushes_the_reports_screen() {
        let mut app = App::new();
        app.update(Action::OpenReports);
        assert_eq!(app.stack.last().unwrap().title(), "Reports");
    }
}
