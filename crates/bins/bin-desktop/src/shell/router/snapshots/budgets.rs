//! What the integration tests read back of the Budgets page. The page Entity builds its part; the
//! open Dialog is the Dialog host's, so it is filled in here: the Switcher and Manage budgets are
//! hosted Entities, the other Budgets Dialogs still the legacy host's (#955).

use crate::budgets::BudgetsDialog;
use crate::budgets::view::dialog::{List, ListDialog};
#[doc(hidden)]
pub use crate::budgets::view::snapshot::{BudgetRowSnapshot, BudgetsSnapshot};
use crate::shell::router::Root;

impl Root {
    /// The Budgets page for a test, as its Entity builds it, plus the open Budgets dialog.
    #[doc(hidden)]
    pub fn budgets_snapshot(&self, cx: &gpui::App) -> BudgetsSnapshot {
        let mut snapshot = self.views.budgets.read(cx).snapshot();
        if let Some(hosted) = self
            .hosted_dialog()
            .and_then(|dialog| dialog.downcast::<ListDialog>())
        {
            let hosted = hosted.read(cx);
            snapshot.dialog = Some(
                match hosted.list {
                    List::Switcher(_) => "Switcher",
                    List::Manage(_) => "Manage",
                }
                .to_string(),
            );
            if let Some((names, selected)) = hosted.switcher_rows() {
                snapshot.switcher_names = Some(names);
                snapshot.switcher_selected = Some(selected);
            }
            return snapshot;
        }
        let dialog = self.dialog::<BudgetsDialog>();
        snapshot.dialog = dialog.map(|dialog| {
            match dialog {
                BudgetsDialog::Budget(_) => "Budget",
                BudgetsDialog::CategoryDetail(_) => "Detail",
                BudgetsDialog::EditLimit(_) => "EditLimit",
                BudgetsDialog::Fill { .. } => "Fill",
                BudgetsDialog::Stop(_) => "Stop",
            }
            .to_string()
        });
        match dialog {
            Some(BudgetsDialog::EditLimit(form)) => {
                snapshot.limit_amount = Some(form.amount.text().to_string());
            }
            Some(BudgetsDialog::Budget(form)) => {
                snapshot.budget_name = Some(form.name.text().to_string());
            }
            Some(BudgetsDialog::Fill { source, .. }) => {
                snapshot.fill_source = Some(format!("{source:?}"));
            }
            Some(BudgetsDialog::Stop(form)) => snapshot.stop_category = Some(form.category_id),
            Some(BudgetsDialog::CategoryDetail(_)) | None => {}
        }
        snapshot
    }
}
