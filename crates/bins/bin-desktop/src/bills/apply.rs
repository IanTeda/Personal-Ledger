//! The Bills Dialogs' confirm cycle (ADR-0037): the command each form describes, a `lib_ledger`
//! command in each variant, confirmed through [`confirm_ledger`].

use chrono::NaiveDate;
use lib_core::BillPlanId;
use lib_ledger::{AddBillPlan, EditBillPlan, Ledger, LedgerCommand, MatchBill, PayBill, SkipBill};

use super::{BillError, BillPlanDraft, BillsDialog, EntryId, pay_form::PayAction};
use crate::chrome::dialog_apply::{DialogForm, FormError, OutcomeToast};

/// What confirming a Bills Dialog asks for.
#[derive(Debug, Clone)]
pub enum BillCommand {
    /// Add (`editing` is `None`) or save the Plan; Schedule regeneration is the rules'.
    SavePlan {
        editing: Option<BillPlanId>,
        draft: BillPlanDraft,
        is_active: bool,
        today: NaiveDate,
    },
    /// Settle the entry by creating a Transaction or matching a Split.
    Pay {
        entry: EntryId,
        action: PayAction,
    },
    Skip(EntryId),
}

/// What a landed [`BillCommand`] did, for the View and status line that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillOutcome {
    /// Added or edited: the Plan to select on the Planner tab.
    PlanSaved(BillPlanId),
    Paid,
    Skipped,
    /// The entry resolved or was superseded underneath the Skip Dialog. It has no form to show an
    /// error on, so it closes with the reason in the status line.
    SkipRefused,
}

impl OutcomeToast for BillOutcome {}

/// A pay or match writes the Transaction and resolves the entry in the same command.
impl LedgerCommand for BillCommand {
    type Outcome = BillOutcome;
    type Refusal = BillError;

    fn run(self, ledger: &mut Ledger) -> Result<BillOutcome, BillError> {
        match self {
            Self::SavePlan {
                editing: None,
                draft,
                today,
                ..
            } => AddBillPlan { draft, today }
                .run(ledger)
                .map(BillOutcome::PlanSaved),
            Self::SavePlan {
                editing: Some(id),
                draft,
                is_active,
                today,
            } => EditBillPlan {
                id,
                draft,
                is_active,
                today,
            }
            .run(ledger)
            .map(|()| BillOutcome::PlanSaved(id)),
            Self::Pay {
                entry,
                action: PayAction::Pay { amount, date },
            } => PayBill {
                entry,
                amount,
                date,
            }
            .run(ledger)
            .map(|_| BillOutcome::Paid),
            Self::Pay {
                entry,
                action: PayAction::Match(split),
            } => MatchBill { entry, split }
                .run(ledger)
                .map(|()| BillOutcome::Paid),
            Self::Skip(entry) => Ok(match SkipBill(entry).run(ledger) {
                Ok(()) => BillOutcome::Skipped,
                Err(_) => BillOutcome::SkipRefused,
            }),
        }
    }
}

impl DialogForm for BillsDialog {
    type Command = BillCommand;
    type Refusal = BillError;

    fn command(&self) -> Result<BillCommand, FormError> {
        match self {
            Self::Add(form) => form
                .draft()
                .ok_or(FormError)
                .map(|draft| BillCommand::SavePlan {
                    editing: None,
                    draft,
                    is_active: form.is_active,
                    today: form.today(),
                }),
            Self::Edit(id, form) => {
                form.draft()
                    .ok_or(FormError)
                    .map(|draft| BillCommand::SavePlan {
                        editing: Some(*id),
                        draft,
                        is_active: form.is_active,
                        today: form.today(),
                    })
            }
            Self::Pay(form) => form
                .action()
                .ok_or(FormError)
                .map(|action| BillCommand::Pay {
                    entry: form.entry,
                    action,
                }),
            Self::Skip(entry) => Ok(BillCommand::Skip(*entry)),
        }
    }

    fn refuse(&mut self, refusal: BillError) {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.error = Some(refusal),
            Self::Pay(form) => form.error = Some(refusal),
            Self::Skip(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{bills::pay_form::PayAction, chrome::dialog_apply::confirm_ledger};

    fn ledger(cx: &mut TestAppContext) -> gpui::Entity<crate::ledger_store::LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                accounts: lib_accounts::default_accounts(),
                categories: lib_categories::default_categories(),
                ..LedgerParts::default()
            },
        )
    }

    fn entry() -> EntryId {
        EntryId {
            plan_id: BillPlanId::seed(1),
            due: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        }
    }

    #[gpui::test]
    fn a_skip_of_a_vanished_entry_closes_with_the_refusal_as_an_outcome(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = BillsDialog::Skip(entry());
        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));
        assert_eq!(
            landed.map(|landed| landed.outcome),
            Some(BillOutcome::SkipRefused)
        );
    }

    #[gpui::test]
    fn a_pay_on_a_vanished_entry_is_refused(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let command = BillCommand::Pay {
            entry: entry(),
            action: PayAction::Pay {
                amount: "10".parse().expect("a valid amount"),
                date: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
            },
        };
        let refusal = ledger.update(cx, |store, cx| store.apply(cx, command));
        assert!(refusal.is_err(), "no such Plan or entry");
    }
}
