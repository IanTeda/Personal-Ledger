//! The Accounts destination's Entities (ADR-0032, ADR-0037). [`AccountsStore`] reads the rows
//! through [`LedgerStore`], which holds the only copy. [`AccountsView`] is Settings' Accounts page:
//! it owns the selection, renders itself (`impl Render` in `settings::accounts`), handles the
//! page's keys and movement, builds its dialogs and reaches `Shell` only through [`ViewEvent`]. The
//! Settings View forwards to it while the page is open; `Shell`'s Dialog host draws and confirms
//! the dialogs it builds.

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_accounts::{Account, NameLookup, display_order, find_by_name};
use lib_ledger::Domain;

use super::hints::accounts_hints;
use crate::{
    accounts::form::{AccountForm, AccountOptions, AccountsDialog, DeleteAccountForm},
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    form::field::TextField,
    ledger_store::{LedgerChanged, LedgerStore, notify_on},
    navigation::{command::AccountsVerb, key_router::Movement},
    view::event::{Reveal, ViewEvent},
};

/// The shared Accounts rows. Every reader (the Accounts page, Transactions, Budgets, Bills and
/// Documents) reads them from here, so a change is seen everywhere on the next render.
pub struct AccountsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl AccountsStore {
    /// A store over `ledger`'s Accounts.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Accounts) {
                cx.notify();
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn accounts<'a>(&self, cx: &'a App) -> &'a [Account] {
        self.ledger.read(cx).ledger().accounts()
    }
}

/// The three selects' option lists, read live from Settings (so an institution added there
/// appears in the Accounts dialogs) and the fixed type order.
pub fn dialog_options(ledger: &Entity<LedgerStore>, cx: &App) -> AccountOptions {
    AccountOptions::new(
        ledger
            .read(cx)
            .ledger()
            .institutions()
            .iter()
            .map(|institution| institution.name.clone())
            .collect(),
        ledger
            .read(cx)
            .ledger()
            .units()
            .iter()
            .map(|unit| unit.code.clone())
            .collect(),
    )
}

/// The stores the Accounts page reads: its rows, and the Ledger (the Units its heading reads and
/// the Units and Institutions its dialogs offer).
pub struct AccountsSources {
    pub accounts: Entity<AccountsStore>,
    pub ledger: Entity<LedgerStore>,
}

impl EventEmitter<ViewEvent> for AccountsView {}

/// The Accounts page's own state: which row is selected. The selection is a position in
/// [`display_order`], not an index into the rows, because the page groups accounts by type.
pub struct AccountsView {
    sources: AccountsSources,
    selected: usize,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Units its heading reads change.
    _units_changed: Subscription,
}

impl AccountsView {
    pub fn new(sources: AccountsSources, cx: &mut Context<'_, Self>) -> Self {
        let _units_changed = notify_on(&sources.ledger, Domain::Units, cx);
        Self {
            sources,
            selected: 0,
            page_focused: false,
            _units_changed,
        }
    }

    pub(crate) fn sources(&self) -> &AccountsSources {
        &self.sources
    }

