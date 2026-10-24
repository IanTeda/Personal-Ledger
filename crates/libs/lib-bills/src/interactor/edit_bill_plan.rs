//! Edit bill plan: saves the Plan's fields and its activation in one write, regenerating its
//! Schedule as the rules say. The Unit is locked.

use chrono::NaiveDate;
use lib_core::BillPlanId;

use super::Rows;
use crate::protocols::{BillAccounts, BillCategories, BillsRepository};
use crate::{BillError, BillPlanDraft, edit_plan, set_active};

#[derive(Debug, Clone, PartialEq)]
pub struct EditBillPlan {
    pub id: BillPlanId,
    pub draft: BillPlanDraft,
    pub is_active: bool,
    pub today: NaiveDate,
}

impl EditBillPlan {
    /// Writes the Plan and each entry the edit supersedes or generates. A refusal in either step
    /// writes nothing.
    ///
    /// # Errors
    /// `PlanNotFound` for a Plan that is gone, `UnitLocked` for a changed Unit, or the draft's
    /// refusal as [`super::AddBillPlan`] checks it.
    pub fn run(
        self,
        bills: &mut dyn BillsRepository,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> Result<(), BillError> {
        let (id, is_active) = (self.id, self.is_active);
        let outcome = self.apply(bills, accounts, categories);
        match &outcome {
            Ok(()) => tracing::info!(bill_plan_id = %id, is_active, "Edited a Bill Plan"),
            Err(refusal) => {
                tracing::info!(bill_plan_id = %id, is_active, %refusal, "Refused to edit a Bill Plan");
            }
        }
        outcome
    }

    fn apply(
        self,
        bills: &mut dyn BillsRepository,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> Result<(), BillError> {
        let mut rows = Rows::read(bills);
        edit_plan(
            &mut rows.plans,
            &mut rows.entries,
            self.id,
            &self.draft,
            &categories.bill_categories(),
            &accounts.bill_accounts(),
            self.today,
        )?;
        set_active(
            &mut rows.plans,
            &mut rows.entries,
            self.id,
            self.is_active,
            self.today,
        )?;
        rows.write_back(bills);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lib_core::UnitId;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::AddBillPlan;
    use crate::interactor::test_support::{date, draft, dues, stub, today};
    use crate::{BillPlan, Recurrence};

    fn add(ports: &mut InMemoryPorts, draft: BillPlanDraft) -> BillPlanId {
        AddBillPlan {
            draft,
            today: today(),
        }
        .run(&mut ports.bills, &ports.reads, &ports.reads)
        .expect("a valid draft")
    }

    fn edit(
        ports: &mut InMemoryPorts,
        id: BillPlanId,
        draft: BillPlanDraft,
        is_active: bool,
    ) -> Result<(), BillError> {
        EditBillPlan {
            id,
            draft,
            is_active,
            today: today(),
        }
        .run(&mut ports.bills, &ports.reads, &ports.reads)
    }

    fn plan(ports: &InMemoryPorts, id: BillPlanId) -> BillPlan {
        ports
            .bills
            .plans
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .expect("the Plan is stored")
    }

    #[test]
    fn a_new_recurrence_supersedes_future_open_entries_only() {
        let mut ports = stub();
        let id = add(&mut ports, draft(Recurrence::Monthly, date(2026, 8, 25)));

        edit(
            &mut ports,
            id,
            draft(Recurrence::Monthly, date(2026, 8, 28)),
            true,
        )
        .expect("a valid edit");

        // Aug 25 is Overdue and survives the edit.
        assert_eq!(
            dues(&ports, id),
            [date(2026, 8, 25), date(2026, 9, 28), date(2026, 10, 28)]
        );
        let superseded: Vec<_> = ports
            .bills
            .entries
            .iter()
            .filter(|e| e.plan_id == id && e.superseded)
            .map(|e| e.due)
            .collect();
        assert_eq!(superseded, [date(2026, 9, 25), date(2026, 10, 25)]);
    }

    #[test]
    fn a_plain_edit_saves_the_plan_and_leaves_the_schedule() {
        let mut ports = stub();
        let id = add(&mut ports, draft(Recurrence::Monthly, date(2026, 9, 25)));
        let entries = ports.bills.entries.clone();

        edit(
            &mut ports,
            id,
            BillPlanDraft {
                name: "Sydney Water".into(),
                ..draft(Recurrence::Monthly, date(2026, 9, 25))
            },
            true,
        )
        .expect("a valid edit");

        assert_eq!(plan(&ports, id).name, "Sydney Water");
        assert_eq!(ports.bills.entries, entries);
    }

    #[test]
    fn deactivating_supersedes_and_reactivating_regenerates_without_backfill() {
        let mut ports = stub();
        let id = add(&mut ports, draft(Recurrence::Weekly, date(2026, 9, 12)));

        edit(
            &mut ports,
            id,
            draft(Recurrence::Weekly, date(2026, 9, 12)),
            false,
        )
        .expect("a valid edit");

        assert!(!plan(&ports, id).is_active);
        assert_eq!(dues(&ports, id), [date(2026, 9, 12)], "Overdue stays");
    }

    #[test]
    fn the_unit_is_locked_and_a_refusal_writes_nothing() {
        let mut ports = stub();
        let id = add(&mut ports, draft(Recurrence::Monthly, today()));
        let before = ports.bills.clone();

        let refused = edit(
            &mut ports,
            id,
            BillPlanDraft {
                unit: UnitId::SEED_BTC,
                ..draft(Recurrence::Monthly, today())
            },
            false,
        );

        assert_eq!(refused, Err(BillError::UnitLocked(UnitId::SEED_AUD)));
        assert_eq!(ports.bills, before);
        assert_eq!(
            edit(
                &mut ports,
                BillPlanId::seed(9_999),
                draft(Recurrence::Monthly, today()),
                true
            ),
            Err(BillError::PlanNotFound)
        );
    }
}
