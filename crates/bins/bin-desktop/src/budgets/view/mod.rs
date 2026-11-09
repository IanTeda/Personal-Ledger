//! The Budgets module's Views (`docs/ux/desktop-mockups/14-budgets-v2/`): the page Presenter's
//! `PageView`, which the generic `ModulePage` hosts and which draws its header, tab strip and the
//! active tab (`progress`, `plan`, `history`), and each Dialog Presenter's `DialogView`, which a
//! `ModuleDialog` hosts: the Switcher and Manage budgets (`switcher`, `manage`), and the Budget
//! form, Edit budget, Stop budgeting, Fill and the Category detail (`form_dialog`).

pub mod form_dialog;
mod history;
mod manage;
pub mod page;
mod plan;
pub(crate) mod progress;
pub mod snapshot;
pub mod switcher;
