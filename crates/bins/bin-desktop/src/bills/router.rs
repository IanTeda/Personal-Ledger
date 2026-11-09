//! `impl BillsRouter` for the Desktop: where the Bills page's Presenter asks to go. The Plan form
//! and Skip open as a `ModuleDialog` in the Dialog host, Pay opens through the Bill Payments
//! module's entry point and a Transaction is shown through the Transactions module's, all through
//! the page's gpui context, which is why the Router is built for each `handle` call
//! (`PageView::with_router`). A Notice goes onto the bin's queue (`crate::notices`), which the
//! input's `send` drains.

use gpui::{AppContext as _, Context, Entity};
use lib_bill_payments::SharedPayments;
use lib_bills::presenter::{PlanFormPresenter, SkipPresenter};
use lib_bills::{BillsOutcome, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, Clock, DateStyle, TransactionId};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogKeys;

use super::{BillsPage, BillsSetup};
use crate::announcer::Announcer;
use crate::bill_payments::router::{PayOpening, PayStores};
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

/// What the Bills page's Router is built from for each call: the ports its Dialogs open on, the
/// port Pay it directly writes through, the day they open on and the Dialog keys their hints show.
pub struct BillsRoutes {
    pub ports: lib_bills::SharedPorts,
    pub payments: SharedPayments,
    pub clock: Clock,
    pub dialog_keys: DialogKeys,
}

pub(crate) struct PageRouter<'a, 'b> {
    pub routes: &'a BillsRoutes,
    /// The date style the Plan form and Pay write and read their dates in: the Presenter's.
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, BillsPage>,
}

impl BillsRouter for PageRouter<'_, '_> {
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool {
        let routes = self.routes;
        let (ports, clock, keys) = (
            routes.ports.clone(),
            routes.clock.clone(),
            routes.dialog_keys.clone(),
        );
        let presenter = match plan {
            None => Some(PlanFormPresenter::add(ports, clock, self.date_style, keys)),
            Some(id) => PlanFormPresenter::edit(ports, id, clock, self.date_style, keys),
        };
        let Some(presenter) = presenter else {
            return false;
        };
        let landed = BillsPage::landing(self.cx);
        ModuleDialog::open(self.cx, presenter, landed);
        true
    }

    fn open_skip(&mut self, entry: EntryId) -> bool {
        let routes = self.routes;
        let Some(presenter) =
            SkipPresenter::new(routes.ports.clone(), entry, routes.dialog_keys.clone())
        else {
            return false;
        };
        let landed = BillsPage::landing(self.cx);
        ModuleDialog::open(self.cx, presenter, landed);
        true
    }

    /// The Bill Payments module's Pay Dialog; its outcome lands as `Paid`, and the page redraws
    /// from its ports.
    fn open_pay(&mut self, request: PayRequest) -> bool {
        let routes = self.routes;
        let landing = BillsPage::landing(self.cx);
        crate::bill_payments::router::open_pay(
            self.cx,
            PayStores {
                bills: &routes.ports,
                payments: &routes.payments,
            },
            request,
            PayOpening {
                clock: routes.clock.clone(),
                date_style: self.date_style,
                keys: routes.dialog_keys.clone(),
            },
            move |_paid, cx| landing(BillsOutcome::Paid, cx),
        )
    }

    fn show_transaction(&mut self, id: TransactionId) {
        crate::transactions::router::show(self.cx, id);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Bills module's entry point: its page, built on `setup`'s ports, which the Shell hosts as
/// the Bills destination. `clock` reads the calendar day the page dates its rows by.
pub fn bills_page(
    setup: BillsSetup,
    announcer: &Entity<Announcer>,
    clock: Clock,
    cx: &mut gpui::App,
) -> Entity<BillsPage> {
    let BillsSetup {
        ports,
        payments,
        keymap,
        dialog_keys,
    } = setup;
    let presenter =
        lib_bills::presenter::BillsPagePresenter::new(ports.clone(), clock.clone(), keymap);
    let routes = BillsRoutes {
        ports,
        payments,
        clock,
        dialog_keys,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, announcer, cx))
}
