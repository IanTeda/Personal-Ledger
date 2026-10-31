//! The Budgets Dialogs still drawn by `Root`'s legacy Dialog host (#955): the Category detail
//! (9d, `detail_dialog`), Edit budget (9e, `limit_dialog`), Fill (9f, `fill_dialog`), Stop
//! budgeting (9g, `stop_dialog`) and the New budget form (11c, `budget_dialog`), with their
//! status-line legends (`hints`). The page, the Switcher and Manage budgets run on `lib-budgets`'
//! Presenters (`crate::budgets::view`).

pub mod budget_dialog;
pub mod detail_dialog;
pub mod fill_dialog;
pub(crate) mod hints;
pub mod limit_dialog;
pub mod stop_dialog;

use crate::period::Period;

/// A period as the Locale writes a month and year (`September 2026`).
pub fn period_label(period: Period) -> String {
    lib_budgets::presenter::period_label(period)
}
