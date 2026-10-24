//! The Payees writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert or delete
//! per Payee it wrote (ADR-0039).
//!
//! The rules, the delete refusal included, are `lib_payees`'. What the Ledger adds is the answer
//! to `lib_payees`' [`PayeeUsage`] port, counted from its own Splits and Bill Plans, until the
//! Payees adapter answers it from storage (#831).

use lib_bills::BillPlan;
use lib_core::PayeeId;
use lib_payees::{
    DeletePayeeRefusal, PayeeDraft, PayeeError, PayeeUsage, PayeeUse, delete_payee, edit_payee,
    get, insert_payee,
};
use lib_transactions::Transaction;

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Ledger's Splits and Bill Plans as the [`PayeeUsage`] port, borrowed apart from its Payees
/// so a delete can count and remove in one call.
struct PayeeNames<'a> {
    transactions: &'a [Transaction],
    bill_plans: &'a [BillPlan],
}

impl PayeeUsage for PayeeNames<'_> {
    fn payee_usage(&self, id: PayeeId) -> PayeeUse {
        PayeeUse {
            splits: self
                .transactions
                .iter()
                .flat_map(|t| &t.splits)
                .filter(|split| split.payee_id == Some(id))
                .count(),
            bill_plans: self
                .bill_plans
                .iter()
                .filter(|plan| plan.payee_id == Some(id))
                .count(),
        }
    }
}

/// How many Splits and Bill Plans name a Payee, for the Delete Dialog.
impl PayeeUsage for Ledger {
    fn payee_usage(&self, id: PayeeId) -> PayeeUse {
        PayeeNames {
            transactions: &self.transactions,
            bill_plans: &self.bill_plans,
        }
        .payee_usage(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SetPayeeActiveRefusal {
    #[error("payee not found")]
    NotFound,
}

impl Ledger {
    /// Adds a new, active Payee and returns its id.
    pub fn add_payee(&mut self, draft: &PayeeDraft) -> Result<PayeeId, PayeeError> {
        let id = insert_payee(&mut self.payees, draft)?;
        self.record_payee(id);
        Ok(id)
    }

    pub fn edit_payee(&mut self, id: PayeeId, draft: &PayeeDraft) -> Result<(), PayeeError> {
        edit_payee(&mut self.payees, id, draft)?;
        self.record_payee(id);
        Ok(())
    }

    /// Hard-deletes Payee `id`, refused while a Split or a Bill Plan names it.
    pub fn delete_payee(&mut self, id: PayeeId) -> Result<(), DeletePayeeRefusal> {
        let names = PayeeNames {
            transactions: &self.transactions,
            bill_plans: &self.bill_plans,
        };
        delete_payee(&mut self.payees, &names, id)?;
        self.pending.push(LedgerChange::DeletePayee(id));
        Ok(())
    }

    pub fn set_payee_active(
        &mut self,
        id: PayeeId,
        is_active: bool,
    ) -> Result<(), SetPayeeActiveRefusal> {
        lib_payees::set_active(&mut self.payees, id, is_active)
            .map_err(|_| SetPayeeActiveRefusal::NotFound)?;
        self.record_payee(id);
        Ok(())
    }

    fn record_payee(&mut self, id: PayeeId) {
        if let Some(payee) = get(&self.payees, id) {
            self.pending.push(LedgerChange::UpsertPayee(payee.clone()));
        }
    }
}

/// [`Ledger::add_payee`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPayee(pub PayeeDraft);

impl LedgerCommand for AddPayee {
    type Outcome = PayeeId;
    type Refusal = PayeeError;
    fn run(self, ledger: &mut Ledger) -> Result<PayeeId, PayeeError> {
        ledger.add_payee(&self.0)
    }
}

/// [`Ledger::edit_payee`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPayee {
    pub id: PayeeId,
    pub draft: PayeeDraft,
}

impl LedgerCommand for EditPayee {
    type Outcome = ();
    type Refusal = PayeeError;
    fn run(self, ledger: &mut Ledger) -> Result<(), PayeeError> {
        ledger.edit_payee(self.id, &self.draft)
    }
}

/// [`Ledger::delete_payee`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePayee(pub PayeeId);

impl LedgerCommand for DeletePayee {
    type Outcome = ();
    type Refusal = DeletePayeeRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), DeletePayeeRefusal> {
        ledger.delete_payee(self.0)
    }
}

/// [`Ledger::set_payee_active`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPayeeActive {
    pub id: PayeeId,
    pub is_active: bool,
}

