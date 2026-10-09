//! The Accounts destination's Entities (ADR-0032). [`AccountsStore`] owns the rows, read through
//! `lib_accounts`' service; [`AccountsView`] owns the page's selection and reads the rows from
//! the store. The view reports what the user asked for as [`AccountsEvent`]s, and `Shell` opens
//! the dialogs, since the dialog host is `Shell`'s.

use gpui::{App, Context, Entity, EventEmitter};
use lib_accounts::{Account, AccountService, display_order};

use crate::{accounts::step_selection, navigation::key_router::Movement};

/// The shared Accounts rows. Every reader (the Accounts page, Transactions, Budgets, Bills and
/// Documents) reads them from here, so a change is seen everywhere on the next render.
pub struct AccountsStore {
    service: AccountService,
}

impl AccountsStore {
    /// A store seeded with the mock rows, as the Desktop has no persistence yet.
    pub fn seeded() -> Self {
        Self {
            service: AccountService::seeded(),
        }
    }

    pub fn accounts(&self) -> &[Account] {
        self.service.accounts()
    }

    pub fn service(&self) -> &AccountService {
        &self.service
    }

    /// Applies `change` to the rows, then tells every subscriber they changed. Mutations go
    /// through here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut AccountService) -> R,
    ) -> R {
        let result = change(&mut self.service);
        cx.notify();
        result
    }
}

/// `Ctrl-d`/`Ctrl-u` on the Accounts page: half of a typical screenful of rows.
pub const ACCOUNTS_HALF_PAGE: isize = 5;

/// What the Accounts page asks `Shell` to do. The page never opens a dialog itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsEvent {
    /// `n`: open the Add account dialog.
    Add,
    /// `e`, or the row's Edit button, for account `id`.
    Edit(u32),
    /// `d`, or the row's Delete button, for account `id`.
    Delete(u32),
    /// `enter`: open Transactions filtered to account `id`.
    OpenLedger(u32),
}

impl EventEmitter<AccountsEvent> for AccountsView {}

/// The Accounts page's own state: which row is selected. The selection is a position in
/// [`display_order`], not an index into the rows, because the page groups accounts by type.
pub struct AccountsView {
    store: Entity<AccountsStore>,
    selected: usize,
}

impl AccountsView {
    pub fn new(store: Entity<AccountsStore>) -> Self {
        Self { store, selected: 0 }
    }

    /// The stored selection position, as [`display_order`] counts it. Clamp it with
    /// [`Self::selected_index`] before using it.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The selected account's index in the store's rows, `None` when there are none. The stored
    /// position is clamped, so removing accounts can never leave it pointing past the end.
    pub fn selected_index(&self, cx: &App) -> Option<usize> {
        let order = display_order(self.store.read(cx).accounts());
        order
            .get(self.selected.min(order.len().saturating_sub(1)))
            .copied()
    }

    /// The selected account's id, or `None` when the page has no rows.
    pub fn selected_id(&self, cx: &App) -> Option<u32> {
        let index = self.selected_index(cx)?;
        self.store
            .read(cx)
            .accounts()
            .get(index)
            .map(|account| account.id)
    }

    /// Selects the row for account `id`, if it still exists.
    pub fn select_id(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let rows = self.store.read(cx).accounts();
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
        let len = self.store.read(cx).accounts().len();
        self.selected = self.selected.min(len.saturating_sub(1));
        cx.notify();
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the row selection. `enter` asks for the selected
    /// account's ledger.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let len = self.store.read(cx).accounts().len();
        let selected = self.selected;
        self.selected = match movement {
            Movement::Next => step_selection(selected, len, 1),
            Movement::Prev => step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                if let Some(id) = self.selected_id(cx) {
                    cx.emit(AccountsEvent::OpenLedger(id));
                }
                selected
            }
        };
        cx.notify();
    }

    /// The page's bare `n`, `e` and `d`. Returns whether the key was one of them.
    pub fn handle_key(&mut self, key: &str, cx: &mut Context<'_, Self>) -> bool {
        match key {
            "n" => cx.emit(AccountsEvent::Add),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    cx.emit(AccountsEvent::Edit(id));
                }
            }
            "d" => {
                if let Some(id) = self.selected_id(cx) {
                    cx.emit(AccountsEvent::Delete(id));
                }
            }
            _ => return false,
        }
        true
    }
}
