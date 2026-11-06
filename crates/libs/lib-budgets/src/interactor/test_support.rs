//! The fixtures the Interactors' tests share: the sample Budgets and reads, as of 21 September 2026.

pub(crate) use chrono::NaiveDate;
pub(crate) use lib_core::{AccountId, BudgetId, CategoryId, Money, Period, UnitId};

pub(crate) use crate::fake::{InMemoryBudgetReads, InMemoryBudgets};
pub(crate) use crate::*;

pub(crate) const JOINT: BudgetId = BudgetId::seed(2);
pub(crate) const GROCERIES: CategoryId = CategoryId::seed(7);
pub(crate) const RENT: CategoryId = CategoryId::seed(2);
pub(crate) const FOOD: CategoryId = CategoryId::seed(6);
pub(crate) const ANZ_EVERYDAY: AccountId = AccountId::seed(2);

pub(crate) fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
}

pub(crate) fn current() -> Period {
    Period::of(today())
}

pub(crate) fn repo() -> InMemoryBudgets {
    InMemoryBudgets::sample(today())
}

pub(crate) fn reads() -> InMemoryBudgetReads {
    InMemoryBudgetReads::sample()
}

pub(crate) fn budget(repo: &InMemoryBudgets, id: BudgetId) -> Budget {
    repo.budget(id).unwrap()
}

pub(crate) fn amount_in(repo: &InMemoryBudgets, id: BudgetId, month: Period) -> Option<Money> {
    applied(budget(repo, id).chain(GROCERIES), month).map(|found| found.amount)
}

pub(crate) fn holiday() -> NewBudget {
    NewBudget {
        name: "Holiday".into(),
        unit: UnitId::SEED_AUD,
        account_ids: vec![AccountId::seed(1)],
        start_from: StartFrom::Empty,
    }
}

pub(crate) fn set(span: Span, month: Period, amount: i64) -> SetAmount {
    SetAmount {
        id: PERSONAL_SPENDING_ID,
        category_id: GROCERIES,
        month,
        span,
        amount: cents_money(amount),
        rollover: None,
        today: today(),
    }
}
