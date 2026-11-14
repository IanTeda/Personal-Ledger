//! `impl DashboardRouter` for the Desktop: where the Dashboard's Presenter asks to go. It goes
//! through the generic [`PageRouter`]'s gpui context, which is why the Router is built for each
//! `handle` call, and queues a Notice for the input's `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_core::Clock;
use lib_dashboard::{DashboardEntryId, DashboardPresenter, DashboardReads, DashboardRouter};

use super::view::page::DashboardPage;
use crate::announcer::Announcer;
use crate::bills::EntryId;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;
use crate::view::event::{Reveal, ViewEvent};

impl DashboardRouter for PageRouter<'_, '_, DashboardPresenter> {
    /// The Bills Schedule tab with the entry selected, as a Reveal `Root` hands the Bills page.
    fn reveal_bill_entry(&mut self, id: DashboardEntryId) {
        self.cx.emit(ViewEvent::Reveal(Reveal::BillEntry(EntryId {
            plan_id: id.plan_id,
            due: id.due,
        })));
    }
}

/// The Dashboard module's entry point: its page over `reads`, dated by `clock`, its keys under
/// `config`.
pub fn dashboard_page(
    reads: std::rc::Rc<dyn DashboardReads>,
    announcer: &Entity<Announcer>,
    config: &lib_config::KeyBindingConfig,
    clock: Clock,
    cx: &mut gpui::App,
) -> Entity<DashboardPage> {
    let presenter = DashboardPresenter::new(reads, clock, config);
    cx.new(|cx| ModulePage::new(presenter, (), announcer, cx))
}