impl LedgerCommand for SetPayeeActive {
    type Outcome = ();
    type Refusal = SetPayeeActiveRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), SetPayeeActiveRefusal> {
        ledger.set_payee_active(self.id, self.is_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str) -> PayeeDraft {
        PayeeDraft {
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
        }
    }

    fn id_of(ledger: &Ledger, name: &str) -> PayeeId {
        lib_payees::find_by_name(ledger.payees(), name).expect("seeded Payee")
    }

    #[test]
    fn add_records_one_upsert_of_the_row_now_in_the_ledger() {
        let mut ledger = Ledger::stub();
        let id = AddPayee(draft("Corner Shop")).run(&mut ledger).unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
    }

    #[test]
    fn a_refused_add_or_edit_changes_nothing_and_records_nothing() {
        let mut ledger = Ledger::stub();
        let before = ledger.payees().to_vec();
        let taken = before[0].name.clone();

        assert!(AddPayee(draft(&taken)).run(&mut ledger).is_err());
        assert!(
            EditPayee {
                id: PayeeId::seed(u32::MAX),
                draft: draft("x")
            }
            .run(&mut ledger)
            .is_err()
        );
        assert_eq!(ledger.payees(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn edit_records_the_edited_row() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "Netflix");
        EditPayee {
            id,
            draft: draft("Netflix AU"),
        }
        .run(&mut ledger)
        .unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert_eq!(row.name, "Netflix AU");
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
    }

    #[test]
    fn an_unused_payee_deletes_and_records_the_delete() {
        let mut ledger = Ledger::stub();
        let id = ledger.add_payee(&draft("Corner Shop")).unwrap();
        ledger.drain_changes();
        assert!(!ledger.payee_usage(id).is_used());

        DeletePayee(id).run(&mut ledger).unwrap();

        assert!(get(ledger.payees(), id).is_none());
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::DeletePayee(id)]);
    }

    #[test]
    fn a_payee_a_split_names_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        let before = ledger.payees().to_vec();
        let used = ledger.payee_usage(id);
        assert!(used.splits > 0);

        assert_eq!(
            DeletePayee(id).run(&mut ledger),
            Err(DeletePayeeRefusal::InUse {
                splits: used.splits,
                bill_plans: used.bill_plans
            })
        );
        assert_eq!(ledger.payees(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_payee_only_a_bill_plan_names_is_in_use() {
        let mut ledger = Ledger::stub();
        let plan = ledger
            .bill_plans()
            .iter()
            .find(|plan| plan.payee_id.is_some())
            .expect("the stub has a Bill Plan with a Payee");
        let id = plan.payee_id.unwrap();
        ledger.transactions.clear();

        let used = ledger.payee_usage(id);
        assert_eq!(used.splits, 0);
        assert!(used.bill_plans > 0);
        assert!(matches!(
            DeletePayee(id).run(&mut ledger),
            Err(DeletePayeeRefusal::InUse { splits: 0, .. })
        ));
    }

    #[test]
    fn deleting_an_unknown_payee_is_not_found() {
        let mut ledger = Ledger::stub();
        assert_eq!(
            DeletePayee(PayeeId::seed(u32::MAX)).run(&mut ledger),
            Err(DeletePayeeRefusal::NotFound)
        );
    }

    #[test]
    fn set_active_records_the_row_and_refuses_an_unknown_id() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        SetPayeeActive {
            id,
            is_active: false,
        }
        .run(&mut ledger)
        .unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert!(!row.is_active);
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
        assert_eq!(
            SetPayeeActive {
                id: PayeeId::seed(u32::MAX),
                is_active: true
            }
            .run(&mut ledger),
            Err(SetPayeeActiveRefusal::NotFound)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn an_in_use_payee_can_be_deactivated_and_the_change_is_recorded() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        assert!(ledger.payee_usage(id).is_used());

        SetPayeeActive {
            id,
            is_active: false,
        }
        .run(&mut ledger)
        .unwrap();

        assert!(!get(ledger.payees(), id).unwrap().is_active);
        assert_eq!(ledger.drain_changes().len(), 1);
    }
}
