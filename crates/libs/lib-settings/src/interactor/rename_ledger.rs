use lib_core::LedgerName;

use crate::protocols::LedgerPreferences;

/// Rename the Ledger. [`LedgerName`] has already checked the name, so nothing can refuse it
/// (ADR-0061).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameLedger {
    pub name: LedgerName,
}

impl RenameLedger {
    /// The name is user-entered text, so the log says only that it changed.
    pub fn run(self, preferences: &dyn LedgerPreferences) {
        preferences.set_name(self.name);
        tracing::info!("Renamed the Ledger");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryLedgerUnits;
    use crate::protocols::LedgerReads;

    #[test]
    fn the_new_name_reads_back() {
        let ledger = InMemoryLedgerUnits::sample();
        let name = LedgerName::new("Household").expect("a short, non-blank name");

        RenameLedger { name: name.clone() }.run(&ledger);

        assert_eq!(ledger.ledger_name(), name);
    }
}
