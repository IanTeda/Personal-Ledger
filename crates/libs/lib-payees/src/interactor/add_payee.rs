use crate::protocols::PayeeRepository;
use crate::{Payee, PayeeDraft, PayeeError, get, insert_payee};

/// Add a Payee, active, with its name and aliases cleaned by ADR-0012's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPayee(pub PayeeDraft);

impl AddPayee {
    /// Returns the Payee as written.
    pub fn run(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let outcome = self.apply(payees);
        match &outcome {
            Ok(payee) => tracing::info!(payee_id = %payee.id, "Added a Payee"),
            Err(refusal) => tracing::info!(%refusal, "Refused to add a Payee"),
        }
        outcome
    }

    fn apply(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let mut rows = payees.payees();
        let id = insert_payee(&mut rows, &self.0)?;
        let payee = get(&rows, id).cloned().ok_or(PayeeError::NotFound)?;
        payees.upsert_payee(payee.clone());
        Ok(payee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;

    fn draft(name: &str, aliases: &[&str]) -> PayeeDraft {
        PayeeDraft {
            name: name.to_string(),
            default_category: None,
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        }
    }

    #[test]
    fn adding_a_payee_writes_it_active_with_its_aliases_normalised() {
        let mut ports = InMemoryPorts::stub();

        let payee = AddPayee(draft(" Corner Shop ", &["corner", " CORNER "]))
            .run(&mut ports.payees)
            .expect("a free name");

        assert_eq!(payee.name, "Corner Shop");
        assert_eq!(payee.aliases, ["CORNER"]);
        assert!(payee.is_active);
        assert_eq!(ports.payees.payee(payee.id), Some(payee));
    }

    #[test]
    fn a_taken_name_or_alias_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.clone();

        assert_eq!(
            AddPayee(draft("coles", &[])).run(&mut ports.payees),
            Err(PayeeError::DuplicateName("Coles".into()))
        );
        assert!(matches!(
            AddPayee(draft("Corner Shop", &["woolies"])).run(&mut ports.payees),
            Err(PayeeError::AliasTaken { .. })
        ));
        assert_eq!(ports, before);
    }
}
