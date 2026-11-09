//! `impl AccountsRouter` for the Desktop: where the Accounts page's Presenter asks to go. The Add
//! and Edit Dialogs open as a `ModuleDialog` in the Dialog host through the page's gpui context,
//! which is why the Router is built for each `handle` call (`PageView::with_router`), and a Notice
//! goes onto the bin's queue (`crate::notices`), which the input's `send` drains. The Delete
//! Dialog is the Account Deletion module's, opened through its entry point.

use gpui::{AppContext as _, Context, Entity};
use lib_account_deletion::AccountDeleted;
use lib_accounts::presenter::{AccountFormPresenter, AccountsCommand, AccountsPageEvent};
use lib_accounts::{AccountsDialog as Which, AccountsRouter};
use lib_core::AccountId;
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::{AccountsPage, AccountsRoutes, AccountsSetup};
use crate::announcer::Announcer;
use crate::navigation::command::AccountsVerb;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a AccountsRoutes,
    pub cx: &'a mut Context<'b, AccountsPage>,
}

impl AccountsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = AccountsPage::landing(self.cx);
        let (ports, keys) = (routes.ports.clone(), routes.dialog_keys.get());
        let today = routes.clock.today();
        let form = match dialog {
            Which::AddAccount { name } => AccountFormPresenter::add(ports, &name, today, keys),
            Which::EditAccount(id) => {
                let Some(form) = AccountFormPresenter::edit(ports, id, today, keys) else {
                    return false;
                };
                form
            }
        };
        ModuleDialog::open(self.cx, form, landed);
        true
    }

    /// The Account Deletion module's Delete Dialog, through its entry point; a confirmed delete
    /// comes back to the page, which selects the row that slid into the deleted one's place.
    fn open_delete(&mut self, id: AccountId) -> bool {
        let routes = self.routes;
        let page = self.cx.weak_entity();
        let landed = move |_deleted: AccountDeleted, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.send(AccountsPageEvent::Deleted, cx));
        };
        crate::account_deletion::router::open_delete(
            self.cx,
            &routes.deletion,
            id,
            routes.dialog_keys.get(),
            landed,
        )
    }

    /// Shows the Account's ledger through the Transactions entry point: the Transactions
    /// filtered to it, from their first row. The selection is untouched, so returning to Accounts
    /// finds the same row selected.
    fn open_ledger(&mut self, id: AccountId) {
        let filters = TransactionFilters::for_account(self.routes.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Accounts module's entry point: its Settings page, built on `setup`'s ports and dated by
/// `clock`, which the Settings View hosts as any other page.
pub fn accounts_page(
    setup: AccountsSetup,
    announcer: Entity<Announcer>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<AccountsPage> {
    let AccountsSetup {
        presenter,
        ports,
        deletion,
        dialog_keys,
    } = setup;
    let routes = AccountsRoutes {
        ports,
        deletion,
        dialog_keys,
        clock,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
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
