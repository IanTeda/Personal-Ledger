//! The per-view model each `View` implements, hosted by `Shell` — the new navigation shape
//! ADR-0013 (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in
//! place of the breadcrumb `App`/`Screen` stack (`app.rs`/`screen/`, left compiling but
//! disconnected from `main.rs`). `Shell` hosts exactly one active `View` at a time — no
//! navigation stack, no breadcrumb; the shell's own command window (a later ticket) is the
//! only way the design specifies for switching which `View` is active.

pub mod accounts;
pub mod balance_checks;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod help;
pub mod payees;
pub mod reports;
pub mod transactions;
pub mod units;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

/// A message `Shell` or the active `View` reacts to. Deliberately minimal for now — the full
/// action registry the command window will dispatch through (ADR-0013) is later work; this
/// only needs enough to drive the event loop and let a `View` react to a tick.
///
/// The `CommandPopup*` variants drive `Shell`'s own command window (`crate::popup::command`)
/// rather than the active `View` — they exist here because `Shell`'s event loop only redraws
/// in response to an `Action`, and every `View::update` implementation ignores variants it
/// doesn't care about, so adding shell-chrome messages alongside `Quit`/`Tick` costs a `View`
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input.
    Tick,
    /// `Ctrl+C` — the hard-quit safety net, recognised by `Shell` itself before any `View`
    /// sees the key.
    Quit,
    /// `Ctrl+;` — opens the command popup overlay.
    OpenCommandPopup,
    /// `Esc`, or `Ctrl+;` again — closes the command popup overlay.
    CloseCommandPopup,
    /// A printable character typed while the command popup is open, appended to its input
    /// buffer.
    CommandPopupInput(char),
    /// `Backspace` while the command popup is open — removes the last character of its input.
    CommandPopupBackspace,
    /// `↑` while the command popup is open — moves the selection up one candidate row.
    CommandPopupMoveUp,
    /// `↓` while the command popup is open — moves the selection down one candidate row.
    CommandPopupMoveDown,
    /// `Ctrl+U`, or `Enter` on the command popup's `unit` command — opens the placeholder
    /// Units view (`docs/ux/tui/units/README.md`), the first domain to land a real (if still
    /// wireframe-stage) destination behind the command popup's `unit` entry.
    OpenUnits,
    /// `g d` (the `docs/ux/tui/README.md` "Jumps" table's chord), or `Enter` on the command
    /// popup's `dashboard` command — returns to the Dashboard view.
    OpenDashboard,
    /// `g a`, or `Enter` on the command popup's `account list` command — opens the
    /// placeholder Accounts view.
    OpenAccounts,
    /// `g k`, or `Enter` on the command popup's `check list` command — opens the placeholder
    /// Balance Checks view.
    OpenBalanceChecks,
    /// `g b`, or `Enter` on the command popup's `budget list [period]` command — opens the
    /// placeholder Budgets view.
    OpenBudgets,
    /// `g c`, or `Enter` on the command popup's `category list` command — opens the
    /// placeholder Categories view.
    OpenCategories,
    /// `?`, or `Enter` on the command popup's `help` command — opens the placeholder Help
    /// view.
    OpenHelp,
    /// `g p`, or `Enter` on the command popup's `payee list` command — opens the placeholder
    /// Payees view.
    OpenPayees,
    /// `g r`, or `Enter` on the command popup's `report list` command — opens the placeholder
    /// Reports view.
    OpenReports,
    /// `g t`, or `Enter` on the command popup's `txn recent` command — opens the placeholder
    /// Transactions view.
    OpenTransactions,
}

/// The single view `Shell` hosts at a time.
pub trait View {
    /// Called once when the view becomes active, with a sender any background work (e.g. a
    /// data load) can use to report results back as an [`Action`]. Most views have no
    /// background work and can rely on this default no-op.
    fn init(&mut self, _action_tx: UnboundedSender<Action>) {}

    /// First refusal on a raw key press while this view is active: interpret it as an
    /// [`Action`], or return `None` to fall through to `Shell`'s own global keys (`Ctrl+C`).
    /// The default no-op means a view with nothing view-specific to bind just gets the
    /// global keys.
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    /// Reacts to an [`Action`] that wasn't handled at the `Shell` level.
    fn update(&mut self, action: &Action);

    /// Renders the view into the given area of the frame — the full-bleed view region below
    /// the status line and above the command line.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Short name for the view, shown in the shell's status line.
    fn title(&self) -> &'static str;
}
