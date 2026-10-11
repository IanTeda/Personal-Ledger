//! The Bills Dialogs' confirm cycle (ADR-0037): the command each form describes and the [`Apply`]
//! that runs it against the Ledger as Ledger commands.

use chrono::NaiveDate;
use gpui::{App, Entity};

use lib_ledger::{AddBillPlan, EditBillPlan, MatchBill, PayBill, SkipBill};

use super::{BillError, BillPlanDraft, BillsDialog, EntryId, pay_form::PayAction};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    ledger_store::LedgerStore,
};

/// What confirming a Bills Dialog asks for.
#[derive(Debug, Clone)]
pub enum BillCommand {
    /// Add (`editing` is `None`) or save the Plan; Schedule regeneration is the rules'.
    SavePlan {
        editing: Option<u32>,
        draft: BillPlanDraft,
        is_active: bool,
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
    PlanSaved(u32),
    Paid,
    Skipped,
    /// The entry resolved or was superseded underneath the Skip Dialog. It has no form to show an
    /// error on, so it closes with the reason in the status line.
    SkipRefused,
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
                }),
            Self::Edit(id, form) => {
                form.draft()
                    .ok_or(FormError)
                    .map(|draft| BillCommand::SavePlan {
                        editing: Some(*id),
                        draft,
                        is_active: form.is_active,
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

/// Runs [`BillCommand`]s against the Ledger as Ledger commands. A Pay writes the Transaction and
/// resolves the entry in the same command.
pub struct BillsApply {
    pub ledger: Entity<LedgerStore>,
    pub today: NaiveDate,
}

impl Apply<BillCommand> for BillsApply {
    type Outcome = BillOutcome;
    type Refusal = BillError;

    fn apply(&mut self, command: BillCommand, cx: &mut App) -> Result<BillOutcome, BillError> {
        let today = self.today;
        match command {
            BillCommand::SavePlan {
                editing: None,
                draft,
                ..
            } => self
                .ledger
                .update(cx, |store, cx| {
                    store.apply(cx, AddBillPlan { draft, today })
                })
                .map(BillOutcome::PlanSaved),
            BillCommand::SavePlan {
                editing: Some(id),
                draft,
                is_active,
            } => self
                .ledger
                .update(cx, |store, cx| {
                    store.apply(
                        cx,
                        EditBillPlan {
                            id,
                            draft,
                            is_active,
                            today,
                        },
                    )
                })
                .map(|()| BillOutcome::PlanSaved(id)),
            BillCommand::Pay {
                entry,
                action: PayAction::Pay { amount, date },
            } => self
                .ledger
                .update(cx, |store, cx| {
                    store.apply(
                        cx,
                        PayBill {
                            entry,
                            amount,
                            date,
                        },
                    )
                })
                .map(|_| BillOutcome::Paid),
            BillCommand::Pay {
                entry,
                action: PayAction::Match(split),
            } => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, MatchBill { entry, split }))
                .map(|()| BillOutcome::Paid),
            BillCommand::Skip(entry) => {
                let skipped = self
                    .ledger
                    .update(cx, |store, cx| store.apply(cx, SkipBill(entry)));
                Ok(if skipped.is_ok() {
                    BillOutcome::Skipped
                } else {
                    BillOutcome::SkipRefused
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{bills::pay_form::PayAction, chrome::dialog_apply::confirm};

    fn apply(cx: &mut TestAppContext) -> BillsApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                accounts: crate::accounts::default_accounts(),
                categories: lib_categories::default_categories(),
                ..LedgerParts::default()
            },
        );
        BillsApply {
            ledger,
            today: NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
        }
    }

    fn entry() -> EntryId {
        EntryId {
            plan_id: 1,
            due: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        }
    }

    #[gpui::test]
    fn a_skip_of_a_vanished_entry_closes_with_the_refusal_as_an_outcome(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = BillsDialog::Skip(entry());
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(BillOutcome::SkipRefused));
    }

    #[gpui::test]
    fn a_pay_on_a_vanished_entry_is_refused(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let command = BillCommand::Pay {
            entry: entry(),
            action: PayAction::Pay {
                amount: "10".parse().expect("a valid amount"),
                date: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
            },
        };
        let refusal = cx.update(|cx| apply.apply(command, cx));
        assert!(refusal.is_err(), "no such Plan or entry");
    }
}
