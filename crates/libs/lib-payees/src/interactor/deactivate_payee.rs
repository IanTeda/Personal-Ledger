use lib_core::PayeeId;

use crate::protocols::PayeeRepository;
use crate::{Payee, PayeeError};

/// Deactivate a Payee, which keeps it on every row that names it but stops its aliases matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeactivatePayee(pub PayeeId);

impl DeactivatePayee {
    /// Returns the Payee as written.
    pub fn run(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let outcome = self.apply(payees);
        match &outcome {
            Ok(_) => tracing::info!(payee_id = %self.0, "Deactivated a Payee"),
            Err(refusal) => {
                tracing::info!(payee_id = %self.0, %refusal, "Refused to deactivate a Payee");
            }
        }
        outcome
    }

    fn apply(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let mut payee = payees.payee(self.0).ok_or(PayeeError::NotFound)?;
        payee.is_active = false;
        payees.upsert_payee(payee.clone());
        Ok(payee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;

    #[test]
    fn deactivating_keeps_the_payee_and_its_aliases_and_only_clears_active() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.payees.payee(PayeeId::seed(1)).expect("a stub Payee");

        let payee = DeactivatePayee(before.id)
            .run(&mut ports.payees)
            .expect("a stub Payee");

        assert_eq!(
            payee,
            Payee {
                is_active: false,
                ..before
            }
        );
        assert_eq!(ports.payees.payee(payee.id), Some(payee));
    }
}
