//! `impl BillsRouter` for the Desktop: where the Bills page's Presenter asks to go. The Plan form
//! and Skip open in today's Dialog host, Pay opens today's Pay Dialog, a Transaction is shown
//! through the Transactions module's entry point, and a Toast and the status line reach the Shell
//! as [`ViewEvent`]s, all through the page's gpui context, which is why the Router is built for
//! each `handle` call.

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_bills::presenter::{BillsPageEvent, PlanFormPresenter, SkipPresenter};
use lib_bills::{BillsOutcome, BillsPorts as _, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, DateStyle, TransactionId};
use lib_toast::ToastKind;

use super::pay_form::PayForm;
use super::ports::DesktopPorts;
use super::view::dialog::BillsDialog;
use super::view::page::BillsPage;
use crate::chrome::dialog_host::{OpenDialog, open_dialog};
use crate::ledger_store::LedgerStore;
use crate::view::event::ViewEvent;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    /// The day and date style the page's Dialogs are opened on.
    pub today: NaiveDate,
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, BillsPage>,
}

impl PageRouter<'_, '_> {
    /// Hosts `dialog`; its confirmed outcome comes back to the page through
    /// [`BillsPage::dialog_landed`].
    fn host(&mut self, dialog: Option<BillsDialog>) -> bool {
        let Some(dialog) = dialog else {
            return false;
        };
        open_dialog(dialog, BillsPage::dialog_landed, self.cx);
        true
    }
}

impl BillsRouter for PageRouter<'_, '_> {
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool {
        let ports = self.ports.clone();
        let presenter = match plan {
            None => Some(PlanFormPresenter::add(ports, self.today, self.date_style)),
            Some(id) => PlanFormPresenter::edit(ports, id, self.today, self.date_style),
        };
        let dialog = presenter.map(|presenter| BillsDialog::plan_form(presenter, &self.ports));
        self.host(dialog)
    }

    fn open_skip(&mut self, entry: EntryId) -> bool {
        let dialog = SkipPresenter::new(self.ports.clone(), entry)
            .map(|presenter| BillsDialog::skip(presenter, &self.ports));
        self.host(dialog)
    }

    /// Today's Pay Dialog, drawn and confirmed by the Shell until `lib-bill-payments` (#779).
    fn open_pay(&mut self, request: PayRequest) -> bool {
        let Some(plan) = self.ports.borrow().bills().bill_plan(request.entry.plan_id) else {
            return false;
        };
        let form = PayForm::new(
            request.entry,
            &plan,
            request.candidates,
            request.preselected,
            self.today,
            self.date_style,
        );
        self.cx
            .emit(ViewEvent::OpenDialog(Box::new(OpenDialog::PayBill(
                Box::new(form),
            ))));
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<BillsOutcome>) {}

    fn show_transaction(&mut self, id: TransactionId) {
        crate::transactions::router::show(self.cx, id);
    }

    fn raise_toast(&mut self, kind: ToastKind, text: String) {
        self.cx.emit(ViewEvent::RaiseToast { kind, text });
    }

    fn set_status(&mut self, text: Option<String>) {
        self.cx.emit(ViewEvent::SetStatus(text));
    }
}

/// The Bills module's entry point: its page, built on `ports`, which the Shell hosts as the Bills
/// destination. `today` reads the calendar day the page dates its rows by.
pub fn bills_page(
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    keymap: lib_viper::keymap::Keymap<BillsPageEvent>,
    today: lib_bills::presenter::Clock,
    cx: &mut gpui::App,
) -> Entity<BillsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| BillsPage::new(ports, ledger, keymap, today, cx))
}
