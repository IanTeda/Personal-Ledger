//! `impl BillsRouter` for the Desktop: where the Bills page's Presenter asks to go. The Plan form
//! and Skip open as a `ModuleDialog` in the Dialog host, Pay opens through the Bill Payments
//! module's entry point and a Transaction is shown through the Transactions module's, all through
//! the generic [`PageRouter`]'s gpui context, which is why the Router is built for each `handle`
//! call (`PageView::with_router`). The page lends it the Presenter's date style, which the Plan
//! form and Pay write and read their dates in; it queues a Notice for the input's `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_bills::presenter::{BillsPagePresenter, PlanFormPresenter, SkipPresenter};
use lib_bills::{BillsOutcome, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, DateStyle, TransactionId};

use super::{BillsPage, BillsSetup};
use crate::announcer::Announcer;
use crate::bill_payments::router::{PayOpening, PayStores};
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

/// The Bills page's Router, lent the date style the Plan form and Pay write and read their dates
/// in: the Presenter's.
pub(crate) type BillsPageRouter<'a, 'b> = PageRouter<'a, 'b, BillsPagePresenter, Option<DateStyle>>;

impl BillsRouter for BillsPageRouter<'_, '_> {
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool {
        let setup = self.setup;
        let (ports, clock, keys) = (
            setup.ports.clone(),
            setup.clock.clone(),
            setup.dialog_keys.get(),
        );
        let presenter = match plan {
            None => Some(PlanFormPresenter::add(ports, clock, self.lent, keys)),
            Some(id) => PlanFormPresenter::edit(ports, id, clock, self.lent, keys),
        };
        let Some(presenter) = presenter else {
            return false;
        };
        self.open(presenter);
        true
    }

    fn open_skip(&mut self, entry: EntryId) -> bool {
        let setup = self.setup;
        let Some(presenter) =
            SkipPresenter::new(setup.ports.clone(), entry, setup.dialog_keys.get())
        else {
            return false;
        };
        self.open(presenter);
        true
    }

    /// The Bill Payments module's Pay Dialog; its outcome lands as `Paid`, and the page redraws
    /// from its ports.
    fn open_pay(&mut self, request: PayRequest) -> bool {
        let setup = self.setup;
        let landing = self.landing();
        crate::bill_payments::router::open_pay(
            self.cx,
            PayStores {
                bills: &setup.ports,
                payments: &setup.payments,
            },
            request,
            PayOpening {
                clock: setup.clock.clone(),
                date_style: self.lent,
                keys: setup.dialog_keys.get(),
            },
            move |_paid, cx| landing(BillsOutcome::Paid, cx),
        )
    }

    fn show_transaction(&mut self, id: TransactionId) {
        crate::transactions::router::show(self.cx, id);
    }
}

/// The Bills module's entry point: its page, `presenter` routed over `setup`, which the Shell
/// hosts as the Bills destination.
pub fn bills_page(
    presenter: BillsPagePresenter,
    setup: BillsSetup,
    announcer: &Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<BillsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, announcer, cx))
}
