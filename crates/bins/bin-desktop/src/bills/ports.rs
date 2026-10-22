//! The Bills ports as the Desktop holds them (#774): the forwarding shim that saves today's Bills
//! writers' changes through the store `build_shell` is given, and the start-up generation.
//!
//! The Bills View and its writers are still `lib-ledger`'s `bill_commands`: each runs as a
//! `LedgerCommand` on the Ledger and records a `LedgerChange`. [`forward`] sends the Bills ones to
//! the store in the same commit as the command, and the Ledger's Bill Plans and Bill Schedule
//! become a read copy refreshed from the store after each commit and on rollback. Each writer
//! leaves the shim when it migrates to `lib-bills`' Interactors (#778, #779).

use lib_bills::{BillPlanRepository, BillScheduleRepository, populate_stored, unmatch_transaction};
use lib_database::BillsStore;
use lib_ledger::LedgerChange;

/// Sends each Bill Plan and entry in `changes` to `store`, in the order recorded, and Unmatches
/// the entries a deleted Transaction paid, since deleting it deletes the Splits their Matches are
/// on (ADR-0019). The writers it carries: Bill Plan add and edit, Pay, Match, Skip and their undo,
/// and Category reassign; a Transaction delete comes from Account delete. Returns whether it wrote
/// anything, so the caller refreshes the read copy.
pub fn forward(store: &mut dyn BillsStore, changes: &[LedgerChange]) -> bool {
    let mut wrote = false;
    for change in changes {
        match change {
            LedgerChange::UpsertBillPlan(plan) => {
                store.plans_mut().upsert_bill_plan(plan.clone());
                wrote = true;
            }
            LedgerChange::UpsertBillEntry(entry) => {
                store.entries_mut().upsert_entry(entry.clone());
                wrote = true;
            }
            LedgerChange::DeleteTransaction(id) => {
                let before = store.entries().entries();
                let mut entries = before.clone();
                unmatch_transaction(&mut entries, *id);
                for entry in entries.into_iter().filter(|entry| !before.contains(entry)) {
                    store.entries_mut().upsert_entry(entry);
                    wrote = true;
                }
            }
            _ => {}
        }
    }
    wrote
}

/// Brings the stored Bill Schedule up to the horizon as of `today` (ADR-0019's "on Client start"),
/// writing the entries generation adds. The caller commits them.
pub fn generate<P>(store: &mut P, today: chrono::NaiveDate)
where
    P: BillPlanRepository + BillScheduleRepository,
{
    let written = populate_stored(store, today);
    tracing::debug!(written, "Generated the Bill Schedule on start");
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_bills::fake::InMemoryBills;
    use lib_bills::{BillScheduleEntry, Resolution};

    use super::*;

    fn stub() -> (lib_ledger::Ledger, InMemoryBills) {
        let ledger =
            lib_ledger::Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"));
        let bills = InMemoryBills {
            plans: ledger.bill_plans().to_vec(),
            entries: ledger.bill_entries().to_vec(),
        };
        (ledger, bills)
    }

    #[test]
    fn a_deleted_transaction_unmatches_the_entries_it_paid() {
        let (_ledger, mut bills) = stub();
        let (entry_id, transaction_id) = bills
            .entries
            .iter()
            .find_map(|entry| match entry.resolution {
                Resolution::Paid(split) => Some((entry.id(), split.transaction_id)),
                _ => None,
            })
            .expect("the stub pays an entry");

        assert!(forward(
            &mut bills,
            &[LedgerChange::DeleteTransaction(transaction_id)]
        ));

        assert_eq!(
            lib_bills::entry(&bills.entries, entry_id).map(|entry| &entry.resolution),
            Some(&Resolution::Unresolved)
        );
    }

    #[test]
    fn a_deleted_transaction_that_paid_nothing_writes_nothing() {
        let (ledger, mut bills) = stub();
        let unpaid = ledger
            .transactions()
            .iter()
            .find(|t| !lib_bills::carries_bill(&bills.entries, t.id))
            .expect("the stub has a Transaction no entry names");
        let before = bills.clone();

        assert!(!forward(
            &mut bills,
            &[LedgerChange::DeleteTransaction(unpaid.id)]
        ));
        assert_eq!(bills, before);
    }

    #[test]
    fn bill_plan_and_entry_changes_reach_the_store() {
        let (_ledger, mut bills) = stub();
        let mut plan = bills.plans[0].clone();
        plan.name = "Renamed".into();
        let entry = BillScheduleEntry {
            superseded: true,
            ..bills.entries[0].clone()
        };

        assert!(forward(
            &mut bills,
            &[
                LedgerChange::UpsertBillPlan(plan.clone()),
                LedgerChange::UpsertBillEntry(entry.clone()),
            ]
        ));

        assert_eq!(bills.plans[0], plan);
        assert_eq!(bills.entries[0], entry);
    }
}
