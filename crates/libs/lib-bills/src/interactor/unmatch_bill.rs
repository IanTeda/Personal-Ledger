//! Unmatch: undoes Paid, clearing the Match and leaving the Transaction where it is.

use super::Rows;
use crate::protocols::BillsRepository;
use crate::{BillError, EntryId, unmatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmatchBill {
    pub entry: EntryId,
}

impl UnmatchBill {
    /// Writes the entry unresolved again.
    ///
    /// # Errors
    /// `EntryNotFound` for an entry that is gone and `NotPaid` for one that isn't Paid.
    pub fn run(self, bills: &mut dyn BillsRepository) -> Result<(), BillError> {
        let mut rows = Rows::read(bills);
        unmatch(&mut rows.entries, self.entry)?;
        rows.write_back(bills);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::{date, entry_of, stub, today};
    use crate::{BillStatus, Resolution, entry, status};

    #[test]
    fn unmatching_returns_the_entry_to_its_status_and_keeps_the_transaction() {
        let mut ports = stub();
        let streaming = entry_of(&ports, "Streaming Bundle", date(2026, 9, 5));
        let Some(Resolution::Paid(split)) =
            entry(&ports.bills.entries, streaming).map(|e| e.resolution.clone())
        else {
            panic!("the stub pays Streaming Bundle this month");
        };
        let transactions = ports.reads.transactions.clone();

        UnmatchBill { entry: streaming }
            .run(&mut ports.bills)
            .expect("a Paid entry");

        let found = entry(&ports.bills.entries, streaming).expect("still stored");
        assert_eq!(status(found, today()), BillStatus::Overdue);
        assert_eq!(ports.reads.transactions, transactions);
        assert!(transactions.iter().any(|t| t.id == split.transaction_id));
    }

    #[test]
    fn an_entry_that_isnt_paid_is_refused_and_nothing_is_written() {
        let mut ports = stub();
        let telstra = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let before = ports.bills.clone();

        assert_eq!(
            UnmatchBill { entry: telstra }.run(&mut ports.bills),
            Err(BillError::NotPaid)
        );
        assert_eq!(ports.bills, before);
    }
}
