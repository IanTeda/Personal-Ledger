//! The contract every stored Bills port keeps (ADR-0046), run against both the in-memory ports here
//! and the `lib-database` adapter, so the fake the Clients' tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the ports, reloads as a restart would, and expects the same rows back. The Splits Bills reads
//! are written by the Transactions, so the suite only seeds them and reads them back.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{
    AccountId, BillPlanId, BillScheduleEntryId, CategoryId, Money, PayeeId, TransactionId, UnitId,
};

use crate::fake::{InMemoryBillReads, InMemoryBills};
use crate::{
    AmountKind, BillPlan, BillPlanRepository, BillScheduleEntry, BillScheduleRepository, BillSplit,
    BillTransactions, EntryId, Recurrence, Resolution, SplitRef,
};

/// Builds the ports a contract check runs on.
pub trait PortsFactory {
    type Ports: BillPlanRepository + BillScheduleRepository;
    type Reads: BillTransactions;

    /// Ports holding `seed`'s Bill Plans and entries, as if stored and loaded, beside `splits`,
    /// which a Paid entry may name. Every Plan's Unit is the stub's aud, and the Splits of one
    /// Transaction share its date, Account and memo.
    fn create(&mut self, seed: InMemoryBills, splits: Vec<BillSplit>) -> Self::Ports;

    /// The Splits [`Self::create`] stored, read as Bills reads them.
    fn reads(&mut self) -> Self::Reads;

    /// Ends the writes made on `ports`, then loads fresh ports from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one call's writes on `ports`, sending them to storage as an adapter's owner does.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Bills domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

/// The in-memory ports are their own storage, so a reload hands them back as they are. They keep
/// a copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryBills>,
    splits: Vec<BillSplit>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryBills;
    type Reads = InMemoryBillReads;

    fn create(&mut self, seed: InMemoryBills, splits: Vec<BillSplit>) -> InMemoryBills {
        self.committed = Some(seed.clone());
        self.splits = splits;
        seed
    }

    fn reads(&mut self) -> InMemoryBillReads {
        InMemoryBillReads {
            splits: self.splits.clone(),
            ..InMemoryBillReads::default()
        }
    }

    fn reload(&mut self, mut ports: InMemoryBills) -> InMemoryBills {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryBills) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryBills) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryBills) -> (InMemoryBills, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

/// The ports' Bill Plans in id order and entries in Plan and due date order, so two ports compare
/// whatever order they list in.
pub fn snapshot(
    ports: &(impl BillPlanRepository + BillScheduleRepository),
) -> (Vec<BillPlan>, Vec<BillScheduleEntry>) {
    let mut plans = ports.bill_plans();
    plans.sort_by_key(|plan| plan.id);
    let mut entries = ports.entries();
    entries.sort_by_key(BillScheduleEntry::id);
    (plans, entries)
}

fn cents(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, day).expect("a September day")
}

fn plan(id: u32) -> BillPlan {
    BillPlan {
        id: BillPlanId::seed(id),
        name: format!("Plan {id}"),
        category_id: CategoryId::seed(7),
        unit: UnitId::SEED_AUD,
        account_id: AccountId::seed(1),
        payee_id: Some(PayeeId::seed(3)),
        planned_amount: cents(8_999),
        amount_kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: date(1),
        ends_on: None,
        attention_lead: Some(5),
        is_active: true,
        generate_from: date(1),
    }
}

fn entry(row: u32, plan: u32, day: u32, resolution: Resolution) -> BillScheduleEntry {
    BillScheduleEntry {
        row_id: BillScheduleEntryId::seed(row),
        plan_id: BillPlanId::seed(plan),
        due: date(day),
        resolution,
        superseded: false,
    }
}

fn paid(transaction: u32, split_index: usize) -> Resolution {
    Resolution::Paid(SplitRef {
        transaction_id: TransactionId::seed(transaction),
        split_index,
    })
}

/// The two Splits of one stored Transaction, which the Paid checks Match: one with a Payee, one
/// without.
fn receipt() -> Vec<BillSplit> {
    let split = |split_index, amount, payee_id| BillSplit {
        split: SplitRef {
            transaction_id: TransactionId::seed(1),
            split_index,
        },
        date: date(2),
        account_id: AccountId::seed(1),
        description: Some("Receipt".into()),
        amount: cents(amount),
        category_id: CategoryId::seed(7),
        payee_id,
    };
    vec![
        split(0, -8_999, Some(PayeeId::seed(3))),
        split(1, -4_500, None),
    ]
}

/// `splits` in Transaction and Split order, so two reads compare whatever order they list in.
fn sorted(mut splits: Vec<BillSplit>) -> Vec<BillSplit> {
    splits.sort_by_key(|s| (s.split.transaction_id, s.split.split_index));
    splits
}

