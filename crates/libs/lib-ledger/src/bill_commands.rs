//! The Bills writes (ADR-0038): Bill Plan add and edit, and a Bill Schedule entry's pay, match,
//! skip and undo. Each is a command value with its own refusal, checked before anything is
//! written so a refusal leaves the Ledger untouched, and recording an upsert per row it wrote
//! (ADR-0039).
//!
//! Single-Plan and single-entry rules stay in `lib_bills`. What lives here crosses domains:
//! a Plan is checked against the Accounts and Categories, and Pay writes a Transaction.

use chrono::NaiveDate;
use lib_bills::{
    self as bills, BillError, BillPlan, BillPlanDraft, BillScheduleEntry, EntryId, SplitRef,
};
use lib_core::{BillPlanId, Money, TransactionId};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Adds the Bill Plan `draft` describes, active, and generates its entries.
    pub fn add_bill_plan(
        &mut self,
        draft: &BillPlanDraft,
        today: NaiveDate,
    ) -> Result<BillPlanId, BillError> {
        let mut plans = self.bill_plans.clone();
        let mut entries = self.bill_entries.clone();
        let id = bills::insert_plan(
            &mut plans,
            &mut entries,
            draft,
            &self.categories,
            &self.accounts,
            today,
        )?;
        self.land_bills(plans, entries, id);
        Ok(id)
    }

    /// Saves Bill Plan `id` and sets whether it is active, regenerating its Schedule as the rules
    /// say. A refusal in either step leaves both undone.
    pub fn edit_bill_plan(
        &mut self,
        id: BillPlanId,
        draft: &BillPlanDraft,
        is_active: bool,
        today: NaiveDate,
    ) -> Result<(), BillError> {
        let mut plans = self.bill_plans.clone();
        let mut entries = self.bill_entries.clone();
        bills::edit_plan(
            &mut plans,
            &mut entries,
            id,
            draft,
            &self.categories,
            &self.accounts,
            today,
        )?;
        bills::set_active(&mut plans, &mut entries, id, is_active, today)?;
        self.land_bills(plans, entries, id);
        Ok(())
    }

    /// Pays entry `entry` directly: books an Open Transaction for `amount` on `date` and matches
    /// the entry to it. Returns the new Transaction's id.
    pub fn pay_bill(
        &mut self,
        entry: EntryId,
        amount: &Money,
        date: NaiveDate,
    ) -> Result<TransactionId, BillError> {
        let mut entries = self.bill_entries.clone();
        let mut transactions = self.transactions.clone();
        let transaction_id = bills::pay(
            &self.bill_plans,
            &mut entries,
            &mut transactions,
            entry,
            amount,
            date,
        )?;
        self.transactions = transactions;
        if let Some(transaction) = self.transactions.iter().find(|t| t.id == transaction_id) {
            self.pending
                .push(LedgerChange::UpsertTransaction(transaction.clone()));
        }
        self.land_bills(self.bill_plans.clone(), entries, entry.plan_id);
        Ok(transaction_id)
    }

    /// Matches entry `entry` to the existing `split`, one of its candidates.
    pub fn match_bill(&mut self, entry: EntryId, split: SplitRef) -> Result<(), BillError> {
        let mut entries = self.bill_entries.clone();
        bills::match_split(
            &self.bill_plans,
            &mut entries,
            &self.transactions,
            &self.accounts,
            &self.categories,
            entry,
            split,
        )?;
        self.land_bills(self.bill_plans.clone(), entries, entry.plan_id);
        Ok(())
    }

    /// Skips open entry `entry`.
    pub fn skip_bill(&mut self, entry: EntryId) -> Result<(), BillError> {
        let mut entries = self.bill_entries.clone();
        bills::skip(&self.bill_plans, &mut entries, entry)?;
        self.land_bills(self.bill_plans.clone(), entries, entry.plan_id);
        Ok(())
    }

    /// Undoes a Paid entry: clears the match and leaves the Transaction in place.
    pub fn unmatch_bill(&mut self, entry: EntryId) -> Result<(), BillError> {
        let mut entries = self.bill_entries.clone();
        bills::unmatch(&mut entries, entry)?;
        self.land_bills(self.bill_plans.clone(), entries, entry.plan_id);
        Ok(())
    }

    /// Undoes a Skipped entry.
    pub fn unskip_bill(&mut self, entry: EntryId) -> Result<(), BillError> {
        let mut entries = self.bill_entries.clone();
        bills::unskip(&mut entries, entry)?;
        self.land_bills(self.bill_plans.clone(), entries, entry.plan_id);
        Ok(())
    }

    /// Replaces the Plans and entries with the checked result and records Plan `plan_id`'s upsert
    /// and an upsert per entry that is new or differs from before. Entries of other Plans are
    /// never touched by a command, so they're not compared.
    fn land_bills(
        &mut self,
        plans: Vec<BillPlan>,
        entries: Vec<BillScheduleEntry>,
        plan_id: BillPlanId,
    ) {
        let changed: Vec<BillScheduleEntry> = entries
            .iter()
            .filter(|e| e.plan_id == plan_id && !self.bill_entries.contains(e))
            .cloned()
            .collect();
        let plan_changed = bills::get(&plans, plan_id)
            .filter(|plan| bills::get(&self.bill_plans, plan_id) != Some(*plan))
            .cloned();
        self.bill_plans = plans;
        self.bill_entries = entries;
        if let Some(plan) = plan_changed {
            self.pending.push(LedgerChange::UpsertBillPlan(plan));
        }
        self.pending
            .extend(changed.into_iter().map(LedgerChange::UpsertBillEntry));
    }
}

