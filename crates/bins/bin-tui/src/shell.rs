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
    payee::AliasSource,
    popup::{
        Dim,
        account::{
            AccountPopup, delete::DeleteAccountPopup, edit::EditAccountPopup, new::NewAccountPopup,
        },
        category::{
            CategoryPopup, edit_popup::EditPopup, move_popup::MovePopup, new_popup::NewPopup,
        },
        command::CommandPopup,
        payee::{
            PayeePopup, delete::DeleteCommit, delete::DeletePayeePopup, edit::EditPayeePopup,
            matches::ComposeCommit, matches::PayeeMatchesPopup, new::NewPayeePopup,
        },
        settings::{SettingsPopup, edit::EditSettingPopup, guard::BaseUnitGuardPopup},
        tag::{TagPopup, edit::EditTagPopup, new::NewTagPopup},
        unit::{UnitPopup, delete::DeleteUnitPopup, edit::EditUnitPopup, new::NewUnitPopup},
    },
    tui::Tui,
    view::{
        Action, View, accounts::AccountsView, balance_checks::BalanceChecksView,
        budgets::BudgetsView, categories::CategoriesView, dashboard::DashboardView, help::HelpView,
        payees::PayeesView, reports::ReportsView, settings::SettingsView, tags::TagsView,
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
    /// The Settings-domain popup (`crate::popup::settings`) — `Some` while one is open. Owned
    /// here, mirroring `unit_popup`. Mutually exclusive with `command_popup`/`unit_popup`/
    /// `category_popup`.
    settings_popup: Option<SettingsPopup>,
    /// The Account-domain popup (`crate::popup::account`) — `Some` while one is open. Owned
    /// here, mirroring `category_popup`: it acts on `AccountsView`'s own fixture, reached via
    /// `View::account_store`, not on anything `Shell` owns directly. Mutually exclusive with
    /// every other popup field.
    account_popup: Option<AccountPopup>,
    /// The Tag-domain popup (`crate::popup::tag`) — `Some` while one is open. Owned here,
    /// mirroring `account_popup`: it acts on `TagsView`'s own fixture, reached via
    /// `View::tag_store`, not on anything `Shell` owns directly. Mutually exclusive with every
    /// other popup field.
    tag_popup: Option<TagPopup>,
    /// The Payee-domain popup (`crate::popup::payee`) — `Some` while one is open. Owned here,
    /// mirroring `account_popup`/`tag_popup`: it acts on `PayeesView`'s own fixture, reached via
    /// `View::payee_store`, not on anything `Shell` owns directly. Mutually exclusive with
    /// every other popup field.
    payee_popup: Option<PayeePopup>,
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
            settings_popup: None,
            account_popup: None,
            tag_popup: None,
            payee_popup: None,
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
    /// even mid-chord; then, while the command popup, a unit form, the Category popup, or a
    /// settings popup is open, it takes every other key over the active view (per §3a, the view
    /// behind it is inert while it's up); then a pending `g` leader consumes the very next key
    /// as its chord
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
                if self.settings_popup.is_some() {
                    return map_settings_popup_key(key);
                }
                if self.account_popup.is_some() {
                    return self.map_account_popup_key(key);
                }
                if self.tag_popup.is_some() {
                    return self.map_tag_popup_key(key);
                }
                if self.payee_popup.is_some() {
                    return self.map_payee_popup_key(key);
                }
                if self.pending_leader {
                    self.pending_leader = false;
                    return match key.code {
                        KeyCode::Char('a') => Some(Action::OpenAccounts),
                        KeyCode::Char('b') => Some(Action::OpenBudgets),
                        KeyCode::Char('c') => Some(Action::OpenCategories),
                        KeyCode::Char('d') => Some(Action::OpenDashboard),
                        // `t` is already Transactions', so Tags gets the leader key doubled
                        // instead — the one letter left unclaimed once every other domain had
                        // already taken its own initial.
                        KeyCode::Char('g') => Some(Action::OpenTags),
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
                Some("category") => Some(Action::OpenCategories),
                // The command popup has no real typed-argument resolution (`popup::command::
                // commands::categories`'s own module doc), so `<cat>`/`<parent>`/`<name>` all
                // mean "the Categories tree's current selection" here — the same target its
                // own `n`/`e`/`m`/`a` keys act on. Falls back to the ordinary "not yet built"
                // message when there isn't one (Categories isn't the active view), rather than
                // silently doing nothing.
                Some(name @ "category new <name> [parent]") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryNewPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "category edit <cat>") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryEditPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "category move <cat> <parent>") => self
                    .view
                    .category_selection()
                    .map(Action::OpenCategoryMovePopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "category archive <cat>") => self
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
                Some("account") => Some(Action::OpenAccounts),
                // Mirrors the Categories arm above — the command popup has no real
                // typed-argument resolution (`popup::command::commands::accounts`'s own module
                // doc), so `<acct>`/`<name>`/`<type>`/`<unit>` all mean "the Accounts list's
                // current selection" here, the same target its own `e`/`d`/`a` keys act on.
                // `account new` doesn't need a selection (it opens blank either way), but still
                // only opens when the Accounts view is actually active — `account_store`
                // serves as that guard since there's no selection prerequisite to check
                // instead.
                Some(name @ "account new <name> <type> <unit>") => {
                    if self.view.account_store().is_some() {
                        Some(Action::OpenAccountNewPopup)
                    } else {
                        Some(Action::CommandPopupSetNotYetBuilt(name))
                    }
                }
                Some(name @ "account edit <acct>") => self
                    .view
                    .account_selection()
                    .map(Action::OpenAccountEditPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "account delete <acct> [into <acct>]") => self
                    .view
                    .account_selection()
                    .map(Action::OpenAccountDeletePopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "account off <acct>") => self
                    .view
                    .account_selection()
                    .map(|id| Action::SetAccountActive { id, active: false })
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "account on <acct>") => self
                    .view
                    .account_selection()
                    .map(|id| Action::SetAccountActive { id, active: true })
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                // "account check <acct> <amount> [date]" falls through to the catch-all below —
                // Balance Check/reconcile is out of this map's destination entirely (README
                // §*Not yet designed*), so it's never special-cased, same as Categories' own
                // "rename"/"merge"/"tree".
                Some("tag") => Some(Action::OpenTags),
                // Mirrors the Accounts arm above — `tag new` has no selection prerequisite,
                // only the guard that Tags is actually the active view.
                Some(name @ "tag new <name>") => {
                    if self.view.tag_store().is_some() {
                        Some(Action::OpenTagNewPopup)
                    } else {
                        Some(Action::CommandPopupSetNotYetBuilt(name))
                    }
                }
                Some(name @ "tag edit <tag>") => self
                    .view
                    .tag_selection()
                    .map(Action::OpenTagEditPopup)
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "tag off <tag>") => self
                    .view
                    .tag_selection()
                    .map(|id| Action::SetTagActive { id, active: false })
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                Some(name @ "tag on <tag>") => self
                    .view
                    .tag_selection()
                    .map(|id| Action::SetTagActive { id, active: true })
                    .or(Some(Action::CommandPopupSetNotYetBuilt(name))),
                // "tag delete <tag>" only arms the same lightweight confirm the bare `d` key
                // does (`popup::command::commands::tags`'s own module doc) — it never deletes
                // on its own, so this carries no id, just the same guard every other Tags
                // entry above uses.
                Some(name @ "tag delete <tag>") => self
                    .view
                    .tag_selection()
                    .map(|_| Action::ArmTagDelete)
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

    /// Routes a key while the Account popup is open, resolving it into a concrete `Action`
    /// right here against the active view's `AccountStore` (`View::account_store`) — mirrors
    /// `map_category_popup_key`. A create that doesn't yet validate resolves to `None` (a
    /// no-op) rather than a doomed `Action`.
    fn map_account_popup_key(&self, key: KeyEvent) -> Option<Action> {
        let store = self.view.account_store()?;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match &self.account_popup {
            Some(AccountPopup::New(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseAccountPopup),
                KeyCode::Backspace => Some(Action::AccountNewPopupBackspace),
                KeyCode::Tab => Some(Action::AccountNewPopupTab),
                KeyCode::Char('h') if !ctrl && popup.type_field_focused() => {
                    Some(Action::AccountNewPopupTypeLeft)
                }
                KeyCode::Char('l') if !ctrl && popup.type_field_focused() => {
                    Some(Action::AccountNewPopupTypeRight)
                }
                KeyCode::Char('s') if ctrl => popup.create_fields(store).map(
                    |(name, account_type, unit, starting_balance, active)| Action::CreateAccount {
                        name,
                        account_type: account_type.as_str().to_string(),
                        unit_code: unit.code,
                        unit_decimal_places: unit.decimal_places,
                        starting_balance,
                        active,
                        close_after: true,
                    },
                ),
                KeyCode::Char('a') if ctrl => popup.create_fields(store).map(
                    |(name, account_type, unit, starting_balance, active)| Action::CreateAccount {
                        name,
                        account_type: account_type.as_str().to_string(),
                        unit_code: unit.code,
                        unit_decimal_places: unit.decimal_places,
                        starting_balance,
                        active,
                        close_after: false,
                    },
                ),
                KeyCode::Char(c) if !ctrl => Some(Action::AccountNewPopupInput(c)),
                _ => None,
            },
            Some(AccountPopup::Edit(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseAccountPopup),
                KeyCode::Backspace => Some(Action::AccountEditPopupBackspace),
                KeyCode::Tab => Some(Action::AccountEditPopupTab),
                KeyCode::Char('h') if !ctrl && popup.type_field_focused() => {
                    Some(Action::AccountEditPopupTypeLeft)
                }
                KeyCode::Char('l') if !ctrl && popup.type_field_focused() => {
                    Some(Action::AccountEditPopupTypeRight)
                }
                KeyCode::Char('s') if ctrl => {
                    popup.save_fields().map(|(id, name, account_type, active)| {
                        Action::UpdateAccount {
                            id,
                            name,
                            account_type: account_type.as_str().to_string(),
                            active,
                        }
                    })
                }
                KeyCode::Char('a') if ctrl => {
                    popup
                        .deactivate_fields()
                        .map(|(id, name, account_type, active)| Action::UpdateAccount {
                            id,
                            name,
                            account_type: account_type.as_str().to_string(),
                            active,
                        })
                }
                KeyCode::Char('d') if ctrl => {
                    Some(Action::OpenAccountDeletePopup(popup.editing_id()))
                }
                KeyCode::Char(c) if !ctrl => Some(Action::AccountEditPopupInput(c)),
                _ => None,
            },
            Some(AccountPopup::Delete(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseAccountPopup),
                KeyCode::Backspace => Some(Action::AccountDeletePopupBackspace),
                KeyCode::Tab => Some(Action::AccountDeletePopupTab),
                KeyCode::Char('s') if ctrl => popup
                    .delete_fields(store)
                    .map(|(id, target)| Action::DeleteAccount { id, target }),
                KeyCode::Char('a') if ctrl => {
                    popup
                        .deactivate_fields(store)
                        .map(|(id, name, account_type, active)| Action::UpdateAccount {
                            id,
                            name,
                            account_type: account_type.as_str().to_string(),
                            active,
                        })
                }
                KeyCode::Char(c) if !ctrl => Some(Action::AccountDeletePopupInput(c)),
                _ => None,
            },
            None => None,
        }
    }

    /// Routes a key while the Tag popup is open, resolving it into a concrete `Action` right
    /// here against the active view's `TagStore` (`View::tag_store`) — mirrors
    /// `map_account_popup_key`. A create that doesn't yet validate resolves to `None` (a
    /// no-op) rather than a doomed `Action`.
    fn map_tag_popup_key(&self, key: KeyEvent) -> Option<Action> {
        let store = self.view.tag_store()?;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match &self.tag_popup {
            Some(TagPopup::New(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseTagPopup),
                KeyCode::Backspace => Some(Action::TagNewPopupBackspace),
                KeyCode::Tab => Some(Action::TagNewPopupTab),
                KeyCode::Char('s') if ctrl => {
                    popup
                        .create_fields(store)
                        .map(|(name, active)| Action::CreateTag {
                            name,
                            active,
                            close_after: true,
                        })
                }
                KeyCode::Char('a') if ctrl => {
                    popup
                        .create_fields(store)
                        .map(|(name, active)| Action::CreateTag {
                            name,
                            active,
                            close_after: false,
                        })
                }
                KeyCode::Char(c) if !ctrl => Some(Action::TagNewPopupInput(c)),
                _ => None,
            },
            Some(TagPopup::Edit(popup)) => match key.code {
                KeyCode::Esc => Some(Action::CloseTagPopup),
                KeyCode::Backspace => Some(Action::TagEditPopupBackspace),
                KeyCode::Tab => Some(Action::TagEditPopupTab),
                KeyCode::Char('s') if ctrl => popup
                    .save_fields(store)
                    .map(|(id, name, active)| Action::UpdateTag { id, name, active }),
                KeyCode::Char('a') if ctrl => popup
                    .deactivate_fields(store)
                    .map(|(id, name, active)| Action::UpdateTag { id, name, active }),
                KeyCode::Char(c) if !ctrl => Some(Action::TagEditPopupInput(c)),
                _ => None,
            },
            None => None,
        }
    }

    /// Routes a key while the Payee popup is open, resolving it into a concrete `Action` right
    /// here against the active view's `PayeeStore` (`View::payee_store`) — mirrors
    /// `map_account_popup_key`/`map_tag_popup_key`. A create that doesn't yet validate resolves
    /// to `None` (a no-op) rather than a doomed `Action`.
    fn map_payee_popup_key(&self, key: KeyEvent) -> Option<Action> {
        let store = self.view.payee_store()?;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match &self.payee_popup {
            Some(PayeePopup::New(popup)) => match key.code {
                KeyCode::Esc => Some(Action::ClosePayeePopup),
                KeyCode::Backspace => Some(Action::PayeeNewPopupBackspace),
                KeyCode::Tab => Some(Action::PayeeNewPopupTab),
                KeyCode::Char('s') if ctrl => popup.create_fields(store).map(
                    |(name, website, icon_url, icon_derived, default_category_path, active)| {
                        Action::CreatePayee {
                            name,
                            website,
                            icon_url,
                            icon_derived,
                            default_category_path,
                            active,
                            close_after: true,
                        }
                    },
                ),
                KeyCode::Char('a') if ctrl => popup.create_fields(store).map(
                    |(name, website, icon_url, icon_derived, default_category_path, active)| {
                        Action::CreatePayee {
                            name,
                            website,
                            icon_url,
                            icon_derived,
                            default_category_path,
                            active,
                            close_after: false,
                        }
                    },
                ),
                KeyCode::Char(c) if !ctrl => Some(Action::PayeeNewPopupInput(c)),
                _ => None,
            },
            Some(PayeePopup::Edit(popup)) => match key.code {
                KeyCode::Esc => Some(Action::ClosePayeePopup),
                KeyCode::Backspace => Some(Action::PayeeEditPopupBackspace),
                KeyCode::Tab => Some(Action::PayeeEditPopupTab),
                KeyCode::Char('s') if ctrl => popup.save_fields(store).map(
                    |(id, name, website, icon_url, icon_derived, default_category_path, active)| {
                        Action::UpdatePayee {
                            id,
                            name,
                            website,
                            icon_url,
                            icon_derived,
                            default_category_path,
                            active,
                        }
                    },
                ),
                KeyCode::Char('a') if ctrl => popup.deactivate_fields(store).map(
                    |(id, name, website, icon_url, icon_derived, default_category_path, active)| {
                        Action::UpdatePayee {
                            id,
                            name,
                            website,
                            icon_url,
                            icon_derived,
                            default_category_path,
                            active,
                        }
                    },
                ),
                // `m`/`^d` jump to the rename-matches (8d) / delete (8e) popups, mirroring
                // `popup::account::edit`'s own `^d` hand-off to its delete popup.
                KeyCode::Char('m') if !ctrl => {
                    Some(Action::OpenPayeeMatchesPopup(popup.editing_id()))
                }
                KeyCode::Char('d') if ctrl => {
                    Some(Action::OpenPayeeDeletePopup(popup.editing_id()))
                }
                KeyCode::Char(c) if !ctrl => Some(Action::PayeeEditPopupInput(c)),
                _ => None,
            },
            Some(PayeePopup::Matches(popup)) => {
                if popup.is_composing() {
                    match key.code {
                        KeyCode::Esc => Some(Action::PayeeMatchesPopupCancelCompose),
                        KeyCode::Backspace => Some(Action::PayeeMatchesPopupBackspace),
                        KeyCode::Tab => Some(Action::PayeeMatchesPopupToggleMode),
                        KeyCode::Char('s') if ctrl => {
                            popup.commit(store).map(|commit| match commit {
                                ComposeCommit::Add {
                                    payee_id,
                                    typed,
                                    mode,
                                } => Action::AddPayeeAlias {
                                    payee_id,
                                    typed,
                                    mode,
                                },
                                ComposeCommit::Replace {
                                    old_alias_id,
                                    payee_id,
                                    typed,
                                    mode,
                                } => Action::ReplacePayeeAlias {
                                    old_alias_id,
                                    payee_id,
                                    typed,
                                    mode,
                                },
                            })
                        }
                        KeyCode::Char(c) if !ctrl => Some(Action::PayeeMatchesPopupInput(c)),
                        _ => None,
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => Some(Action::ClosePayeePopup),
                        KeyCode::Char('j') | KeyCode::Down => {
                            Some(Action::PayeeMatchesPopupMoveDown)
                        }
                        KeyCode::Char('k') | KeyCode::Up => Some(Action::PayeeMatchesPopupMoveUp),
                        KeyCode::Char('a') => Some(Action::PayeeMatchesPopupBeginAdd),
                        KeyCode::Char('t') => Some(Action::PayeeMatchesPopupBeginTest),
                        // `e`/`d` only ever produce an action for a `source = Manual` row —
                        // a `source = Rename` row's protection is stated once, permanently, in
                        // the popup's own closing note rather than a per-keypress refusal (see
                        // `popup::payee::matches`'s own module doc).
                        KeyCode::Char('e') => popup
                            .selected_alias(store)
                            .filter(|alias| alias.source == AliasSource::Manual)
                            .map(|_| Action::PayeeMatchesPopupBeginEdit),
                        KeyCode::Char('d') => popup
                            .selected_alias(store)
                            .filter(|alias| alias.source == AliasSource::Manual)
                            .map(|alias| Action::RemovePayeeAlias(alias.id)),
                        _ => None,
                    }
                }
            }
            Some(PayeePopup::Delete(popup)) => match key.code {
                KeyCode::Esc => Some(Action::ClosePayeePopup),
                KeyCode::Backspace => Some(Action::PayeeDeletePopupBackspace),
                KeyCode::Tab => Some(Action::PayeeDeletePopupTab),
                KeyCode::Char('m') if !ctrl && !popup.confirm_field_focused() => {
                    Some(Action::OpenPayeeMatchesPopup(popup.editing_id()))
                }
                KeyCode::Char('s') if ctrl => popup.commit(store).map(|commit| match commit {
                    DeleteCommit::Deactivate(id) => Action::SetPayeeActive { id, active: false },
                    DeleteCommit::Delete(id) => Action::DeletePayee(id),
                }),
                KeyCode::Char(c) if !ctrl => Some(Action::PayeeDeletePopupInput(c)),
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
            Action::OpenEditSettingPopup => {
                self.settings_popup = Some(SettingsPopup::Edit(EditSettingPopup::new()));
                self.command_popup = None;
            }
            Action::OpenBaseUnitGuardPopup => {
                self.settings_popup = Some(SettingsPopup::BaseUnitGuard(BaseUnitGuardPopup::new()));
                self.command_popup = None;
            }
            Action::CloseSettingsPopup => self.settings_popup = None,
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
                self.settings_popup = None;
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
                self.settings_popup = None;
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
                self.settings_popup = None;
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
            Action::OpenAccountNewPopup => {
                self.account_popup = Some(AccountPopup::New(NewAccountPopup::new()));
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
            }
            Action::CloseAccountPopup => self.account_popup = None,
            Action::AccountNewPopupInput(c) => {
                if let Some(AccountPopup::New(popup)) = &mut self.account_popup {
                    popup.push_char(c);
                }
            }
            Action::AccountNewPopupBackspace => {
                if let Some(AccountPopup::New(popup)) = &mut self.account_popup {
                    popup.backspace();
                }
            }
            Action::AccountNewPopupTab => {
                if let Some(store) = self.view.account_store()
                    && let Some(AccountPopup::New(popup)) = &mut self.account_popup
                {
                    popup.tab(store);
                }
            }
            Action::AccountNewPopupTypeLeft => {
                if let Some(AccountPopup::New(popup)) = &mut self.account_popup {
                    popup.type_left();
                }
            }
            Action::AccountNewPopupTypeRight => {
                if let Some(AccountPopup::New(popup)) = &mut self.account_popup {
                    popup.type_right();
                }
            }
            Action::CreateAccount { close_after, .. } => {
                self.view.update(&action);
                if close_after {
                    self.account_popup = None;
                } else if let Some(AccountPopup::New(popup)) = &mut self.account_popup {
                    popup.reset_for_next_account();
                }
            }
            Action::OpenAccountEditPopup(id) => {
                if let Some(store) = self.view.account_store() {
                    self.account_popup = Some(AccountPopup::Edit(EditAccountPopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
            }
            Action::AccountEditPopupInput(c) => {
                if let Some(AccountPopup::Edit(popup)) = &mut self.account_popup {
                    popup.push_char(c);
                }
            }
            Action::AccountEditPopupBackspace => {
                if let Some(AccountPopup::Edit(popup)) = &mut self.account_popup {
                    popup.backspace();
                }
            }
            Action::AccountEditPopupTab => {
                if let Some(AccountPopup::Edit(popup)) = &mut self.account_popup {
                    popup.tab();
                }
            }
            Action::AccountEditPopupTypeLeft => {
                if let Some(AccountPopup::Edit(popup)) = &mut self.account_popup {
                    popup.type_left();
                }
            }
            Action::AccountEditPopupTypeRight => {
                if let Some(AccountPopup::Edit(popup)) = &mut self.account_popup {
                    popup.type_right();
                }
            }
            Action::UpdateAccount { .. } => {
                self.view.update(&action);
                self.account_popup = None;
            }
            Action::OpenAccountDeletePopup(id) => {
                if let Some(store) = self.view.account_store() {
                    self.account_popup =
                        Some(AccountPopup::Delete(DeleteAccountPopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
            }
            Action::AccountDeletePopupInput(c) => {
                if let Some(AccountPopup::Delete(popup)) = &mut self.account_popup {
                    popup.push_char(c);
                }
            }
            Action::AccountDeletePopupBackspace => {
                if let Some(AccountPopup::Delete(popup)) = &mut self.account_popup {
                    popup.backspace();
                }
            }
            Action::AccountDeletePopupTab => {
                if let Some(store) = self.view.account_store()
                    && let Some(AccountPopup::Delete(popup)) = &mut self.account_popup
                {
                    popup.tab(store);
                }
            }
            Action::DeleteAccount { .. } => {
                self.view.update(&action);
                self.account_popup = None;
            }
            // No popup involved — `a` on the list and `:account off`/`on` both just relay
            // straight through to `AccountsView::update`.
            Action::SetAccountActive { .. } => self.view.update(&action),
            Action::OpenTags => self.open(TagsView::new()),
            Action::OpenTagNewPopup => {
                self.tag_popup = Some(TagPopup::New(NewTagPopup::new()));
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
            }
            Action::CloseTagPopup => self.tag_popup = None,
            Action::TagNewPopupInput(c) => {
                if let Some(TagPopup::New(popup)) = &mut self.tag_popup {
                    popup.push_char(c);
                }
            }
            Action::TagNewPopupBackspace => {
                if let Some(TagPopup::New(popup)) = &mut self.tag_popup {
                    popup.backspace();
                }
            }
            Action::TagNewPopupTab => {
                if let Some(TagPopup::New(popup)) = &mut self.tag_popup {
                    popup.tab();
                }
            }
            Action::CreateTag { close_after, .. } => {
                self.view.update(&action);
                if close_after {
                    self.tag_popup = None;
                } else if let Some(TagPopup::New(popup)) = &mut self.tag_popup {
                    popup.reset_for_next_tag();
                }
            }
            Action::OpenTagEditPopup(id) => {
                if let Some(store) = self.view.tag_store() {
                    self.tag_popup = Some(TagPopup::Edit(EditTagPopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
            }
            Action::TagEditPopupInput(c) => {
                if let Some(TagPopup::Edit(popup)) = &mut self.tag_popup {
                    popup.push_char(c);
                }
            }
            Action::TagEditPopupBackspace => {
                if let Some(TagPopup::Edit(popup)) = &mut self.tag_popup {
                    popup.backspace();
                }
            }
            Action::TagEditPopupTab => {
                if let Some(TagPopup::Edit(popup)) = &mut self.tag_popup {
                    popup.tab();
                }
            }
            Action::UpdateTag { .. } => {
                self.view.update(&action);
                self.tag_popup = None;
            }
            // No popup involved — both relay straight through to `TagsView::update`, mirroring
            // `Action::SetAccountActive` above.
            Action::SetTagActive { .. } => self.view.update(&action),
            Action::ArmTagDelete => self.view.update(&action),
            Action::OpenPayeeNewPopup => {
                self.payee_popup = Some(PayeePopup::New(NewPayeePopup::new()));
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
                self.tag_popup = None;
            }
            Action::ClosePayeePopup => self.payee_popup = None,
            Action::PayeeNewPopupInput(c) => {
                if let Some(PayeePopup::New(popup)) = &mut self.payee_popup {
                    popup.push_char(c);
                }
            }
            Action::PayeeNewPopupBackspace => {
                if let Some(PayeePopup::New(popup)) = &mut self.payee_popup {
                    popup.backspace();
                }
            }
            Action::PayeeNewPopupTab => {
                if let Some(store) = self.view.payee_store()
                    && let Some(PayeePopup::New(popup)) = &mut self.payee_popup
                {
                    popup.tab(store);
                }
            }
            Action::CreatePayee { close_after, .. } => {
                self.view.update(&action);
                if close_after {
                    self.payee_popup = None;
                } else if let Some(PayeePopup::New(popup)) = &mut self.payee_popup {
                    popup.reset_for_next_payee();
                }
            }
            Action::OpenPayeeEditPopup(id) => {
                if let Some(store) = self.view.payee_store() {
                    self.payee_popup = Some(PayeePopup::Edit(EditPayeePopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
                self.tag_popup = None;
            }
            Action::PayeeEditPopupInput(c) => {
                if let Some(PayeePopup::Edit(popup)) = &mut self.payee_popup {
                    popup.push_char(c);
                }
            }
            Action::PayeeEditPopupBackspace => {
                if let Some(PayeePopup::Edit(popup)) = &mut self.payee_popup {
                    popup.backspace();
                }
            }
            Action::PayeeEditPopupTab => {
                if let Some(store) = self.view.payee_store()
                    && let Some(PayeePopup::Edit(popup)) = &mut self.payee_popup
                {
                    popup.tab(store);
                }
            }
            Action::UpdatePayee { .. } => {
                self.view.update(&action);
                self.payee_popup = None;
            }
            Action::OpenPayeeMatchesPopup(id) => {
                self.payee_popup = Some(PayeePopup::Matches(PayeeMatchesPopup::new(id)));
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
                self.tag_popup = None;
            }
            Action::PayeeMatchesPopupMoveUp => {
                if let Some(store) = self.view.payee_store()
                    && let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup
                {
                    popup.move_up(store);
                }
            }
            Action::PayeeMatchesPopupMoveDown => {
                if let Some(store) = self.view.payee_store()
                    && let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup
                {
                    popup.move_down(store);
                }
            }
            Action::PayeeMatchesPopupBeginAdd => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.begin_add();
                }
            }
            Action::PayeeMatchesPopupBeginEdit => {
                if let Some(store) = self.view.payee_store()
                    && let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup
                {
                    popup.begin_edit(store);
                }
            }
            Action::PayeeMatchesPopupBeginTest => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.begin_test();
                }
            }
            Action::PayeeMatchesPopupCancelCompose => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.cancel_compose();
                }
            }
            Action::PayeeMatchesPopupInput(c) => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.push_char(c);
                }
            }
            Action::PayeeMatchesPopupBackspace => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.backspace();
                }
            }
            Action::PayeeMatchesPopupToggleMode => {
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.toggle_mode();
                }
            }
            // `RemovePayeeAlias`/`AddPayeeAlias`/`ReplacePayeeAlias` were already validated in
            // `map_payee_popup_key` against the store `Shell` itself has no other access to —
            // relaying to the active `View`'s own `update` is where the mutation actually
            // happens, mirroring `Action::MoveCategory`'s own identical pattern. The popup
            // itself only needs its compose slot cleared afterwards; its list re-reads the
            // store fresh on the very next render.
            Action::RemovePayeeAlias(_) => self.view.update(&action),
            Action::AddPayeeAlias { .. } | Action::ReplacePayeeAlias { .. } => {
                self.view.update(&action);
                if let Some(PayeePopup::Matches(popup)) = &mut self.payee_popup {
                    popup.cancel_compose();
                }
            }
            Action::OpenPayeeDeletePopup(id) => {
                if let Some(store) = self.view.payee_store() {
                    self.payee_popup = Some(PayeePopup::Delete(DeletePayeePopup::new(store, id)));
                }
                self.command_popup = None;
                self.unit_popup = None;
                self.category_popup = None;
                self.settings_popup = None;
                self.account_popup = None;
                self.tag_popup = None;
            }
            Action::PayeeDeletePopupInput(c) => {
                if let Some(PayeePopup::Delete(popup)) = &mut self.payee_popup {
                    popup.push_char(c);
                }
            }
            Action::PayeeDeletePopupBackspace => {
                if let Some(PayeePopup::Delete(popup)) = &mut self.payee_popup {
                    popup.backspace();
                }
            }
            Action::PayeeDeletePopupTab => {
                if let Some(PayeePopup::Delete(popup)) = &mut self.payee_popup {
                    popup.tab();
                }
            }
            // No popup involved when dispatched from the list's own bare `a` — relays
            // straight through to `PayeesView::update`, mirroring `Action::SetAccountActive`.
            // When dispatched from the delete popup's own `^s`, also closes it.
            Action::SetPayeeActive { .. } => {
                self.view.update(&action);
                self.payee_popup = None;
            }
            Action::DeletePayee(_) => {
                self.view.update(&action);
                self.payee_popup = None;
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
            self.settings_popup = None;
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
        self.settings_popup = None;
        self.account_popup = None;
        self.tag_popup = None;
        self.payee_popup = None;
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
        let edit_setting_popup_open = matches!(self.settings_popup, Some(SettingsPopup::Edit(_)));
        let base_unit_guard_popup_open =
            matches!(self.settings_popup, Some(SettingsPopup::BaseUnitGuard(_)));
        let account_popup_open = self.account_popup.is_some();
        let tag_popup_open = self.tag_popup.is_some();
        let payee_popup_open = self.payee_popup.is_some();

        // Header Frame — the status line names the mode whenever it isn't the resting
        // NORMAL state, per `docs/ux/tui/README.md`'s "show the mode ... whenever it is not
        // NORMAL" — `COMMAND` for the command popup, `INSERT` for a unit form or the Category
        // popup (it has a text field too), per "Modal, vim-flavoured ... INSERT only inside
        // forms ... COMMAND while the palette is open". `EDIT`/`CONFIRM` for the settings popups
        // match `docs/ux/tui/settings/README.md` §4b's own "`EDIT · uncommitted`" and §4c's own
        // "`confirm base unit`" status line text (the `uncommitted`/`base unit` half of each
        // isn't reproduced here — the shell's status line is a flat title, not the design's own
        // breadcrumb, the same simplification every other view already makes).
        let mode = if command_popup_open {
            " · COMMAND"
        } else if unit_popup_open
            || category_popup_open
            || account_popup_open
            || tag_popup_open
            || payee_popup_open
        {
            " · INSERT"
        } else if edit_setting_popup_open {
            " · EDIT"
        } else if base_unit_guard_popup_open {
            " · CONFIRM"
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
        } else if edit_setting_popup_open {
            Line::from(" esc close edit form ").style(Style::default().fg(Color::DarkGray))
        } else if base_unit_guard_popup_open {
            Line::from(" esc close dialog ").style(Style::default().fg(Color::DarkGray))
        } else if account_popup_open {
            Line::from(" esc close account form ").style(Style::default().fg(Color::DarkGray))
        } else if tag_popup_open {
            Line::from(" esc close tag form ").style(Style::default().fg(Color::DarkGray))
        } else if payee_popup_open {
            Line::from(" esc close payee form ").style(Style::default().fg(Color::DarkGray))
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
        } else if let Some(popup) = &self.settings_popup {
            frame.render_widget(Dim, rows[1]);
            popup.render(frame, frame.area());
        } else if let Some(popup) = &self.account_popup {
            frame.render_widget(Dim, rows[1]);
            if let Some(store) = self.view.account_store() {
                popup.render(frame, frame.area(), store);
            }
        } else if let Some(popup) = &self.tag_popup {
            frame.render_widget(Dim, rows[1]);
            if let Some(store) = self.view.tag_store() {
                popup.render(frame, frame.area(), store);
            }
        } else if let Some(popup) = &self.payee_popup {
            frame.render_widget(Dim, rows[1]);
            if let Some(store) = self.view.payee_store() {
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

/// Routes a key while a settings popup (the §4b editor or the §4c guard) is open. Only `Esc`
/// does anything yet, mirroring [`map_unit_popup_key`] — no field is editable and nothing
/// commits until the settings registry lands.
fn map_settings_popup_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::CloseSettingsPopup),
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
    fn g_then_g_opens_the_tags_view() {
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
                KeyCode::Char('g'),
                KeyModifiers::NONE,
            )))
            .expect("the second `g` completing the `g g` chord always maps to an action");
        shell.update(action);

        assert_eq!(shell.view.title(), "Tags");
        assert!(!shell.pending_leader);
    }

    #[test]
    fn g_then_letter_opens_the_matching_view() {
        let cases: &[(char, &str)] = &[
            ('a', "Accounts"),
            ('b', "Budgets"),
            ('c', "Categories"),
            ('d', "Dashboard"),
            ('g', "Tags"),
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
        // popup stays open rather than navigating anywhere. `category list` (now `category`)
        // and `account list` (now `account`) are no longer among them — "Categories:
        // :category command grammar" and "Accounts: :acct command grammar" gave them real
        // dispatch.
        let cases: &[&str] = &[
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
    fn e_on_the_settings_view_opens_the_edit_setting_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenSettings);
        assert!(shell.settings_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on the settings view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.settings_popup, Some(SettingsPopup::Edit(_))));
    }

    #[test]
    fn enter_on_the_settings_view_opens_the_base_unit_guard_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenSettings);
        assert!(shell.settings_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .expect("enter on the settings view always maps to an action");
        shell.update(action);

        assert!(matches!(
            shell.settings_popup,
            Some(SettingsPopup::BaseUnitGuard(_))
        ));
    }

    #[test]
    fn esc_closes_an_open_settings_popup() {
        for open in [Action::OpenEditSettingPopup, Action::OpenBaseUnitGuardPopup] {
            let mut shell = Shell::new();
            shell.update(open);

            let action = shell
                .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
                .expect("esc while open always maps to an action");
            shell.update(action);

            assert!(shell.settings_popup.is_none());
        }
    }

    #[test]
    fn other_keys_are_swallowed_while_a_settings_popup_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenEditSettingPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None);
    }

    #[test]
    fn opening_a_settings_popup_closes_an_open_command_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        assert!(shell.command_popup.is_some());

        shell.update(Action::OpenEditSettingPopup);

        assert!(shell.settings_popup.is_some());
        assert!(shell.command_popup.is_none());
    }

    #[test]
    fn opening_a_settings_popup_replaces_any_other_open_settings_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenEditSettingPopup);
        assert!(matches!(shell.settings_popup, Some(SettingsPopup::Edit(_))));

        shell.update(Action::OpenBaseUnitGuardPopup);

        assert!(matches!(
            shell.settings_popup,
            Some(SettingsPopup::BaseUnitGuard(_))
        ));
    }

    #[test]
    fn leaving_the_settings_view_closes_an_open_settings_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenSettings);
        shell.update(Action::OpenEditSettingPopup);
        assert!(shell.settings_popup.is_some());

        shell.update(Action::OpenDashboard);

        assert!(shell.settings_popup.is_none());
    }

    #[test]
    fn status_line_and_footer_reflect_the_open_settings_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenSettings);
        shell.update(Action::OpenEditSettingPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the edit popup open should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(text.contains("· EDIT"), "expected the EDIT mode tag");
        assert!(
            text.contains("esc close edit form"),
            "expected the edit popup's own footer hint"
        );

        shell.update(Action::OpenBaseUnitGuardPopup);
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the guard popup open should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(text.contains("· CONFIRM"), "expected the CONFIRM mode tag");
        assert!(
            text.contains("esc close dialog"),
            "expected the guard popup's own footer hint"
        );
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

    fn tag_exists(shell: &Shell, name: &str) -> bool {
        shell
            .view
            .tag_store()
            .expect("Tags view should expose its store")
            .tags()
            .iter()
            .any(|tag| tag.name == name)
    }

    fn tag_id_by_name(shell: &Shell, name: &str) -> lib_core::RowID {
        shell
            .view
            .tag_store()
            .expect("Tags view should expose its store")
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a tag named {name}"))
            .id
    }

    fn account_exists(shell: &Shell, name: &str) -> bool {
        shell
            .view
            .account_store()
            .expect("Accounts view should expose its store")
            .accounts()
            .iter()
            .any(|account| account.name == name)
    }

    fn payee_exists(shell: &Shell, name: &str) -> bool {
        shell
            .view
            .payee_store()
            .expect("Payees view should expose its store")
            .payees()
            .iter()
            .any(|payee| payee.name == name)
    }

    fn account_id_by_name(shell: &Shell, name: &str) -> lib_core::RowID {
        shell
            .view
            .account_store()
            .expect("Accounts view should expose its store")
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
            .id
    }

    fn payee_id_by_name(shell: &Shell, name: &str) -> lib_core::RowID {
        shell
            .view
            .payee_store()
            .expect("Payees view should expose its store")
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
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
    fn n_on_the_accounts_view_opens_the_new_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        assert!(shell.account_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::NONE,
            )))
            .expect("n on the accounts view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.account_popup, Some(AccountPopup::New(_))));
    }

    #[test]
    fn esc_closes_the_account_popup_without_creating_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let original_count = shell.view.account_store().unwrap().accounts().len();

        shell.update(Action::OpenAccountNewPopup);
        for c in "Side Hustle".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.account_popup.is_none());
        assert_eq!(
            shell.view.account_store().unwrap().accounts().len(),
            original_count
        );
        assert!(!account_exists(&shell, "Side Hustle"));
    }

    #[test]
    fn typing_a_name_and_unit_then_ctrl_s_creates_the_account_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let original_count = shell.view.account_store().unwrap().accounts().len();

        shell.update(Action::OpenAccountNewPopup);
        // Focus starts on `type` (the handoff's own default); cycle is type -> unit ->
        // starting bal -> active -> name.
        shell.update(Action::AccountNewPopupTab); // type -> unit
        for c in "AUD".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }
        shell.update(Action::AccountNewPopupTab); // unit -> starting bal (exact match, advances)
        shell.update(Action::AccountNewPopupTab); // starting bal -> active
        shell.update(Action::AccountNewPopupTab); // active -> name
        for c in "Side Hustle".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.account_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.account_store().unwrap();
        assert_eq!(store.accounts().len(), original_count + 1);
        assert!(account_exists(&shell, "Side Hustle"));
    }

    #[test]
    fn account_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        shell.update(Action::OpenAccountNewPopup);
        shell.update(Action::AccountNewPopupTab); // off `type`, onto `unit`
        for c in "AUD".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.account_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn ctrl_s_does_nothing_while_the_unit_is_unresolved() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        shell.update(Action::OpenAccountNewPopup);
        shell.update(Action::AccountNewPopupTab); // type -> unit
        for c in "ZZZ".chars() {
            // No known Unit matches "ZZZ" — unlike an empty `unit_input` (which `tab()`
            // would auto-complete to the alphabetically-first known Unit, since every string
            // starts with ""), this stays genuinely unresolved across the tabs below.
            shell.update(Action::AccountNewPopupInput(c));
        }
        shell.update(Action::AccountNewPopupTab); // unit (unresolved) -> starting bal
        shell.update(Action::AccountNewPopupTab); // starting bal -> active
        shell.update(Action::AccountNewPopupTab); // active -> name
        for c in "New Account".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None, "an unresolved unit should never validate");
    }

    #[test]
    fn ctrl_a_creates_and_keeps_the_popup_open_reset_for_another_account() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);

        shell.update(Action::OpenAccountNewPopup);
        shell.update(Action::AccountNewPopupTab); // type -> unit
        for c in "AUD".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }
        shell.update(Action::AccountNewPopupTab); // unit -> starting bal
        shell.update(Action::AccountNewPopupTab); // starting bal -> active
        shell.update(Action::AccountNewPopupTab); // active -> name
        for c in "First".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.account_popup.is_some(),
            "popup should stay open after ctrl+a"
        );
        assert!(account_exists(&shell, "First"));

        // The popup should be reset — name cleared, unit kept, but focus back on `type` (its
        // own initial-focus convention) — so tab back to `name` before typing the next one.
        shell.update(Action::AccountNewPopupTab); // type -> unit
        shell.update(Action::AccountNewPopupTab); // unit (AUD, exact match) -> starting bal
        shell.update(Action::AccountNewPopupTab); // starting bal -> active
        shell.update(Action::AccountNewPopupTab); // active -> name
        for c in "Second".chars() {
            shell.update(Action::AccountNewPopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with the second account's valid draft always maps to an action");
        shell.update(action);

        assert!(account_exists(&shell, "Second"));
        assert!(shell.account_popup.is_none());
    }

    #[test]
    fn renders_the_open_account_new_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        shell.update(Action::OpenAccountNewPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the account popup open should not error");
    }

    #[test]
    fn e_on_the_accounts_view_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        assert!(shell.account_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on the accounts view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.account_popup, Some(AccountPopup::Edit(_))));
    }

    #[test]
    fn esc_closes_the_account_edit_popup_without_changing_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");

        shell.update(Action::OpenAccountEditPopup(wallet));
        for c in "!!!".chars() {
            shell.update(Action::AccountEditPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.account_popup.is_none());
        let store = shell.view.account_store().unwrap();
        assert_eq!(store.find(wallet).unwrap().name, "Wallet");
    }

    #[test]
    fn typing_a_new_name_and_ctrl_s_renames_the_account_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");

        shell.update(Action::OpenAccountEditPopup(wallet));
        for c in " Renamed".chars() {
            shell.update(Action::AccountEditPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.account_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.account_store().unwrap();
        assert_eq!(store.find(wallet).unwrap().name, "Wallet Renamed");
    }

    #[test]
    fn edit_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenAccountEditPopup(wallet));
        for _ in 0.."Wallet".chars().count() {
            shell.update(Action::AccountEditPopupBackspace);
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.account_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn h_on_the_type_field_steps_the_account_type_pick() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let everyday = account_id_by_name(&shell, "Everyday Spending"); // Bank
        shell.update(Action::OpenAccountEditPopup(everyday));
        shell.update(Action::AccountEditPopupTab); // name -> type

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('l'),
                KeyModifiers::NONE,
            )))
            .expect("l on a focused type field always maps to an action");
        shell.update(action);

        let ctrl_s = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(ctrl_s);

        let store = shell.view.account_store().unwrap();
        assert_eq!(
            store.find(everyday).unwrap().account_type,
            lib_core::AccountType::CreditCard
        );
    }

    #[test]
    fn ctrl_a_deactivates_the_account_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        assert!(
            shell
                .view
                .account_store()
                .unwrap()
                .find(wallet)
                .unwrap()
                .is_active
        );

        shell.update(Action::OpenAccountEditPopup(wallet));
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.account_popup.is_none());
        let store = shell.view.account_store().unwrap();
        assert!(!store.find(wallet).unwrap().is_active);
    }

    #[test]
    fn renders_the_open_account_edit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenAccountEditPopup(wallet));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the account edit popup open should not error");
    }

    #[test]
    fn d_on_the_accounts_view_opens_the_delete_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        assert!(shell.account_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::NONE,
            )))
            .expect("d on the accounts view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.account_popup, Some(AccountPopup::Delete(_))));
    }

    #[test]
    fn ctrl_d_from_the_edit_popup_swaps_to_the_delete_popup_for_the_same_account() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenAccountEditPopup(wallet));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+d from the edit popup always maps to an action");
        shell.update(action);

        assert!(matches!(shell.account_popup, Some(AccountPopup::Delete(_))));
    }

    #[test]
    fn esc_closes_the_delete_popup_without_deleting_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenAccountDeletePopup(wallet));
        for c in "Wallet".chars() {
            shell.update(Action::AccountDeletePopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.account_popup.is_none());
        assert!(account_exists(&shell, "Wallet"));
    }

    #[test]
    fn typing_the_exact_name_and_ctrl_s_deletes_an_empty_account() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        let original_count = shell.view.account_store().unwrap().accounts().len();

        shell.update(Action::OpenAccountDeletePopup(wallet));
        for c in "Wallet".chars() {
            shell.update(Action::AccountDeletePopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.account_popup.is_none());
        let store = shell.view.account_store().unwrap();
        assert_eq!(store.accounts().len(), original_count - 1);
        assert!(!account_exists(&shell, "Wallet"));
    }

    #[test]
    fn delete_ctrl_s_does_nothing_on_a_non_empty_account_without_a_transfer_target() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let everyday = account_id_by_name(&shell, "Everyday Spending");
        shell.update(Action::OpenAccountDeletePopup(everyday));
        for c in "Everyday Spending".chars() {
            shell.update(Action::AccountDeletePopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None, "no transfer target resolved yet");
        assert!(account_exists(&shell, "Everyday Spending"));
    }

    #[test]
    fn typing_a_target_and_the_exact_name_transfers_then_deletes_a_non_empty_account() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let everyday = account_id_by_name(&shell, "Everyday Spending");
        let mortgage_offset = account_id_by_name(&shell, "Mortgage Offset");

        shell.update(Action::OpenAccountDeletePopup(everyday));
        for c in "Mortgage Offset".chars() {
            shell.update(Action::AccountDeletePopupInput(c));
        }
        shell.update(Action::AccountDeletePopupTab); // move to -> confirm
        for c in "Everyday Spending".chars() {
            shell.update(Action::AccountDeletePopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft and resolved target always maps to an action");
        shell.update(action);

        assert!(shell.account_popup.is_none());
        let store = shell.view.account_store().unwrap();
        assert!(store.find(everyday).is_none());
        let target = store.find(mortgage_offset).expect("target survives");
        assert_eq!(target.transaction_count, 60 + 1_284);
    }

    #[test]
    fn ctrl_a_from_the_delete_popup_deactivates_instead_of_deleting() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenAccountDeletePopup(wallet));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a always maps to an action while the delete popup is open");
        shell.update(action);

        assert!(shell.account_popup.is_none());
        let store = shell.view.account_store().unwrap();
        assert!(account_exists(&shell, "Wallet"), "deactivated, not deleted");
        assert!(!store.find(wallet).unwrap().is_active);
    }

    #[test]
    fn renders_the_open_account_delete_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let everyday = account_id_by_name(&shell, "Everyday Spending");
        shell.update(Action::OpenAccountDeletePopup(everyday));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the account delete popup open should not error");
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
    fn selecting_the_category_command_and_pressing_enter_opens_the_categories_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        // "category" alone is ambiguous (it also substring-matches e.g. `budget new
        // <category> <limit>`) — "categories" (the domain name) narrows to just this domain's
        // own eight commands, with the bare `category` first among them.
        let action = select_and_enter(&mut shell, "categories");
        assert_eq!(action, Action::OpenCategories);
    }

    #[test]
    fn selecting_category_new_while_categories_is_active_opens_the_new_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "category new");
        assert_eq!(action, Action::OpenCategoryNewPopup(income));
    }

    #[test]
    fn selecting_category_edit_while_categories_is_active_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "category edit");
        assert_eq!(action, Action::OpenCategoryEditPopup(income));
    }

    #[test]
    fn selecting_category_move_while_categories_is_active_opens_the_move_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        let income = category_id_by_name(&shell, "Income");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "category move");
        assert_eq!(action, Action::OpenCategoryMovePopup(income));
    }

    #[test]
    fn selecting_category_archive_while_categories_is_active_archives_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCategories);
        // Move the tree selection off the default Income root (`category archive` would just
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
        let action = select_and_enter(&mut shell, "category archive");
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
    fn category_new_edit_move_and_archive_fall_back_to_not_yet_built_when_categories_is_not_active()
    {
        for (filter, name) in [
            ("category new", "category new <name> [parent]"),
            ("category edit", "category edit <cat>"),
            ("category move", "category move <cat> <parent>"),
            ("category archive", "category archive <cat>"),
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
    fn category_rename_merge_and_tree_always_show_not_yet_built() {
        for (filter, name) in [
            ("category rename", "category rename <cat> <name>"),
            ("category merge", "category merge <from> <into>"),
            ("category tree", "category tree [root]"),
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

    #[test]
    fn selecting_the_account_command_and_pressing_enter_opens_the_accounts_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        // "account" alone is ambiguous too (e.g. it substring-matches "account new <name>
        // <type> <unit>" before the bare "account" entry) — "accounts" narrows to just this
        // domain's own seven commands, with the bare "account" entry first among them.
        let action = select_and_enter(&mut shell, "accounts");
        assert_eq!(action, Action::OpenAccounts);
    }

    #[test]
    fn selecting_account_new_opens_the_new_popup_even_without_a_selection_prerequisite() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "account new");
        assert_eq!(action, Action::OpenAccountNewPopup);
    }

    #[test]
    fn selecting_account_edit_while_accounts_is_active_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "account edit");
        assert_eq!(action, Action::OpenAccountEditPopup(wallet));
    }

    #[test]
    fn selecting_account_delete_while_accounts_is_active_opens_the_delete_popup_for_the_selection()
    {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "account delete");
        assert_eq!(action, Action::OpenAccountDeletePopup(wallet));
    }

    #[test]
    fn selecting_account_off_while_accounts_is_active_deactivates_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "account off");
        assert_eq!(
            action,
            Action::SetAccountActive {
                id: wallet,
                active: false
            }
        );
        shell.update(action);
        assert!(
            !shell
                .view
                .account_store()
                .unwrap()
                .find(wallet)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn selecting_account_on_while_accounts_is_active_reactivates_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts);
        let wallet = account_id_by_name(&shell, "Wallet");
        // `za` first, so deactivating Wallet below doesn't hide it and move the selection
        // away — `SetAccountActive`'s own `recover_selection` only moves off a row that's
        // actually no longer visible.
        shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));
        shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        )));

        shell.update(Action::SetAccountActive {
            id: wallet,
            active: false,
        });
        assert_eq!(
            shell.view.account_selection(),
            Some(wallet),
            "za should have kept Wallet selected"
        );
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "account on");
        assert_eq!(
            action,
            Action::SetAccountActive {
                id: wallet,
                active: true
            }
        );
        shell.update(action);
        assert!(
            shell
                .view
                .account_store()
                .unwrap()
                .find(wallet)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn account_new_edit_delete_off_and_on_fall_back_to_not_yet_built_when_accounts_is_not_active() {
        for (filter, name) in [
            ("account edit", "account edit <acct>"),
            ("account delete", "account delete <acct> [into <acct>]"),
            ("account off", "account off <acct>"),
            ("account on", "account on <acct>"),
        ] {
            let mut shell = Shell::new();
            shell.update(Action::OpenCommandPopup);
            let action = select_and_enter(&mut shell, filter);
            assert_eq!(
                action,
                Action::CommandPopupSetNotYetBuilt(name),
                "{filter} should fall back when Accounts isn't the active view"
            );
        }

        // "account new" is the one exception — it has no selection prerequisite, so it falls
        // back only when Accounts itself isn't active, which is exactly this case.
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        let action = select_and_enter(&mut shell, "account new");
        assert_eq!(
            action,
            Action::CommandPopupSetNotYetBuilt("account new <name> <type> <unit>")
        );
    }

    #[test]
    fn account_check_always_shows_not_yet_built() {
        let mut shell = Shell::new();
        shell.update(Action::OpenAccounts); // even with Accounts active...
        shell.update(Action::OpenCommandPopup);
        let action = select_and_enter(&mut shell, "account check");
        assert_eq!(
            action,
            Action::CommandPopupSetNotYetBuilt("account check <acct> <amount> [date]"),
            "account check should have no real dispatch — reconcile is out of this map's destination"
        );
    }

    #[test]
    fn n_on_the_tags_view_opens_the_new_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        assert!(shell.tag_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::NONE,
            )))
            .expect("n on the tags view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.tag_popup, Some(TagPopup::New(_))));
    }

    #[test]
    fn esc_closes_the_tag_popup_without_creating_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let original_count = shell.view.tag_store().unwrap().tags().len();

        shell.update(Action::OpenTagNewPopup);
        for c in "Side Project".chars() {
            shell.update(Action::TagNewPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.tag_popup.is_none());
        assert_eq!(shell.view.tag_store().unwrap().tags().len(), original_count);
        assert!(!tag_exists(&shell, "Side Project"));
    }

    #[test]
    fn typing_a_name_then_ctrl_s_creates_the_tag_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let original_count = shell.view.tag_store().unwrap().tags().len();

        shell.update(Action::OpenTagNewPopup);
        for c in "Side Project".chars() {
            shell.update(Action::TagNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.tag_popup.is_none(), "popup should close after ctrl+s");
        let store = shell.view.tag_store().unwrap();
        assert_eq!(store.tags().len(), original_count + 1);
        assert!(tag_exists(&shell, "Side Project"));
    }

    #[test]
    fn tag_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        shell.update(Action::OpenTagNewPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.tag_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn tag_ctrl_s_does_nothing_while_the_name_clashes_case_insensitively() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        shell.update(Action::OpenTagNewPopup);
        // The fixture seeds "Japan Trip 2026" — a clash regardless of case.
        for c in "japan trip 2026".chars() {
            shell.update(Action::TagNewPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            action, None,
            "a case-insensitive clash should never validate"
        );
    }

    #[test]
    fn ctrl_a_creates_and_keeps_the_tag_popup_open_reset_for_another_tag() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);

        shell.update(Action::OpenTagNewPopup);
        for c in "First".chars() {
            shell.update(Action::TagNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.tag_popup.is_some(),
            "popup should stay open after ctrl+a"
        );
        assert!(tag_exists(&shell, "First"));

        for c in "Second".chars() {
            shell.update(Action::TagNewPopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with the second tag's valid draft always maps to an action");
        shell.update(action);

        assert!(tag_exists(&shell, "Second"));
        assert!(shell.tag_popup.is_none());
    }

    #[test]
    fn renders_the_open_tag_new_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        shell.update(Action::OpenTagNewPopup);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the tag popup open should not error");
    }

    #[test]
    fn e_on_the_tags_view_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        assert!(shell.tag_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on the tags view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.tag_popup, Some(TagPopup::Edit(_))));
    }

    #[test]
    fn esc_closes_the_tag_edit_popup_without_changing_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");

        shell.update(Action::OpenTagEditPopup(home_renovation));
        for c in "!!!".chars() {
            shell.update(Action::TagEditPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.tag_popup.is_none());
        let store = shell.view.tag_store().unwrap();
        assert_eq!(store.find(home_renovation).unwrap().name, "Home Renovation");
    }

    #[test]
    fn typing_a_new_name_and_ctrl_s_renames_the_tag_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");

        shell.update(Action::OpenTagEditPopup(home_renovation));
        for c in " Reno".chars() {
            shell.update(Action::TagEditPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.tag_popup.is_none(), "popup should close after ctrl+s");
        let store = shell.view.tag_store().unwrap();
        assert_eq!(
            store.find(home_renovation).unwrap().name,
            "Home Renovation Reno"
        );
    }

    #[test]
    fn tag_edit_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        shell.update(Action::OpenTagEditPopup(home_renovation));
        for _ in 0.."Home Renovation".chars().count() {
            shell.update(Action::TagEditPopupBackspace);
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.tag_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn tag_edit_ctrl_s_does_nothing_while_the_rename_clashes_case_insensitively() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        shell.update(Action::OpenTagEditPopup(home_renovation));
        for _ in 0.."Home Renovation".chars().count() {
            shell.update(Action::TagEditPopupBackspace);
        }
        // The fixture seeds "Tax Deductible" — a clash regardless of case.
        for c in "tax deductible".chars() {
            shell.update(Action::TagEditPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            action, None,
            "a case-insensitive clash should never validate"
        );
    }

    #[test]
    fn ctrl_a_deactivates_the_tag_without_toggling_the_checkbox_first_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        assert!(
            shell
                .view
                .tag_store()
                .unwrap()
                .find(home_renovation)
                .unwrap()
                .is_active
        );

        shell.update(Action::OpenTagEditPopup(home_renovation));
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.tag_popup.is_none());
        let store = shell.view.tag_store().unwrap();
        assert!(!store.find(home_renovation).unwrap().is_active);
    }

    #[test]
    fn renders_the_open_tag_edit_popup_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        shell.update(Action::OpenTagEditPopup(home_renovation));

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the tag edit popup open should not error");
    }

    #[test]
    fn selecting_tag_opens_the_tags_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        let action = select_and_enter(&mut shell, "tag");
        assert_eq!(action, Action::OpenTags);
    }

    #[test]
    fn selecting_tag_new_opens_the_new_popup_even_without_a_selection_prerequisite() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "tag new");
        assert_eq!(action, Action::OpenTagNewPopup);
    }

    #[test]
    fn selecting_tag_edit_while_tags_is_active_opens_the_edit_popup_for_the_selection() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "tag edit");
        assert_eq!(action, Action::OpenTagEditPopup(home_renovation));
    }

    #[test]
    fn selecting_tag_off_while_tags_is_active_deactivates_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "tag off");
        assert_eq!(
            action,
            Action::SetTagActive {
                id: home_renovation,
                active: false
            }
        );
        shell.update(action);
        assert!(
            !shell
                .view
                .tag_store()
                .unwrap()
                .find(home_renovation)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn selecting_tag_on_while_tags_is_active_reactivates_the_selection_immediately() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let home_renovation = tag_id_by_name(&shell, "Home Renovation");
        // `za` first, so deactivating Home Renovation below doesn't hide it and move the
        // selection away — mirrors `selecting_account_on_while_accounts_is_active_
        // reactivates_the_selection_immediately`'s own reasoning.
        shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));
        shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        )));

        shell.update(Action::SetTagActive {
            id: home_renovation,
            active: false,
        });
        assert_eq!(
            shell.view.tag_selection(),
            Some(home_renovation),
            "za should have kept Home Renovation selected"
        );
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "tag on");
        assert_eq!(
            action,
            Action::SetTagActive {
                id: home_renovation,
                active: true
            }
        );
        shell.update(action);
        assert!(
            shell
                .view
                .tag_store()
                .unwrap()
                .find(home_renovation)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn selecting_tag_delete_while_tags_is_active_arms_the_same_confirm_the_bare_d_key_does() {
        let mut shell = Shell::new();
        shell.update(Action::OpenTags);
        let before_count = shell.view.tag_store().unwrap().tags().len();
        shell.update(Action::OpenCommandPopup);

        let action = select_and_enter(&mut shell, "tag delete");
        assert_eq!(action, Action::ArmTagDelete);
        shell.update(action);

        // Arming alone never deletes anything — only `y` on the Tags view itself does.
        assert_eq!(shell.view.tag_store().unwrap().tags().len(), before_count);
    }

    #[test]
    fn tag_new_edit_off_on_and_delete_fall_back_to_not_yet_built_when_tags_is_not_active() {
        for (filter, name) in [
            ("tag edit", "tag edit <tag>"),
            ("tag off", "tag off <tag>"),
            ("tag on", "tag on <tag>"),
            ("tag delete", "tag delete <tag>"),
        ] {
            let mut shell = Shell::new();
            shell.update(Action::OpenCommandPopup);
            let action = select_and_enter(&mut shell, filter);
            assert_eq!(
                action,
                Action::CommandPopupSetNotYetBuilt(name),
                "{filter} should fall back when Tags isn't the active view"
            );
        }

        // "tag new" is the one exception — it has no selection prerequisite, so it falls back
        // only when Tags itself isn't active, which is exactly this case.
        let mut shell = Shell::new();
        shell.update(Action::OpenCommandPopup);
        let action = select_and_enter(&mut shell, "tag new");
        assert_eq!(action, Action::CommandPopupSetNotYetBuilt("tag new <name>"));
    }

    #[test]
    fn n_on_the_payees_view_opens_the_new_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        assert!(shell.payee_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::NONE,
            )))
            .expect("n on the payees view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::New(_))));
    }

    #[test]
    fn esc_closes_the_payee_popup_without_creating_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let original_count = shell.view.payee_store().unwrap().payees().len();

        shell.update(Action::OpenPayeeNewPopup);
        for c in "New Vendor".chars() {
            shell.update(Action::PayeeNewPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.payee_popup.is_none());
        assert_eq!(
            shell.view.payee_store().unwrap().payees().len(),
            original_count
        );
        assert!(!payee_exists(&shell, "New Vendor"));
    }

    #[test]
    fn typing_a_name_then_ctrl_s_creates_the_payee_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let original_count = shell.view.payee_store().unwrap().payees().len();

        shell.update(Action::OpenPayeeNewPopup);
        for c in "New Vendor".chars() {
            shell.update(Action::PayeeNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.payee_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.payee_store().unwrap();
        assert_eq!(store.payees().len(), original_count + 1);
        assert!(payee_exists(&shell, "New Vendor"));
    }

    #[test]
    fn payee_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        shell.update(Action::OpenPayeeNewPopup);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.payee_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn payee_ctrl_s_does_nothing_while_the_name_collides() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        shell.update(Action::OpenPayeeNewPopup);
        // "Woolworths" is one of `PayeeFixture`'s own seeded payees.
        for c in "Woolworths".chars() {
            shell.update(Action::PayeeNewPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None, "a colliding name should never validate");
        assert!(shell.payee_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn ctrl_a_creates_and_keeps_the_payee_popup_open_reset_for_another_payee() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);

        shell.update(Action::OpenPayeeNewPopup);
        for c in "First Vendor".chars() {
            shell.update(Action::PayeeNewPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.payee_popup.is_some(),
            "popup should stay open after ctrl+a"
        );
        assert!(payee_exists(&shell, "First Vendor"));

        for c in "Second Vendor".chars() {
            shell.update(Action::PayeeNewPopupInput(c));
        }
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.payee_popup.is_none());
        assert!(payee_exists(&shell, "Second Vendor"));
    }

    #[test]
    fn e_on_the_payees_view_opens_the_edit_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        assert!(shell.payee_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("e on the payees view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::Edit(_))));
    }

    #[test]
    fn esc_closes_the_payee_edit_popup_without_saving_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");

        shell.update(Action::OpenPayeeEditPopup(woolworths));
        for c in " Renamed".chars() {
            shell.update(Action::PayeeEditPopupInput(c));
        }
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.payee_popup.is_none());
        let store = shell.view.payee_store().unwrap();
        assert_eq!(store.find(woolworths).unwrap().name, "Woolworths");
    }

    #[test]
    fn typing_a_new_name_and_ctrl_s_renames_the_payee_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        let aliases_before = shell.view.payee_store().unwrap().aliases(woolworths).len();

        shell.update(Action::OpenPayeeEditPopup(woolworths));
        for c in " Group".chars() {
            shell.update(Action::PayeeEditPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        assert!(
            shell.payee_popup.is_none(),
            "popup should close after ctrl+s"
        );
        let store = shell.view.payee_store().unwrap();
        assert_eq!(store.find(woolworths).unwrap().name, "Woolworths Group");
        // The rename should have left a fresh alias behind for the prior name.
        assert_eq!(store.aliases(woolworths).len(), aliases_before + 1);
    }

    #[test]
    fn payee_edit_ctrl_s_does_nothing_while_the_name_is_empty() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeEditPopup(woolworths));
        for _ in 0.."Woolworths".chars().count() {
            shell.update(Action::PayeeEditPopupBackspace);
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None);
        assert!(shell.payee_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn payee_edit_ctrl_s_does_nothing_while_the_new_name_collides() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let coles_central = payee_id_by_name(&shell, "Coles Central");
        shell.update(Action::OpenPayeeEditPopup(coles_central));
        for _ in 0.."Coles Central".chars().count() {
            shell.update(Action::PayeeEditPopupBackspace);
        }
        for c in "Woolworths".chars() {
            shell.update(Action::PayeeEditPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None, "a colliding name should never validate");
        assert!(shell.payee_popup.is_some(), "popup should stay open");
    }

    #[test]
    fn payee_edit_ctrl_a_deactivates_the_payee_and_closes_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeEditPopup(woolworths));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+a with a valid draft always maps to an action");
        shell.update(action);

        assert!(shell.payee_popup.is_none());
        let store = shell.view.payee_store().unwrap();
        assert!(!store.find(woolworths).unwrap().is_active);
    }

    #[test]
    fn m_on_the_payees_view_opens_the_matches_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        assert!(shell.payee_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('m'),
                KeyModifiers::NONE,
            )))
            .expect("m on the payees view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::Matches(_))));
    }

    #[test]
    fn m_on_the_payee_edit_popup_jumps_to_the_matches_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeEditPopup(woolworths));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('m'),
                KeyModifiers::NONE,
            )))
            .expect("m always maps to an action while the edit popup is open");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::Matches(_))));
    }

    #[test]
    fn adding_a_fresh_pattern_saves_it_as_a_manual_alias_and_returns_to_the_list() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let coles_central = payee_id_by_name(&shell, "Coles Central");
        let aliases_before = shell
            .view
            .payee_store()
            .unwrap()
            .aliases(coles_central)
            .len();

        shell.update(Action::OpenPayeeMatchesPopup(coles_central));
        shell.update(Action::PayeeMatchesPopupBeginAdd);
        for c in "Coles Supermarket".chars() {
            shell.update(Action::PayeeMatchesPopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid draft always maps to an action");
        shell.update(action);

        let store = shell.view.payee_store().unwrap();
        let aliases = store.aliases(coles_central);
        assert_eq!(aliases.len(), aliases_before + 1);
        assert!(
            aliases
                .iter()
                .any(|alias| alias.pattern == "(?i)^Coles Supermarket$")
        );
        // The popup stays open (list mode), just with its compose slot cleared.
        assert!(matches!(shell.payee_popup, Some(PayeePopup::Matches(_))));
    }

    #[test]
    fn adding_a_colliding_pattern_does_nothing() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let coles_central = payee_id_by_name(&shell, "Coles Central");

        shell.update(Action::OpenPayeeMatchesPopup(coles_central));
        shell.update(Action::PayeeMatchesPopupBeginAdd);
        for c in "Woolworths".chars() {
            shell.update(Action::PayeeMatchesPopupInput(c));
        }

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(action, None, "a colliding pattern should never validate");
    }

    #[test]
    fn d_removes_a_manual_alias_but_refuses_on_a_rename_sourced_one() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        let aliases_before = shell.view.payee_store().unwrap().aliases(woolworths).len();

        shell.update(Action::OpenPayeeMatchesPopup(woolworths));
        // Selection starts on index 0 — the seeded rename alias.
        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            action, None,
            "removing a rename-sourced alias should refuse"
        );
        assert_eq!(
            shell.view.payee_store().unwrap().aliases(woolworths).len(),
            aliases_before
        );

        // Move to index 1 — the seeded manual alias — and remove it for real.
        shell.update(Action::PayeeMatchesPopupMoveDown);
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::NONE,
            )))
            .expect("removing a manual alias always maps to an action");
        shell.update(action);

        assert_eq!(
            shell.view.payee_store().unwrap().aliases(woolworths).len(),
            aliases_before - 1
        );
    }

    #[test]
    fn e_refuses_on_a_rename_sourced_alias_and_edits_a_manual_one() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");

        shell.update(Action::OpenPayeeMatchesPopup(woolworths));
        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, None, "editing a rename-sourced alias should refuse");

        shell.update(Action::PayeeMatchesPopupMoveDown); // onto the manual alias
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('e'),
                KeyModifiers::NONE,
            )))
            .expect("editing a manual alias always maps to an action");
        shell.update(action);

        let Some(PayeePopup::Matches(popup)) = &shell.payee_popup else {
            panic!("expected the matches popup to still be open, composing");
        };
        assert!(popup.is_composing());
    }

    #[test]
    fn esc_cancels_the_compose_draft_without_closing_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeMatchesPopup(woolworths));
        shell.update(Action::PayeeMatchesPopupBeginAdd);
        shell.update(Action::PayeeMatchesPopupInput('x'));

        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while composing"));

        let Some(PayeePopup::Matches(popup)) = &shell.payee_popup else {
            panic!("expected the matches popup to still be open");
        };
        assert!(!popup.is_composing());

        // A second `Esc`, with nothing composing, closes the popup itself.
        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action"));
        assert!(shell.payee_popup.is_none());
    }

    #[test]
    fn bare_a_on_the_payees_view_deactivates_the_selection_directly() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let selected = shell
            .view
            .payee_selection()
            .expect("the payees view always starts with a selection");
        assert!(
            shell
                .view
                .payee_store()
                .unwrap()
                .find(selected)
                .unwrap()
                .is_active
        );

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            )))
            .expect("a on the payees view always maps to an action");
        shell.update(action);

        assert!(shell.payee_popup.is_none(), "no popup involved");
        assert!(
            !shell
                .view
                .payee_store()
                .unwrap()
                .find(selected)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn d_on_the_payees_view_opens_the_delete_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        assert!(shell.payee_popup.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::NONE,
            )))
            .expect("d on the payees view always maps to an action");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::Delete(_))));
    }

    #[test]
    fn ctrl_d_on_the_payee_edit_popup_jumps_to_the_delete_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeEditPopup(woolworths));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+d always maps to an action while the edit popup is open");
        shell.update(action);

        assert!(matches!(shell.payee_popup, Some(PayeePopup::Delete(_))));
    }

    #[test]
    fn ctrl_s_on_the_delete_popup_deactivates_by_default() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeDeletePopup(woolworths));

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with deactivate preselected always maps to an action");
        shell.update(action);

        assert!(shell.payee_popup.is_none());
        assert!(
            !shell
                .view
                .payee_store()
                .unwrap()
                .find(woolworths)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn a_referenced_payee_can_never_be_switched_to_delete_via_the_popup() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeDeletePopup(woolworths));

        // Space on the `action` field would normally toggle the radio, but a referenced
        // payee can never select `delete` — the popup itself refuses the switch.
        shell.update(Action::PayeeDeletePopupInput(' '));
        shell.update(Action::PayeeDeletePopupTab); // action -> confirm
        for c in "Woolworths".chars() {
            shell.update(Action::PayeeDeletePopupInput(c));
        }

        // Even with the exact name typed, ctrl+s never deletes it — deactivate is still the
        // only thing selected.
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with deactivate selected always maps to an action");
        shell.update(action);

        let store = shell.view.payee_store().unwrap();
        assert!(store.find(woolworths).is_some(), "still there, not deleted");
        assert!(
            !store.find(woolworths).unwrap().is_active,
            "deactivated instead"
        );
    }

    #[test]
    fn deleting_an_unreferenced_payee_requires_the_exact_name_and_removes_it() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);

        // "Old Vendor" is inactive and hidden by default — reveal it with `za` before it can
        // be found by name.
        // The lone `z` only arms the chord (`PayeesView::handle_key` mutates its own
        // `pending_z` in place and returns `None` — there's nothing for `Shell` to dispatch
        // yet); the following `a` completes it and does map to an action.
        shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            )))
            .expect("completing the za chord always maps to an action");
        shell.update(action);

        let old_vendor = payee_id_by_name(&shell, "Old Vendor");
        shell.update(Action::OpenPayeeDeletePopup(old_vendor));
        shell.update(Action::PayeeDeletePopupInput(' ')); // select delete — now allowed
        shell.update(Action::PayeeDeletePopupTab); // action -> confirm
        for c in "Old Vendor".chars() {
            shell.update(Action::PayeeDeletePopupInput(c));
        }

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+s with a valid delete draft always maps to an action");
        shell.update(action);

        assert!(shell.payee_popup.is_none());
        assert!(shell.view.payee_store().unwrap().find(old_vendor).is_none());
    }

    #[test]
    fn m_on_the_delete_popup_jumps_to_the_matches_popup_but_not_while_typing_confirm() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeDeletePopup(woolworths));
        shell.update(Action::PayeeDeletePopupTab); // action -> confirm

        // `m` while `confirm` has focus must be typed, not hijacked — "Woolworths" itself
        // has no `m`, but a future Payee name could, so this proves the guard rather than
        // relying on the fixture's own names never containing one.
        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('m'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::PayeeDeletePopupInput('m')));

        shell.update(Action::PayeeDeletePopupTab); // confirm -> action
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('m'),
                KeyModifiers::NONE,
            )))
            .expect("m on the action field always maps to an action");
        shell.update(action);
        assert!(matches!(shell.payee_popup, Some(PayeePopup::Matches(_))));
    }

    #[test]
    fn esc_closes_the_delete_popup_without_mutating_anything() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPayees);
        let woolworths = payee_id_by_name(&shell, "Woolworths");
        shell.update(Action::OpenPayeeDeletePopup(woolworths));

        let action = shell.map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        shell.update(action.expect("esc always maps to an action while the popup is open"));

        assert!(shell.payee_popup.is_none());
        assert!(
            shell
                .view
                .payee_store()
                .unwrap()
                .find(woolworths)
                .unwrap()
                .is_active
        );
    }
}
