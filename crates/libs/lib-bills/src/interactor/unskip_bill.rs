//! Unskip: undoes Skipped, returning the entry to the status its due date gives it.

use super::Rows;
use crate::protocols::BillsRepository;
use crate::{BillError, EntryId, unskip};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnskipBill {
    pub entry: EntryId,
}

impl UnskipBill {
    /// Writes the entry unresolved again.
    ///
    /// # Errors
    /// `EntryNotFound` for an entry that is gone and `NotSkipped` for one that isn't Skipped.
    pub fn run(self, bills: &mut dyn BillsRepository) -> Result<(), BillError> {
        let mut rows = Rows::read(bills);
        unskip(&mut rows.entries, self.entry)?;
        rows.write_back(bills);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::SkipBill;
    use crate::interactor::test_support::{date, entry_of, stub, today};
    use crate::{BillStatus, entry, status};

    #[test]
    fn unskipping_returns_the_entry_to_its_status() {
        let mut ports = stub();
        let origin = entry_of(&ports, "Origin Energy", date(2026, 9, 24));
        SkipBill { entry: origin }
            .run(&mut ports.bills)
            .expect("an open entry");

        UnskipBill { entry: origin }
            .run(&mut ports.bills)
            .expect("a Skipped entry");

        let found = entry(&ports.bills.entries, origin).expect("still stored");
        assert_eq!(status(found, today()), BillStatus::Due);
    }

    #[test]
    fn an_entry_that_isnt_skipped_is_refused_and_nothing_is_written() {
        let mut ports = stub();
        let origin = entry_of(&ports, "Origin Energy", date(2026, 9, 24));
        let before = ports.bills.clone();

        assert_eq!(
            UnskipBill { entry: origin }.run(&mut ports.bills),
            Err(BillError::NotSkipped)
        );
        assert_eq!(ports.bills, before);
    }
}
