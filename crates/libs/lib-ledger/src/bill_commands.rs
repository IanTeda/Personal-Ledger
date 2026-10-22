//! The Bills write `lib-bills` can't own yet (ADR-0038): Pay it directly, which writes a
//! Transaction as well as the entry. It checks everything before writing, so a refusal leaves the
//! Ledger untouched, and records an upsert per row it wrote (ADR-0039). Today's Pay Dialog runs it
//! through the Desktop's forwarding shim until `lib-bill-payments` takes it (#779); the other
//! Bills writes are `lib-bills`' Interactors (#778).

use chrono::NaiveDate;
use lib_bills::{self as bills, BillError, BillPlan, BillScheduleEntry, EntryId};
use lib_core::{BillPlanId, Money, TransactionId};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
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

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use lib_bills::{Resolution, SplitRef};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn ledger() -> Ledger {
        let mut ledger = Ledger::stub_on(today());
        ledger.drain_changes();
        ledger
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
}
