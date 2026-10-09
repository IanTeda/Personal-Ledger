//! The Accounts destination's wiring: keys, clicks, the Add, Edit and Delete dialogs, and the `accounts` command. The pure rules live in `accounts`; the chrome in `view::accounts`, and its View state in the `AccountsView` Entity.

use super::{Shell, delete_account};

use chrono::Local;

use crate::{
    accounts::{
        self, NameLookup,
        form::{AccountField, AccountForm, AccountOptions, AccountsDialog, DeleteAccountForm},
    },
    chrome::dialog_host::OpenDialog,
    form::field::TextField,
    navigation::command::AccountsVerb,
    navigation::key_router::Movement,
    navigation::nav::{FocusZone, Noun},
    settings::{SettingsFocus, SettingsSection},
    transactions::{edit_transactions, query::TransactionFilters},
    view::accounts::state::AccountsEvent,
};

use gpui::{App, Context, Keystroke};

impl Shell {
    /// Whether the Accounts rows own the keyboard: Settings' Accounts page with focus in the page
    /// rather than on the index.
    pub(super) fn accounts_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Accounts
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the Accounts page's row selection instead of
    /// scrolling it; `Enter` opens the account's ledger (Transactions filtered to it).
    pub(super) fn apply_accounts_movement(
        &mut self,
        movement: Movement,
        cx: &mut Context<'_, Self>,
    ) {
        self.accounts_view
            .update(cx, |view, cx| view.apply_movement(movement, cx));
    }

    /// Carries out what the Accounts page asked for. The page holds only its selection; the
    /// dialogs and the ledger jump are `Shell`'s.
    pub(super) fn handle_accounts_event(
        &mut self,
        event: AccountsEvent,
        cx: &mut Context<'_, Self>,
    ) {
        match event {
            AccountsEvent::Add => self.open_add_account_dialog(""),
            AccountsEvent::Edit(id) => self.open_edit_account_dialog(id, cx),
            AccountsEvent::Delete(id) => self.open_delete_account_dialog(id, cx),
            AccountsEvent::OpenLedger(id) => self.open_account_ledger(id, cx),
        }
        cx.notify();
    }

