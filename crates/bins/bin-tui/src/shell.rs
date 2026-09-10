//! `Shell` — owns terminal lifecycle, the async event loop, and hosts one active `View`
//! (ADR-0013, `docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`). Replaces the
//! breadcrumb-stack `App` (`app.rs`, left compiling but disconnected from `main.rs`) for the
//! shell chrome and dashboard being rebuilt against `docs/ux/tui/README.md`: a status line,
//! one full-bleed view region, and a keybind hint bar — no breadcrumb, no navigation stack.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc;

use crate::{
    event::{Event, EventHandler},
    popup::{
        Dim,
        command::CommandPopup,
        unit::{UnitPopup, delete::DeleteUnitPopup, edit::EditUnitPopup, new::NewUnitPopup},
    },
    tui::Tui,
    view::{
        Action, View, accounts::AccountsView, balance_checks::BalanceChecksView,
        budgets::BudgetsView, categories::CategoriesView, dashboard::DashboardView, help::HelpView,
        payees::PayeesView, reports::ReportsView, transactions::TransactionsView, units::UnitsView,
    },
};

/// How often an [`Action::Tick`] fires in the absence of input.
const TICK_RATE: Duration = Duration::from_millis(250);

/// Owns terminal lifecycle and the single active `View`, and drives the async event loop.
pub struct Shell {
    view: Box<dyn View>,
    should_quit: bool,
    /// Where a view's `init()` (e.g. a background load) reports results back as an
    /// [`Action`].
    action_rx: mpsc::UnboundedReceiver<Action>,
    /// Kept so a view swap (e.g. [`Action::OpenUnits`]) can hand the freshly-hosted view its
    /// own clone, the same way `new()` hands one to the initial Dashboard view.
    action_tx: mpsc::UnboundedSender<Action>,
    /// The command popup overlay (`docs/ux/tui/README.md` §3a) — `Some` while open. Owned
    /// here rather than by the active `View`: it floats over whatever view is on screen and
    /// intercepts keys before the view sees them, per `view/mod.rs`'s "shell's own command
    /// window" note.
    command_popup: Option<CommandPopup>,
    /// Whichever unit-domain form (`docs/ux/tui/units/README.md` "The forms") is open —
    /// `Some` while one is. Owned here for the same reason as `command_popup`: it floats over
    /// whatever view is on screen and intercepts keys before the view sees them. Mutually
    /// exclusive with `command_popup` (opening one closes the other), and with itself — only
    /// one unit form is ever open at a time, hence the single `Option<UnitPopup>` rather than
    /// one field per form.
    unit_popup: Option<UnitPopup>,
    /// `true` after a lone `g` keypress with no completing chord yet — the leader half of the
    /// `g <letter>` jump chords in `docs/ux/tui/README.md`'s "Jumps" table (e.g. `g d`
    /// dashboard). Cleared by the very next key regardless of whether it completed a known
    /// chord, so an aborted chord never leaks into later keypresses.
    pending_leader: bool,
}

impl Shell {
    /// Creates the shell with the placeholder Dashboard as its active view.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut view: Box<dyn View> = Box::new(DashboardView::new());
        view.init(action_tx.clone());