/// [`Ledger::add_bill_plan`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct AddBillPlan {
    pub draft: BillPlanDraft,
    pub today: NaiveDate,
}

impl LedgerCommand for AddBillPlan {
    type Outcome = BillPlanId;
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<BillPlanId, BillError> {
        ledger.add_bill_plan(&self.draft, self.today)
    }
}

/// [`Ledger::edit_bill_plan`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct EditBillPlan {
    pub id: BillPlanId,
    pub draft: BillPlanDraft,
    pub is_active: bool,
    pub today: NaiveDate,
}

impl LedgerCommand for EditBillPlan {
    type Outcome = ();
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BillError> {
        ledger.edit_bill_plan(self.id, &self.draft, self.is_active, self.today)
    }
}

/// [`Ledger::pay_bill`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct PayBill {
    pub entry: EntryId,
    pub amount: Money,
    pub date: NaiveDate,
}

impl LedgerCommand for PayBill {
    type Outcome = TransactionId;
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<TransactionId, BillError> {
        ledger.pay_bill(self.entry, &self.amount, self.date)
    }
}

/// [`Ledger::match_bill`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBill {
    pub entry: EntryId,
    pub split: SplitRef,
}

impl LedgerCommand for MatchBill {
    type Outcome = ();
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BillError> {
        ledger.match_bill(self.entry, self.split)
    }
}

/// [`Ledger::skip_bill`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipBill(pub EntryId);

impl LedgerCommand for SkipBill {
    type Outcome = ();
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BillError> {
        ledger.skip_bill(self.0)
    }
}

/// [`Ledger::unmatch_bill`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmatchBill(pub EntryId);

impl LedgerCommand for UnmatchBill {
    type Outcome = ();
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BillError> {
        ledger.unmatch_bill(self.0)
    }
}

/// [`Ledger::unskip_bill`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnskipBill(pub EntryId);

