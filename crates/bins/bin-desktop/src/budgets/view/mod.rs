//! The Budgets module's Views (`docs/ux/desktop-mockups/14-budgets-v2/`): the page, an Entity
//! holding the `lib-budgets` page Presenter that draws its header, tab strip and the active tab
//! (`progress`, `plan`, `history`), and the Switcher and Manage budgets as one hosted Dialog
//! Entity (`dialog`, drawn by `switcher` and `manage`). The Budget form, Edit budget, Stop
//! budgeting, Fill and the Category detail are another (`form_dialog`).

pub mod dialog;
pub mod form_dialog;
mod history;
mod manage;
pub mod page;
mod plan;
pub(crate) mod progress;
pub mod snapshot;
mod switcher;
