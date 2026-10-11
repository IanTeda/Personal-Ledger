//! What the integration tests read back of the Budgets page. The Budgets Entity builds the
//! page's part; the open dialog is the Dialog host's, so it is filled in here.

use crate::app::Shell;
use crate::budgets::{self, BudgetsDialog};
#[doc(hidden)]
pub use crate::view::budgets::snapshot::{BudgetRowSnapshot, BudgetsSnapshot};

impl Shell {
    /// The Budgets page for a test, as its Entity builds it, plus the open Budgets dialog.
    #[doc(hidden)]
    pub fn budgets_snapshot(&self, cx: &gpui::App) -> BudgetsSnapshot {
        let mut snapshot = self.budgets_view.read(cx).snapshot(cx);
        let dialog = self.dialog::<BudgetsDialog>();
        snapshot.dialog = dialog.map(|dialog| {
            match dialog {
                BudgetsDialog::Switcher(_) => "Switcher",
                BudgetsDialog::Budget(_) => "Budget",
                BudgetsDialog::Manage(_) => "Manage",
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
            Some(BudgetsDialog::Switcher(switcher)) => {
                let all = self.budgets(cx);
                snapshot.switcher_names = Some(
                    budgets::switcher_ids(all, switcher.query.text())
                        .into_iter()
                        .filter_map(|id| all.get(id).map(|budget| budget.name.clone()))
                        .collect(),
                );
                snapshot.switcher_selected = Some(switcher.selected);
            }
            Some(BudgetsDialog::Budget(form)) => {
                snapshot.budget_name = Some(form.name.text().to_string());
            }
            Some(BudgetsDialog::Fill { source, .. }) => {
                snapshot.fill_source = Some(format!("{source:?}"));
            }
            Some(BudgetsDialog::Stop(form)) => snapshot.stop_category = Some(form.category_id),
            Some(BudgetsDialog::Manage(_) | BudgetsDialog::CategoryDetail(_)) | None => {}
        }
        snapshot
    }
}
