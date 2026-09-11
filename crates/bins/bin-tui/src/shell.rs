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
        category::{
            CategoryPopup, edit_popup::EditPopup, move_popup::MovePopup, new_popup::NewPopup,
        },
        command::CommandPopup,
        unit::{UnitPopup, delete::DeleteUnitPopup, edit::EditUnitPopup, new::NewUnitPopup},
    },
    tui::Tui,
    view::{
        Action, View, accounts::AccountsView, balance_checks::BalanceChecksView,
        budgets::BudgetsView, categories::CategoriesView, dashboard::DashboardView, help::HelpView,
        payees::PayeesView, reports::ReportsView, settings::SettingsView,
        transactions::TransactionsView, units::UnitsView,
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
    /// The Category-domain popup (`crate::popup::category`) — `Some` while one is open. Owned
    /// here, mirroring `unit_popup`, per "Categories: 5b move popup"'s own instruction to
    /// follow `popup::unit`'s structure — even though the tree it acts on lives inside
    /// `CategoriesView`, not here; see `popup::category::move_popup`'s module doc for how it
    /// still reaches it. Mutually exclusive with `command_popup`/`unit_popup`.
    category_popup: Option<CategoryPopup>,
    /// `true` after a lone `g` keypress with no completing chord yet — the leader half of the
    /// `g <letter>` jump chords in `docs/ux/tui/README.md`'s "Jumps" table (e.g. `g d`
    /// dashboard). Cleared by the very next key regardless of whether it completed a known
    /// chord, so an aborted chord never leaks into later keypresses.
    pending_leader: bool,
    /// Every view displaced by an `Open*` action, most-recently-displaced last — `Esc` (when
    /// no popup is open) pops one off and makes it active again. Opening Dashboard clears
    /// this entirely rather than pushing onto it (it's the app's one home view); re-opening
    /// the already-active view leaves it untouched (`Shell::open`).
    view_stack: Vec<Box<dyn View>>,
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
            category_popup: None,
            pending_leader: false,
            view_stack: Vec::new(),
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
    /// even mid-chord; then, while the command popup, a unit form, or the Category popup is
    /// open, it takes every other key over the active view (per §3a, the view behind it is
    /// inert while it's up); then a pending `g` leader consumes the very next key as its chord
    /// completion (or aborts
    /// silently if it doesn't complete one); otherwise `Ctrl+;` opens the command popup,
    /// `Ctrl+U` opens the placeholder Units view directly, `?` opens the placeholder Help
    /// view, `Esc` pops the view-navigation stack ([`Action::PopView`]), `Q` (shift) quits,
    /// `q` (lowercase) also quits via its own [`Action::GracefulQuit`], a lone `g` arms the
    /// leader, and anything left falls to the active view's own `handle_key` (which is how the
    /// Units view's own `n`/`e`/`d` reach [`Action::OpenNewUnitPopup`]/
    /// [`Action::OpenEditUnitPopup`]/[`Action::OpenDeleteUnitPopup`]). `Event::Resize` never
    /// reaches here — `run` intercepts it directly to clear the terminal, since that's a
    /// `Tui`-level concern with no `Action` of its own.
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
                if self.category_popup.is_some() {
                    return self.map_category_popup_key(key);
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
                        KeyCode::Char('s') => Some(Action::OpenSettings),
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
                if key.code == KeyCode::Esc {
                    return Some(Action::PopView);
                }
                if is_quit(key) {
                    return Some(Action::Quit);
                }
                if is_graceful_quit(key) {
                    return Some(Action::GracefulQuit);
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
    /// closes it; typing, `Backspace` and `↑`/`↓` drive the input buffer and selection; `Tab`
    /// clears a showing "not yet built" message (completion itself isn't built yet). `Enter`
    /// runs the highlighted command if it's one of the 6 with real content behind them
    /// (`unit`, `unit new/edit/delete`, `dashboard`, `settings`); on any other command it
    /// shows [`Action::CommandPopupSetNotYetBuilt`] instead — the popup stays open either way.
    fn map_command_popup_key(&self, key: KeyEvent) -> Option<Action> {
        if is_open_command_popup(key) {
            return Some(Action::CloseCommandPopup);
        }
        match key.code {
            KeyCode::Esc => Some(Action::CloseCommandPopup),
            KeyCode::Up => Some(Action::CommandPopupMoveUp),
            KeyCode::Down => Some(Action::CommandPopupMoveDown),
            KeyCode::Backspace => Some(Action::CommandPopupBackspace),
            KeyCode::Tab => Some(Action::CommandPopupTab),
            KeyCode::Enter => match self.command_popup.as_ref()?.selected_command_name() {
                Some("unit") => Some(Action::OpenUnits),
                Some("unit new <code> <type>") => Some(Action::OpenNewUnitPopup),
                Some("unit edit <code>") => Some(Action::OpenEditUnitPopup),
                Some("unit delete <code>") => Some(Action::OpenDeleteUnitPopup),
                Some("dashboard") => Some(Action::OpenDashboard),
                Some("settings") => Some(Action::OpenSettings),
                Some("cat") => Some(Action::OpenCategories),
                // The command popup has no real typed-argument resolution (`popup::command::
                // commands::categories`'s own module doc), so `<cat>`/`<parent>`/`<name>` all
                // mean "the Categories tree's current selection" here — the same target its
                // own `n`/`e`/`m`/`a` keys act on. Falls back to the ordinary "not yet built"
                // message when there isn't one (Categories isn't the active view), rather than
                // silently doing nothing.
                Some(name @ "cat new <name> [parent]") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryNewPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "cat edit <cat>") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryEditPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "cat move <cat> <parent>") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryMovePopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "cat archive <cat>") => self
                    .view
                    .category_selection()
                    .and_then(|id| {
                        let node = self.view.category_store()?.find(id)?;
                        Some(Action::UpdateCategory {
                            id,
                            name: node.name.clone(),
                            note: node.note.clone(),
                            active: false,
                        })
                    })
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name) => Some(Action::CommandPopupSetNotYetBuilt(name)),
                None => None,
            },
            KeyCode::Char(c) => Some(Action::CommandPopupInput(c)),
            _ => None,
        }
    }

    /// Routes a key while a Category popup is open, resolving it into a concrete `Action`
    /// right here against the active view's `CategoryStore` (`View::category_store`) rather
    /// than carrying raw typed text any further — see `popup::category::move_popup`'s module
    /// doc for why. `Esc` closes either popup; otherwise the two have different field sets, so
    /// each gets its own arm. A create/move that doesn't yet validate resolves to `None` (a
    /// no-op) rather than a doomed `Action`.
    fn map_category_popup_key(&self, key: KeyEvent) -> Option<Action> {
        let store = self.view.category_store()?;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match &self.category_popup {
            Some(CategoryPopup::Move(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseCategoryPopup),
                KeyCode::Backspace => Some(Action::CategoryMovePopupBackspace),
                KeyCode::Tab => Some(Action::CategoryMovePopupTab),
                KeyCode::Char('n') if ctrl => popup
                    .creatable(store)
                    .map(|(parent, name)| Action::CreateCategoryChild { parent, name }),
                KeyCode::Char('s') if ctrl => popup
                    .resolved_parent(store)
                    .filter(|new_parent| {
                        store.validate_move(popup.moving_id(), *new_parent).is_ok()
                    })
                    .map(|new_parent| Action::MoveCategory {
                        id: popup.moving_id(),
                        new_parent,
                    }),
                KeyCode::Char(c) if !ctrl => Some(Action::CategoryMovePopupInput(c)),
                _ => None,
            },
            Some(CategoryPopup::New(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseCategoryPopup),
                KeyCode::Backspace => Some(Action::CategoryNewPopupBackspace),
                KeyCode::Tab => Some(Action::CategoryNewPopupTab),
                KeyCode::Char('s') if ctrl => {
                    popup
                        .create_fields(store)
                        .map(|(parent, name, note, active)| Action::CreateCategory {
                            parent,
                            name,
                            note,
                            active,
                            close_after: true,
                        })
                }
                KeyCode::Char('a') if ctrl => {
                    popup
                        .create_fields(store)
                        .map(|(parent, name, note, active)| Action::CreateCategory {
                            parent,
                            name,
                            note,
                            active,
                            close_after: false,
                        })
                }
                KeyCode::Char(c) if !ctrl => Some(Action::CategoryNewPopupInput(c)),
                _ => None,
            },
            Some(CategoryPopup::Edit(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseCategoryPopup),
                KeyCode::Backspace => Some(Action::CategoryEditPopupBackspace),
                KeyCode::Tab => Some(Action::CategoryEditPopupTab),
                KeyCode::Char('s') if ctrl => {
                    popup.save_fields(store).map(|(id, name, note, active)| {
                        Action::UpdateCategory {
                            id,
                            name,
                            note,
                            active,
                        }
                    })
                }
                KeyCode::Char('a') if ctrl => {
                    popup.archive_fields(store).map(|(id, name, note, active)| {
                        Action::UpdateCategory {
                            id,
                            name,
                            note,
                            active,
                        }
                    })
                }
                // `X` (no `Ctrl`, matching the handoff's own bare key) — merge stub.
                KeyCode::Char('X') if !ctrl => Some(Action::CategoryMerge),
                KeyCode::Char(c) if !ctrl => Some(Action::CategoryEditPopupInput(c)),
                _ => None,
            },
            None => None,
        }
    }

    /// Applies an [`Action`] to shell state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::GracefulQuit => self.should_quit = true,
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
            Action::CommandPopupTab => {
                if let Some(popup) = &mut self.command_popup {
                    popup.tab();
                }
            }
            Action::CommandPopupSetNotYetBuilt(name) => {
                if let Some(popup) = &mut self.command_popup {
                    popup.set_not_yet_built(name);
                }
            }
            Action::PopView => {
                if let Some(previous) = self.view_stack.pop() {
                    self.view = previous;
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
            Action::OpenSettings => self.open(SettingsView::new()),
            Action::OpenTransactions => self.open(TransactionsView::new()),
            Action::NoOp => {}
            Action::OpenCategoryMovePopup(id) => {
                if let Some(store) = self.view.category_store() {
                    self.category_popup = Some(CategoryPopup::Move(MovePopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
            }
            Action::CloseCategoryPopup => self.category_popup = None,
            Action::CategoryMovePopupInput(c) => {
                if let Some(CategoryPopup::Move(popup)) = &mut self.category_popup {
                    popup.push_char(c);
                }
            }
            Action::CategoryMovePopupBackspace => {
                if let Some(CategoryPopup::Move(popup)) = &mut self.category_popup {
                    popup.backspace();
                }
            }
            Action::CategoryMovePopupTab => {
                if let Some(store) = self.view.category_store()
                    && let Some(CategoryPopup::Move(popup)) = &mut self.category_popup
                {
                    popup.tab_complete(store);
                }
            }
            // `MoveCategory`/`CreateCategoryChild`/`CreateCategory` were already validated in
            // `map_category_popup_key` against the store `Shell` itself has no other access
            // to — relaying to the active `View`'s own `update` is where the mutation actually
            // happens (`CategoriesView::update`, "Categories: fixture data seam and mutable
            // View state pattern"'s state-ownership decision).
            Action::MoveCategory { .. } => {
                self.view.update(&action);
                self.category_popup = None;
            }
            Action::CreateCategoryChild { .. } => self.view.update(&action),
            Action::OpenCategoryNewPopup(id) => {
                if let Some(store) = self.view.category_store() {
                    self.category_popup = Some(CategoryPopup::New(NewPopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
            }
            Action::CategoryNewPopupInput(c) => {
                if let Some(CategoryPopup::New(popup)) = &mut self.category_popup {
                    popup.push_char(c);
                }
            }
            Action::CategoryNewPopupBackspace => {
                if let Some(CategoryPopup::New(popup)) = &mut self.category_popup {
                    popup.backspace();
                }
            }
            Action::CategoryNewPopupTab => {
                if let Some(store) = self.view.category_store()
                    && let Some(CategoryPopup::New(popup)) = &mut self.category_popup
                {
                    popup.tab(store);
                }
            }
            Action::CreateCategory { close_after, .. } => {
                self.view.update(&action);
                if close_after {
                    self.category_popup = None;
                } else if let Some(CategoryPopup::New(popup)) = &mut self.category_popup {
                    popup.reset_for_next_sibling();
                }
            }
            Action::OpenCategoryEditPopup(id) => {
                if let Some(store) = self.view.category_store() {
                    self.category_popup = Some(CategoryPopup::Edit(EditPopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
            }
            Action::CategoryEditPopupInput(c) => {
                if let Some(CategoryPopup::Edit(popup)) = &mut self.category_popup {
                    popup.push_char(c);
                }
            }
            Action::CategoryEditPopupBackspace => {
                if let Some(CategoryPopup::Edit(popup)) = &mut self.category_popup {
                    popup.backspace();
                }
            }
            Action::CategoryEditPopupTab => {
                if let Some(CategoryPopup::Edit(popup)) = &mut self.category_popup {
                    popup.tab();
                }
            }
            Action::CategoryMerge => {
                if let Some(CategoryPopup::Edit(popup)) = &mut self.category_popup {
                    popup.trigger_not_yet_built();
                }
                // The tree screen's own bare `X` never reaches `Shell` at all — `CategoriesView
                // ::handle_key` sets its `merge_hint` directly and returns `Action::NoOp`
                // instead, since it already owns the state that needs to change. This arm only
                // exists for the Edit popup's `X`, which `Shell` (not `CategoriesView`) owns.
            }
            Action::UpdateCategory { .. } => {
                self.view.update(&action);
                self.category_popup = None;
            }
        }
    }

    /// Swaps the active view, hands it a fresh clone of `action_tx` (as `new()` does for the
    /// initial Dashboard), and closes both popups — the common tail of every `Open*` action.
    /// Also drives `view_stack`: opening the already-active view (compared by `View::title()`)
    /// is a no-op for the stack and the view itself; opening Dashboard clears the stack
    /// entirely, since it's the app's one home view and going there always resets navigation;
    /// opening anything else pushes the outgoing view onto the stack first, so `Esc`
    /// ([`Action::PopView`]) can return to it.
    fn open<V: View + 'static>(&mut self, view: V) {
        let mut view: Box<dyn View> = Box::new(view);

        if view.title() == self.view.title() {
            self.command_popup = None;
            self.unit_popup = None;
            self.category_popup = None;
            return;
        }

        view.init(self.action_tx.clone());

        if view.title() == "Dashboard" {
            self.view_stack.clear();
            self.view = view;
        } else {
            let previous = std::mem::replace(&mut self.view, view);
            self.view_stack.push(previous);
        }

        self.command_popup = None;
        self.unit_popup = None;
        self.category_popup = None;
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
        let category_popup_open = self.category_popup.is_some();

        // Header Frame — the status line names the mode whenever it isn't the resting
        // NORMAL state, per `docs/ux/tui/README.md`'s "show the mode ... whenever it is not
        // NORMAL" — `COMMAND` for the command popup, `INSERT` for a unit form or the Category
        // popup (it has a text field too), per "Modal, vim-flavoured ... INSERT only inside
        // forms ... COMMAND while the palette is open".
        let mode = if command_popup_open {
            " · COMMAND"
        } else if unit_popup_open || category_popup_open {
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
        } else if category_popup_open {
            // Generic across Move/New (and, later, Edit) — mirrors `unit_popup`'s own footer,
            // which likewise doesn't tailor its wording per form variant.
            Line::from(" esc close category form ").style(Style::default().fg(Color::DarkGray))
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
        } else if let Some(popup) = &self.category_popup {
            frame.render_widget(Dim, rows[1]);
            if let Some(store) = self.view.category_store() {
                popup.render(frame, frame.area(), store);
            }
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

/// `q` (lowercase) — quits the app from anywhere the command popup/unit forms aren't
/// intercepting keys, identically to [`is_quit`]'s `Q` today, but through its own
/// [`Action::GracefulQuit`] so a future confirm-before-quit check only needs to change that
/// one match arm. Distinct from [`is_quit`] since crossterm reports `Shift+q` as `Char('Q')`
/// regardless of which one physically fired.
fn is_graceful_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
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
    fn enter_on_a_command_with_no_real_view_yet_shows_a_not_yet_built_message() {
        // Filtered down to `report account-balance` — a specific report, reached only via the
        // Reports screen's own picker (`popup/command/commands/reports.rs`) — which has no
        // dispatch of its own even though `report list` does. `Enter` should show the
        // "not yet built" message rather than open a view, and the popup should stay open.
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

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on an unbuilt command still maps to an action");
        assert_eq!(
            action,
            Action::CommandPopupSetNotYetBuilt("report account-balance")
        );
        shell.update(action);

        assert!(shell.command_popup.is_some(), "popup should stay open");
        assert_eq!(shell.view.title(), "Dashboard", "no view should open");
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
            ('s', "Settings"),
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
    fn selecting_the_quit_command_and_pressing_enter_shows_a_not_yet_built_message() {
        // `quit` isn't one of the 6 commands with real content behind them — Enter on it
        // shows the "not yet built" message rather than quitting; the app still quits via the
        // global `Q`/`q` keys or `Ctrl+C`, just not through the command popup.
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
            .expect("enter on `quit` still maps to an action");
        shell.update(action);

        assert!(!shell.should_quit);
        assert!(shell.command_popup.is_some(), "popup should stay open");
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
    fn selecting_each_no_longer_dispatched_list_command_and_pressing_enter_shows_not_yet_built() {
        // These used to navigate to an empty placeholder box on `Enter`; per this ticket they
        // now show the "not yet built" message like every other undispatched command, and the
        // popup stays open rather than navigating anywhere. `category list` (now `cat`) is no
        // longer one of them — "Categories: :cat command grammar" gave it real dispatch.
        let cases: &[&str] = &[
            "account list",
            "check list",
            "budget list",
            "help",
            "payee list",
            "report list",
            "txn recent",
        ];

        for filter in cases {
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

            assert_eq!(shell.view.title(), "Dashboard", "{filter}");
            assert!(shell.command_popup.is_some(), "{filter}");
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

    #[test]
    fn selecting_the_settings_command_and_pressing_enter_opens_the_settings_view() {
        // `settings` is real, wireframe-stage content (`docs/ux/tui/settings/README.md` §4a),
        // so it's dispatched from `Enter` alongside `unit`/`dashboard`, unlike the 8 empty
        // placeholder boxes.
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        for c in "settings".chars() {
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
            .expect("enter on the `settings` command always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Settings");
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn selecting_a_command_with_args_shows_its_argument_preview_row() {
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

        let popup = shell.command_popup.as_ref().expect("popup should be open");
        assert_eq!(popup.selected_command_name(), Some("unit edit <code>"));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with an argument preview should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("<code> — VDHG"),
            "expected the highlighted command's argument preview to render"
        );
    }

    #[test]
    fn a_zero_arg_command_shows_no_argument_preview_row() {
        let mut shell = Shell::new();
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

        let popup = shell.command_popup.as_ref().expect("popup should be open");
        assert_eq!(popup.selected_command_name(), Some("dashboard"));
        assert_eq!(popup.info_row(), None);
    }

    #[test]
    fn typing_after_a_not_yet_built_message_clears_it() {
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
            .expect("enter on `quit` still maps to an action");
        shell.update(action);
        assert!(
            shell
                .command_popup
                .as_ref()
                .unwrap()
                .info_row()
                .is_some_and(|(_, is_message)| is_message)
        );

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Backspace,
                KeyModifiers::NONE,
            )))
            .expect("backspace always maps to an action while the popup is open");
        shell.update(action);

        assert!(
            shell
                .command_popup
                .as_ref()
                .unwrap()
                .info_row()
                .is_none_or(|(_, is_message)| !is_message),
            "the not-yet-built message should be cleared by a mutating key"
        );
    }

    #[test]
    fn tab_clears_a_showing_not_yet_built_message() {
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
            .expect("enter on `quit` still maps to an action");
        shell.update(action);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .expect("tab always maps to an action while the popup is open");
        assert_eq!(action, Action::CommandPopupTab);
        shell.update(action);

        assert!(
            shell
                .command_popup
                .as_ref()
                .unwrap()
                .info_row()
                .is_none_or(|(_, is_message)| !is_message)
        );
    }

    #[test]
    fn esc_at_rest_is_a_no_op_with_an_empty_view_stack() {
        let mut shell = Shell::new();
        assert!(shell.view_stack.is_empty());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .expect("esc while no popup is open always maps to an action");
        assert_eq!(action, Action::PopView);
        shell.update(action);

        assert_eq!(shell.view.title(), "Dashboard");
        assert!(shell.view_stack.is_empty());
    }

    #[test]
    fn opening_a_new_view_pushes_the_outgoing_one_and_esc_pops_back_to_it() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert_eq!(shell.view.title(), "Units & Prices");
        assert_eq!(shell.view_stack.len(), 1);

        shell.update(Action::OpenAccounts);
        assert_eq!(shell.view.title(), "Accounts");
        assert_eq!(shell.view_stack.len(), 2);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .expect("esc while no popup is open always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Units & Prices");
        assert_eq!(shell.view_stack.len(), 1);
    }

    #[test]
    fn opening_dashboard_clears_the_view_stack() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        shell.update(Action::OpenAccounts);
        assert_eq!(shell.view_stack.len(), 2);

        shell.update(Action::OpenDashboard);

        assert_eq!(shell.view.title(), "Dashboard");
        assert!(shell.view_stack.is_empty());
    }

    #[test]
    fn reopening_the_already_active_view_is_a_no_op() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        assert_eq!(shell.view_stack.len(), 1);

        shell.update(Action::OpenUnits);

        assert_eq!(shell.view.title(), "Units & Prices");
        assert_eq!(
            shell.view_stack.len(),
            1,
            "the stack should stay untouched when re-opening the active view"
        );
    }

    #[test]
    fn reopening_the_already_active_view_still_closes_an_open_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenUnits);
        shell.update(Action::OpenCommandPopup);
        assert!(shell.command_popup.is_some());

        shell.update(Action::OpenUnits);

        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn lowercase_q_quits_the_shell_at_rest() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))
            .expect("q always maps to an action while no popup is open");
        assert_eq!(action, Action::GracefulQuit);
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn lowercase_q_is_ordinary_input_while_the_command_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::CommandPopupInput('q')));
        assert!(!shell.should_quit);
    }

    fn category_id_by_name(shell: &Shell, name: &str) -> lib_core::RowID {
        shell
            .view
            .category_store()
            .expect("Categories view should expose its store")
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
            .id
    }

    #[test]
    fn m_on_the_categories_view_opens_the_move_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        assert!(shell.category_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('m'),
                KeyModifiers::NONE,
            )))
            .expect("m on the categories view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.category_popup, Some(CategoryPopup::Move(_))));
    }

    #[test]
    fn esc_closes_an_open_category_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryMovePopup(groceries));
        assert!(shell.category_popup.is_some());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .expect("esc while open always maps to an action");
        shell.update(action);

        assert!(shell.category_popup.is_none());
    }

    #[test]
    fn other_keys_become_typed_input_while_the_move_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryMovePopup(groceries));

        // 'j' moves the tree selection on the bare view; while the popup is open it's typed
        // input for the `new parent` field instead.
        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::CategoryMovePopupInput('j')));
    }

    #[test]
    fn opening_the_move_popup_closes_an_open_command_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCommandPopup);
        assert!(shell.command_popup.is_some());

        shell.update(Action::OpenCategoryMovePopup(groceries));

        assert!(shell.category_popup.is_some());
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn typing_a_valid_path_and_ctrl_s_moves_the_category_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        let transport = category_id_by_name(&shell, "Transport");

        shell.update(Action::OpenCategoryMovePopup(groceries));
        // Clear the prefilled "expenses/food" and type a different, valid existing path.
        for _ in 0.."expenses/food".chars().count() {
            shell.update(Action::CategoryMovePopupBackspace);
        }
        for c in "expenses/transport".chars() {
            shell.update(Action::CategoryMovePopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid resolved path always maps to an action");
        shell.update(action);

        assert!(
            shell.category_popup.is_none(),
            "popup should close after a successful move"
        );
        let store = shell.view.category_store().unwrap();
        assert_eq!(store.find(groceries).unwrap().parent_id, Some(transport));
    }

    #[test]
    fn ctrl_s_does_nothing_while_the_path_is_unresolved() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryMovePopup(groceries));
        for _ in 0.."expenses/food".chars().count() {
            shell.update(Action::CategoryMovePopupBackspace);
        }
        for c in "not/a/real/path".chars() {
            shell.update(Action::CategoryMovePopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.category_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn ctrl_s_refuses_a_cycle_even_when_the_path_resolves() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");
        shell.update(Action::OpenCategoryMovePopup(food));
        // Food's prefilled input is its current parent's path ("expenses"); replace it with
        // its own descendant Groceries' path — a cycle.
        for _ in 0.."expenses".chars().count() {
            shell.update(Action::CategoryMovePopupBackspace);
        }
        for c in "expenses/food/groceries".chars() {
            shell.update(Action::CategoryMovePopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            action, None,
            "a cycle should never resolve to a MoveCategory action"
        );
    }

    #[test]
    fn ctrl_n_creates_the_missing_segment_then_ctrl_s_moves_into_it() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        let food = category_id_by_name(&shell, "Food");
        let original_count = shell.view.category_store().unwrap().nodes().len();

        shell.update(Action::OpenCategoryMovePopup(groceries));
        for _ in 0.."expenses/food".chars().count() {
            shell.update(Action::CategoryMovePopupBackspace);
        }
        for c in "expenses/food/daily".chars() {
            shell.update(Action::CategoryMovePopupInput(c));
        }

        let create_action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+n with a creatable path always maps to an action");
        shell.update(create_action);

        let daily_id = {
            let store = shell.view.category_store().unwrap();
            assert_eq!(
                store.nodes().len(),
                original_count + 1,
                "daily should now exist"
            );
            let daily = store
                .nodes()
                .iter()
                .find(|node| node.name == "daily")
                .expect("ctrl+n should have created daily");
            assert_eq!(daily.parent_id, Some(food));
            daily.id
        };
        assert!(
            shell.category_popup.is_some(),
            "popup should stay open after ^n"
        );

        let move_action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s now resolves since daily exists");
        shell.update(move_action);

        let store = shell.view.category_store().unwrap();
        assert_eq!(store.find(groceries).unwrap().parent_id, Some(daily_id));
    }

    #[test]
    fn renders_the_open_category_move_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryMovePopup(groceries));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the move popup open should not error");
    }

    #[test]
    fn n_on_the_categories_view_opens_the_new_popup_for_a_child_of_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        assert!(shell.category_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::NONE,
            )))
            .expect("n on the categories view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.category_popup, Some(CategoryPopup::New(_))));
    }

    #[test]
    fn shift_n_on_a_root_selection_does_nothing() {
        // The default selection is the Income root, which has no parent for a sibling to
        // attach under.
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('N'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None);
        assert!(shell.category_popup.is_none());
    }

    #[test]
    fn typing_a_name_and_ctrl_s_creates_the_category_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");
        let original_count = shell.view.category_store().unwrap().nodes().len();

        shell.update(Action::OpenCategoryNewPopup(food));
        for c in "Snacks".chars() {
            shell.update(Action::CategoryNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.category_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.category_store().unwrap();
        assert_eq!(store.nodes().len(), original_count + 1);
        let snacks = store
            .nodes()
            .iter()
            .find(|node| node.name == "Snacks")
            .expect("Snacks should now exist");
        assert_eq!(snacks.parent_id, Some(food));
    }

    #[test]
    fn ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");
        shell.update(Action::OpenCategoryNewPopup(food));

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.category_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn ctrl_s_does_nothing_on_a_case_insensitive_sibling_clash() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");
        shell.update(Action::OpenCategoryNewPopup(food));
        for c in "groceries".chars() {
            // Food already has "Groceries".
            shell.update(Action::CategoryNewPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
    }

    #[test]
    fn ctrl_a_creates_and_keeps_the_popup_open_reset_for_another_sibling() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");

        shell.update(Action::OpenCategoryNewPopup(food));
        for c in "Snacks".chars() {
            shell.update(Action::CategoryNewPopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.category_popup.is_some(),
            "popup should stay open after ctrl+a"
        );
        let store = shell.view.category_store().unwrap();
        assert!(store.nodes().iter().any(|node| node.name == "Snacks"));

        // The popup should be reset (name cleared) but still targeting Food, so a second
        // sibling can be typed straight away.
        for c in "Drinks".chars() {
            shell.update(Action::CategoryNewPopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with the second sibling's valid draft always maps to an action");
        shell.update(action);

        let store = shell.view.category_store().unwrap();
        let drinks = store
            .nodes()
            .iter()
            .find(|node| node.name == "Drinks")
            .expect("Drinks should now exist");
        assert_eq!(drinks.parent_id, Some(food));
        assert!(shell.category_popup.is_none());
    }

    #[test]
    fn renders_the_open_category_new_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let food = category_id_by_name(&shell, "Food");
        shell.update(Action::OpenCategoryNewPopup(food));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the new popup open should not error");
    }

    #[test]
    fn e_on_the_categories_view_opens_the_edit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        assert!(shell.category_popup.is_none());

        // The default selection is the Income root, which `e` refuses (no editable name/
        // note/active) — move onto one of its children first.
        let move_down = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            )))
            .expect("j always maps to an action");
        shell.update(move_down);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on a non-root category always maps to an action");
        shell.update(action);

        assert!(matches!(shell.category_popup, Some(CategoryPopup::Edit(_))));
    }

    #[test]
    fn typing_a_new_name_and_ctrl_s_renames_the_category_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");

        shell.update(Action::OpenCategoryEditPopup(groceries));
        for _ in 0.."Groceries".chars().count() {
            shell.update(Action::CategoryEditPopupBackspace);
        }
        for c in "Fresh Food".chars() {
            shell.update(Action::CategoryEditPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.category_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.category_store().unwrap();
        assert_eq!(store.find(groceries).unwrap().name, "Fresh Food");
        assert_eq!(
            store.find(groceries).unwrap().note.as_deref(),
            Some("supermarket, greengrocer"),
            "note should be preserved, not cleared, by an unrelated rename"
        );
    }

    #[test]
    fn ctrl_s_does_nothing_while_the_edit_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryEditPopup(groceries));
        for _ in 0.."Groceries".chars().count() {
            shell.update(Action::CategoryEditPopupBackspace);
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.category_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn ctrl_a_archives_the_category_keeping_the_typed_name_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryEditPopup(groceries));
        assert!(
            shell
                .view
                .category_store()
                .unwrap()
                .find(groceries)
                .unwrap()
                .active
        );

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.category_popup.is_none());
        let store = shell.view.category_store().unwrap();
        assert_eq!(store.find(groceries).unwrap().name, "Groceries");
        assert!(!store.find(groceries).unwrap().active);
    }

    #[test]
    fn capital_x_on_the_edit_popup_shows_a_not_yet_built_hint_and_does_not_mutate() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryEditPopup(groceries));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('X'),
                KeyModifiers::NONE,
            )))
            .expect("X on the edit popup always maps to an action");
        shell.update(action);

        assert!(
            shell.category_popup.is_some(),
            "the not-yet-built fallback shouldn't close the popup"
        );
        let store = shell.view.category_store().unwrap();
        assert!(store.find(groceries).unwrap().active);
    }

    #[test]
    fn renders_the_open_category_edit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let groceries = category_id_by_name(&shell, "Groceries");
        shell.update(Action::OpenCategoryEditPopup(groceries));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the edit popup open should not error");
    }

    /// Filters the open command popup down to `filter`, then presses `Enter` — the action
    /// dispatching whatever's now highlighted.
    fn select_and_enter(shell: &mut Shell, filter: &str) -> Action {
        for c in filter.chars() {
            let action = shell
                .map_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(c),
                    KeyModifiers::NONE,
                )))
                .expect("typing a filter character always maps to an action");
            shell.update(action);
        }
        shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on a filtered-to-one command always maps to an action")
    }

    #[test]
    fn selecting_the_cat_command_and_pressing_enter_opens_the_categories_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        // "cat" alone is ambiguous (it also substring-matches e.g. `budget new <category>
        // <limit>`) — "categories" (the domain name) narrows to just this domain's own eight
        // commands, with the bare `cat` first among them.
        let action = select_and_enter(&mut shell, "categories");
        assert_eq!(action, Action::OpenCategories);
    }

    #[test]
    fn selecting_cat_new_while_categories_is_active_opens_the_new_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "cat new");
        assert_eq!(action, Action::OpenCategoryNewPopup(income));
    }

    #[test]
    fn selecting_cat_edit_while_categories_is_active_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "cat edit");
        assert_eq!(action, Action::OpenCategoryEditPopup(income));
    }

    #[test]
    fn selecting_cat_move_while_categories_is_active_opens_the_move_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "cat move");
        assert_eq!(action, Action::OpenCategoryMovePopup(income));
    }

    #[test]
    fn selecting_cat_archive_while_categories_is_active_archives_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        // Move the tree selection off the default Income root (`cat archive` would just
        // refuse it, via `set_active`'s own `IsRoot` check) onto its first child.
        let move_down = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            )))
            .expect("j always maps to an action");
        shell.update(move_down);
        let investments = category_id_by_name(&shell, "Investments");
        assert!(
            shell
                .view
                .category_store()
                .unwrap()
                .find(investments)
                .unwrap()
                .active
        );

        shell.update(Action::OpenCommandPopup);
        let action = select_and_enter(&mut shell, "cat archive");
        assert_eq!(
            action,
            Action::UpdateCategory {
                id: investments,
                name: "Investments".to_string(),
                note: None,
                active: false,
            }
        );

        shell.update(action);
        assert!(
            !shell
                .view
                .category_store()
                .unwrap()
                .find(investments)
                .unwrap()
                .active
        );
    }

    #[test]
    fn cat_new_edit_move_and_archive_fall_back_to_not_yet_built_when_categories_is_not_active() {
        for (filter, name) in [
            ("cat new", "cat new <name> [parent]"),
            ("cat edit", "cat edit <cat>"),
            ("cat move", "cat move <cat> <parent>"),
            ("cat archive", "cat archive <cat>"),
        ] {
            let mut shell = Shell::new();
            shell.update(Action::OpenCommandPopup);
            let action = select_and_enter(&mut shell, filter);
            assert_eq!(
                action,
                Action::CommandPopupSetNotYetBuilt(name),
                "{filter} should fall back when Categories isn't the active view"
            );
        }
    }

    #[test]
    fn cat_rename_merge_and_tree_always_show_not_yet_built() {
        for (filter, name) in [
            ("cat rename", "cat rename <cat> <name>"),
            ("cat merge", "cat merge <from> <into>"),
            ("cat tree", "cat tree [root]"),
        ] {
            let mut shell = Shell::new();
            shell.update(Action::OpenCategories); // even with Categories active...
            shell.update(Action::OpenCommandPopup);
            let action = select_and_enter(&mut shell, filter);
            assert_eq!(
                action,
                Action::CommandPopupSetNotYetBuilt(name),
                "{filter} should have no real dispatch yet"
            );
        }
    }
}
