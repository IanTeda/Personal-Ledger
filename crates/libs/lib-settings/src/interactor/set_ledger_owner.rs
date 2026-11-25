use lib_core::Owner;

use crate::protocols::LedgerPreferences;

/// Set the Ledger's Owner, or clear it with `None`. [`Owner`] has already checked the text, so
/// nothing can refuse it (ADR-0061).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetLedgerOwner {
    pub owner: Option<Owner>,
}

impl SetLedgerOwner {
    /// The Owner is personal, so the log says only whether it was set or cleared.
    #[tracing::instrument(skip_all)]
    pub fn run(self, preferences: &dyn LedgerPreferences) {
        let cleared = self.owner.is_none();
        preferences.set_owner(self.owner);
        if cleared {
            tracing::info!("Cleared the Ledger's Owner");
        } else {
            tracing::info!("Set the Ledger's Owner");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryLedgerUnits;
    use crate::protocols::LedgerReads;

    #[test]
    fn the_new_owner_reads_back() {
        let ledger = InMemoryLedgerUnits::sample();
        let owner = Owner::new("Alex Example");

        SetLedgerOwner {
            owner: owner.clone(),
        }
        .run(&ledger);

        assert!(owner.is_some());
        assert_eq!(ledger.ledger_owner(), owner);
    }

    #[test]
    fn a_cleared_owner_reads_back_as_none() {
        let ledger = InMemoryLedgerUnits::sample();

        SetLedgerOwner { owner: None }.run(&ledger);

        assert_eq!(ledger.ledger_owner(), None);
    }
}
