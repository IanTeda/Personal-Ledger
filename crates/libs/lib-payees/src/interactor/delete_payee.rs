use lib_core::PayeeId;

use crate::protocols::PortsMut;
use crate::{DeletePayeeRefusal, Payee, delete_payee};

/// Hard-delete a Payee and its aliases, refused while a Split or a Bill Plan names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePayee(pub PayeeId);

impl DeletePayee {
    /// Returns the row deleted.
    pub fn run(self, ports: PortsMut<'_>) -> Result<Payee, DeletePayeeRefusal> {
        let outcome = self.apply(ports);
        match &outcome {
            Ok(_) => tracing::info!(payee_id = %self.0, "Deleted a Payee"),
            Err(refusal) => {
                tracing::info!(payee_id = %self.0, %refusal, "Refused to delete a Payee");
            }
        }
        outcome
    }

    fn apply(self, ports: PortsMut<'_>) -> Result<Payee, DeletePayeeRefusal> {
        let mut rows = ports.payees.payees();
        let payee = delete_payee(&mut rows, ports.usage, self.0)?;
        ports.payees.delete_payee(self.0);
        Ok(payee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::{PayeeRepository as _, PayeeUse};

    #[test]
    fn an_unused_payee_is_deleted_and_returned() {
        let mut ports = InMemoryPorts::stub();
        let netflix = PayeeId::seed(3);

        let payee = DeletePayee(netflix)
            .run(ports.split_mut())
            .expect("nothing names it");

        assert_eq!(payee.name, "Netflix");
        assert_eq!(ports.payees.payee(netflix), None);
    }

    #[test]
    fn a_used_missing_or_uncounted_payee_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub();
        let woolworths = PayeeId::seed(1);
        let used = PayeeUse {
            splits: 2,
            bill_plans: 1,
        };
        ports.usage.uses.insert(woolworths, used);
        let before = ports.clone();

        assert_eq!(
            DeletePayee(woolworths).run(ports.split_mut()),
            Err(DeletePayeeRefusal::InUse {
                splits: 2,
                bill_plans: 1
            })
        );
        assert_eq!(
            DeletePayee(PayeeId::new()).run(ports.split_mut()),
            Err(DeletePayeeRefusal::NotFound)
        );
        let mut unreadable = before.clone();
        unreadable.usage.unreadable = true;
        assert_eq!(
            DeletePayee(PayeeId::seed(3)).run(unreadable.split_mut()),
            Err(DeletePayeeRefusal::Unchecked),
            "a Payee whose use can't be counted may be in use"
        );
        assert_eq!(ports, before);
        assert_eq!(unreadable.payees, before.payees);
    }
}
