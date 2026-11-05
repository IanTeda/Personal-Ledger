//! The domains a write can touch. A module announces the one it wrote, and the Desktop's
//! `LedgerChanged` carries it so a reader can skip a change that isn't its own.

/// A part of the Ledger a write can touch: what the Desktop's `LedgerChanged { what }` carries
/// when a module has written its store, so a reader can skip a change that isn't its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Accounts,
    Categories,
    Payees,
    Tags,
    Transactions,
    Bills,
    Budgets,
    Documents,
    DocumentTypes,
    Inventory,
    Institutions,
    Units,
}
