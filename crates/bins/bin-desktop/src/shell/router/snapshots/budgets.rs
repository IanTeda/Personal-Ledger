//! What the integration tests read back of the Budgets page. The page Entity builds its part; the
//! open Dialog is the Dialog host's, so it is filled in here: the Switcher and Manage budgets and
//! the form Dialogs are hosted Entities.

use crate::budgets::view::dialog::{List, ListDialog};
use crate::budgets::view::form_dialog::FormDialog;
#[doc(hidden)]
pub use crate::budgets::view::snapshot::{BudgetRowSnapshot, BudgetsSnapshot};
use crate::shell::router::Root;

impl Root {
    /// The Budgets page for a test, as its Entity builds it, plus the open Budgets dialog.
    #[doc(hidden)]
    pub fn budgets_snapshot(&self, cx: &gpui::App) -> BudgetsSnapshot {
        let mut snapshot = self.views.budgets.read(cx).snapshot();
        let Some(open) = self.hosted_dialog() else {
            return snapshot;
        };
        if let Some(hosted) = open.downcast::<ListDialog>() {
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
        } else if let Some(hosted) = open.downcast::<FormDialog>() {
            let hosted = hosted.read(cx);
            snapshot.dialog = Some(hosted.kind().to_string());
            snapshot.limit_amount = hosted.limit_amount();
            snapshot.budget_name = hosted.budget_name();
            snapshot.fill_source = hosted.fill_source();
            snapshot.stop_category = hosted.stop_category();
        }
        snapshot
    }
}