    /// The Accounts page's own `n`/`e`/`d` (only while it is the active noun and the view has
    /// focus, in `Normal` mode -- `route_key` hands back `NoOp` for these bare keys). Each goes
    /// through the same handler its button does.
    pub(super) fn handle_accounts_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.accounts_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        self.accounts_view
            .update(cx, |view, cx| view.handle_key(&keystroke.key, cx))
    }

    /// Opens Transactions pre-filtered to the account `id`: fresh defaults plus that account, the
    /// search cleared and the table back on its first row. The Accounts selection is untouched, so
    /// returning to Accounts finds the same row selected.
    pub(super) fn open_account_ledger(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_account(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// Selects the account with `id`, if it still exists.
    pub(super) fn select_account(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.accounts_view
            .update(cx, |view, cx| view.select_id(id, cx));
    }

    /// A click on an account row: selects it and, as `enter` does, tries to open its ledger.
    pub(super) fn handle_accounts_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id, cx);
        self.open_account_ledger(id, cx);
        cx.notify();
    }

    /// The page's **+ Add account** button: the same handler `n` reaches.
    pub(super) fn handle_accounts_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_account_dialog("");
        cx.notify();
    }

    /// The three selects' option lists, read live from Settings (so an institution added there
    /// appears here) and the fixed type order.
    pub(super) fn account_dialog_options(&self) -> AccountOptions {
        AccountOptions::new(
            self.settings_institutions
                .iter()
                .map(|institution| institution.name.clone())
                .collect(),
            self.settings_units
                .iter()
                .map(|unit| unit.code.clone())
                .collect(),
        )
    }

    /// Opens the Add account dialog on a fresh form (Unit starting on Settings' default Unit),
    /// with Name pre-filled from `name` -- empty for `n` and the button, the typed argument for
    /// `accounts new <account name>`.
    pub(super) fn open_add_account_dialog(&mut self, name: &str) {
        let options = self.account_dialog_options();
        let default_unit = self
            .settings_units
            .iter()
            .find(|unit| unit.is_default)
            .map(|unit| unit.code.as_str());
        let mut form = AccountForm::new(&options, default_unit);
        form.name = TextField::new(name.trim());
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Add(form)));
    }

    /// A click on a field of the Add account dialog: focuses a text field, or focuses a select
    /// and toggles its list.
    pub(super) fn handle_accounts_dialog_field_click(
        &mut self,
        field: AccountField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .accounts_dialog_mut()
            .and_then(AccountsDialog::form_mut)
        {
            if field.is_select() {
                form.click_select(field);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    /// A click on a row of an open dropdown list.
    pub(super) fn handle_accounts_dialog_option_click(
        &mut self,
        field: AccountField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .accounts_dialog_mut()
            .and_then(AccountsDialog::form_mut)
        {
            form.choose_option(field, index);
        }
        cx.notify();
    }

    pub(super) fn handle_accounts_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_accounts_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// Applies a confirmed Accounts dialog (reached through [`Self::confirm_open_dialog`] from
    /// the Add account button, the Edit dialog's **Save**, the Delete dialog's **Delete account**
    /// or `Enter`): Add builds the account, appends it and selects it; Edit writes the changes
    /// onto the existing row (which regroups if its Type changed) and keeps it selected; Delete
    /// removes it. The form has already validated.
    pub(super) fn apply_accounts_dialog(
        &mut self,
        dialog: AccountsDialog,
        cx: &mut Context<'_, Self>,
    ) {
        let is_currency = match &dialog {
            AccountsDialog::Add(form) => form
                .unit
                .value()
                .and_then(|code| self.settings_units.iter().find(|unit| unit.code == code))
                .is_none_or(|unit| unit.kind == "currency"),
            _ => true,
        };
        let changed_id = match dialog {
            AccountsDialog::Add(form) => {
                let opened_at = Local::now().date_naive();
                self.accounts.update(cx, |store, cx| {
                    store.mutate(cx, |service| {
                        let id = service.next_id();
                        let account = form.into_account(id, opened_at, is_currency)?;
                        service.insert(account);
                        Some(id)
                    })
                })
            }
            AccountsDialog::Edit(id, form) => self.accounts.update(cx, |store, cx| {
                store.mutate(cx, |service| {
                    service
                        .get_mut(id)
                        .and_then(|account| form.apply_to(account).then_some(id))
                })
            }),
            AccountsDialog::Delete(id, _) => {
                let removed = edit_transactions(&self.transactions_store, cx, |transactions| {
                    let before = transactions.len();
                    transactions.retain(|transaction| transaction.account_id != id);
                    before - transactions.len()
                });
                let (kind, text) = self.accounts.update(cx, |store, cx| {
                    store.mutate(cx, |service| delete_account(service, removed, id))
                });
                self.raise_toast(kind, text);
                // Keep the selection in range, so it lands on the account that slid into the
                // deleted row's place (or the last one).
                self.accounts_view
                    .update(cx, |view, cx| view.clamp_selection(cx));
                None
            }
        };
        if let Some(id) = changed_id {
            self.select_account(id, cx);
        }
    }

    /// Opens the Delete account dialog on `id`. A no-op if the account is gone.
    pub(super) fn open_delete_account_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(account) = self
            .accounts
            .read(cx)
            .accounts()
            .iter()
            .find(|account| account.id == id)
        else {
            return;
        };
        let form = DeleteAccountForm::new(account.name.as_str());
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Delete(id, form)));
    }

    /// Opens the Edit account dialog on `id`, pre-filled. A no-op if the account is gone.
    pub(super) fn open_edit_account_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let options = self.account_dialog_options();
        let Some(account) = self
            .accounts
            .read(cx)
            .accounts()
            .iter()
            .find(|account| account.id == id)
        else {
            return;
        };
        let form = AccountForm::from_account(account, &options);
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Edit(id, form)));
    }

    pub(super) fn handle_accounts_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id, cx);
        self.open_edit_account_dialog(id, cx);
        cx.notify();
    }

    pub(super) fn handle_accounts_delete_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id, cx);
        self.open_delete_account_dialog(id, cx);
        cx.notify();
    }

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, then opens the
    /// same dialog the page's `n`/`e`/`d` and buttons do. `new` pre-fills Name with the argument.
    /// `edit` and `delete` resolve the typed name ([`accounts::find_by_name`]), or use the
    /// selected row when none is given; a name that fits nothing or several accounts flashes a
    /// status-line message naming the problem rather than guessing.
    pub(super) fn run_accounts_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_settings_page(SettingsSection::Accounts, cx);
        if verb == AccountsVerb::New {
            self.open_add_account_dialog(argument);
            return;
        }

        let index = if argument.is_empty() {
            match self.accounts_view.read(cx).selected_index(cx) {
                Some(index) => index,
                None => {
                    self.chrome.status_message = Some(crate::msg::desktop_status_no_accounts(
                        &format!(":{command_name}"),
                    ));
                    return;
                }
            }
        } else {
            match accounts::find_by_name(self.accounts.read(cx).accounts(), argument) {
                NameLookup::Found(index) => index,
                NameLookup::NotFound => {
                    self.chrome.status_message = Some(crate::msg::desktop_status_no_account_named(
                        &format!(":{command_name}"),
                        argument,
                    ));
                    return;
                }
                NameLookup::Ambiguous(names) => {
                    self.chrome.status_message =
                        Some(crate::msg::desktop_status_account_ambiguous(
                            &format!(":{command_name}"),
                            argument,
                            &names.join(", "),
                        ));
                    return;
                }
            }
        };
        let id = self.accounts.read(cx).accounts()[index].id;
        self.select_account(id, cx);
        match verb {
            AccountsVerb::Edit => self.open_edit_account_dialog(id, cx),
            AccountsVerb::Delete => self.open_delete_account_dialog(id, cx),
            AccountsVerb::New => {}
        }
    }
}
