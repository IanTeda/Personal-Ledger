//! The Bills Dialogs' confirm cycle (ADR-0037): the command each form describes and the [`Apply`]
//! that runs it against the Bills, Accounts, Categories and Transactions stores. When
//! `LedgerStore` replaces the per-domain stores only [`BillsApply`] changes.

use chrono::NaiveDate;
use gpui::{App, Entity};

use super::{
    self as bills, BillError, BillPlanDraft, BillsDialog, BillsStore, EntryId, edit_bills,
    edit_bills_and_transactions, pay_form::PayAction,
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    transactions::TransactionsStore,
    view::{accounts::state::AccountsStore, categories::state::CategoriesStore},
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

/// Runs [`BillCommand`]s against the Bills store, reading the Accounts and Categories a Plan or a
/// Match is checked against and writing the Transaction a Pay creates.
pub struct BillsApply {
    pub bills: Entity<BillsStore>,
    pub accounts: Entity<AccountsStore>,
    pub categories: Entity<CategoriesStore>,
    pub transactions: Entity<TransactionsStore>,
    pub today: NaiveDate,
}

impl Apply<BillCommand> for BillsApply {
    type Outcome = BillOutcome;
    type Refusal = BillError;

    fn apply(&mut self, command: BillCommand, cx: &mut App) -> Result<BillOutcome, BillError> {
        match command {
            BillCommand::SavePlan {
                editing,
                draft,
                is_active,
            } => {
                let today = self.today;
                let accounts = self.accounts.read(cx).accounts(cx).to_vec();
                let categories = self.categories.read(cx).categories(cx).to_vec();
                edit_bills(&self.bills, cx, |plans, entries| match editing {
                    None => {
                        bills::insert_plan(plans, entries, &draft, &categories, &accounts, today)
                    }
                    Some(id) => {
                        bills::edit_plan(plans, entries, id, &draft, &categories, &accounts, today)
                            .and_then(|()| bills::set_active(plans, entries, id, is_active, today))
                            .map(|()| id)
                    }
                })
                .map(BillOutcome::PlanSaved)
            }
            BillCommand::Pay { entry, action } => {
                let accounts = self.accounts.read(cx).accounts(cx).to_vec();
                let categories = self.categories.read(cx).categories(cx).to_vec();
                edit_bills_and_transactions(&self.bills, cx, |plans, entries, transactions| {
                    match action {
                        PayAction::Pay { amount, date } => {
                            bills::pay(plans, entries, transactions, entry, &amount, date)
                                .map(|_| ())
                        }
                        PayAction::Match(split) => bills::match_split(
                            plans,
                            entries,
                            transactions,
                            &accounts,
                            &categories,
                            entry,
                            split,
                        ),
                    }
                })
                .map(|()| BillOutcome::Paid)
            }
            BillCommand::Skip(entry) => {
                let skipped = edit_bills(&self.bills, cx, |plans, entries| {
                    bills::skip(plans, entries, entry)
                });
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
    use gpui::{AppContext, TestAppContext};
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
            bills: cx.new(|cx| BillsStore::new(ledger.clone(), cx)),
            accounts: cx.new(|cx| AccountsStore::new(ledger.clone(), cx)),
            categories: cx.new(|cx| CategoriesStore::new(ledger.clone(), cx)),
            transactions: cx.new(|cx| TransactionsStore::new(ledger, cx)),
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
