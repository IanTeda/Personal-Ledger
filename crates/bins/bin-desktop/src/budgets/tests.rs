//! The Budgets tests that need the Desktop: the Switcher and tab state in `surface.rs`, and the
//! seed tests that read `default_transactions`, which still lives in this crate. The rest of the
//! Budgets model's tests live in `lib_budgets`. A small Ledger stand-in is kept here for these.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use lib_core::{Money, Period};

use super::*;
use crate::{
    accounts::{Account, default_accounts},
    categories::{self, Category, default_categories},
    payees::default_payees,
    tags::default_tags,
    transactions::{Transaction, default_transactions},
};
use lib_budgets::{check_accounts, check_leaf};

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

/// The Accounts, Categories and Transactions a Ledger figure reads; no Bills.
struct World {
    accounts: Vec<Account>,
    categories: Vec<Category>,
    transactions: Vec<Transaction>,
}

impl World {
    fn new() -> Self {
        Self {
            accounts: default_accounts(),
            categories: default_categories(),
            transactions: Vec::new(),
        }
    }

    fn ledger(&self) -> Ledger<'_> {
        Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &[],
            entries: &[],
        }
    }

    fn category(&self, name: &str) -> u32 {
        categories::find_by_name(&self.categories, name).unwrap_or_default()
    }
}

fn row(figures: &PeriodFigures, category_id: u32) -> Option<&CategoryFigures> {
    figures.rows.iter().find(|r| r.category_id == category_id)
}

fn amount_in(chain: &[LimitRecord], month: Period) -> Option<Money> {
    applied(chain, month).map(|a| a.amount)
}

fn unreachable_budget() -> Budget {
    Budget {
        id: 0,
        name: String::new(),
        method: Method::Limits,
        unit: String::new(),
        account_ids: Vec::new(),
        is_default: false,
        archived_at: None,
        limits: BTreeMap::new(),
    }
}

// -- the Switcher -------------------------------------------------------------------------------

#[test]
fn the_switcher_lists_active_budgets_then_archived_and_filters_by_name() {
    let world = World::new();
    let mut budgets = default_budgets(&world.accounts, &world.categories, today());
    let ids: Vec<u32> = budgets.all().iter().map(|b| b.id).collect();
    assert!(ids.len() >= 2, "the seed has more than one Budget");
    assert_eq!(switcher_ids(&budgets, ""), ids);

    // Archiving moves a Budget below the active ones.
    let other = ids
        .iter()
        .copied()
        .find(|id| *id != PERSONAL_SPENDING_ID)
        .unwrap_or_default();
    assert_eq!(budgets.archive(other, today()), Ok(()));
    assert_eq!(switcher_ids(&budgets, "").last(), Some(&other));

    assert_eq!(
        switcher_ids(&budgets, "  PERSONAL "),
        vec![PERSONAL_SPENDING_ID]
    );
    assert!(switcher_ids(&budgets, "no such budget").is_empty());
}

#[test]
fn the_switcher_opens_on_the_current_budget_and_steps_inside_its_rows() {
    let world = World::new();
    let budgets = default_budgets(&world.accounts, &world.categories, today());
    let ids = switcher_ids(&budgets, "");
    let last = *ids.last().unwrap_or(&0);
    let mut switcher = Switcher::new(&budgets, last);
    assert_eq!(switcher.chosen(), Some(last));
    switcher.step(true);
    assert_eq!(switcher.chosen(), Some(last), "stops at the end");
    for _ in 0..ids.len() {
        switcher.step(false);
    }
    assert_eq!(switcher.chosen(), ids.first().copied());

    // Typing narrows the rows and puts the highlight back on the first.
    switcher.step(true);
    for c in "personal".chars() {
        switcher.type_char(c);
    }
    assert_eq!(switcher.chosen(), Some(PERSONAL_SPENDING_ID));
    switcher.type_char('z');
    assert_eq!(switcher.chosen(), None);
    switcher.backspace();
    assert_eq!(switcher.chosen(), Some(PERSONAL_SPENDING_ID));
}

// -- surface state and seed -------------------------------------------------------------------------------

#[test]
fn tabs_cycle_and_the_digits_pick_them() {
    assert_eq!(BudgetsTab::default(), BudgetsTab::Progress);
    assert_eq!(BudgetsTab::Progress.next(), BudgetsTab::Plan);
    assert_eq!(BudgetsTab::History.next(), BudgetsTab::Progress);
    assert_eq!(BudgetsTab::from_digit('3'), Some(BudgetsTab::History));
    assert_eq!(BudgetsTab::from_digit('4'), None);
}

fn seeded() -> (World, Budgets) {
    let mut world = World::new();
    world.transactions = default_transactions(
        &world.accounts,
        &world.categories,
        &default_payees(),
        &default_tags(),
        today(),
    );
    let budgets = default_budgets(&world.accounts, &world.categories, today());
    (world, budgets)
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
    let joint = budgets.get(2);
    assert!(joint.is_some_and(|j| {
        j.account_ids
            .iter()
            .all(|a| personal.is_some_and(|p| p.account_ids.contains(a)))
            && j.category_ids()
                .any(|c| personal.is_some_and(|p| !p.chain(c).is_empty()))
    }));
    assert!(world.category("Groceries") > 0);
}

#[test]
fn the_seed_only_budgets_valid_leaves_and_accounts_in_the_unit() {
    let (world, budgets) = seeded();
    for budget in budgets.all() {
        for id in budget.category_ids() {
            assert_eq!(check_leaf(&world.categories, id), Ok(()));
        }
        assert_eq!(
            check_accounts(&world.accounts, &budget.unit, &budget.account_ids),
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
    let figures = period_figures(&personal, &world.ledger(), sep(), today());
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
