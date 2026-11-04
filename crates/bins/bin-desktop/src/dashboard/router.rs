//! `impl DashboardRouter` for the Desktop: where the Dashboard's Presenter asks to go. It goes
//! through the page's gpui context, which is why the Router is built for each `handle` call. A
//! Notice goes onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::Context;
use lib_dashboard::{DashboardEntryId, DashboardRouter, NoLedgerCommand};
use lib_toast::{Notice, Notify};

use super::view::page::DashboardPage;
use crate::bills::EntryId;
use crate::view::event::{Reveal, ViewEvent};

pub(super) struct PageRouter<'a, 'b> {
    pub cx: &'a mut Context<'b, DashboardPage>,
}

impl DashboardRouter for PageRouter<'_, '_> {
    /// The Bills Schedule tab with the entry selected, as a Reveal `Root` hands the Bills page.
    fn reveal_bill_entry(&mut self, id: DashboardEntryId) {
        self.cx.emit(ViewEvent::Reveal(Reveal::BillEntry(EntryId {
            plan_id: id.plan_id,
            due: id.due,
        })));
    }

    /// Never reached on the Desktop: the page is built only on an open Ledger, and the no Ledger
    /// open content that names `:open` and `:new` is the Shell's, whose clicks reach `Root`
    /// directly (`shell/view/no_ledger.rs`).
    fn run_no_ledger_command(&mut self, _command: NoLedgerCommand) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}