    pub(crate) fn accounts<'a>(&self, cx: &'a App) -> &'a [Account] {
        self.sources.accounts.read(cx).accounts(cx)
    }

    pub fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page is open (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The page claims no key ahead of the router: its `n`/`e`/`d` are router leftovers.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page's bare `n`, `e` and `d`, each through the same handler its button uses. Returns
    /// whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_dialog("", cx),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_edit_dialog(id, cx);
                }
            }
            "d" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_delete_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the row selection. `enter` opens the selected
    /// account's ledger: Transactions filtered to it.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let len = self.accounts(cx).len();
        match movement.step() {
            Some(step) => self.selected = lib_list_cursor::step(self.selected, len, step),
            None => {
                if let Some(id) = self.selected_id(cx) {
                    open_ledger(id, cx);
                }
            }
        }
        cx.notify();
    }

    /// The page has nothing of its own to close: `Esc` is left to Settings, which steps back to
    /// the index.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Opening the page keeps its selection, so returning finds the same row selected.
    pub fn on_enter(&mut self, _cx: &mut Context<'_, Self>) {}

    /// Selects an Account another View handed off.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::Account(id) = reveal {
            self.select_id(id, cx);
        }
    }

    /// The page's legend and its account count.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: accounts_hints(),
            right: crate::msg::desktop_accounts_count(
                i64::try_from(self.accounts(cx).len()).unwrap_or(i64::MAX),
            ),
        })
    }

    // -----------------------------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------------------------

    /// The selected account's index in the store's rows, `None` when there are none. The stored
    /// position is clamped, so removing accounts can never leave it pointing past the end.
    pub fn selected_index(&self, cx: &App) -> Option<usize> {
        let order = display_order(self.accounts(cx));
        order
            .get(self.selected.min(order.len().saturating_sub(1)))
            .copied()
    }

    /// The selected account's id, or `None` when the page has no rows.
    pub fn selected_id(&self, cx: &App) -> Option<u32> {
        let index = self.selected_index(cx)?;
        self.accounts(cx).get(index).map(|account| account.id)
    }

    /// Selects the row for account `id`, if it still exists.
    pub fn select_id(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let rows = self.accounts(cx);
        let Some(index) = rows.iter().position(|account| account.id == id) else {
            return;
        };
        if let Some(position) = display_order(rows).iter().position(|&i| i == index) {
            self.selected = position;
            cx.notify();
        }
    }

    /// Keeps the selection in range after rows are removed, so it lands on the account that
    /// slid into the deleted row's place (or the last one).
    pub fn clamp_selection(&mut self, cx: &mut Context<'_, Self>) {
        let len = self.accounts(cx).len();
        self.selected = self.selected.min(len.saturating_sub(1));
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on an account row: selects it and, as `enter` does, opens its ledger.
    pub(crate) fn click_row(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        open_ledger(id, cx);
    }

    /// **+ Add account**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog("", cx);
    }

    pub(crate) fn click_edit(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_delete(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_delete_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs and the `accounts` command
    // -----------------------------------------------------------------------------------------

    fn dialog_options(&self, cx: &App) -> AccountOptions {
        dialog_options(&self.sources.ledger, cx)
    }

    /// Opens the Add account dialog on a fresh form (Unit starting on Settings' default Unit),
    /// with Name pre-filled from `name` -- empty for `n` and the button, the typed argument for
    /// `accounts new <account name>`.
    fn open_add_dialog(&mut self, name: &str, cx: &mut Context<'_, Self>) {
        let options = self.dialog_options(cx);
        let default_unit = self
            .sources
            .ledger
            .read(cx)
            .ledger()
            .units()
            .iter()
            .find(|unit| unit.is_default)
            .map(|unit| unit.code.as_str());
        let mut form = AccountForm::new(&options, default_unit);
        form.name = TextField::new(name.trim());
        open_dialog(AccountsDialog::Add(form), cx);
    }

    /// Opens the Edit account dialog on `id`, pre-filled. A no-op if the account is gone.
    fn open_edit_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let options = self.dialog_options(cx);
        let Some(account) = self.accounts(cx).iter().find(|account| account.id == id) else {
            return;
        };
        let form = AccountForm::from_account(account, &options);
        open_dialog(AccountsDialog::Edit(id, form), cx);
    }

    /// Opens the Delete account dialog on `id`. A no-op if the account is gone.
    fn open_delete_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(account) = self.accounts(cx).iter().find(|account| account.id == id) else {
            return;
        };
        let form = DeleteAccountForm::new(account.name.as_str());
        open_dialog(AccountsDialog::Delete(id, form), cx);
    }

    /// `accounts new|edit|delete [<account name>]`, once `Shell` has opened the page: opens the
    /// same dialog the page's `n`/`e`/`d` and buttons do. `new` pre-fills Name with the argument.
    /// `edit` and `delete` resolve the typed name ([`find_by_name`]), or use the selected row when
    /// none is given; a name that fits nothing or several accounts puts a status-line message
    /// naming the problem rather than guessing.
    pub fn run_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        if verb == AccountsVerb::New {
            self.open_add_dialog(argument, cx);
            return;
        }
        let command = format!(":{command_name}");
        let index = if argument.is_empty() {
            self.selected_index(cx)
                .ok_or_else(|| crate::msg::desktop_status_no_accounts(&command))
        } else {
            match find_by_name(self.accounts(cx), argument) {
                NameLookup::Found(index) => Ok(index),
                NameLookup::NotFound => Err(crate::msg::desktop_status_no_account_named(
                    &command, argument,
                )),
                NameLookup::Ambiguous(names) => Err(crate::msg::desktop_status_account_ambiguous(
                    &command,
                    argument,
                    &names.join(", "),
                )),
            }
        };
        let index = match index {
            Ok(index) => index,
            Err(message) => {
                cx.emit(ViewEvent::SetStatus(Some(message)));
                return;
            }
        };
        let id = self.accounts(cx)[index].id;
        self.select_id(id, cx);
        match verb {
            AccountsVerb::Edit => self.open_edit_dialog(id, cx),
            AccountsVerb::Delete => self.open_delete_dialog(id, cx),
            AccountsVerb::New => {}
        }
    }
}

/// Asks `Shell` for the account's ledger: Transactions filtered to it, from their first row. The
/// selection is untouched, so returning to Accounts finds the same row selected.
fn open_ledger(id: u32, cx: &mut Context<'_, AccountsView>) {
    cx.emit(ViewEvent::Reveal(Reveal::AccountTransactions(id)));
}

/// Asks `Shell`'s Dialog host to open an Accounts dialog.
fn open_dialog(dialog: AccountsDialog, cx: &mut Context<'_, AccountsView>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Accounts(
        dialog,
    ))));
}
