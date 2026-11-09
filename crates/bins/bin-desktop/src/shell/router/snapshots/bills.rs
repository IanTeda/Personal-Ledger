//! What the integration tests read back of the Bills page. The Bills page builds its part from
//! its view model; the open Dialog is the Dialog host's Entity, so it is filled in here through
//! its readback: the Plan form's or Skip's, or the Pay Dialog's the page opened.

use lib_bill_payments::presenter::PayPresenter;
use lib_bills::presenter::{PlanFormPresenter, SkipPresenter};

#[doc(hidden)]
pub use crate::bills::{BillRowSnapshot, BillsSnapshot};
use crate::shell::router::Root;
use crate::shell::view::dialog_entity::ModuleDialog;

impl Root {
    /// The Bills page for a test, as its Entity builds it, plus the open Bills Dialog.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &gpui::App) -> BillsSnapshot {
        let mut snapshot = self.open().views.bills.read(cx).snapshot();
        let Some(open) = self.hosted_dialog() else {
            return snapshot;
        };
        if open.downcast::<ModuleDialog<PlanFormPresenter>>().is_some() {
            let readback = open.readback(cx);
            snapshot.dialog = Some(readback.kind.to_string());
            snapshot.plan_name = readback.value("name").map(str::to_string);
            snapshot.plan_unit = readback.value("unit").map(str::to_string);
            snapshot.plan_payee = readback.value("payee").map(str::to_string);
        } else if open.downcast::<ModuleDialog<SkipPresenter>>().is_some() {
            snapshot.dialog = Some(open.readback(cx).kind.to_string());
        } else if open.downcast::<ModuleDialog<PayPresenter>>().is_some() {
            let readback = open.readback(cx);
            snapshot.dialog = Some(readback.kind.to_string());
            snapshot.pay_mode = readback.value("mode").map(str::to_string);
            snapshot.pay_candidates = readback.value("candidates").and_then(|n| n.parse().ok());
        }
        snapshot
    }
}
