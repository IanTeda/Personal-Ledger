//! Pay it directly, the module's one Interactor: it writes a Transaction and the entry it settles,
//! two domains, so it lives here rather than in `lib-bills` (Match, which writes only the entry, is
//! `lib-bills`' `MatchBill`). It checks everything before writing, so a refusal writes nothing.

use bigdecimal::Signed;
use chrono::NaiveDate;
use lib_bills::{BillError, BillScheduleEntry, EntryId, Resolution, SplitRef};
use lib_core::{Money, TransactionId, TransactionStatus};
use lib_transactions::{Split, Transaction};

use crate::protocols::BillPayments;

#[derive(Debug, Clone, PartialEq)]
pub struct PayBill {
    pub entry: EntryId,
    pub amount: Money,
    pub date: NaiveDate,
}

impl PayBill {
    /// Books an Open Transaction on `date` in the Plan's Account, with one Split of `-amount`
    /// carrying the Plan's Category and Payee, and records the entry Paid by that Split. Returns
    /// the new Transaction's id. (ADR-0019 says Pending; the Transactions' least-confirmed status
    /// is Open.)
    ///
    /// # Errors
    /// `EntryNotFound` or `EntryNotOpen` for an entry that is gone or settled, `PlanNotFound` for
    /// a Plan that is gone, and `AmountNotPositive` for an amount of zero or less.
    pub fn run(self, payments: &mut dyn BillPayments) -> Result<TransactionId, BillError> {
        self.run_as(TransactionId::new(), payments)
    }

    /// [`Self::run`] under a given Transaction id, for the stub seed, whose ids are fixed.
    ///
    /// # Errors
    /// As [`Self::run`].
    pub fn run_as(
        self,
        transaction_id: TransactionId,
        payments: &mut dyn BillPayments,
    ) -> Result<TransactionId, BillError> {
        let entry = payments.entry(self.entry).ok_or(BillError::EntryNotFound)?;
        if !entry.is_open() {
            return Err(BillError::EntryNotOpen);
        }
        let plan = payments
            .bill_plan(self.entry.plan_id)
            .ok_or(BillError::PlanNotFound)?;
        if !self.amount.0.is_positive() {
            return Err(BillError::AmountNotPositive);
        }
        let transaction = Transaction {
            id: transaction_id,
            date: self.date,
            account_id: plan.account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: Some(plan.name),
            splits: vec![Split {
                amount: Money(-self.amount.0),
                category_id: plan.category_id,
                payee_id: plan.payee_id,
                tag_ids: Vec::new(),
            }],
        };
        payments.record_payment(
            transaction,
            BillScheduleEntry {
                resolution: Resolution::Paid(SplitRef {
                    transaction_id,
                    split_index: 0,
                }),
                ..entry
            },
        );
        Ok(transaction_id)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_bills::EntryId;
    use lib_bills::fake::InMemoryPorts;
    use lib_core::Money;

    // `date`, `today` and `cents` repeat `lib-bills`' own test helpers on purpose: those sit
    // behind `#[cfg(test)]`, and sharing three one-liners would mean a public test-support
    // surface on `lib-bills` that a feature crate's API shouldn't carry.
    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// The day the stub is seeded on, as `lib-bills`' tests use.
    pub fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    pub fn cents(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    /// The Bills stub seed as of [`today`], shared.
    pub fn stub() -> Rc<RefCell<InMemoryPorts>> {
        Rc::new(RefCell::new(InMemoryPorts::stub(today())))
    }

    /// The stub Plan named `name`'s entry due on `due`.
    pub fn entry_of(ports: &Rc<RefCell<InMemoryPorts>>, name: &str, due: NaiveDate) -> EntryId {
        let plan = ports
            .borrow()
            .bills
            .plans
            .iter()
            .find(|plan| plan.name == name)
            .map(|plan| plan.id)
            .unwrap_or_else(|| panic!("the stub has {name}"));
        EntryId { plan_id: plan, due }
    }
}

#[cfg(test)]
mod tests {
    use lib_bills::{
        BillPlanRepository as _, BillStatus, amount, carries_bill, entry, paid_on, status,
    };
    use lib_core::{AccountId, BillPlanId};

    use super::test_support::{cents, date, entry_of, stub, today};
    use super::*;
    use crate::fake::{InMemoryPayments, PortsPayments};

    #[test]
    fn paying_books_an_open_transaction_and_matches_its_split() {
        let ports = stub();
        let due = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let mut payments = InMemoryPayments {
            bills: ports.borrow().bills.clone(),
            ..InMemoryPayments::default()
        };

        let tid = PayBill {
            entry: due,
            amount: cents(8_900),
            date: today(),
        }
        .run(&mut payments)
        .expect("an open entry");

        let [t] = payments.transactions.transactions.as_slice() else {
            panic!("one Transaction is booked");
        };
        assert_eq!(t.id, tid);
        assert_eq!(t.status, TransactionStatus::Open);
        assert_eq!(t.total(), cents(-8_900));
        assert_eq!(t.account_id, AccountId::seed(2));
        assert_eq!(t.description.as_deref(), Some("Telstra Internet"));
        let plan = payments
            .bills
            .bill_plan(due.plan_id)
            .expect("the Plan is there");
        assert_eq!(t.splits.len(), 1);
        assert_eq!(t.splits[0].category_id, plan.category_id);
        assert_eq!(t.splits[0].payee_id, plan.payee_id);
        let entries = &payments.bills.entries;
        assert!(carries_bill(entries, tid));
        let paid = entry(entries, due).expect("the entry is there");
        assert_eq!(status(paid, today()), BillStatus::Paid);
    }

    /// The Bills page reads the payment at once, through the Splits it reads.
    #[test]
    fn a_payment_is_read_back_as_its_split() {
        let ports = stub();
        let due = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let before = ports.borrow().reads.splits.len();

        PayBill {
            entry: due,
            amount: cents(8_900),
            date: today(),
        }
        .run(&mut PortsPayments(ports.clone()))
        .expect("an open entry");

        let ports = ports.borrow();
        let splits = &ports.reads.splits;
        assert_eq!(splits.len(), before + 1);
        let paid = entry(&ports.bills.entries, due).expect("the entry is there");
        assert_eq!(paid_on(paid, splits), Some(today()));
        assert_eq!(amount(paid, &ports.bills.plans, splits), Some(cents(8_900)));
    }

    #[test]
    fn a_refused_pay_writes_nothing() {
        let ports = stub();
        let due = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let before = ports.borrow().clone();
        let pay = |entry, amount| {
            PayBill {
                entry,
                amount,
                date: today(),
            }
            .run(&mut PortsPayments(ports.clone()))
        };

        assert_eq!(pay(due, cents(0)), Err(BillError::AmountNotPositive));
        assert_eq!(pay(due, cents(-100)), Err(BillError::AmountNotPositive));
        let gone = EntryId {
            plan_id: BillPlanId::seed(9_999),
            due: today(),
        };
        assert_eq!(pay(gone, cents(100)), Err(BillError::EntryNotFound));
        assert_eq!(*ports.borrow(), before);
    }

    #[test]
    fn a_settled_entry_cannot_be_paid_again() {
        let ports = stub();
        let due = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let pay = || {
            PayBill {
                entry: due,
                amount: cents(100),
                date: today(),
            }
            .run(&mut PortsPayments(ports.clone()))
        };

        assert!(pay().is_ok());
        assert_eq!(pay(), Err(BillError::EntryNotOpen));
    }
}
