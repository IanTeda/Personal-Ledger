//! What the integration tests read back of the Bills page. The Bills Entity builds the page's
//! part; the open dialog is the Dialog host's, so it is filled in here.

use crate::app::Shell;
use crate::bills::BillsDialog;
#[doc(hidden)]
pub use crate::view::bills::snapshot::{BillRowSnapshot, BillsSnapshot};

impl Shell {
    /// The Bills page for a test, as its Entity builds it, plus the open Bills dialog.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &gpui::App) -> BillsSnapshot {
        let mut snapshot = self.bills_view.read(cx).snapshot(cx);
        let dialog = self.dialog::<BillsDialog>();
        snapshot.dialog = dialog.map(|dialog| {
            match dialog {
                BillsDialog::Add(_) => "Add",
                BillsDialog::Edit(..) => "Edit",
                BillsDialog::Pay(_) => "Pay",
                BillsDialog::Skip(_) => "Skip",
            }
            .to_string()
        });
        snapshot.plan_name = match dialog {
            Some(BillsDialog::Add(form) | BillsDialog::Edit(_, form)) => {
                Some(form.name.text().to_string())
            }
            _ => None,
        };
        snapshot.pay_mode = match dialog {
            Some(BillsDialog::Pay(form)) => Some(format!("{:?}", form.mode)),
            _ => None,
        };
        snapshot.pay_candidates = match dialog {
            Some(BillsDialog::Pay(form)) => Some(form.candidates.len()),
            _ => None,
        };
        snapshot
    }
}