/// Two Plans, one with an open entry.
fn seed() -> InMemoryBills {
    InMemoryBills {
        plans: vec![plan(1), plan(2)],
        entries: vec![entry(1, 1, 1, Resolution::Unresolved)],
    }
}

/// Every field round-trips: a new Plan with no Payee, Ends On or Attention Lead, an edited Plan,
/// and entries left open, superseded, Skipped with the amount it was skipped at, and Paid.
pub fn plans_and_entries_round_trip<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed(), receipt());

    ports.upsert_bill_plan(BillPlan {
        payee_id: None,
        amount_kind: AmountKind::Estimated,
        recurrence: Recurrence::Quarterly,
        ends_on: Some(date(30)),
        attention_lead: None,
        is_active: false,
        generate_from: date(9),
        ..plan(3)
    });
    ports.upsert_bill_plan(BillPlan {
        name: "Renamed".into(),
        planned_amount: cents(12_345),
        ..plan(2)
    });
    ports.upsert_entry(BillScheduleEntry {
        superseded: true,
        ..entry(2, 2, 8, Resolution::Unresolved)
    });
    ports.upsert_entry(entry(
        3,
        2,
        15,
        Resolution::Skipped {
            planned: cents(12_345),
        },
    ));
    ports.upsert_entry(entry(4, 3, 3, paid(1, 1)));

    let before = snapshot(&ports);
    let ports = factory.reload(ports);
    assert_eq!(snapshot(&ports), before);
    assert_eq!((before.0.len(), before.1.len()), (3, 4));
}

/// An entry is found by its Bill Plan and due date, and writing one for a stored Plan and date
/// replaces that row and keeps its id, so generation never makes a second row for a date.
pub fn an_entry_is_found_by_its_plan_and_due_date<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed(), receipt());
    let key = EntryId {
        plan_id: BillPlanId::seed(1),
        due: date(1),
    };

    ports.upsert_entry(BillScheduleEntry {
        superseded: true,
        ..entry(99, 1, 1, Resolution::Unresolved)
    });

    let ports = factory.reload(ports);
    assert_eq!(
        ports.entry(key),
        Some(BillScheduleEntry {
            superseded: true,
            ..entry(1, 1, 1, Resolution::Unresolved)
        })
    );
    assert_eq!(ports.entries().len(), 1);
    assert_eq!(
        ports.entry(EntryId {
            due: date(2),
            ..key
        }),
        None
    );
}

/// A Paid entry's Split comes back as it was written, moves when the entry is Matched to another
/// Split, and is gone once the entry is Unmatched.
pub fn the_paid_link_round_trips<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed(), receipt());
    let key = entry(1, 1, 1, Resolution::Unresolved).id();

    ports.upsert_entry(entry(1, 1, 1, paid(1, 0)));
    let mut ports = factory.reload(ports);
    assert_eq!(ports.entry(key).map(|e| e.resolution), Some(paid(1, 0)));

    ports.upsert_entry(entry(1, 1, 1, paid(1, 1)));
    let mut ports = factory.reload(ports);
    assert_eq!(ports.entry(key).map(|e| e.resolution), Some(paid(1, 1)));

    ports.upsert_entry(entry(1, 1, 1, Resolution::Unresolved));
    let ports = factory.reload(ports);
    assert_eq!(
        ports.entry(key).map(|e| e.resolution),
        Some(Resolution::Unresolved)
    );
}

/// The Splits come back as they were stored: every field, by a window that includes both ends,
/// and one by one as a Paid entry names it.
pub fn the_splits_read_back_as_they_were_stored<F: PortsFactory>(factory: &mut F) {
    factory.create(seed(), receipt());
    let reads = factory.reads();

    assert_eq!(sorted(reads.splits_between(date(2), date(2))), receipt());
    assert_eq!(reads.splits_between(date(3), date(30)), Vec::new());
    for split in receipt() {
        assert_eq!(reads.paid_split(split.split), Some(split));
    }
    assert_eq!(
        reads.paid_split(SplitRef {
            transaction_id: TransactionId::seed(1),
            split_index: 2,
        }),
        None
    );
}

/// A write that fails is reported once, naming the Bills domain, and the ports go back to the last
/// committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed(), receipt());
    let stored = snapshot(&ports);

    factory.break_writes(&mut ports);
    ports.upsert_bill_plan(plan(3));
    ports.upsert_entry(entry(2, 3, 1, Resolution::Unresolved));
    factory.commit(&mut ports);

    let (ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Bills domain");
    assert_eq!(snapshot(&ports), stored, "the refused writes are gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_plans_and_entries_round_trip() {
        plans_and_entries_round_trip(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_entry_is_found_by_its_plan_and_due_date() {
        an_entry_is_found_by_its_plan_and_due_date(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_the_paid_link_round_trips() {
        the_paid_link_round_trips(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_the_splits_read_back_as_they_were_stored() {
        the_splits_read_back_as_they_were_stored(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
