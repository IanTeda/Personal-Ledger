//! `impl PayeesRouter` for the Desktop: where the Payees page's Presenter asks to go. A Dialog
//! opens as a `ModuleDialog` in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call (`PageView::with_router`), and a Notice goes onto the
//! bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::{AppContext as _, Context, Entity};
use lib_payees::presenter::{DeletePayeePresenter, PayeeFormPresenter};
use lib_payees::{PayeesDialog as Which, PayeesRouter};
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::{PayeesPage, PayeesRoutes, PayeesSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a PayeesRoutes,
    pub cx: &'a mut Context<'b, PayeesPage>,
}

impl PayeesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = PayeesPage::landing(self.cx);
        let (ports, keys) = (routes.ports.clone(), routes.dialog_keys.clone());
        match dialog {
            Which::AddPayee => {
                ModuleDialog::open(self.cx, PayeeFormPresenter::add(ports, keys), landed);
            }
            Which::EditPayee(id) => {
                let Some(form) = PayeeFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeletePayee(id) => {
                let Some(delete) = DeletePayeePresenter::new(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, delete, landed);
            }
        }
        true
    }

    /// Shows the Payee's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::PayeeId) {
        let filters = TransactionFilters::for_payee(self.routes.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Payees module's entry point: its Settings page, built on `setup`'s ports and dated by
/// `clock`, which the Settings View hosts as any other page.
pub fn payees_page(
    setup: PayeesSetup,
    announcer: Entity<Announcer>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<PayeesPage> {
    let PayeesSetup {
        presenter,
        ports,
        dialog_keys,
    } = setup;
    let routes = PayeesRoutes {
        ports,
        dialog_keys,
        clock,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
