//! Today's Pay Dialog's confirm cycle (ADR-0037). Pay it directly is `lib-ledger`'s `PayBill`, which
//! writes the Transaction and the entry together, confirmed through [`confirm_ledger`] and saved by
//! the forwarding shim until `lib-bill-payments` takes it (#779). Match is `lib-bills`' Interactor,
//! which the Shell runs on the Bills ports (`Shell::confirm_pay_dialog`).
//!
//! [`confirm_ledger`]: crate::chrome::dialog_apply::confirm_ledger

use lib_bills::BillError;
use lib_core::TransactionId;
use lib_ledger::PayBill;

use super::pay_form::{PayAction, PayForm};
use crate::chrome::dialog_apply::{DialogForm, FormError, OutcomeToast};

/// A landed Pay raises no Toast: the row turning Paid is the feedback.
impl OutcomeToast for TransactionId {}

impl DialogForm for PayForm {
    type Command = PayBill;
    type Refusal = BillError;

    /// Pay it directly's command; the Match panel has none, since it runs an Interactor.
    fn command(&self) -> Result<PayBill, FormError> {
        match self.action() {
            Some(PayAction::Pay { amount, date }) => Ok(PayBill {
                entry: self.entry,
                amount,
                date,
            }),
            Some(PayAction::Match(_)) | None => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: BillError) {
        self.error = Some(refusal);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use gpui::TestAppContext;
    use lib_bills::EntryId;
    use lib_core::BillPlanId;
    use lib_ledger::LedgerParts;

    use crate::ledger_store::test_ledger;

    #[gpui::test]
    fn a_pay_on_a_vanished_entry_is_refused(cx: &mut TestAppContext) {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                accounts: lib_accounts::default_accounts(),
                categories: lib_categories::default_categories(),
                ..LedgerParts::default()
            },
        );
        let command = lib_ledger::PayBill {
            entry: EntryId {
                plan_id: BillPlanId::seed(1),
                due: NaiveDate::from_ymd_opt(2026, 9, 20).expect("a real date"),
            },
            amount: "10".parse().expect("a valid amount"),
            date: NaiveDate::from_ymd_opt(2026, 9, 20).expect("a real date"),
        };
        let refusal = ledger.update(cx, |store, cx| store.apply(cx, command));
        assert!(refusal.is_err(), "no such Plan or entry");
    }
}