        Self {
            view,
            should_quit: false,
            action_rx,
            action_tx,
            command_popup: None,
            unit_popup: None,
            pending_leader: false,
        }
    }

    /// Runs the shell until the user quits.
    pub async fn run(&mut self) -> crate::Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.draw(frame))?;

        loop {
            let action = tokio::select! {
                event = events.next() => match event {
                    // A resize can reveal rows the terminal emulator never had ratatui-drawn
                    // content in; clear before the next draw rather than risk stray artifacts.
                    Some(Event::Resize) => {
                        tui.clear()?;
                        continue;
                    }
                    Some(event) => match self.map_event(event) {
                        Some(action) => action,
                        None => continue,
                    },
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

    /// Translates a raw terminal event into an [`Action`]. Precedence: `Ctrl+C` always quits,
    /// even mid-chord; then, while the command popup or a unit form is open, it takes every
    /// other key over the active view (per §3a, the view behind it is inert while it's up);
    /// then a pending `g` leader consumes the very next key as its chord completion (or aborts
    /// silently if it doesn't complete one); otherwise `Ctrl+;` opens the command popup,
    /// `Ctrl+U` opens the placeholder Units view directly, `?` opens the placeholder Help
    /// view, `Q` quits, a lone `g` arms the leader, and anything left falls to the active
    /// view's own `handle_key` (which is how the Units view's own `n`/`e`/`d` reach
    /// [`Action::OpenNewUnitPopup`]/[`Action::OpenEditUnitPopup`]/
    /// [`Action::OpenDeleteUnitPopup`]). `Event::Resize` never reaches here — `run` intercepts
    /// it directly to clear the terminal, since that's a `Tui`-level concern with no `Action`
    /// of its own.
    fn map_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => {
                if is_hard_quit(key) {
                    return Some(Action::Quit);
                }
                if self.command_popup.is_some() {
                    return self.map_command_popup_key(key);
                }
                if self.unit_popup.is_some() {
                    return map_unit_popup_key(key);
                }
                if self.pending_leader {
                    self.pending_leader = false;
                    return match key.code {
                        KeyCode::Char('a') => Some(Action::OpenAccounts),
                        KeyCode::Char('b') => Some(Action::OpenBudgets),
                        KeyCode::Char('c') => Some(Action::OpenCategories),
                        KeyCode::Char('d') => Some(Action::OpenDashboard),
                        KeyCode::Char('k') => Some(Action::OpenBalanceChecks),
                        KeyCode::Char('p') => Some(Action::OpenPayees),
                        KeyCode::Char('r') => Some(Action::OpenReports),
                        KeyCode::Char('t') => Some(Action::OpenTransactions),
                        KeyCode::Char('u') => Some(Action::OpenUnits),
                        _ => None,
                    };
                }
                if is_open_command_popup(key) {
                    return Some(Action::OpenCommandPopup);
                }
                if is_open_units(key) {
                    return Some(Action::OpenUnits);
                }
                if is_open_help(key) {
                    return Some(Action::OpenHelp);
                }
                if is_quit(key) {
                    return Some(Action::Quit);
                }
                if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE {
                    self.pending_leader = true;
                    return None;
                }
                self.view.handle_key(key)
            }
            Event::Resize => None,
        }
    }

    /// Routes a key while the command popup is open. `Ctrl+;` toggles it shut again; `Esc`
    /// closes it; typing, `Backspace` and `↑`/`↓` drive the input buffer and selection.
    /// `Enter` runs the highlighted command if it's one of the domain "list" (or `unit`/
    /// `dashboard`/`help`/`quit`) commands with a real effect behind it; any other key (e.g.
    /// `Tab` — completion is the action-registry's "later ticket", per `view/mod.rs`) is
    /// swallowed without effect, since the popup owns every key while it's up.
    fn map_command_popup_key(&self, key: KeyEvent) -> Option<Action> {
        if is_open_command_popup(key) {
            return Some(Action::CloseCommandPopup);
        }
        match key.code {
            KeyCode::Esc => Some(Action::CloseCommandPopup),
            KeyCode::Up => Some(Action::CommandPopupMoveUp),
            KeyCode::Down => Some(Action::CommandPopupMoveDown),
            KeyCode::Backspace => Some(Action::CommandPopupBackspace),
            KeyCode::Enter => match self.command_popup.as_ref()?.selected_command_name() {
                Some("unit") => Some(Action::OpenUnits),
                Some("unit new <code> <type>") => Some(Action::OpenNewUnitPopup),
                Some("unit edit <code>") => Some(Action::OpenEditUnitPopup),
                Some("unit delete <code>") => Some(Action::OpenDeleteUnitPopup),
                Some("dashboard") => Some(Action::OpenDashboard),
                Some("account list") => Some(Action::OpenAccounts),
                Some("check list") => Some(Action::OpenBalanceChecks),
                Some("budget list [period]") => Some(Action::OpenBudgets),
                Some("category list") => Some(Action::OpenCategories),
                Some("help") => Some(Action::OpenHelp),
                Some("payee list") => Some(Action::OpenPayees),
                Some("quit") => Some(Action::Quit),
                Some("report list") => Some(Action::OpenReports),
                Some("txn recent") => Some(Action::OpenTransactions),
                _ => None,
            },
            KeyCode::Char(c) => Some(Action::CommandPopupInput(c)),
            _ => None,
        }
    }

    /// Applies an [`Action`] to shell state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Tick => self.view.update(&action),
            Action::OpenCommandPopup => self.command_popup = Some(CommandPopup::new()),
            Action::CloseCommandPopup => self.command_popup = None,
            Action::CommandPopupInput(c) => {
                if let Some(popup) = &mut self.command_popup {
                    popup.push_char(c);
                }
            }
            Action::CommandPopupBackspace => {
                if let Some(popup) = &mut self.command_popup {
                    popup.backspace();
                }
            }
            Action::CommandPopupMoveUp => {
                if let Some(popup) = &mut self.command_popup {
                    popup.move_up();
                }
            }
            Action::CommandPopupMoveDown => {
                if let Some(popup) = &mut self.command_popup {
                    popup.move_down();
                }
            }
            Action::OpenNewUnitPopup => {
                self.unit_popup = Some(UnitPopup::New(NewUnitPopup::new()));
                self.command_popup = None;
            }
            Action::OpenEditUnitPopup => {
                self.unit_popup = Some(UnitPopup::Edit(EditUnitPopup::new()));
                self.command_popup = None;
            }
            Action::OpenDeleteUnitPopup => {
                // Always the refused (§4d) variant for now — no reference-count resolution is
                // wired up yet to pick between it and `DeleteUnitPopup::allowed` (see
                // `popup::unit::delete`'s own module doc).
                self.unit_popup = Some(UnitPopup::Delete(DeleteUnitPopup::refused()));
                self.command_popup = None;
            }
            Action::CloseUnitPopup => self.unit_popup = None,
            Action::OpenUnits => self.open(UnitsView::new()),
            Action::OpenDashboard => self.open(DashboardView::new()),
            Action::OpenAccounts => self.open(AccountsView::new()),
            Action::OpenBalanceChecks => self.open(BalanceChecksView::new()),
            Action::OpenBudgets => self.open(BudgetsView::new()),
            Action::OpenCategories => self.open(CategoriesView::new()),
            Action::OpenHelp => self.open(HelpView::new()),
            Action::OpenPayees => self.open(PayeesView::new()),
            Action::OpenReports => self.open(ReportsView::new()),
            Action::OpenTransactions => self.open(TransactionsView::new()),
        }
    }

    /// Swaps the active view, hands it a fresh clone of `action_tx` (as `new()` does for the
    /// initial Dashboard), and closes both popups — the common tail of every `Open*` action.
    fn open<V: View + 'static>(&mut self, view: V) {
        let mut view: Box<dyn View> = Box::new(view);
        view.init(self.action_tx.clone());
        self.view = view;
        self.command_popup = None;
        self.unit_popup = None;
    }

    /// Renders the shell chrome — status line, full-bleed view region, a rule, then the
    /// keybind hint bar — around the active view, per `docs/ux/tui/README.md`.
    fn draw(&self, frame: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status line
                Constraint::Min(0),    // view
                Constraint::Length(1), // rule, separating the view from the footer
                Constraint::Length(1), // footer
            ])
            .split(frame.area());

        let command_popup_open = self.command_popup.is_some();
        let unit_popup_open = self.unit_popup.is_some();

        // Header Frame — the status line names the mode whenever it isn't the resting
        // NORMAL state, per `docs/ux/tui/README.md`'s "show the mode ... whenever it is not
        // NORMAL" — `COMMAND` for the command popup, `INSERT` for a unit form, per "Modal,
        // vim-flavoured ... INSERT only inside forms ... COMMAND while the palette is open".
        let mode = if command_popup_open {
            " · COMMAND"
        } else if unit_popup_open {
            " · INSERT"
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(Line::from(format!(
                " 📒 Personal Ledger | {}{mode} ",
                self.view.title()
            )))
            .style(Style::default().add_modifier(Modifier::REVERSED)),
            rows[0],
        );

        // Screen Frame / View
        self.view.view(frame, rows[1]);

        // Rule Frame — separates the view from the footer, replacing the footer's old
        // background fill as the visual boundary between them.
        frame.render_widget(Block::new().borders(Borders::TOP), rows[2]);

        // Footer Frame — each keybind's key is bolded to stand out from its label. While a
        // popup is open the whole bar greys out and gains its own close hint: §3a for the
        // command popup, "The forms" ("the app's footer greyed to `esc close unit form`") for
        // a unit form.
        let footer = if command_popup_open {
            Line::from(" : command · / search · ? help · esc close command window ")
                .style(Style::default().fg(Color::DarkGray))
        } else if unit_popup_open {
            Line::from(" esc close unit form ").style(Style::default().fg(Color::DarkGray))
        } else {
            let key = Style::default().add_modifier(Modifier::BOLD);
            Line::from(vec![
                Span::raw(" "),
                Span::styled(":", key),
                Span::raw(" command · "),
                Span::styled("/", key),
                Span::raw(" search · "),
                Span::styled("?", key),
                Span::raw(" help "),
            ])
        };
        frame.render_widget(Paragraph::new(footer), rows[3]);

        // Popup overlays — dim the view behind them (never hide it) and float over the whole
        // frame, per §3a. Mutually exclusive: only one is ever `Some` at a time.
        if let Some(popup) = &self.command_popup {
            frame.render_widget(Dim, rows[1]);
            popup.render(frame, frame.area());
        } else if let Some(popup) = &self.unit_popup {
            frame.render_widget(Dim, rows[1]);
            popup.render(frame, frame.area());
        }
    }
}

