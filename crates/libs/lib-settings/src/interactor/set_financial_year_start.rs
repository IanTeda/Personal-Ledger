use lib_core::FinancialYearStart;

use crate::protocols::LedgerPreferences;

/// Set the month the Ledger's Financial Year starts in. [`FinancialYearStart`] is always a month,
/// so nothing can refuse it (ADR-0061).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetFinancialYearStart {
    pub start: FinancialYearStart,
}

impl SetFinancialYearStart {
    pub fn run(self, preferences: &dyn LedgerPreferences) {
        preferences.set_financial_year_start(self.start);
        tracing::info!(
            month = self.start.month(),
            "Set the Ledger's Financial year start"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryLedgerUnits;
    use crate::protocols::LedgerReads;

    #[test]
    fn the_new_start_reads_back() {
        let ledger = InMemoryLedgerUnits::sample();

        SetFinancialYearStart {
            start: FinancialYearStart::APRIL,
        }
        .run(&ledger);

        assert_eq!(
            ledger.ledger_financial_year_start(),
            FinancialYearStart::APRIL
        );
    }
}
