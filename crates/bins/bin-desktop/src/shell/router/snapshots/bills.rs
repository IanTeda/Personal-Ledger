//! What the integration tests read back of the Bills page. The Bills Entity builds the page's
//! part from its view model; the open Dialog is the Dialog host's Entity, so it is filled in here
//! through its readback: a Bills Dialog's, or the Pay Dialog's the page opened.

use crate::bill_payments::PayDialog;
use crate::bills::BillsDialog;
#[doc(hidden)]
pub use crate::bills::{BillRowSnapshot, BillsSnapshot};
use crate::shell::router::Root;

impl Root {
    /// The Bills page for a test, as its Entity builds it, plus the open Bills Dialog.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &gpui::App) -> BillsSnapshot {
        let mut snapshot = self.views.bills.read(cx).snapshot();
        let open = self.hosted_dialog();
        if let Some(dialog) = open.and_then(|open| open.downcast::<BillsDialog>()) {
            let readback =
                crate::shell::view::dialog_entity::HostedDialog::readback(dialog.read(cx), cx);
            snapshot.dialog = Some(readback.kind.to_string());
            if readback.kind != "Skip" {
                snapshot.plan_name = readback.value("name").map(str::to_string);
                snapshot.plan_unit = readback.value("unit").map(str::to_string);
            }
        }
        if let Some(dialog) = open.and_then(|open| open.downcast::<PayDialog>()) {
            let readback =
                crate::shell::view::dialog_entity::HostedDialog::readback(dialog.read(cx), cx);
            snapshot.dialog = Some(readback.kind.to_string());
            snapshot.pay_mode = readback.value("mode").map(str::to_string);
            snapshot.pay_candidates = readback.value("candidates").and_then(|n| n.parse().ok());
        }
        snapshot
    }
}
