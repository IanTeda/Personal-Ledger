//! The Budgets write Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger, which reads the Accounts, Categories,
//! Transactions and Bills the rules check against.
//!
//! Only the forms that write are here. The Switcher, Manage and Category detail lists answer
//! with a request that opens another Dialog or moves the cursor, which stays `Shell`'s.

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_ledger::{
    AddBudget, EditBudget, FillBudget, LedgerCommand, SetBudgetAmount, StopBudgeting,
};

use super::{BudgetError, BudgetsDialog, FillSource, form::BudgetDraft, limit_form::LimitDraft};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    ledger_store::LedgerStore,
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
    Stop { category_id: u32, month: Period },
    /// 9f: write the chosen source's Month-only amounts into the Budget on show.
    Fill { month: Period, source: FillSource },
}

/// What a landed [`BudgetCommand`] did, for the View that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetOutcome {
    /// A Budget was created: the surface switches to it.
    Created(u32),
    Saved,
}

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

/// Runs [`BudgetCommand`]s against the Ledger; `current` is the Budget on show.
pub struct BudgetsApply {
    pub ledger: Entity<LedgerStore>,
    pub current: u32,
    pub today: NaiveDate,
}

impl Apply<BudgetCommand> for BudgetsApply {
    type Outcome = BudgetOutcome;
    type Refusal = BudgetRefusal;

    fn apply(
        &mut self,
        command: BudgetCommand,
        cx: &mut App,
    ) -> Result<BudgetOutcome, BudgetRefusal> {
        let today = self.today;
        let current = self.current;
        match command {
            BudgetCommand::SaveBudget(BudgetDraft::Create(new)) => {
                let id = self.run(cx, AddBudget { new, today })?;
                Ok(BudgetOutcome::Created(id))
            }
            BudgetCommand::SaveBudget(BudgetDraft::Edit {
                id,
                name,
                account_ids,
            }) => {
                self.run(
                    cx,
                    EditBudget {
                        id,
                        name,
                        account_ids,
                    },
                )?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::SetLimit(draft) => {
                self.run(
                    cx,
                    SetBudgetAmount {
                        budget_id: current,
                        category_id: draft.category_id,
                        month: draft.month,
                        span: draft.span,
                        amount: draft.amount,
                        rollover: Some(draft.rollover),
                        today,
                    },
                )?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Stop { category_id, month } => {
                self.run(
                    cx,
                    StopBudgeting {
                        budget_id: current,
                        category_id,
                        month,
                        today,
                    },
                )?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Fill { month, source } => {
                let filled = self.run(
                    cx,
                    FillBudget {
                        budget_id: current,
                        target: month,
                        source,
                        today,
                    },
                )?;
                if filled > 0 {
                    Ok(BudgetOutcome::Saved)
                } else {
                    Err(BudgetRefusal::NothingToFill)
                }
            }
        }
    }
}

impl BudgetsApply {
    fn run<C>(&self, cx: &mut App, command: C) -> Result<C::Outcome, BudgetError>
    where
        C: LedgerCommand<Refusal = BudgetError>,
    {
        self.ledger.update(cx, |store, cx| store.apply(cx, command))
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;

    fn apply(cx: &mut TestAppContext) -> BudgetsApply {
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let accounts = crate::accounts::default_accounts();
        let categories = lib_categories::default_categories();
        let seeded = crate::budgets::default_budgets(&accounts, &categories, today);
        let current = crate::budgets::PERSONAL_SPENDING_ID;
        let ledger = test_ledger(
            cx,
            LedgerParts {
                accounts,
                categories,
                budgets: seeded,
                ..LedgerParts::default()
            },
        );
        BudgetsApply {
            ledger,
            current,
            today,
        }
    }

    #[gpui::test]
    fn a_stop_on_a_category_that_is_not_budgeted_is_refused_and_the_lists_are_not_commands(
        cx: &mut TestAppContext,
    ) {
        let mut apply = apply(cx);
        let month = Period::of(apply.today);
        let refused = cx.update(|cx| {
            apply.apply(
                BudgetCommand::Stop {
                    category_id: 9999,
                    month,
                },
                cx,
            )
        });
        assert!(matches!(refused, Err(BudgetRefusal::Budget(_))));

        let today = apply.today;
        let seeded = crate::budgets::default_budgets(
            &crate::accounts::default_accounts(),
            &lib_categories::default_categories(),
            today,
        );
        let switcher =
            BudgetsDialog::Switcher(crate::budgets::Switcher::new(&seeded, apply.current));
        assert!(switcher.command().is_err(), "a list answers with a request");
    }
}
