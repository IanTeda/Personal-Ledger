//! The domains a write can touch, and the row change the Ledger's repository takes (ADR-0039).
//!
//! No write records a [`LedgerChange`] any more: every domain is written by its module, through its
//! ports, and the command trait that recorded them went with slice 10 (#1006).
//! [`LedgerChange`] keeps one variant so [`crate::LedgerRepository`] still stands until #816
//! decides how it and the Ledger go.

use std::collections::BTreeSet;

use lib_core::CategoryId;

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

/// One row a write changed, as the repository will receive it (ADR-0039). Nothing records one now
/// (#1006); the last variant stays with [`crate::LedgerRepository`] for #816.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerChange {
    DeleteCategory(CategoryId),
}

impl LedgerChange {
    pub fn domain(&self) -> Domain {
        match self {
            Self::DeleteCategory(_) => Domain::Categories,
        }
    }
}

/// The domains a batch of changes touched, each once.
pub fn domains(changes: &[LedgerChange]) -> BTreeSet<Domain> {
    changes.iter().map(LedgerChange::domain).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_names_each_touched_domain_once() {
        let changes = [
            LedgerChange::DeleteCategory(CategoryId::seed(1)),
            LedgerChange::DeleteCategory(CategoryId::seed(2)),
        ];
        assert_eq!(domains(&changes), BTreeSet::from([Domain::Categories]));
    }

    #[test]
    fn no_changes_touch_no_domain() {
        assert!(domains(&[]).is_empty());
    }
}
