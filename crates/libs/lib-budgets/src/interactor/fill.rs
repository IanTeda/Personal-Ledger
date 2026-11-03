//! Fill: writes a month's Month-only amounts from a source (9f).

use chrono::NaiveDate;
use lib_core::{BudgetId, Period};

use super::support::{logged, write_budget};
use crate::{BudgetReads, BudgetRepository, FillSource, Result};

/// Fills `target`'s Month-only amounts from `source` (9f); returns how many Categories changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub id: BudgetId,
    pub target: Period,
    pub source: FillSource,
    pub today: NaiveDate,
}

impl Fill {
    pub fn run(self, reads: &dyn BudgetReads, repo: &mut dyn BudgetRepository) -> Result<usize> {
        logged(
            "fill",
            write_budget(repo, self.id, |budget| {
                budget.fill(reads, self.target, self.source, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn filling_writes_the_months_amounts_and_says_how_many() {
        let mut repo = repo();
        let target = current().next();
        let fill = Fill {
            id: PERSONAL_SPENDING_ID,
            target,
            source: FillSource::PreviousMonth,
            today: today(),
        };

        let wrote = fill.run(&reads(), &mut repo).unwrap();

        let personal = budget(&repo, PERSONAL_SPENDING_ID);
        let month_only = personal
            .limits
            .values()
            .filter(|chain| {
                chain.iter().any(|record| {
                    record.month == target && matches!(record.limit, Limit::MonthOnly { .. })
                })
            })
            .count();
        assert_eq!(wrote, month_only);
        let closed = Fill {
            target: current().prev(),
            ..fill
        }
        .run(&reads(), &mut repo);
        assert_eq!(closed, Err(Error::ClosedMonth));
    }
}