/// `Ctrl+C` — the one key that always quits immediately, regardless of the active view.
fn is_hard_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
}

/// `Ctrl+;` — opens the command popup from anywhere, shift optional. `Tui` requests
/// `DISAMBIGUATE_ESCAPE_CODES` so a Kitty-protocol terminal reports the unshifted key as
/// `Char(';')` regardless of whether `Shift` is also held (physically producing `:`); `':'`
/// is matched too as a defensive fallback for a terminal or layout that reports the shifted
/// symbol instead. Either way `Shift`'s presence is ignored.
fn is_open_command_popup(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(';') | KeyCode::Char(':'))
}

/// `Ctrl+U` — opens the placeholder Units view directly, alongside the command popup's own
/// `unit` / `g u` route (`docs/ux/tui/units/README.md`).
fn is_open_units(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('u'))
}

/// `?` — opens the placeholder Help view, matching the footer's own `? help` hint and the
/// popup's `help` command (`docs/ux/tui/README.md`).
fn is_open_help(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('?'))
}

/// `Q` — quits the app from anywhere the command popup/unit forms aren't intercepting keys,
/// matching the footer's `docs/ux/tui/README.md` global keybind table (`Q` quit, distinct from
/// its still-unbuilt lowercase `q` "close view" sibling) and the popup's own `quit` command.
/// Unlike [`is_hard_quit`] (`Ctrl+C`), this doesn't fire mid-chord or while a popup is open —
/// there, `Q` is ordinary filter/chord input instead.
fn is_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('Q'))
}

