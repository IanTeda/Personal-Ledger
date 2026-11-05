//! The Dashboard as a VIPER module (`docs/development/viper-architecture.md`): its Presenter, the
//! view model a Client draws, and the read ports it reads the other domains through. Toolkit-free
//! and I/O-free, it depends on no feature crate: which Bills need attention and how the default
//! Budget's month stands are answered through [`protocols`] by the domains that own those rules.
//!
//! Only the Needs Attention Bill rows and the budget list are modelled here. The page's date
//! header, figure row, charts and lower band stay literal in the Client until #144 gives them data,
//! and the no Ledger open content is the Shell's, since this module exists only once a Ledger is
//! open.

pub mod msg;
pub mod presenter;
pub mod protocols;
pub mod test_support;

pub use crate::presenter::{
    AttentionRow, BudgetBarView, BudgetListView, DashboardEvent, DashboardPresenter,
    DashboardViewModel,
};
pub use crate::protocols::{
    DashboardBill, DashboardBills, DashboardBudgetBar, DashboardBudgetMonth, DashboardBudgets,
    DashboardElapsed, DashboardEntryId, DashboardReads, DashboardRouter,
};
