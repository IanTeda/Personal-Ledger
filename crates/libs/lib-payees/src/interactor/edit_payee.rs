use lib_core::PayeeId;

use crate::protocols::PayeeRepository;
use crate::{Payee, PayeeDraft, PayeeError, edit_payee, get};

/// Edit a Payee's name, Default category and aliases. A rename keeps the old name as an alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPayee {
    pub id: PayeeId,
    pub draft: PayeeDraft,
}

impl EditPayee {
    /// Returns the Payee as written.
    pub fn run(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let id = self.id;
        let outcome = self.apply(payees);
        match &outcome {
            Ok(_) => tracing::info!(payee_id = %id, "Edited a Payee"),
            Err(refusal) => tracing::info!(payee_id = %id, %refusal, "Refused to edit a Payee"),
        }
        outcome
    }

    fn apply(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let mut rows = payees.payees();
        edit_payee(&mut rows, self.id, &self.draft)?;
        let payee = get(&rows, self.id).cloned().ok_or(PayeeError::NotFound)?;
        payees.upsert_payee(payee.clone());
        Ok(payee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;

    #[test]
    fn a_rename_keeps_the_old_name_as_an_alias() {
        let mut ports = InMemoryPorts::stub();
        let coles = ports.payees.payee(PayeeId::seed(2)).expect("a stub Payee");

        let payee = EditPayee {
            id: coles.id,
            draft: PayeeDraft {
                name: "Coles Central".into(),
                default_category: None,
                aliases: coles.aliases.clone(),
            },
        }
        .run(&mut ports.payees)
        .expect("a free name");

        assert_eq!(payee.name, "Coles Central");
        assert!(payee.aliases.contains(&"COLES".to_string()));
        assert_eq!(payee.default_category, None);
        assert_eq!(ports.payees.payee(coles.id), Some(payee));
    }

    #[test]
    fn editing_a_payee_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.clone();

        let refused = EditPayee {
            id: PayeeId::new(),
            draft: PayeeDraft {
                name: "Anyone".into(),
                ..PayeeDraft::default()
            },
        }
        .run(&mut ports.payees);

        assert_eq!(refused, Err(PayeeError::NotFound));
        assert_eq!(ports, before);
    }
}
