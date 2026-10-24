use lib_core::PayeeId;

use crate::protocols::PayeeRepository;
use crate::{Payee, PayeeError};

/// Reactivate a deactivated Payee, so it counts and matches again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactivatePayee(pub PayeeId);

impl ReactivatePayee {
    /// Returns the Payee as written.
    pub fn run(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let outcome = self.apply(payees);
        match &outcome {
            Ok(_) => tracing::info!(payee_id = %self.0, "Reactivated a Payee"),
            Err(refusal) => {
                tracing::info!(payee_id = %self.0, %refusal, "Refused to reactivate a Payee");
            }
        }
        outcome
    }

    fn apply(self, payees: &mut dyn PayeeRepository) -> Result<Payee, PayeeError> {
        let mut payee = payees.payee(self.0).ok_or(PayeeError::NotFound)?;
        payee.is_active = true;
        payees.upsert_payee(payee.clone());
        Ok(payee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::DeactivatePayee;

    #[test]
    fn deactivating_then_reactivating_puts_the_payee_back_as_it_was() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.clone();
        let woolworths = PayeeId::seed(1);

        let inactive = DeactivatePayee(woolworths)
            .run(&mut ports.payees)
            .expect("a stub Payee");
        assert!(!inactive.is_active);
        assert_eq!(ports.payees.payee(woolworths), Some(inactive));

        let active = ReactivatePayee(woolworths)
            .run(&mut ports.payees)
            .expect("a stub Payee");
        assert!(active.is_active);
        assert_eq!(ports, before);
    }

    #[test]
    fn a_payee_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.clone();

        assert_eq!(
            DeactivatePayee(PayeeId::new()).run(&mut ports.payees),
            Err(PayeeError::NotFound)
        );
        assert_eq!(
            ReactivatePayee(PayeeId::new()).run(&mut ports.payees),
            Err(PayeeError::NotFound)
        );
        assert_eq!(ports, before);
    }
}
