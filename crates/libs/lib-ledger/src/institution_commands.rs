//! The Institutions writes (ADR-0038): each a command value that records an upsert per row it
//! wrote (ADR-0039). The add rule is `lib_institutions`', run over the Ledger's own rows until the
//! Desktop's Institutions page writes through the module's port (#850).

use lib_core::InstitutionId;
use lib_institutions::{InstitutionDraft, InstitutionError, insert_institution};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Appends an Institution, its name trimmed, and returns its id. `account_type` is the joined
    /// label the dialog built, stored as given. A blank or taken name is refused and writes nothing.
    pub fn add_institution(
        &mut self,
        name: String,
        account_type: String,
    ) -> Result<InstitutionId, InstitutionError> {
        let row = insert_institution(
            &mut self.institutions,
            &InstitutionDraft { name, account_type },
        )?;
        let id = row.id;
        self.pending.push(LedgerChange::UpsertInstitution(row));
        Ok(id)
    }
}

/// [`Ledger::add_institution`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddInstitution {
    pub name: String,
    pub account_type: String,
}

impl LedgerCommand for AddInstitution {
    type Outcome = InstitutionId;
    type Refusal = InstitutionError;
    fn run(self, ledger: &mut Ledger) -> Result<InstitutionId, InstitutionError> {
        ledger.add_institution(self.name, self.account_type)
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

        let id = AddInstitution {
            name: "Bank X".into(),
            account_type: "savings \u{b7} offset".into(),
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(ledger.institutions().len(), before + 1);
        let added = ledger.institutions().last().unwrap().clone();
        assert_eq!(added.id, id);
        assert_eq!(added.name, "Bank X");
        assert_eq!(added.account_type, "savings \u{b7} offset");
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertInstitution(added)]
        );
    }

    #[test]
    fn a_taken_name_is_refused_and_records_nothing() {
        let mut ledger = Ledger::stub();
        let before = ledger.institutions().to_vec();

        let refusal = AddInstitution {
            name: "anz banking group".into(),
            account_type: "savings".into(),
        }
        .run(&mut ledger);

        assert_eq!(
            refusal,
            Err(InstitutionError::DuplicateName("ANZ Banking Group".into()))
        );
        assert_eq!(ledger.institutions(), before);
        assert!(ledger.drain_changes().is_empty());
    }
}
