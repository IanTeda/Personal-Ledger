//! Add bill plan: a new active Plan, checked against the Accounts and Categories, with its
//! Schedule generated from its First Due.

use chrono::NaiveDate;
use lib_core::BillPlanId;

use super::Rows;
use crate::protocols::{BillAccounts, BillCategories, BillsRepository};
use crate::{BillError, BillPlanDraft, insert_plan};

#[derive(Debug, Clone, PartialEq)]
pub struct AddBillPlan {
    pub draft: BillPlanDraft,
    pub today: NaiveDate,
}

impl AddBillPlan {
    /// Writes the Plan and each entry it generates, returning the Plan's id.
    ///
    /// # Errors
    /// The draft's refusal: a blank name, a Category that isn't an Expense leaf, an Account that
    /// takes no Transactions or isn't in the Unit, an amount not above zero, or a bad Ends On.
    pub fn run(
        self,
        bills: &mut dyn BillsRepository,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> Result<BillPlanId, BillError> {
        let mut rows = Rows::read(bills);
        let id = insert_plan(
            &mut rows.plans,
            &mut rows.entries,
            &self.draft,
            &categories.bill_categories(),
            &accounts.bill_accounts(),
            self.today,
        )?;
        rows.write_back(bills);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use lib_core::CategoryId;

    use super::*;
    use crate::Recurrence;
    use crate::interactor::test_support::{draft, dues, stub, today};

    fn add(
        ports: &mut crate::fake::InMemoryPorts,
        draft: BillPlanDraft,
    ) -> Result<BillPlanId, BillError> {
        let ports = &mut *ports;
        AddBillPlan {
            draft,
            today: today(),
        }
        .run(&mut ports.bills, &ports.reads, &ports.reads)
    }

    #[test]
    fn adding_writes_the_plan_and_its_schedule() {
        let mut ports = stub();
        let before = ports.bills.plans.len();

        let id = add(&mut ports, draft(Recurrence::Monthly, today())).expect("a valid draft");

        assert_eq!(ports.bills.plans.len(), before + 1);
        let plan = ports
            .bills
            .plans
            .iter()
            .find(|p| p.id == id)
            .expect("written");
        assert!(plan.is_active);
        assert_eq!(dues(&ports, id).len(), 2, "this month and next");
    }

    #[test]
    fn a_refused_draft_writes_nothing() {
        let mut ports = stub();
        let before = ports.bills.clone();

        let refused = add(
            &mut ports,
            BillPlanDraft {
                category_id: CategoryId::seed(11),
                ..draft(Recurrence::Monthly, today())
            },
        );

        assert_eq!(refused, Err(BillError::CategoryNotExpenseLeaf));
        assert_eq!(ports.bills, before);
    }
}
