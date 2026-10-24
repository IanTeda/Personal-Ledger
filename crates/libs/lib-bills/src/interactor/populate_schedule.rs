//! Generation: brings the Bill Schedule up to the horizon as of a day. A Client runs it on start
//! and when it crosses into a new calendar month while running (ADR-0019).

use chrono::NaiveDate;

use super::Rows;
use crate::populate;
use crate::protocols::BillsRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulateSchedule {
    pub today: NaiveDate,
}

impl PopulateSchedule {
    /// Writes each entry generation adds or revives, returning how many. Never refused: a Plan
    /// with nothing due adds nothing.
    pub fn run(self, bills: &mut dyn BillsRepository) -> usize {
        let mut rows = Rows::read(bills);
        populate(&rows.plans, &mut rows.entries, self.today);
        let written = rows.write_back(bills);
        tracing::info!(written, today = %self.today, "Populated the Bill Schedule");
        written
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, BillPlanId, BillScheduleEntryId, CategoryId, UnitId};

    use super::*;
    use crate::fake::InMemoryBills;
    use crate::interactor::test_support::{cents, date, stub, today};
    use crate::{AmountKind, BillPlan, BillScheduleEntry, Recurrence, Resolution};

    fn rent() -> BillPlan {
        BillPlan {
            id: BillPlanId::seed(1),
            name: "Rent".into(),
            category_id: CategoryId::seed(1),
            unit: UnitId::SEED_AUD,
            account_id: AccountId::seed(1),
            payee_id: None,
            planned_amount: cents(10_000),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::Monthly,
            first_due: date(2026, 9, 20),
            ends_on: None,
            attention_lead: None,
            is_active: true,
            generate_from: date(2026, 9, 20),
        }
    }

    #[test]
    fn it_writes_only_the_entries_it_adds() {
        let stored = BillScheduleEntry {
            row_id: BillScheduleEntryId::new(),
            plan_id: BillPlanId::seed(1),
            due: date(2026, 9, 20),
            resolution: Resolution::Unresolved,
            superseded: false,
        };
        let mut bills = InMemoryBills {
            plans: vec![rent()],
            entries: vec![stored.clone()],
        };

        assert_eq!(PopulateSchedule { today: today() }.run(&mut bills), 1);
        assert_eq!(PopulateSchedule { today: today() }.run(&mut bills), 0);
        assert_eq!(
            bills.entries[0], stored,
            "the stored entry is left as it was"
        );
        assert_eq!(bills.entries[1].due, date(2026, 10, 20));
    }

    #[test]
    fn the_stub_is_already_generated() {
        let mut ports = stub();
        assert_eq!(PopulateSchedule { today: today() }.run(&mut ports.bills), 0);
    }

    #[test]
    fn a_new_month_generates_the_month_after_it() {
        let mut ports = stub();
        let rent = ports
            .bills
            .plans
            .iter()
            .find(|plan| plan.name == "Rent")
            .expect("the stub has Rent")
            .id;
        let november = |ports: &crate::fake::InMemoryPorts| {
            ports
                .bills
                .entries
                .iter()
                .any(|e| e.plan_id == rent && e.due == date(2026, 11, 1))
        };
        assert!(!november(&ports), "past September's horizon");

        let written = PopulateSchedule {
            today: date(2026, 10, 1),
        }
        .run(&mut ports.bills);

        assert!(written > 0);
        assert!(november(&ports));
    }
}