impl LedgerCommand for UnskipBill {
    type Outcome = ();
    type Refusal = BillError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BillError> {
        ledger.unskip_bill(self.0)
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use lib_bills::{AmountKind, Recurrence, Resolution};
    use lib_core::UnitId;

    use super::*;
    use lib_core::{AccountId, CategoryId};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn ledger() -> Ledger {
        let mut ledger = Ledger::stub_on(today());
        ledger.drain_changes();
        ledger
    }

    fn draft() -> BillPlanDraft {
        BillPlanDraft {
            name: "Water".to_owned(),
            category_id: CategoryId::seed(5),
            unit: UnitId::SEED_AUD,
            account_id: AccountId::seed(2),
            payee_id: None,
            planned_amount: Money(BigDecimal::new(9_000.into(), 2)),
            amount_kind: AmountKind::Estimated,
            recurrence: Recurrence::Monthly,
            first_due: today(),
            ends_on: None,
            attention_lead: None,
        }
    }

    /// An open entry of the stub's first Plan that has one.
    fn open_entry(ledger: &Ledger) -> EntryId {
        ledger
            .bill_entries()
            .iter()
            .find(|e| e.is_open())
            .expect("the stub has an open entry")
            .id()
    }

    fn assert_untouched(ledger: &mut Ledger, before: &Ledger) {
        assert_eq!(ledger.bill_plans(), before.bill_plans());
        assert_eq!(ledger.bill_entries(), before.bill_entries());
        assert_eq!(ledger.transactions(), before.transactions());
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn adding_a_plan_records_it_and_each_generated_entry() {
        let mut ledger = ledger();
        let id = AddBillPlan {
            draft: draft(),
            today: today(),
        }
        .run(&mut ledger)
        .unwrap();

        let plan = bills::get(ledger.bill_plans(), id).unwrap().clone();
        assert!(plan.is_active);
        let generated = ledger
            .bill_entries()
            .iter()
            .filter(|e| e.plan_id == id)
            .count();
        assert!(generated > 0);
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1 + generated);
        assert_eq!(changes[0], LedgerChange::UpsertBillPlan(plan));
        assert!(
            changes[1..]
                .iter()
                .all(|c| matches!(c, LedgerChange::UpsertBillEntry(e) if e.plan_id == id))
        );
    }

    #[test]
    fn a_refused_add_changes_nothing() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let refused = AddBillPlan {
            draft: BillPlanDraft {
                name: "  ".to_owned(),
                ..draft()
            },
            today: today(),
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(BillError::NameRequired));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn editing_a_plan_saves_it_and_deactivating_supersedes_its_entries() {
        let mut ledger = ledger();
        let id = ledger.bill_plans()[0].id;
        let base = ledger.bill_plans()[0].clone();
        let edit = BillPlanDraft {
            name: "Renamed".to_owned(),
            category_id: base.category_id,
            unit: base.unit,
            account_id: base.account_id,
            payee_id: base.payee_id,
            planned_amount: base.planned_amount.clone(),
            amount_kind: base.amount_kind,
            recurrence: base.recurrence,
            first_due: base.first_due,
            ends_on: base.ends_on,
            attention_lead: base.attention_lead,
        };

        EditBillPlan {
            id,
            draft: edit,
            is_active: false,
            today: today(),
        }
        .run(&mut ledger)
        .unwrap();

        let plan = bills::get(ledger.bill_plans(), id).unwrap().clone();
        assert_eq!(plan.name, "Renamed");
        assert!(!plan.is_active);
        let changes = ledger.drain_changes();
        assert_eq!(changes[0], LedgerChange::UpsertBillPlan(plan));
        assert!(changes[1..].iter().all(
            |c| matches!(c, LedgerChange::UpsertBillEntry(e) if e.plan_id == id && e.superseded)
        ));
    }

    #[test]
    fn a_refused_edit_changes_nothing() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let refused = EditBillPlan {
            id: BillPlanId::seed(9_999),
            draft: draft(),
            is_active: true,
            today: today(),
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(BillError::PlanNotFound));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn paying_books_a_transaction_and_matches_the_entry() {
        let mut ledger = ledger();
        let entry = open_entry(&ledger);
        let before = ledger.transactions().len();

        let transaction_id = PayBill {
            entry,
            amount: Money(BigDecimal::new(1_234.into(), 2)),
            date: today(),
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(ledger.transactions().len(), before + 1);
        let found = bills::entry(ledger.bill_entries(), entry).unwrap().clone();
        assert_eq!(
            found.resolution,
            Resolution::Paid(SplitRef {
                transaction_id,
                split_index: 0
            })
        );
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 2);
        assert!(
            matches!(&changes[0], LedgerChange::UpsertTransaction(t) if t.id == transaction_id)
        );
        assert_eq!(changes[1], LedgerChange::UpsertBillEntry(found));
    }

    #[test]
    fn a_refused_pay_changes_nothing() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let entry = open_entry(&ledger);

        let refused = PayBill {
            entry,
            amount: Money(BigDecimal::new(0.into(), 2)),
            date: today(),
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(BillError::AmountNotPositive));
        assert_untouched(&mut ledger, &before);

        let gone = PayBill {
            entry: EntryId {
                plan_id: BillPlanId::seed(9_999),
                due: today(),
            },
            amount: Money(BigDecimal::new(100.into(), 2)),
            date: today(),
        }
        .run(&mut ledger);
        assert_eq!(gone, Err(BillError::EntryNotFound));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn matching_a_non_candidate_is_refused_and_changes_nothing() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let entry = open_entry(&ledger);
        let refused = MatchBill {
            entry,
            split: SplitRef {
                transaction_id: TransactionId::seed(9_999),
                split_index: 0,
            },
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(BillError::NotACandidate));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn matching_a_candidate_records_the_entry_and_unmatch_undoes_it() {
        let mut ledger = ledger();
        let (entry, split) = ledger
            .bill_entries()
            .iter()
            .filter(|e| e.is_open())
            .find_map(|e| {
                bills::match_candidates(
                    ledger.bill_plans(),
                    ledger.bill_entries(),
                    ledger.transactions(),
                    ledger.accounts(),
                    ledger.categories(),
                    e.id(),
                )
                .first()
                .map(|split| (e.id(), *split))
            })
            .expect("the stub has a matchable entry");

        MatchBill { entry, split }.run(&mut ledger).unwrap();
        let found = bills::entry(ledger.bill_entries(), entry).unwrap().clone();
        assert_eq!(found.resolution, Resolution::Paid(split));
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertBillEntry(found)]
        );

