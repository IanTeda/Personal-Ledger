//! The Dashboard module on the Desktop: the page, `lib-dashboard`'s Presenter's `PageView` in the
//! generic `ModulePage` (`view`), and where the Presenter asks to go (`router`). The Presenter
//! reads the other domains through `lib_dashboard::DashboardReads`
//! (`LedgerPorts::dashboard_reads()`), so the page reads no `Ledger` accessor (#1024).
//!
//! The no Ledger open content is the Shell's (`shell/view/no_ledger.rs`), since this module
//! exists only once a Ledger is open (#975).

pub mod router;
pub mod view;

pub use router::dashboard_page;
pub use view::page::{DashboardBillSnapshot, DashboardPage};
