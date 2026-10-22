//! # Transaction Scope
//!
//! What a per-entity totals report (Category-total FR.35, Payee-total FR.36, ...) scopes its
//! Transaction query to: a single Unit (aggregating every Account denominated in it) or a
//! single Account. Shared by every such report rather than each defining its own
//! near-identical scope enum.

/// What a Transaction-totals query is scoped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionScope {
    /// Every Account denominated in this Unit.
    Unit(lib_core::RowID),
    /// This one Account only.
    Account(lib_core::RowID),
}
