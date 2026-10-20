//! The Budgets write Dialogs' confirm cycle (ADR-0037): the command each form describes, wrapped
//! with the Budget on show and today into an [`OnBudget`] `lib_ledger` command, confirmed through
//! [`confirm_ledger_with`](crate::chrome::dialog_apply::confirm_ledger_with).
//!
//! Only the forms that write are here. The Switcher, Manage and Category detail lists answer
//! with a request that opens another Dialog or moves the cursor, which stays `Shell`'s.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId};
use lib_ledger::{
    AddBudget, EditBudget, FillBudget, Ledger, LedgerCommand, SetBudgetAmount, StopBudgeting,
};

use super::{BudgetError, BudgetsDialog, FillSource, form::BudgetDraft, limit_form::LimitDraft};
use crate::{
    chrome::dialog_apply::{DialogForm, FormError, OutcomeToast},
    period::Period,
};

/// What confirming a Budgets write Dialog asks for.
#[derive(Debug, Clone)]
pub enum BudgetCommand {
    /// 11c: create the Budget, or save its rename and Accounts.
    SaveBudget(BudgetDraft),
    /// 9e: write the Category's amount.
    SetLimit(LimitDraft),
    /// 9g: stop budgeting the Category from the month.
    Stop {
        category_id: CategoryId,
        month: Period,
    },
    /// 9f: write the chosen source's Month-only amounts into the Budget on show.
    Fill { month: Period, source: FillSource },
}

/// A [`BudgetCommand`] with the context the form does not hold: the Budget on show and today.
#[derive(Debug, Clone)]
pub struct OnBudget {
    pub command: BudgetCommand,
    pub budget: BudgetId,
    pub today: NaiveDate,
}

/// What a landed [`OnBudget`] did, for the View that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetOutcome {
    /// A Budget was created: the surface switches to it.
    Created(BudgetId),
    Saved,
}

impl OutcomeToast for BudgetOutcome {}

/// Why a Budgets command didn't land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetRefusal {
    Budget(BudgetError),
    /// A Fill that would change nothing stays open, as its disabled button says.
    NothingToFill,
}

impl From<BudgetError> for BudgetRefusal {
    fn from(error: BudgetError) -> Self {
        Self::Budget(error)
    }
}

impl LedgerCommand for OnBudget {
    type Outcome = BudgetOutcome;
    type Refusal = BudgetRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<BudgetOutcome, BudgetRefusal> {
        let Self {
            command,
            budget,
            today,
        } = self;
        match command {
            BudgetCommand::SaveBudget(BudgetDraft::Create(new)) => {
                let id = AddBudget { new, today }.run(ledger)?;
                Ok(BudgetOutcome::Created(id))
            }
            BudgetCommand::SaveBudget(BudgetDraft::Edit {
                id,
                name,
                account_ids,
            }) => {
                EditBudget {
                    id,
                    name,
                    account_ids,
                }
                .run(ledger)?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::SetLimit(draft) => {
                SetBudgetAmount {
                    budget_id: budget,
                    category_id: draft.category_id,
                    month: draft.month,
                    span: draft.span,
                    amount: draft.amount,
                    rollover: Some(draft.rollover),
                    today,
                }
                .run(ledger)?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Stop { category_id, month } => {
                StopBudgeting {
                    budget_id: budget,
                    category_id,
                    month,
                    today,
                }
                .run(ledger)?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Fill { month, source } => {
                let filled = FillBudget {
                    budget_id: budget,
                    target: month,
                    source,
                    today,
                }
                .run(ledger)?;
                if filled > 0 {
                    Ok(BudgetOutcome::Saved)
                } else {
                    Err(BudgetRefusal::NothingToFill)
                }
            }
        }
    }
}

impl DialogForm for BudgetsDialog {
    type Command = BudgetCommand;
    type Refusal = BudgetRefusal;

    fn command(&self) -> Result<BudgetCommand, FormError> {
        match self {
            Self::Budget(form) => form.draft().map(BudgetCommand::SaveBudget).ok_or(FormError),
            Self::EditLimit(form) => form.draft().map(BudgetCommand::SetLimit).ok_or(FormError),
            Self::Stop(form) => {
                let month = form.month().ok_or(FormError)?;
                Ok(BudgetCommand::Stop {
                    category_id: form.category_id,
                    month,
                })
            }
            Self::Fill { month, source } => Ok(BudgetCommand::Fill {
                month: *month,
                source: *source,
            }),
            // The lists answer with requests, not commands.
            Self::Switcher(_) | Self::Manage(_) | Self::CategoryDetail(_) => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: BudgetRefusal) {
        let BudgetRefusal::Budget(error) = refusal else {
            return;
        };
        match self {
            Self::Budget(form) => form.error = Some(error),
            Self::EditLimit(form) => form.error = Some(error),
            Self::Stop(form) => form.error = Some(error),
            Self::Fill { .. } | Self::Switcher(_) | Self::Manage(_) | Self::CategoryDetail(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::{LedgerStore, test_ledger};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        let accounts = lib_accounts::default_accounts();
        let categories = lib_categories::default_categories();
        let seeded = crate::budgets::default_budgets(&accounts, &categories, today());
        test_ledger(
            cx,
            LedgerParts {
                accounts,
                categories,
                budgets: seeded,
                ..LedgerParts::default()
            },
        )
    }

    #[gpui::test]
    fn a_stop_on_a_category_that_is_not_budgeted_is_refused_and_the_lists_are_not_commands(
        cx: &mut TestAppContext,
    ) {
        let ledger = ledger(cx);
        let command = OnBudget {
            command: BudgetCommand::Stop {
                category_id: CategoryId::seed(9999),
                month: Period::of(today()),
            },
            budget: crate::budgets::PERSONAL_SPENDING_ID,
            today: today(),
        };
        let refused = ledger.update(cx, |store, cx| store.apply(cx, command));
        assert!(matches!(refused, Err(BudgetRefusal::Budget(_))));

        let seeded = crate::budgets::default_budgets(
            &lib_accounts::default_accounts(),
            &lib_categories::default_categories(),
            today(),
        );
        let switcher = BudgetsDialog::Switcher(crate::budgets::Switcher::new(
            &seeded,
            crate::budgets::PERSONAL_SPENDING_ID,
        ));
        assert!(switcher.command().is_err(), "a list answers with a request");
    }
}
