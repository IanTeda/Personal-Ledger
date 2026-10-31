//! The Budgets tests that need the Desktop: the Switcher and tab state in `surface.rs`. The seed
//! tests that read the Transactions seed live in `lib_ledger`, and the rest of the Budgets model's
//! tests live in `lib_budgets`.

use chrono::NaiveDate;
use lib_core::BudgetId;

use super::*;
use lib_accounts::{Account, default_accounts};
use lib_categories::{Category, default_categories};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
}

/// The Accounts and Categories the Switcher reads.
struct World {
    accounts: Vec<Account>,
    categories: Vec<Category>,
}

impl World {
    fn new() -> Self {
        Self {
            accounts: default_accounts(),
            categories: default_categories(),
        }
    }
}

// -- the Switcher -------------------------------------------------------------------------------

#[test]
fn the_switcher_lists_active_budgets_then_archived_and_filters_by_name() {
    let world = World::new();
    let mut budgets = default_budgets(
        &lib_ledger::budget_accounts(&world.accounts),
        &lib_ledger::budget_categories(&world.categories),
        today(),
    );
    let ids: Vec<BudgetId> = budgets.all().iter().map(|b| b.id).collect();
    assert!(ids.len() >= 2, "the seed has more than one Budget");
    assert_eq!(switcher_ids(&budgets, ""), ids);

    // Archiving moves a Budget below the active ones.
    let other = ids
        .iter()
        .copied()
        .find(|id| *id != PERSONAL_SPENDING_ID)
        .expect("a second seeded Budget");
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
    let budgets = default_budgets(
        &lib_ledger::budget_accounts(&world.accounts),
        &lib_ledger::budget_categories(&world.categories),
        today(),
    );
    let ids = switcher_ids(&budgets, "");
    let last = *ids.last().expect("a seeded Budget");
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
