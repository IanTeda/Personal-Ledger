//! What the integration tests read back of the Budgets page. The page builds its part; the open
//! Dialog is the Dialog host's, so it is filled in here: the Switcher's and Manage budgets' rows
//! from their view models, and the form Dialogs' fields through their readback.

use lib_budgets::presenter::{
    BudgetFormPresenter, DetailPresenter, FillPresenter, LimitFormPresenter, ManagePresenter,
    StopPresenter, SwitcherPresenter,
};

#[doc(hidden)]
pub use crate::budgets::view::snapshot::{BudgetRowSnapshot, BudgetsSnapshot};
use crate::shell::router::Root;
use crate::shell::view::dialog_entity::ModuleDialog;

impl Root {
    /// The Budgets page for a test, as its Entity builds it, plus the open Budgets dialog.
    #[doc(hidden)]
    pub fn budgets_snapshot(&self, cx: &gpui::App) -> BudgetsSnapshot {
        let mut snapshot = self.open().views.budgets.read(cx).snapshot();
        let Some(open) = self.hosted_dialog() else {
            return snapshot;
        };
        if let Some(switcher) = open.downcast::<ModuleDialog<SwitcherPresenter>>() {
            let view = switcher.read(cx).view_model();
            snapshot.dialog = Some("Switcher".to_string());
            snapshot.switcher_names = Some(view.rows.into_iter().map(|row| row.name).collect());
            snapshot.switcher_selected = Some(view.selected.unwrap_or(0));
        } else if let Some(manage) = open.downcast::<ModuleDialog<ManagePresenter>>() {
            let view = manage.read(cx).view_model();
            snapshot.dialog = Some("Manage".to_string());
            snapshot.manage_rows = Some(
                view.rows
                    .into_iter()
                    .map(|row| (row.name, row.default.is_some(), row.archived))
                    .collect(),
            );
            snapshot.manage_selected = Some(view.selected);
            snapshot.manage_error = view.error;
        } else if open
            .downcast::<ModuleDialog<BudgetFormPresenter>>()
            .is_some()
            || open
                .downcast::<ModuleDialog<LimitFormPresenter>>()
                .is_some()
            || open.downcast::<ModuleDialog<StopPresenter>>().is_some()
            || open.downcast::<ModuleDialog<FillPresenter>>().is_some()
            || open.downcast::<ModuleDialog<DetailPresenter>>().is_some()
        {
            let readback = open.readback(cx);
            snapshot.dialog = Some(readback.kind.to_string());
            snapshot.limit_amount = readback.value("amount").map(str::to_string);
            snapshot.budget_name = readback.value("name").map(str::to_string);
            snapshot.fill_source = readback.value("source").map(str::to_string);
            snapshot.stop_category = readback.value("category").and_then(|id| id.parse().ok());
        }
        snapshot
    }
}
