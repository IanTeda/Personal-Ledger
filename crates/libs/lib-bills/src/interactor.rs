//! One Interactor per Bills intent, each a request value whose `run` takes the ports it needs.
//! Each reads the Plans and entries, applies `bills`' rules to a copy, and writes back only the
//! rows that changed, so a refusal leaves the ports untouched. Refusals are [`Error`]s.
//!
//! Pay it directly isn't here: it writes a Transaction too, so it is `lib-bill-payments`' `PayBill`.

mod add_bill_plan;
mod edit_bill_plan;
mod match_bill;
mod populate_schedule;
mod skip_bill;
mod unmatch_bill;
mod unskip_bill;

pub use add_bill_plan::AddBillPlan;
pub use edit_bill_plan::EditBillPlan;
pub use match_bill::{MatchBill, MatchCandidates};
pub use populate_schedule::PopulateSchedule;
pub use skip_bill::SkipBill;
pub use unmatch_bill::UnmatchBill;
pub use unskip_bill::UnskipBill;

use crate::protocols::BillsRepository;
use crate::{BillPlan, BillScheduleEntry};

/// The Plans and entries as stored, copied for a rule to change.
struct Rows {
    plans: Vec<BillPlan>,
    entries: Vec<BillScheduleEntry>,
}

impl Rows {
    fn read(bills: &dyn BillsRepository) -> Self {
        Self {
            plans: bills.bill_plans(),
            entries: bills.entries(),
        }
    }

    /// Writes each Plan and entry that is new or differs from `bills`' rows, returning how many.
    fn write_back(self, bills: &mut dyn BillsRepository) -> usize {
        let (plans, entries) = (bills.bill_plans(), bills.entries());
        let changed_plans: Vec<BillPlan> = self
            .plans
            .into_iter()
            .filter(|plan| !plans.contains(plan))
            .collect();
        let changed_entries: Vec<BillScheduleEntry> = self
            .entries
            .into_iter()
            .filter(|entry| !entries.contains(entry))
            .collect();
        let written = changed_plans.len() + changed_entries.len();
        for plan in changed_plans {
            bills.upsert_bill_plan(plan);
        }
        for entry in changed_entries {
            bills.upsert_entry(entry);
        }
        written
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::{AccountId, CategoryId, Money, PayeeId, UnitId};

    use crate::fake::InMemoryPorts;
    use crate::{AmountKind, BillPlanDraft, EntryId, Recurrence};

    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// The day the stub is seeded on, as `lib-ledger`'s Bills tests use.
    pub fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    pub fn cents(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    /// The stub seed as of [`today`].
    pub fn stub() -> InMemoryPorts {
        InMemoryPorts::stub(today())
    }

    /// Water, a Monthly Plan in aud paid from ANZ Everyday.
    pub fn draft(recurrence: Recurrence, first_due: NaiveDate) -> BillPlanDraft {
        BillPlanDraft {
            name: "Water".to_string(),
            category_id: CategoryId::seed(5),
            unit: UnitId::SEED_AUD,
            account_id: AccountId::seed(2),
            payee_id: Some(PayeeId::seed(15)),
            planned_amount: cents(9_000),
            amount_kind: AmountKind::Estimated,
            recurrence,
            first_due,
            ends_on: None,
            attention_lead: None,
        }
    }

    /// The stub Plan named `name`'s entry due on `due`.
    pub fn entry_of(ports: &InMemoryPorts, name: &str, due: NaiveDate) -> EntryId {
        let plan = ports
            .bills
            .plans
            .iter()
            .find(|plan| plan.name == name)
            .unwrap_or_else(|| panic!("the stub has {name}"));
        EntryId {
            plan_id: plan.id,
            due,
        }
    }

    /// The non-superseded due dates of Plan `plan_id`, in order.
    pub fn dues(ports: &InMemoryPorts, plan_id: lib_core::BillPlanId) -> Vec<NaiveDate> {
        let mut dates: Vec<NaiveDate> = ports
            .bills
            .entries
            .iter()
            .filter(|e| e.plan_id == plan_id && !e.superseded)
            .map(|e| e.due)
            .collect();
        dates.sort();
        dates
    }
}
