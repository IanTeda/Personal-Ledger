//! The Ledger's Institutions are a read copy of the Institutions port (#849): a Client refreshes it
//! from the port after each Institutions write and on rollback ([`Ledger::refresh_institutions`]),
//! for the Accounts page and its Institution select, and the Institutions page until it reads its
//! own port (#850). The one writer, the Institutions page's add, records an `UpsertInstitution` a
//! Client forwards to the port. The copy goes when the last reader has its own port.

use lib_institutions::Institution;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Institutions with what the Institutions port holds, in the
    /// port's order. Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_institutions(&mut self, institutions: Vec<Institution>) -> bool {
        if self.institutions == institutions {
            return false;
        }
        self.institutions = institutions;
        true
    }
}

#[cfg(test)]
mod tests {
    use lib_core::InstitutionId;

    use super::*;

    #[test]
    fn a_refresh_replaces_the_copy_and_says_whether_it_changed() {
        let mut ledger = Ledger::stub();
        let mut institutions = ledger.institutions().to_vec();
        assert!(!ledger.refresh_institutions(institutions.clone()));

        institutions.push(Institution {
            id: InstitutionId::new(),
            name: "Bank X".into(),
            account_type: "savings".into(),
        });
        assert!(ledger.refresh_institutions(institutions.clone()));
        assert_eq!(ledger.institutions(), institutions);
    }
}