/// Routes a key while a unit form (new, edit or delete) is open. Only `Esc` does anything yet — no
/// field is editable until each form's fields land (`docs/ux/tui/units/README.md` §4b/§4c);
/// every other key is swallowed, since the popup owns every key while it's up (mirroring
/// `Shell::map_command_popup_key`). A free function rather than a method — unlike the command
/// popup, there's no draft state yet to consult, and `Esc` behaves the same regardless of
/// which form is open.
fn map_unit_popup_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::CloseUnitPopup),
        _ => None,
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_the_shell_layout_without_panicking() {
        let shell = Shell::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell should not error");
    }

    #[test]
    fn footer_has_no_background_and_a_rule_separates_it_from_the_view() {
        let shell = Shell::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell should not error");

        let buffer = terminal.backend().buffer();
        let last = buffer.area.height - 1;
        let rule_row = last - 1;

        assert!(
            (0..buffer.area.width).all(|x| buffer[(x, last)].bg == Color::Reset),
            "footer row should carry no background fill"
        );
        assert!(
            (0..buffer.area.width).all(|x| buffer[(x, rule_row)].symbol() == "─"),
            "a rule should separate the view from the footer"
        );
    }

    #[test]
    fn ctrl_c_is_recognised_as_a_hard_quit() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_hard_quit(ctrl_c));

        let plain_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!is_hard_quit(plain_c));
    }

    #[test]
    fn ctrl_c_quits_the_shell() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+c always maps to an action");
        shell.update(action);
        assert!(shell.should_quit);
    }

    #[test]
    fn tick_reaches_the_active_view() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Tick)
            .expect("a tick always maps to an action");
        assert_eq!(action, Action::Tick);
        shell.update(action);
        assert!(!shell.should_quit);
    }

    #[test]
    fn resize_maps_to_no_action() {
        // `run` intercepts `Event::Resize` directly to clear the terminal, before it would
        // ever reach `map_event` — this just documents that `map_event` itself treats it as
        // unmapped, keeping the match exhaustive without inventing a `Resize` action.
        let mut shell = Shell::new();
        assert_eq!(shell.map_event(Event::Resize), None);
    }

    #[test]
    fn ctrl_semicolon_is_recognised_as_open_command_popup() {
        let ctrl_semicolon = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::CONTROL);
        assert!(is_open_command_popup(ctrl_semicolon));

        let plain_semicolon = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::NONE);
        assert!(!is_open_command_popup(plain_semicolon));
    }

    #[test]
    fn ctrl_semicolon_with_shift_held_is_also_recognised_as_open_command_popup() {
        // Whether `Shift` is also held (physically producing `:` rather than `;`) doesn't
        // matter — see `is_open_command_popup`'s doc.
        let ctrl_shift_semicolon = KeyEvent::new(
            KeyCode::Char(';'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert!(is_open_command_popup(ctrl_shift_semicolon));

        let ctrl_colon = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::CONTROL);
        assert!(is_open_command_popup(ctrl_colon));
    }

    #[test]
    fn ctrl_colon_opens_the_command_popup() {
        let mut shell = Shell::new();
        assert!(shell.command_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char(':'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+: always maps to an action");
        shell.update(action);

        assert!(shell.command_popup.is_some());
    }

    #[test]
    fn ctrl_colon_again_closes_an_open_command_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char(':'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+: while open always maps to an action");
        shell.update(action);

        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn esc_closes_an_open_command_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .expect("esc while open always maps to an action");
        shell.update(action);

        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn typing_while_the_command_popup_is_open_never_reaches_the_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::CommandPopupInput('b')));
    }

    #[test]
    fn ctrl_c_still_quits_while_the_command_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+c always maps to an action");
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn ctrl_u_is_recognised_as_open_units() {
        let ctrl_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert!(is_open_units(ctrl_u));

        let plain_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
        assert!(!is_open_units(plain_u));
    }

    #[test]
    fn ctrl_u_opens_the_units_view() {
        let mut shell = Shell::new();
        assert_eq!(shell.view.title(), "Dashboard");

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+u always maps to an action while no popup is open");
        shell.update(action);

        assert_eq!(shell.view.title(), "Units & Prices");
    }

    #[test]
    fn selecting_the_unit_command_and_pressing_enter_opens_the_units_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "unit".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `unit` command always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Units & Prices");
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn enter_on_a_command_with_no_real_view_yet_is_still_swallowed() {
        // Filtered down to `report account-balance` — a specific report, reached only via the
        // Reports screen's own picker (`popup/command/commands/reports.rs`) — which has no
        // dispatch of its own even though `report list` does. `Enter` should still be a no-op.
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "account-balance".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None);
    }

    #[test]
    fn selecting_the_dashboard_command_and_pressing_enter_opens_the_dashboard_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert_eq!(shell.view.title(), "Units & Prices");

        shell.update(Action::OpenCommandPopup);
        for c in "dashboard".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `dashboard` command always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Dashboard");
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn g_then_d_opens_the_dashboard_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert_eq!(shell.view.title(), "Units & Prices");

        let armed = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::NONE,
        )));
        assert_eq!(armed, None, "a lone `g` arms the leader without an action");
        assert!(shell.pending_leader);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::NONE,
            )))
            .expect("`d` completing the `g d` chord always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Dashboard");
        assert!(!shell.pending_leader);
    }

    #[test]
    fn g_then_an_unknown_letter_aborts_the_chord_without_an_action() {
        let mut shell = Shell::new();
        let _ = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::NONE,
        )));
        assert!(shell.pending_leader);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None);
        assert!(!shell.pending_leader);
    }

    #[test]
    fn ctrl_c_still_quits_mid_chord() {
        let mut shell = Shell::new();
        let _ = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::NONE,
        )));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+c always maps to an action, even mid-chord");
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn g_then_letter_opens_the_matching_view() {
        let cases: &[(char, &str)] = &[
            ('a', "Accounts"),
            ('b', "Budgets"),
            ('c', "Categories"),
            ('d', "Dashboard"),
            ('k', "Balance Checks"),
            ('p', "Payees"),
            ('r', "Reports"),
            ('t', "Transactions"),
            ('u', "Units & Prices"),
        ];

        for (letter, expected_title) in cases {
            let mut shell = Shell::new();
            let armed = shell.map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('g'),
                KeyModifiers::NONE,
            )));
            assert_eq!(armed, None, "g {letter}: a lone `g` should arm the leader");

            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(*letter),
                    KeyModifiers::NONE,
                )))
                .unwrap_or_else(|| panic!("g {letter} should complete a known chord"));
            shell.update(action);

            assert_eq!(shell.view.title(), *expected_title, "g {letter}");
            assert!(!shell.pending_leader, "g {letter}");
        }
    }

    #[test]
    fn question_mark_opens_the_help_view() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('?'),
                KeyModifiers::NONE,
            )))
            .expect("? always maps to an action while no popup is open");
        shell.update(action);

        assert_eq!(shell.view.title(), "Help");
    }

    #[test]
    fn shift_q_quits_the_shell() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('Q'),
                KeyModifiers::NONE,
            )))
            .expect("Q always maps to an action while no popup is open");
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn shift_q_is_swallowed_while_the_command_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('Q'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::CommandPopupInput('Q')));
        assert!(!shell.should_quit);
    }

    #[test]
    fn selecting_the_quit_command_and_pressing_enter_quits_the_shell() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "quit".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `quit` command always maps to an action");
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn question_mark_is_swallowed_while_the_command_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('?'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::CommandPopupInput('?')));
    }

    #[test]
    fn selecting_each_domain_list_command_and_pressing_enter_opens_its_view() {
        let cases: &[(&str, &str)] = &[
            ("account list", "Accounts"),
            ("check list", "Balance Checks"),
            ("budget list", "Budgets"),
            ("category list", "Categories"),
            ("help", "Help"),
            ("payee list", "Payees"),
            ("report list", "Reports"),
            ("txn recent", "Transactions"),
        ];

        for (filter, expected_title) in cases {
            let mut shell = Shell::new();
            shell.update(Action::OpenCommandPopup);
            for c in filter.chars() {
                let action = shell
                    .map_event(Event::Key(KeyEvent::new(
                        KeyCode::Char(c),
                        KeyModifiers::NONE,
                    )))
                    .expect("typing a filter character always maps to an action");
                shell.update(action);
            }

            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                )))
                .unwrap_or_else(|| panic!("enter on `{filter}` should map to an action"));
            shell.update(action);

            assert_eq!(shell.view.title(), *expected_title, "{filter}");
            assert!(shell.command_popup.is_none(), "{filter}");
        }
    }

    #[test]
    fn renders_the_open_command_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the popup open should not error");
    }

    #[test]
    fn n_on_the_units_view_opens_the_new_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert!(shell.unit_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::NONE,
            )))
            .expect("n on the units view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::New(_))));
    }

    #[test]
    fn e_on_the_units_view_opens_the_edit_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert!(shell.unit_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on the units view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Edit(_))));
    }

    #[test]
    fn d_on_the_units_view_opens_the_delete_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert!(shell.unit_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::NONE,
            )))
            .expect("d on the units view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Delete(_))));
    }

    #[test]
    fn esc_closes_an_open_unit_popup() {
        for open in [
            Action::OpenNewUnitPopup,
            Action::OpenEditUnitPopup,
            Action::OpenDeleteUnitPopup,
        ] {
            let mut shell = Shell::new();
            shell.update(open);

            let action = shell
                .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
                .expect("esc while open always maps to an action");
            shell.update(action);

            assert!(shell.unit_popup.is_none());
        }
    }

    #[test]
    fn other_keys_are_swallowed_while_a_unit_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenNewUnitPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None);
    }

    #[test]
    fn opening_a_unit_popup_closes_an_open_command_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        assert!(shell.command_popup.is_some());

        shell.update(Action::OpenNewUnitPopup);

        assert!(shell.unit_popup.is_some());
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn opening_a_unit_popup_replaces_any_other_open_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenNewUnitPopup);
        assert!(matches!(shell.unit_popup, Some(UnitPopup::New(_))));

        shell.update(Action::OpenEditUnitPopup);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Edit(_))));

        shell.update(Action::OpenDeleteUnitPopup);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Delete(_))));
    }

    #[test]
    fn selecting_the_unit_new_command_and_pressing_enter_opens_the_new_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "unit new".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `unit new` command always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::New(_))));
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn selecting_the_unit_edit_command_and_pressing_enter_opens_the_edit_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "unit edit".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `unit edit` command always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Edit(_))));
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn selecting_the_unit_delete_command_and_pressing_enter_opens_the_delete_unit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "unit delete".chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the `unit delete` command always maps to an action");
        shell.update(action);

        assert!(matches!(shell.unit_popup, Some(UnitPopup::Delete(_))));
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn renders_the_open_new_unit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenNewUnitPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the new unit popup open should not error");
    }

    #[test]
    fn renders_the_open_edit_unit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenEditUnitPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the edit unit popup open should not error");
    }

    #[test]
    fn renders_the_open_delete_unit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenDeleteUnitPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the delete unit popup open should not error");
    }
}
