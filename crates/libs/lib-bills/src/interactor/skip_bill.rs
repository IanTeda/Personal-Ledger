//! Skip this cycle: resolves an open entry without paying it, keeping its Planned Amount as it is
//! now. For a One-shot Plan it cancels the bill.

use super::Rows;
use crate::protocols::BillsRepository;
use crate::{EntryId, Error, skip};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipBill {
    pub entry: EntryId,
}

impl SkipBill {
    /// Writes the entry Skipped.
    ///
    /// # Errors
    /// `EntryNotFound` for an entry that is gone, `EntryNotOpen` for one already settled or
    /// superseded, and `PlanNotFound` for an entry whose Plan is gone.
    pub fn run(self, bills: &mut dyn BillsRepository) -> Result<(), Error> {
        let outcome = self.apply(bills);
        let (plan, due) = (self.entry.plan_id, self.entry.due);
        match &outcome {
            Ok(()) => tracing::info!(bill_plan_id = %plan, %due, "Skipped a Bill"),
            Err(refusal) => {
                tracing::info!(bill_plan_id = %plan, %due, %refusal, "Refused to skip a Bill");
            }
        }
        outcome
    }

    fn apply(self, bills: &mut dyn BillsRepository) -> Result<(), Error> {
        let mut rows = Rows::read(bills);
        skip(&rows.plans, &mut rows.entries, self.entry)?;
        rows.write_back(bills);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lib_core::BillPlanId;

    use super::*;
    use crate::interactor::test_support::{cents, date, entry_of, stub, today};
    use crate::{Resolution, entry};

    #[test]
    fn skipping_keeps_the_planned_amount_as_it_was() {
        let mut ports = stub();
        let origin = entry_of(&ports, "Origin Energy", date(2026, 9, 24));

        SkipBill { entry: origin }
            .run(&mut ports.bills)
            .expect("an open entry");

        assert_eq!(
            entry(&ports.bills.entries, origin).map(|e| e.resolution.clone()),
            Some(Resolution::Skipped {
                planned: cents(15_000)
            })
        );
    }

    #[test]
    fn a_settled_or_missing_entry_is_refused_and_nothing_is_written() {
        let mut ports = stub();
        let origin = entry_of(&ports, "Origin Energy", date(2026, 9, 24));
        SkipBill { entry: origin }
            .run(&mut ports.bills)
            .expect("an open entry");
        let before = ports.bills.clone();

        assert_eq!(
            SkipBill { entry: origin }.run(&mut ports.bills),
            Err(Error::EntryNotOpen)
        );
        let missing = EntryId {
            plan_id: BillPlanId::seed(9_999),
            due: today(),
        };
        assert_eq!(
            SkipBill { entry: missing }.run(&mut ports.bills),
            Err(Error::EntryNotFound)
        );
        assert_eq!(ports.bills, before);
    }
}
