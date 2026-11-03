//! What every Budgets Interactor shares: logging how an intent ended, and writing what a rule
//! run on a copy changed.

use lib_core::BudgetId;

use crate::{Budget, BudgetRepository, Budgets, Error, Result};

/// Logs how an intent ended and hands the outcome back.
pub(super) fn logged<R>(intent: &str, outcome: Result<R>) -> Result<R> {
    match &outcome {
        Ok(_) => tracing::info!(intent, "Budgets intent done"),
        Err(refusal) => tracing::info!(intent, %refusal, "Budgets intent refused"),
    }
    outcome
}

/// Runs `rule` on a copy of the Budget set and writes each Budget it added or changed, a Budget
/// losing the default first (storage holds one).
pub(super) fn write_set<R>(
    repo: &mut dyn BudgetRepository,
    rule: impl FnOnce(&mut Budgets) -> Result<R>,
) -> Result<R> {
    let before = Budgets::from_stored(repo.budgets());
    let mut after = before.clone();
    let result = rule(&mut after)?;
    for budget in after.changed_since(&before) {
        repo.upsert_budget(&budget);
    }
    Ok(result)
}

/// Runs `rule` on a copy of Budget `id` and writes it when it changed.
pub(super) fn write_budget<R>(
    repo: &mut dyn BudgetRepository,
    id: BudgetId,
    rule: impl FnOnce(&mut Budget) -> Result<R>,
) -> Result<R> {
    let before = repo.budget(id).ok_or(Error::NotFound)?;
    let mut after = before.clone();
    let result = rule(&mut after)?;
    if after != before {
        repo.upsert_budget(&after);
    }
    Ok(result)
}
