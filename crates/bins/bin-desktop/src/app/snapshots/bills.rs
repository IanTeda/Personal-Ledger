//! What the integration tests read back of the Bills page. The Bills Entity builds the page's
//! part from its view model; the open Dialog is the Dialog host's, so it is filled in here: a
//! hosted Bills Dialog through its readback, today's Pay Dialog from its form.

use crate::app::Shell;
use crate::bills::BillsDialog;
use crate::bills::pay_form::PayForm;
#[doc(hidden)]
pub use crate::bills::{BillRowSnapshot, BillsSnapshot};
use crate::chrome::dialog_host::Dialog as _;

impl Shell {
    /// The Bills page for a test, as its Entity builds it, plus the open Bills Dialog.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &gpui::App) -> BillsSnapshot {
        let mut snapshot = self.views.bills.read(cx).snapshot();
        if let Some(dialog) = self.dialog::<BillsDialog>()
            && let Some(readback) = dialog.readback(&self.dialog_scope(), cx)
        {
            snapshot.dialog = Some(readback.kind.to_string());
            if readback.kind != "Skip" {
                snapshot.plan_name = readback.value("name").map(str::to_string);
                snapshot.plan_unit = readback.value("unit").map(str::to_string);
            }
        }
        if let Some(form) = self.dialog::<PayForm>() {
            snapshot.dialog = Some("Pay".to_string());
            snapshot.pay_mode = Some(format!("{:?}", form.mode));
            snapshot.pay_candidates = Some(form.candidates.len());
        }
        snapshot
    }
}
