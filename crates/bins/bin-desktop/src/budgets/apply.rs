//! The Budgets write Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Budgets store, reading the Accounts, Categories,
//! Transactions and Bills the rules check against. When `LedgerStore` replaces the per-domain
//! stores only [`BudgetsApply`] changes.
//!
//! Only the forms that write are here. The Switcher, Manage and Category detail lists answer
//! with a request that opens another Dialog or moves the cursor, which stays `Shell`'s.

use chrono::NaiveDate;
use gpui::{App, Entity};

use super::{
    self as budgets, BudgetError, BudgetsDialog, FillSource,
    form::BudgetDraft,
    limit_form::{self, LimitDraft},
};
use crate::{
    bills::BillsStore,
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    period::Period,
    transactions::TransactionsStore,
    view::{
        accounts::state::AccountsStore, budgets::state::BudgetsStore,
        categories::state::CategoriesStore,
    },
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

/// Runs [`BudgetCommand`]s against the Budgets store; `current` is the Budget on show.
pub struct BudgetsApply {
    pub budgets: Entity<BudgetsStore>,
    pub accounts: Entity<AccountsStore>,
    pub categories: Entity<CategoriesStore>,
    pub transactions: Entity<TransactionsStore>,
    pub bills: Entity<BillsStore>,
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
        // Copied out so the store can be written while the others are read.
        let accounts = self.accounts.read(cx).accounts(cx).to_vec();
        let categories = self.categories.read(cx).categories(cx).to_vec();
        let today = self.today;
        let current = self.current;
        match command {
            BudgetCommand::SaveBudget(BudgetDraft::Create(new)) => {
                let transactions = self.transactions.read(cx).transactions(cx).to_vec();
                let (plans, entries) = {
                    let bills = self.bills.read(cx);
                    (bills.plans(cx).to_vec(), bills.entries(cx).to_vec())
                };
                let ledger = budgets::BudgetSources {
                    categories: &categories,
                    accounts: &accounts,
                    transactions: &transactions,
                    plans: &plans,
                    entries: &entries,
                };
                let id = self.write(cx, |budgets| budgets.create(&new, &ledger, today))?;
                Ok(BudgetOutcome::Created(id))
            }
            BudgetCommand::SaveBudget(BudgetDraft::Edit {
                id,
                name,
                account_ids,
            }) => {
                self.write(cx, |budgets| {
                    budgets.edit(id, &name, account_ids, &accounts)
                })?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::SetLimit(draft) => {
                self.write(cx, |budgets| {
                    limit_form::save(budgets, current, &categories, &draft, today)
                })?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Stop { category_id, month } => {
                self.write(cx, |budgets| {
                    budgets.stop(current, &categories, category_id, month, today)
                })?;
                Ok(BudgetOutcome::Saved)
            }
            BudgetCommand::Fill { month, source } => {
                let transactions = self.transactions.read(cx).transactions(cx).to_vec();
                let (plans, entries) = {
                    let bills = self.bills.read(cx);
                    (bills.plans(cx).to_vec(), bills.entries(cx).to_vec())
                };
                let ledger = budgets::BudgetSources {
                    categories: &categories,
                    accounts: &accounts,
                    transactions: &transactions,
                    plans: &plans,
                    entries: &entries,
                };
                match self.write(cx, |budgets| {
                    budgets.fill(current, &ledger, month, source, today)
                }) {
                    Ok(count) if count > 0 => Ok(BudgetOutcome::Saved),
                    Ok(_) => Err(BudgetRefusal::NothingToFill),
                    Err(error) => Err(error.into()),
                }
            }
        }
    }
}

impl BudgetsApply {
    fn write<R>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut budgets::Budgets) -> Result<R, BudgetError>,
    ) -> Result<R, BudgetError> {
        self.budgets
            .update(cx, |store, cx| store.mutate(cx, change))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;

    fn apply(cx: &mut TestAppContext) -> BudgetsApply {
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let accounts = crate::accounts::default_accounts();
        let categories = lib_categories::default_categories();
        let seeded = budgets::default_budgets(&accounts, &categories, today);
        let current = budgets::PERSONAL_SPENDING_ID;
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
            budgets: cx.new(|cx| BudgetsStore::new(ledger.clone(), cx)),
            accounts: cx.new(|cx| AccountsStore::new(ledger.clone(), cx)),
            categories: cx.new(|cx| CategoriesStore::new(ledger.clone(), cx)),
            transactions: cx.new(|cx| TransactionsStore::new(ledger.clone(), cx)),
            bills: cx.new(|cx| BillsStore::new(ledger, cx)),
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
        let seeded = budgets::default_budgets(
            &crate::accounts::default_accounts(),
            &lib_categories::default_categories(),
            today,
        );
        let switcher = BudgetsDialog::Switcher(budgets::Switcher::new(&seeded, apply.current));
        assert!(switcher.command().is_err(), "a list answers with a request");
    }
}
