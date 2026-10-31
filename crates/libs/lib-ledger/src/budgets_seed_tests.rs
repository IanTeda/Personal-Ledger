//! The Budgets seed tests that read the other domains' seeds, so they need more than one domain:
//! the Budgets seed over the stub Accounts and Categories, its figures over the Transactions seed.
//! Lives here rather than in `lib_budgets`, which depends on no other domain. The rest of the
//! Budgets model's tests live in `lib_budgets`.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use lib_accounts::default_accounts;
use lib_budgets::{
    Budget, BudgetAccounts, BudgetCategories, Budgets, CategoryFigures, LimitRecord, Method,
    PERSONAL_SPENDING_ID, PeriodFigures, applied, check_accounts, check_leaf, default_budgets,
    period_figures, zero,
};
use lib_categories::{self as categories, default_categories};
use lib_core::{BudgetId, CategoryId, Money, Period, UnitId};
use lib_payees::default_payees;
use lib_tags::default_tags;

use crate::seed::default_transactions;
use crate::{Ledger, LedgerParts};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap_or_default()
}

fn today() -> NaiveDate {
    date(2026, 9, 21)
}

fn sep() -> Period {
    Period::of(today())
}

fn money(text: &str) -> Money {
    text.parse().expect("a valid test amount")
}

/// The stub Accounts and Categories with the Transactions seed; no Bills.
struct World {
    ledger: Ledger,
}

impl World {
    fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    fn category(&self, name: &str) -> CategoryId {
        categories::find_by_name(self.ledger.categories(), name).expect("a seeded Category")
    }
}

fn row(figures: &PeriodFigures, category_id: CategoryId) -> Option<&CategoryFigures> {
    figures.rows.iter().find(|r| r.category_id == category_id)
}

fn amount_in(chain: &[LimitRecord], month: Period) -> Option<Money> {
    applied(chain, month).map(|a| a.amount)
}

fn unreachable_budget() -> Budget {
    Budget {
        id: BudgetId::seed(0),
        name: String::new(),
        method: Method::Limits,
        unit: UnitId::SEED_AUD,
        account_ids: Vec::new(),
        is_default: false,
        archived_at: None,
        limits: BTreeMap::new(),
    }
}

fn seeded() -> (World, Budgets) {
    let (accounts, categories) = (default_accounts(), default_categories());
    let transactions = default_transactions(
        &accounts,
        &categories,
        &default_payees(),
        &default_tags(),
        today(),
    );
    let ledger = Ledger::from_parts(LedgerParts {
        accounts,
        categories,
        transactions,
        ..LedgerParts::default()
    });
    let budgets = default_budgets(
        &ledger.budget_accounts(),
        &ledger.budget_categories(),
        today(),
    );
    (World { ledger }, budgets)
}

#[test]
fn the_seed_has_a_default_personal_spending_and_an_overlapping_second_budget() {
    let (world, budgets) = seeded();
    assert_eq!(budgets.all().len(), 2);
    let personal = budgets.get(PERSONAL_SPENDING_ID);
    assert_eq!(personal.map(|b| b.name.as_str()), Some("Personal spending"));
    assert_eq!(
        budgets.default_budget().map(|b| b.id),
        Some(PERSONAL_SPENDING_ID)
    );
    assert_eq!(budgets.all().iter().filter(|b| b.is_default).count(), 1);
    let joint = budgets.get(BudgetId::seed(2));
    assert!(joint.is_some_and(|j| {
        j.account_ids
            .iter()
            .all(|a| personal.is_some_and(|p| p.account_ids.contains(a)))
            && j.category_ids()
                .any(|c| personal.is_some_and(|p| !p.chain(c).is_empty()))
    }));
    assert!(world.category("Groceries") > CategoryId::seed(0));
}

#[test]
fn the_seed_only_budgets_valid_leaves_and_accounts_in_the_unit() {
    let (world, budgets) = seeded();
    let (categories, accounts) = (
        world.ledger().budget_categories(),
        world.ledger().budget_accounts(),
    );
    for budget in budgets.all() {
        for id in budget.category_ids() {
            assert_eq!(check_leaf(&categories, id), Ok(()));
        }
        assert_eq!(
            check_accounts(&accounts, budget.unit, &budget.account_ids),
            Ok(())
        );
        assert!(!budget.account_ids.is_empty());
    }
}

#[test]
fn the_seed_produces_figures_with_parents_as_rollups() {
    let (world, budgets) = seeded();
    let personal = budgets
        .get(PERSONAL_SPENDING_ID)
        .cloned()
        .unwrap_or_else(unreachable_budget);
    let figures = period_figures(&personal, world.ledger(), sep(), today());
    assert!(figures.budgeted.0 > zero());
    let household = row(&figures, world.category("Household"));
    assert!(household.is_some_and(|r| r.budget.is_some() && !r.is_parent));
    assert!(row(&figures, world.category("Food")).is_some_and(|r| r.is_parent));
}

#[test]
fn dining_steps_up_next_month_in_the_seed() {
    let (world, budgets) = seeded();
    let chain = budgets
        .get(PERSONAL_SPENDING_ID)
        .map(|b| b.chain(world.category("Dining")).to_vec())
        .unwrap_or_default();
    assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep().next()), Some(money("300.00")));
}
