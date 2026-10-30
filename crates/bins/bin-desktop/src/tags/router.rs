//! `impl TagsRouter` for the Desktop: where the Tags page's Presenter asks to go. A Dialog opens as
//! an Entity in the Dialog host through the page's gpui context, which is why the Router is built
//! for each `handle` call, and a Notice goes onto the bin's queue (`crate::notices`), which the
//! input's `send` drains.

use gpui::{Context, Entity};
use lib_tags::{SharedPorts, TagsDialog as Which, TagsOutcome, TagsRouter};
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::TagsSetup;
use super::view::dialog::TagsDialog;
use super::view::page::TagsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: SharedPorts,
    pub ledger: Entity<LedgerStore>,
    pub clock: lib_core::Clock,
    pub cx: &'a mut Context<'b, TagsPage>,
}

impl TagsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`TagsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = TagsDialog::open(dialog, &self.ports, &self.ledger, landed) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// Shows the Tag's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::TagId) {
        let filters = TransactionFilters::for_tag(self.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<TagsOutcome>) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Tags module's entry point: its Settings page, built on `setup`'s store, which the Settings
/// View hosts as any other page.
pub fn tags_page(
    setup: TagsSetup,
    ledger: Entity<LedgerStore>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<TagsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| TagsPage::new(setup, ledger, clock, cx))
}

/// What a Tags Dialog's Presenter asks: closing, and Notices, which go onto the bin's queue for
/// `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
pub(super) struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    pub closed: Option<Option<TagsOutcome>>,
}

impl TagsRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn open_transactions(&mut self, _id: lib_core::TagId) {}

    fn close_dialog(&mut self, outcome: Option<TagsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}