        UnmatchBill(entry).run(&mut ledger).unwrap();
        assert!(
            bills::entry(ledger.bill_entries(), entry)
                .unwrap()
                .is_unresolved()
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn skipping_snapshots_the_planned_amount_and_unskip_undoes_it() {
        let mut ledger = ledger();
        let entry = open_entry(&ledger);
        SkipBill(entry).run(&mut ledger).unwrap();
        let found = bills::entry(ledger.bill_entries(), entry).unwrap().clone();
        assert!(matches!(found.resolution, Resolution::Skipped { .. }));
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertBillEntry(found)]
        );

        UnskipBill(entry).run(&mut ledger).unwrap();
        assert!(
            bills::entry(ledger.bill_entries(), entry)
                .unwrap()
                .is_unresolved()
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn skip_and_the_undos_refuse_without_touching_the_ledger() {
        let mut ledger = ledger();
        let before = ledger.clone();
        let entry = open_entry(&ledger);
        assert_eq!(UnmatchBill(entry).run(&mut ledger), Err(BillError::NotPaid));
        assert_eq!(
            UnskipBill(entry).run(&mut ledger),
            Err(BillError::NotSkipped)
        );
        let missing = EntryId {
            plan_id: BillPlanId::seed(9_999),
            due: today(),
        };
        assert_eq!(
            SkipBill(missing).run(&mut ledger),
            Err(BillError::EntryNotFound)
        );
        assert_untouched(&mut ledger, &before);

        SkipBill(entry).run(&mut ledger).unwrap();
        ledger.drain_changes();
        let after_skip = ledger.clone();
        assert_eq!(
            SkipBill(entry).run(&mut ledger),
            Err(BillError::EntryNotOpen)
        );
        assert_untouched(&mut ledger, &after_skip);
    }
}
