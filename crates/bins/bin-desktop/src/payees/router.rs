//! `impl PayeesRouter` for the Desktop: where the Payees page's Presenter asks to go. A Dialog
//! opens as a `ModuleDialog` in the Dialog host through the generic [`PageRouter`], built for each
//! `handle` call (`PageView::with_router`), which queues a Notice for the input's `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_payees::presenter::{DeletePayeePresenter, PayeeFormPresenter, PayeesPagePresenter};
use lib_payees::{PayeesDialog as Which, PayeesRouter};
use lib_transactions::query::TransactionFilters;

use super::{PayeesPage, PayeesSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl PayeesRouter for PageRouter<'_, '_, PayeesPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let (ports, keys) = (self.setup.ports.clone(), self.setup.dialog_keys.get());
        match dialog {
            Which::AddPayee => self.open(PayeeFormPresenter::add(ports, keys)),
            Which::EditPayee(id) => {
                let Some(form) = PayeeFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::DeletePayee(id) => {
                let Some(delete) = DeletePayeePresenter::new(ports, id, keys) else {
                    return false;
                };
                self.open(delete);
            }
        }
        true
    }

    /// Shows the Payee's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::PayeeId) {
        let filters = TransactionFilters::for_payee(self.setup.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

/// The Payees module's entry point: its Settings page, `presenter` routed over `setup`, which the
/// Settings View hosts as any other page.
pub fn payees_page(
    presenter: PayeesPagePresenter,
    setup: PayeesSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<PayeesPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
