//! `impl AccountsRouter` for the Desktop: where the Accounts page's Presenter asks to go. The Add
//! and Edit Dialogs open as a `ModuleDialog` in the Dialog host through the generic
//! [`PageRouter`], built for each `handle` call (`PageView::with_router`), which queues a Notice
//! for the input's `send` to drain. The Delete Dialog is the Account Deletion module's, opened
//! through its entry point.

use gpui::{AppContext as _, Entity};
use lib_account_deletion::AccountDeleted;
use lib_accounts::presenter::{
    AccountFormPresenter, AccountsCommand, AccountsPageEvent, AccountsPagePresenter,
};
use lib_accounts::{AccountsDialog as Which, AccountsRouter};
use lib_core::AccountId;
use lib_transactions::query::TransactionFilters;

use super::{AccountsPage, AccountsSetup};
use crate::announcer::Announcer;
use crate::navigation::command::AccountsVerb;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl AccountsRouter for PageRouter<'_, '_, AccountsPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let setup = self.setup;
        let (ports, keys) = (setup.ports.clone(), setup.dialog_keys.get());
        let today = setup.clock.today();
        let form = match dialog {
            Which::AddAccount { name } => AccountFormPresenter::add(ports, &name, today, keys),
            Which::EditAccount(id) => {
                let Some(form) = AccountFormPresenter::edit(ports, id, today, keys) else {
                    return false;
                };
                form
            }
        };
        self.open(form);
        true
    }

    /// The Account Deletion module's Delete Dialog, through its entry point; a confirmed delete
    /// comes back to the page, which selects the row that slid into the deleted one's place.
    fn open_delete(&mut self, id: AccountId) -> bool {
        let setup = self.setup;
        let page = self.cx.weak_entity();
        let landed = move |_deleted: AccountDeleted, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.send(AccountsPageEvent::Deleted, cx));
        };
        crate::account_deletion::router::open_delete(
            self.cx,
            &setup.deletion,
            id,
            setup.dialog_keys.get(),
            landed,
        )
    }

    /// Shows the Account's ledger through the Transactions entry point: the Transactions
    /// filtered to it, from their first row. The selection is untouched, so returning to Accounts
    /// finds the same row selected.
    fn open_ledger(&mut self, id: AccountId) {
        let filters = TransactionFilters::for_account(self.setup.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

/// The Accounts module's entry point: its Settings page, `presenter` routed over `setup`, which the
/// Settings View hosts as any other page.
pub fn accounts_page(
    presenter: AccountsPagePresenter,
    setup: AccountsSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<AccountsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}

/// `accounts new|edit|delete [<account name>]` on the Accounts page, once `Root` has opened it: the
/// palette's way into the page, so `Root` reaches the page through this module rather than its
/// methods. The same Dialog the page's `n`/`e`/`d` and buttons open; a name that fits nothing or
/// several Accounts puts a status-line message naming the problem.
pub fn run_command(
    page: &Entity<AccountsPage>,
    command_name: &str,
    verb: AccountsVerb,
    argument: &str,
    cx: &mut gpui::App,
) {
    let command = match verb {
        AccountsVerb::New => AccountsCommand::New,
        AccountsVerb::Edit => AccountsCommand::Edit,
        AccountsVerb::Delete => AccountsCommand::Delete,
    };
    page.update(cx, |page, cx| {
        page.send(
            AccountsPageEvent::Command {
                command: command_name.to_string(),
                verb: command,
                argument: argument.to_string(),
            },
            cx,
        );
    });
}
