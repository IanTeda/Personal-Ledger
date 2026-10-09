//! The calendar month both Bills and Budgets page through lives in `lib_core`; the Desktop
//! re-exports it so its existing `crate::period::Period` paths keep working.

pub use lib_core::Period;
