//! The Institutions writes (ADR-0038): each a command value that records an upsert per row it
//! wrote (ADR-0039).
//!
//! An Institution row has no id yet, so a change names the row itself; it takes an id with the
//! UUIDv7 newtypes (ADR-0040).

use lib_institutions::add_institution;

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Appends an Institution. `account_type` is the joined label the dialog built, stored as given.
    pub fn add_institution(&mut self, name: String, account_type: String) {
        let row = add_institution(&mut self.institutions, name, account_type);
        self.pending.push(LedgerChange::UpsertInstitution(row));
    }
}

/// [`Ledger::add_institution`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddInstitution {
    pub name: String,
    pub account_type: String,
}

impl LedgerCommand for AddInstitution {
    type Outcome = ();
    /// Adding an Institution can't be refused: the form has already validated it.
    type Refusal = std::convert::Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<(), Self::Refusal> {
        ledger.add_institution(self.name, self.account_type);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LedgerChange;

    #[test]
    fn adding_an_institution_appends_it_and_records_the_row() {
        let mut ledger = Ledger::stub();
        let before = ledger.institutions().len();

        AddInstitution {
            name: "Bank X".into(),
            account_type: "savings \u{b7} offset".into(),
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(ledger.institutions().len(), before + 1);
        let added = ledger.institutions().last().unwrap().clone();
        assert_eq!(added.name, "Bank X");
        assert_eq!(added.account_type, "savings \u{b7} offset");
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertInstitution(added)]
        );
    }
}
