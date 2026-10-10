//! The Budgets model's unit tests, kept in their own file so the model's sections stay short.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use lib_accounts::{Account, default_accounts};
use lib_bills::{
    self as bills, AmountKind, BillPlan, BillPlanDraft, BillScheduleEntry, Recurrence,
};
use lib_categories::{self as categories, Category, default_categories};
use lib_core::{CategoryTypes, Money, Period, TransactionStatus};
use lib_transactions::{Split, Transaction};

use super::*;

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

fn category_id(categories: &[Category], name: &str) -> u32 {
    categories::find_by_name(categories, name).unwrap_or_default()
}

/// A tiny hand-built Ledger, so figures are exact rather than random.
struct World {
    accounts: Vec<Account>,
    categories: Vec<Category>,
    transactions: Vec<Transaction>,
    plans: Vec<BillPlan>,
    entries: Vec<BillScheduleEntry>,
    next_transaction: u32,
}

impl World {
    fn new() -> Self {
        Self {
            accounts: default_accounts(),
            categories: default_categories(),
            transactions: Vec::new(),
            plans: Vec::new(),
            entries: Vec::new(),
            next_transaction: 1,
        }
    }

    fn ledger(&self) -> Ledger<'_> {
        Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.plans,
            entries: &self.entries,
        }
    }

    fn account(&self, name: &str) -> u32 {
        self.accounts
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id)
            .unwrap_or_default()
    }

    fn category(&self, name: &str) -> u32 {
        category_id(&self.categories, name)
    }

    /// A single-Split Transaction; `amount` follows the ledger's sign (expenses negative).
    fn post(&mut self, account: &str, category: &str, on: NaiveDate, amount: &str) {
        let id = self.next_transaction;
        self.next_transaction += 1;
        self.transactions.push(Transaction {
            id,
            date: on,
            account_id: self.account(account),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: vec![Split {
                amount: money(amount),
                category_id: self.category(category),
                payee_id: None,
                tag_ids: Vec::new(),
            }],
        });
    }

    fn bill(&mut self, account: &str, category: &str, first_due: NaiveDate, amount: &str) {
        let draft = BillPlanDraft {
            name: format!("{category} bill"),
            category_id: self.category(category),
            unit: "aud".to_string(),
            account_id: self.account(account),
            payee_id: None,
            planned_amount: money(amount),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::OneShot,
            first_due,
            ends_on: None,
            attention_lead: None,
        };
        let inserted = bills::insert_plan(
            &mut self.plans,
            &mut self.entries,
            &draft,
            &self.categories,
            &self.accounts,
            today(),
        );
        assert!(inserted.is_ok(), "{inserted:?}");
    }

    /// An empty Budget on the two ANZ accounts.
    fn budgets(&self) -> (Budgets, u32) {
        let mut budgets = Budgets::default();
        let id = budgets
            .create(
                &NewBudget {
                    name: "Test".to_string(),
                    unit: "aud".to_string(),
                    account_ids: vec![self.account("ANZ Everyday"), self.account("ANZ Offset")],
                    start_from: StartFrom::Empty,
                },
                &self.ledger(),
                today(),
            )
            .unwrap_or_default();
        (budgets, id)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a test shorthand for one set_amount call"
)]
fn set(
    budgets: &mut Budgets,
    world: &World,
    id: u32,
    category: &str,
    month: Period,
    span: Span,
    amount: &str,
    rollover: Option<Rollover>,
) {
    let done = budgets.set_amount(
        id,
        &world.categories,
        world.category(category),
        month,
        span,
        money(amount),
        rollover,
        today(),
    );
    assert_eq!(done, Ok(()));
}

fn row(figures: &PeriodFigures, category_id: u32) -> Option<&CategoryFigures> {
    figures.rows.iter().find(|r| r.category_id == category_id)
}

// -- chains ---------------------------------------------------------------------------------

fn onward(month: Period, amount: &str) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::Onward {
            amount: money(amount),
            rollover: Rollover::None,
        },
    }
}

fn month_only(month: Period, amount: &str) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::MonthOnly {
            amount: money(amount),
            rollover: Rollover::None,
        },
    }
}

fn stop(month: Period) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::Stop,
    }
}

fn amount_in(chain: &[LimitRecord], month: Period) -> Option<Money> {
    applied(chain, month).map(|a| a.amount)
}

#[test]
fn an_onward_amount_runs_until_the_next_record() {
    let chain = [
        onward(sep().prev(), "250.00"),
        onward(sep().next(), "300.00"),
    ];
    assert_eq!(amount_in(&chain, sep().prev().prev()), None);
    assert_eq!(amount_in(&chain, sep().prev()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep().next()), Some(money("300.00")));
    assert_eq!(
        amount_in(&chain, sep().next().next()),
        Some(money("300.00"))
    );
}

#[test]
fn a_month_only_amount_falls_back_to_the_onward_value_the_month_after() {
    let chain = [onward(sep().prev(), "250.00"), month_only(sep(), "400.00")];
    assert_eq!(amount_in(&chain, sep()), Some(money("400.00")));
    assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
}

#[test]
fn a_month_only_amount_with_no_onward_leaves_the_other_months_unbudgeted() {
    let chain = [
        month_only(sep(), "420.00"),
        month_only(sep().next().next().next(), "420.00"),
    ];
    assert_eq!(amount_in(&chain, sep().prev()), None);
    assert_eq!(amount_in(&chain, sep()), Some(money("420.00")));
    assert_eq!(amount_in(&chain, sep().next()), None);
    assert_eq!(
        amount_in(&chain, sep().next().next().next()),
        Some(money("420.00"))
    );
}

#[test]
fn a_stop_ends_the_amount_until_a_later_onward_resumes_it() {
    let chain = [
        onward(sep().prev(), "250.00"),
        stop(sep()),
        onward(sep().next().next(), "275.00"),
    ];
    assert_eq!(amount_in(&chain, sep().prev()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep()), None);
    assert_eq!(amount_in(&chain, sep().next()), None);
    assert_eq!(
        amount_in(&chain, sep().next().next()),
        Some(money("275.00"))
    );
}

#[test]
fn an_explicit_zero_is_budgeted_and_no_amount_is_not() {
    let chain = [onward(sep(), "0.00")];
    assert_eq!(amount_in(&chain, sep()), Some(money("0.00")));
    assert_eq!(amount_in(&chain, sep().prev()), None);
}

#[test]
fn a_record_marks_its_own_month_for_the_bold_cell() {
    let chain = [onward(sep().prev(), "250.00")];
    assert_eq!(
        applied(&chain, sep().prev()).map(|a| a.own_record),
        Some(true)
    );
    assert_eq!(applied(&chain, sep()).map(|a| a.own_record), Some(false));
}

// -- effective budget and carry ---------------------------------------------------------------

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

/// A Budget whose Electricity chain starts in July with the given Rollover.
fn electricity_budget(world: &World, rollover: Rollover) -> Budget {
    let mut budget = world
        .budgets()
        .0
        .get(1)
        .cloned()
        .unwrap_or_else(unreachable_budget);
    budget.limits.insert(
        world.category("Electricity"),
        vec![LimitRecord {
            month: Period::of(date(2026, 7, 1)),
            limit: Limit::Onward {
                amount: money("90.00"),
                rollover,
            },
        }],
    );
    budget
}

fn carried_into_september(world: &World, rollover: Rollover) -> Option<Money> {
    let budget = electricity_budget(world, rollover);
    effective_budget(
        &budget,
        &world.ledger(),
        world.category("Electricity"),
        sep(),
        today(),
    )
    .map(|e| e.carried_in)
}

#[test]
fn carry_unspent_compounds_across_closed_months() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-60.00");
    world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-70.00");
    // July leaves 30; August is 90 + 30 - 70 = 50.
    assert_eq!(
        carried_into_september(&world, Rollover::CarryUnspent),
        Some(money("50.00"))
    );
}

#[test]
fn carry_unspent_ignores_an_overspend_but_carry_both_takes_it() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-120.00");
    world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-90.00");
    assert_eq!(
        carried_into_september(&world, Rollover::CarryUnspent),
        Some(money("0.00"))
    );
    // July overspends by 30, so August's budget is 60 and leaves -30 more: -30 carried.
    assert_eq!(
        carried_into_september(&world, Rollover::CarryBoth),
        Some(money("-30.00"))
    );
}

#[test]
fn no_rollover_carries_nothing() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-10.00");
    assert_eq!(
        carried_into_september(&world, Rollover::None),
        Some(money("0.00"))
    );
}

#[test]
fn the_current_month_never_carries_into_next_month() {
    let world = World::new();
    let budget = electricity_budget(&world, Rollover::CarryBoth);
    let next = effective_budget(
        &budget,
        &world.ledger(),
        world.category("Electricity"),
        sep().next(),
        today(),
    );
    assert_eq!(next.map(|e| e.carried_in), Some(money("0.00")));
}

#[test]
fn a_stop_or_an_unbudgeted_month_resets_the_carry() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-10.00");
    let mut budget = electricity_budget(&world, Rollover::CarryUnspent);
    let electricity = world.category("Electricity");
    // July leaves 80; a Stop in August resets it, and September resumes from nothing.
    let august = Period::of(date(2026, 8, 1));
    if let Some(chain) = budget.limits.get_mut(&electricity) {
        put(chain, stop(august));
        put(
            chain,
            LimitRecord {
                month: sep(),
                limit: Limit::Onward {
                    amount: money("90.00"),
                    rollover: Rollover::CarryUnspent,
                },
            },
        );
    }
    let found = effective_budget(&budget, &world.ledger(), electricity, sep(), today());
    assert_eq!(found.map(|e| e.carried_in), Some(money("0.00")));
}

#[test]
fn a_month_only_amount_carries_on_its_own_rollover() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-40.00");
    let mut budget = electricity_budget(&world, Rollover::None);
    let electricity = world.category("Electricity");
    let august = Period::of(date(2026, 8, 1));
    if let Some(chain) = budget.limits.get_mut(&electricity) {
        put(
            chain,
            LimitRecord {
                month: august,
                limit: Limit::MonthOnly {
                    amount: money("100.00"),
                    rollover: Rollover::CarryUnspent,
                },
            },
        );
    }
    let found = effective_budget(&budget, &world.ledger(), electricity, sep(), today());
    assert_eq!(found.map(|e| e.carried_in), Some(money("60.00")));
}

// -- period figures ---------------------------------------------------------------------------

/// Dining budgeted 250 and Groceries 500 from September; Dining overspent by 68.40.
fn september() -> (World, Budgets, u32) {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 9, 3), "-200.00");
    world.post("ANZ Offset", "Dining", date(2026, 9, 12), "-118.40");
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-120.00");
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 6), "20.00");
    world.post("ANZ Everyday", "Household", date(2026, 9, 7), "-58.00");
    // Off budget: the Amex isn't on this Budget.
    world.post("Amex Platinum", "Groceries", date(2026, 9, 8), "-999.00");
    // An earlier month never lands in September.
    world.post("ANZ Everyday", "Dining", date(2026, 8, 30), "-77.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "500.00",
        None,
    );
    (world, budgets, id)
}

#[test]
fn spent_is_signed_and_counts_only_on_budget_accounts() {
    let (world, budgets, id) = september();
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let groceries = row(&figures, world.category("Groceries"));
    // 120 spent, 20 refunded, the Amex purchase off budget.
    assert_eq!(groceries.map(|r| r.spent.clone()), Some(money("100.00")));
    let dining = row(&figures, world.category("Dining"));
    assert_eq!(dining.map(|r| r.spent.clone()), Some(money("318.40")));
}

#[test]
fn a_leaf_over_its_budget_counts_and_carries_the_negative_left() {
    let (world, budgets, id) = september();
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let dining = row(&figures, world.category("Dining"));
    assert_eq!(dining.map(|r| r.over), Some(true));
    assert_eq!(dining.and_then(|r| r.left.clone()), Some(money("-68.40")));
    assert_eq!(figures.over_count, 1);
}

#[test]
fn totals_reconcile_left_with_budgeted_spent_and_known() {
    let (world, budgets, id) = september();
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    assert_eq!(figures.budgeted, money("750.00"));
    assert_eq!(figures.spent, money("418.40"));
    assert_eq!(figures.known, money("0.00"));
    assert_eq!(figures.left, money("331.60"));
}

#[test]
fn unbudgeted_spending_is_listed_but_not_rolled_up_or_counted() {
    let (world, budgets, id) = september();
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let household = row(&figures, world.category("Household"));
    assert_eq!(household.map(|r| r.budget.clone()), Some(None));
    assert_eq!(household.map(|r| r.spent.clone()), Some(money("58.00")));
    assert_eq!(figures.unbudgeted_spent, money("58.00"));
    assert_eq!(figures.over_count, 1);
}

#[test]
fn a_parent_rolls_up_only_its_budgeted_leaves() {
    let (world, mut budgets, id) = september();
    set(
        &mut budgets,
        &world,
        id,
        "Transport",
        sep(),
        Span::Onward,
        "100.00",
        None,
    );
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let food = row(&figures, world.category("Food"));
    assert_eq!(food.map(|r| r.is_parent), Some(true));
    assert_eq!(food.and_then(|r| r.budget.clone()), Some(money("750.00")));
    assert_eq!(food.map(|r| r.spent.clone()), Some(money("418.40")));
}

#[test]
fn a_parent_leaves_an_unbudgeted_childs_spending_out_of_its_spent() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-50.00");
    world.post("ANZ Everyday", "Dining", date(2026, 9, 6), "-30.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "200.00",
        None,
    );
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let food = row(&figures, world.category("Food"));
    assert_eq!(food.map(|r| r.spent.clone()), Some(money("50.00")));
    let dining = row(&figures, world.category("Dining"));
    assert_eq!(dining.map(|r| r.budget.clone()), Some(None));
    assert_eq!(dining.map(|r| r.spent.clone()), Some(money("30.00")));
}

#[test]
fn a_zero_budget_makes_any_spend_over() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-1.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "0.00",
        None,
    );
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    assert_eq!(
        row(&figures, world.category("Groceries")).map(|r| r.over),
        Some(true)
    );
}

#[test]
fn known_costs_are_current_due_and_carried_overdue_on_budget_accounts() {
    let mut world = World::new();
    // Due later this month, overdue earlier this month, overdue in August, and next month.
    world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
    world.bill("ANZ Everyday", "Electricity", date(2026, 9, 10), "20.00");
    world.bill("ANZ Everyday", "Electricity", date(2026, 8, 15), "5.00");
    world.bill("ANZ Everyday", "Electricity", date(2026, 10, 3), "9.00");
    // A different Account that isn't on this Budget.
    world.bill("Amex Platinum", "Electricity", date(2026, 9, 25), "1000.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Electricity",
        sep(),
        Span::Onward,
        "200.00",
        None,
    );
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let electricity = world.category("Electricity");

    let now = period_figures(&budget, &world.ledger(), sep(), today());
    assert_eq!(
        row(&now, electricity).map(|r| r.known.clone()),
        Some(money("105.00"))
    );
    assert_eq!(now.left, money("95.00"));
    // The four unpaid bills on the Budget's Account: two this month, one carried, none off-budget.
    assert_eq!(now.known_count, 3);

    let next = period_figures(&budget, &world.ledger(), sep().next(), today());
    assert_eq!(
        row(&next, electricity).map(|r| r.known.clone()),
        Some(money("9.00"))
    );

    let past = period_figures(&budget, &world.ledger(), sep().prev(), today());
    assert!(row(&past, electricity).is_none());
}

#[test]
fn a_paid_bill_stops_counting_once_it_is_no_longer_open() {
    let mut world = World::new();
    world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
    if let Some(entry) = world.entries.first_mut() {
        entry.resolution = bills::Resolution::Skipped {
            planned: money("80.00"),
        };
    }
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Electricity",
        sep(),
        Span::Onward,
        "200.00",
        None,
    );
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    assert_eq!(figures.known, money("0.00"));
}

#[test]
fn known_costs_alone_pushing_a_leaf_over_is_at_risk_and_not_counted_as_over() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Electricity", date(2026, 9, 5), "-60.00");
    world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Electricity",
        sep(),
        Span::Onward,
        "100.00",
        None,
    );
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    let row = row(&figures, world.category("Electricity"));
    assert_eq!(row.map(|r| (r.over, r.at_risk)), Some((false, true)));
    assert_eq!((figures.over_count, figures.at_risk_count), (0, 1));
    assert_eq!(row.and_then(|r| r.left.clone()), Some(money("-40.00")));
}

#[test]
fn known_costs_without_a_budget_amount_are_listed_as_unbudgeted() {
    let mut world = World::new();
    world.bill("ANZ Everyday", "Water", date(2026, 9, 28), "40.00");
    let (budgets, id) = world.budgets();
    let figures = period_figures(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        sep(),
        today(),
    );
    assert_eq!(
        row(&figures, world.category("Water")).map(|r| r.known.clone()),
        Some(money("40.00"))
    );
    assert_eq!(figures.unbudgeted_known, money("40.00"));
    assert_eq!(figures.known, money("0.00"));
}

#[test]
fn elapsed_and_per_day_use_days_remaining_including_today() {
    let (world, budgets, id) = september();
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let now = period_figures(&budget, &world.ledger(), sep(), today());
    // Day 21 of 30: 70% through, 10 days left including today.
    assert_eq!(
        now.elapsed,
        Elapsed {
            day: 21,
            days_in_month: 30,
            percent: 70,
            days_left: 10
        }
    );
    assert_eq!(now.per_day, Some(money("33.16")));

    let past = period_figures(&budget, &world.ledger(), sep().prev(), today());
    assert_eq!((past.elapsed.percent, past.elapsed.days_left), (100, 0));
    assert_eq!(past.per_day, None);

    let future = period_figures(&budget, &world.ledger(), sep().next(), today());
    assert_eq!((future.elapsed.percent, future.elapsed.days_left), (0, 31));
}

#[test]
fn health_summarises_the_current_month() {
    let (world, budgets, id) = september();
    let health = health(
        &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
        &world.ledger(),
        today(),
    );
    assert_eq!(
        health,
        Health {
            over: 1,
            left: money("331.60"),
            at_risk: 0
        }
    );
}

// -- history ------------------------------------------------------------------------------------

fn history_world() -> (World, Budget) {
    let mut world = World::new();
    // June under, July over, August under, September so far over.
    world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
    world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-350.00");
    world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-200.00");
    world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-400.00");
    let mut budget = world
        .budgets()
        .0
        .get(1)
        .cloned()
        .unwrap_or_else(unreachable_budget);
    budget.limits.insert(
        world.category("Dining"),
        vec![LimitRecord {
            month: Period::of(date(2026, 6, 1)),
            limit: Limit::Onward {
                amount: money("300.00"),
                rollover: Rollover::None,
            },
        }],
    );
    (world, budget)
}

#[test]
fn history_average_and_over_use_closed_months_only() {
    let (world, budget) = history_world();
    let first = Period::of(date(2026, 6, 1));
    let history = history(&budget, &world.ledger(), first, sep(), today());
    let dining = history
        .rows
        .iter()
        .find(|r| r.category_id == world.category("Dining"));
    // June 100, July 350, August 200: mean 216.67; over in July only; September counts in neither.
    assert_eq!(
        dining.and_then(|r| r.average.clone()),
        Some(money("216.67"))
    );
    assert_eq!(
        dining.map(|r| (r.over_months, r.closed_months)),
        Some((1, 3))
    );
    assert_eq!(dining.map(|r| r.cells.len()), Some(4));
    let september = dining.and_then(|r| r.cells.last().cloned()).flatten();
    assert_eq!(september.map(|c| c.over), Some(true));
}

#[test]
fn history_shows_a_dash_for_an_unbudgeted_month_and_parents_as_rollups() {
    let (world, budget) = history_world();
    let earlier = Period::of(date(2026, 5, 1));
    let history = history(&budget, &world.ledger(), earlier, sep(), today());
    let dining = history
        .rows
        .iter()
        .find(|r| r.category_id == world.category("Dining"));
    assert_eq!(dining.map(|r| r.cells[0].is_none()), Some(true));
    let food = history
        .rows
        .iter()
        .find(|r| r.category_id == world.category("Food"));
    assert_eq!(food.map(|r| r.is_parent), Some(true));
    assert_eq!(
        food.and_then(|r| r.cells[2].clone()).map(|c| c.spent),
        Some(money("350.00"))
    );
    assert_eq!((history.leaves_shown, history.leaves_ever_budgeted), (1, 1));
}

#[test]
fn history_month_totals_count_budgeted_leaves_only_and_mark_the_current_month() {
    let (mut world, budget) = history_world();
    world.post("ANZ Everyday", "Household", date(2026, 7, 9), "-999.00");
    let first = Period::of(date(2026, 6, 1));
    let history = history(&budget, &world.ledger(), first, sep(), today());
    let july = &history.months[1];
    assert_eq!(
        (july.budget.clone(), july.spent.clone(), july.over),
        (money("300.00"), money("350.00"), true)
    );
    assert_eq!(
        history
            .months
            .iter()
            .map(|m| m.is_current)
            .collect::<Vec<_>>(),
        [false, false, false, true]
    );
}

#[test]
fn history_bounds_run_from_the_first_amount_to_the_current_month() {
    let (_, budget) = history_world();
    assert_eq!(
        history_bounds(&budget, today()),
        Some((Period::of(date(2026, 6, 1)), sep()))
    );
    assert_eq!(history_bounds(&unreachable_budget(), today()), None);
}

#[test]
fn the_history_range_is_six_months_inside_the_bounds() {
    let (_, budget) = history_world();
    let jun = Period::of(date(2026, 6, 1));
    // The Budget starts in June, so only four months exist yet.
    assert_eq!(history_range(&budget, sep(), today()), Some((jun, sep())));
    assert_eq!(
        history_range(&budget, sep().next(), today()),
        Some((jun, sep())),
        "never past the current month"
    );
    assert_eq!(
        history_range(&budget, jun, today()),
        Some((jun, sep())),
        "never a shorter window than the Budget allows"
    );
    // A later today gives a full window that can step back.
    let later = date(2027, 3, 10);
    let mar = Period::of(later);
    assert_eq!(
        history_range(&budget, mar, later),
        Some((Period::of(date(2026, 10, 1)), mar))
    );
    assert_eq!(
        history_range(&budget, Period::of(date(2026, 12, 1)), later),
        Some((Period::of(date(2026, 7, 1)), Period::of(date(2026, 12, 1))))
    );
    assert_eq!(history_range(&unreachable_budget(), sep(), today()), None);
}

#[test]
fn the_history_csv_writes_plain_numbers_and_quotes_awkward_names() {
    let (mut world, budget) = history_world();
    let dining = world.category("Dining");
    if let Some(category) = world.categories.iter_mut().find(|c| c.id == dining) {
        category.name = "Dining, \"out\"".to_string();
    }
    let jun = Period::of(date(2026, 6, 1));
    let shown = history(&budget, &world.ledger(), jun, sep(), today());
    let labels = HistoryCsvLabels {
        category: "Category".into(),
        budget: "budget".into(),
        spent: "spent".into(),
        average: "Avg".into(),
        over: "Over".into(),
    };
    let csv = history_csv(&shown, &world.categories, &labels, |month| {
        format!("{}-{:02}", month.year, month.month)
    });
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), shown.rows.len() + 1);
    assert!(lines[0].starts_with("Category,2026-06 budget,2026-06 spent,"));
    assert!(lines[0].ends_with(",2026-09 budget,2026-09 spent,Avg,Over"));
    let row = shown
        .rows
        .iter()
        .position(|row| row.category_id == dining)
        .unwrap_or_else(|| unreachable!("Dining is budgeted"));
    let line = lines[row + 1];
    assert!(
        line.contains("Dining, \"\"out\"\"\",300.00,100.00,300.00,350.00,"),
        "{line}"
    );
    assert!(line.starts_with('"'));
    assert!(!line.contains('\u{25b2}'));
    let dining_row = &shown.rows[row];
    assert!(line.ends_with(&format!(
        ",{}/{}",
        dining_row.over_months, dining_row.closed_months
    )));
    // Every line has the same number of unquoted separators as the header has columns.
    assert_eq!(lines[0].split(',').count(), 1 + shown.months.len() * 2 + 2);
}

// -- averages and Unallocated --------------------------------------------------------------------

#[test]
fn the_three_month_average_rounds_to_the_nearest_ten_and_floors_at_zero() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
    world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-100.00");
    world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-113.00");
    // Mean 104.33 rounds to 100.00. September itself is excluded.
    world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-900.00");
    world.post("ANZ Everyday", "Groceries", date(2026, 8, 4), "20.00");
    let (budgets, id) = world.budgets();
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let ledger = world.ledger();
    assert_eq!(
        three_month_average(&budget, &ledger, world.category("Dining"), sep()),
        money("100.00")
    );
    // A net refund averages below zero and is floored.
    assert_eq!(
        three_month_average(&budget, &ledger, world.category("Groceries"), sep()),
        money("0.00")
    );
}

// -- the Dashboard's bars -------------------------------------------------------------------

#[test]
fn dashboard_bars_take_the_fullest_budgeted_top_level_rows() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-300.00");
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-100.00");
    world.post("ANZ Everyday", "Rent", date(2026, 9, 1), "-900.00");
    world.post("ANZ Everyday", "Transport", date(2026, 9, 2), "-30.00");
    world.post("ANZ Everyday", "Household", date(2026, 9, 3), "-10.00");
    let (mut budgets, id) = world.budgets();
    for (name, amount) in [
        ("Dining", "200.00"),
        ("Groceries", "600.00"),
        ("Rent", "1000.00"),
        ("Transport", "600.00"),
        ("Water", "50.00"),
    ] {
        let set = budgets.set_amount(
            id,
            &world.categories,
            world.category(name),
            sep(),
            Span::Onward,
            money(amount),
            None,
            today(),
        );
        assert_eq!(set, Ok(()), "{name}");
    }
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let figures = period_figures(&budget, &world.ledger(), sep(), today());
    let bars = dashboard_bars(&figures, &world.categories);
    assert!(bars.len() <= DASHBOARD_BARS);
    for bar in &bars {
        assert_eq!(categories::depth(&world.categories, bar.category_id), 0);
    }
    // Rent (90%) leads, as its top-level parent's rollup; Household is unbudgeted and so is
    // never shown.
    let top_of = |name: &str| {
        let id = world.category(name);
        world
            .categories
            .iter()
            .find(|category| category.id == id)
            .and_then(|category| category.parent)
            .unwrap_or(id)
    };
    assert_eq!(
        bars.first().map(|bar| bar.category_id),
        Some(top_of("Rent"))
    );
    assert!(
        bars.iter()
            .all(|bar| bar.category_id != top_of("Household"))
    );
    // Food rolls Dining (over) and Groceries up to 400 of 800: not over as a whole.
    let food = top_of("Dining");
    let food_bar = bars.iter().find(|bar| bar.category_id == food);
    assert_eq!(food_bar.map(|bar| bar.spent.clone()), Some(money("400.00")));
    assert_eq!(food_bar.map(|bar| bar.over), Some(false));
}

// -- Fill ---------------------------------------------------------------------------------------

#[test]
fn fill_targets_the_first_open_month_at_or_after_the_cursor() {
    assert_eq!(fill_target(Some(sep()), today()), sep());
    assert_eq!(
        fill_target(Some(sep().next().next()), today()),
        sep().next().next()
    );
    assert_eq!(fill_target(Some(sep().prev()), today()), sep().next());
    assert_eq!(fill_target(None, today()), sep().next());
}

/// Dining 250 Onward with a Month-only 400 in September, Groceries 500 Onward, Rent 1000 with
/// its own October record, Transport budgeted from October only.
fn fill_world() -> (World, Budgets, u32) {
    let mut world = World::new();
    for month in [6, 7, 8] {
        world.post("ANZ Everyday", "Dining", date(2026, month, 4), "-312.00");
        world.post("ANZ Everyday", "Groceries", date(2026, month, 4), "-500.00");
    }
    let (mut budgets, id) = world.budgets();
    let categories = world.categories.clone();
    let jun = Period::of(date(2026, 6, 1));
    let mut set = |name: &str, month: Period, span: Span, amount: &str| {
        budgets
            .set_amount(
                id,
                &categories,
                category_id(&categories, name),
                month,
                span,
                money(amount),
                Some(Rollover::CarryUnspent),
                // Seeding history: every month is open as of June.
                date(2026, 6, 1),
            )
            .expect("a seeded amount");
    };
    set("Dining", jun, Span::Onward, "250.00");
    set("Dining", sep(), Span::MonthOnly, "400.00");
    set("Groceries", jun, Span::Onward, "500.00");
    set("Rent", jun, Span::Onward, "1000.00");
    set("Rent", sep().next(), Span::Onward, "1100.00");
    set("Transport", sep().next(), Span::Onward, "60.00");
    (world, budgets, id)
}

#[test]
fn fill_from_the_previous_month_copies_amounts_and_keeps_edited_cells() {
    let (world, mut budgets, id) = fill_world();
    let october = sep().next();
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let preview = fill_preview(&budget, &world.ledger(), october, FillSource::PreviousMonth);
    // Dining's September Month-only differs from the plan; Groceries matches it; Rent has its
    // own October record; Transport wasn't budgeted in September.
    assert_eq!(
        preview.changes,
        vec![FillLine {
            category_id: world.category("Dining"),
            before: Some(money("250.00")),
            after: money("400.00"),
        }]
    );
    assert_eq!(preview.kept, vec![world.category("Rent")]);
    assert_eq!(preview.unchanged, 1);
    assert_eq!(preview.total, money("2060.00"));

    let wrote = budgets.fill(
        id,
        &world.ledger(),
        october,
        FillSource::PreviousMonth,
        today(),
    );
    assert_eq!(wrote, Ok(1));
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let dining = budget.chain(world.category("Dining"));
    let filled = applied(dining, october).unwrap_or_else(|| unreachable!("filled"));
    assert_eq!(filled.amount, money("400.00"));
    assert_eq!(filled.rollover, Rollover::CarryUnspent);
    // Month-only: November falls back to the Onward amount.
    assert_eq!(
        applied(dining, october.next()).map(|found| found.amount),
        Some(money("250.00"))
    );
    assert_eq!(
        total_budgeted(&budget, &world.categories, october),
        preview.total
    );
    // A second Fill finds every Category kept or unchanged.
    assert_eq!(
        budgets.fill(
            id,
            &world.ledger(),
            october,
            FillSource::PreviousMonth,
            today()
        ),
        Ok(0)
    );
}

#[test]
fn fill_from_the_average_rounds_spent_and_refuses_a_closed_month() {
    let (world, mut budgets, id) = fill_world();
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    // The current month: the three months before it are June to August.
    let preview = fill_preview(&budget, &world.ledger(), sep(), FillSource::Average);
    // Dining has its own September record and Rent has no spend: 1000.00 becomes 0.00.
    assert_eq!(preview.kept, vec![world.category("Dining")]);
    assert_eq!(
        preview.changes,
        vec![FillLine {
            category_id: world.category("Rent"),
            before: Some(money("1000.00")),
            after: money("0.00"),
        }]
    );
    assert_eq!(preview.unchanged, 1, "Groceries averages its 500.00");
    assert_eq!(
        budgets.fill(
            id,
            &world.ledger(),
            sep().prev(),
            FillSource::Average,
            today()
        ),
        Err(BudgetError::ClosedMonth)
    );
}

#[test]
fn an_average_exactly_on_a_five_rounds_away_from_zero() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-45.00");
    // Total 45 over 3 months is 15.00, which rounds up to 20.
    let (budgets, id) = world.budgets();
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    assert_eq!(
        three_month_average(&budget, &world.ledger(), world.category("Dining"), sep()),
        money("20.00")
    );
}

#[test]
fn unallocated_is_average_income_minus_total_budgeted() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Salary", date(2026, 6, 25), "6000.00");
    world.post("ANZ Everyday", "Salary", date(2026, 7, 25), "6000.00");
    world.post("ANZ Everyday", "Salary", date(2026, 8, 25), "5550.00");
    // Off-budget income doesn't count.
    world.post("Amex Platinum", "Salary", date(2026, 8, 26), "3000.00");
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Rent",
        sep(),
        Span::Onward,
        "1000.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "5000.00",
        None,
    );
    let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
    let ledger = world.ledger();
    assert_eq!(average_income(&budget, &ledger, today()), money("5850.00"));
    assert_eq!(
        total_budgeted(&budget, &world.categories, sep()),
        money("6000.00")
    );
    assert_eq!(
        unallocated(&budget, &ledger, sep(), today()),
        money("-150.00")
    );
}

// -- operations ------------------------------------------------------------------------------------

#[test]
fn saving_replaces_the_record_that_starts_in_that_month() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::Onward,
        "300.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::MonthOnly,
        "320.00",
        None,
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(world.category("Dining")).to_vec())
        .unwrap_or_default();
    assert_eq!(chain.len(), 2);
    assert_eq!(amount_in(&chain, sep().next()), Some(money("320.00")));
    assert_eq!(
        amount_in(&chain, sep().next().next()),
        Some(money("250.00"))
    );
}

#[test]
fn a_new_record_inherits_the_rollover_of_the_one_it_follows() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        Some(Rollover::CarryBoth),
    );
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::Onward,
        "300.00",
        None,
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(world.category("Dining")).to_vec())
        .unwrap_or_default();
    assert_eq!(
        applied(&chain, sep().next()).map(|a| a.rollover),
        Some(Rollover::CarryBoth)
    );
    // With nothing before it, it starts at None.
    set(
        &mut budgets,
        &world,
        id,
        "Water",
        sep(),
        Span::Onward,
        "60.00",
        None,
    );
    let water = budgets
        .get(id)
        .map(|b| b.chain(world.category("Water")).to_vec())
        .unwrap_or_default();
    assert_eq!(
        applied(&water, sep()).map(|a| a.rollover),
        Some(Rollover::None)
    );
}

#[test]
fn only_leaf_expense_categories_hold_amounts() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    for name in ["Food", "Salary"] {
        let refused = budgets.set_amount(
            id,
            &world.categories,
            world.category(name),
            sep(),
            Span::Onward,
            money("10.00"),
            None,
            today(),
        );
        assert_eq!(refused, Err(BudgetError::NotAnExpenseLeaf), "{name}");
    }
}

#[test]
fn closed_months_are_read_only_and_amounts_cant_be_negative() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    let dining = world.category("Dining");
    let closed = budgets.set_amount(
        id,
        &world.categories,
        dining,
        sep().prev(),
        Span::Onward,
        money("1.00"),
        None,
        today(),
    );
    assert_eq!(closed, Err(BudgetError::ClosedMonth));
    let negative = budgets.set_amount(
        id,
        &world.categories,
        dining,
        sep(),
        Span::Onward,
        money("-1.00"),
        None,
        today(),
    );
    assert_eq!(negative, Err(BudgetError::NegativeAmount));
    assert_eq!(
        budgets.stop(id, &world.categories, dining, sep().prev(), today()),
        Err(BudgetError::ClosedMonth)
    );
}

#[test]
fn stopping_ends_the_amount_from_that_month_and_keeps_the_past() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    assert_eq!(
        budgets.stop(
            id,
            &world.categories,
            world.category("Dining"),
            sep().next(),
            today()
        ),
        Ok(())
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(world.category("Dining")).to_vec())
        .unwrap_or_default();
    assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep().next()), None);
}

#[test]
fn stopping_a_category_with_no_amount_writes_nothing() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    assert_eq!(
        budgets.stop(
            id,
            &world.categories,
            world.category("Dining"),
            sep(),
            today()
        ),
        Ok(())
    );
    assert_eq!(budgets.get(id).map(|b| b.category_ids().count()), Some(0));
}

#[test]
fn clearing_a_month_only_record_restores_the_carried_onward_value() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    let dining = world.category("Dining");
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::MonthOnly,
        "400.00",
        None,
    );
    assert_eq!(
        budgets.clear_month_only(id, dining, sep().next(), today()),
        Ok(true)
    );
    // The Onward record itself is not a Month-only, so nothing more comes off.
    assert_eq!(
        budgets.clear_month_only(id, dining, sep(), today()),
        Ok(false)
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(dining).to_vec())
        .unwrap_or_default();
    assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
}

#[test]
fn cycling_the_rollover_writes_an_onward_from_the_current_month() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    let dining = world.category("Dining");
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::MonthOnly,
        "250.00",
        None,
    );
    assert_eq!(
        budgets.cycle_rollover(id, dining, today()),
        Ok(Rollover::CarryUnspent)
    );
    assert_eq!(
        budgets.cycle_rollover(id, dining, today()),
        Ok(Rollover::CarryBoth)
    );
    assert_eq!(
        budgets.cycle_rollover(id, dining, today()),
        Ok(Rollover::None)
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(dining).to_vec())
        .unwrap_or_default();
    assert_eq!(chain.len(), 1);
    assert!(matches!(chain[0].limit, Limit::Onward { .. }));
    assert_eq!(
        budgets.cycle_rollover(id, world.category("Water"), today()),
        Err(BudgetError::NotBudgeted)
    );
}

#[test]
fn the_5c_write_is_onward_on_change_stop_on_clear_and_nothing_when_unchanged() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    let dining = world.category("Dining");
    let write = |budgets: &mut Budgets, amount: Option<&str>| {
        budgets.set_monthly_limit(id, &world.categories, dining, amount.map(money), today())
    };
    assert_eq!(write(&mut budgets, None), Ok(false));
    assert_eq!(write(&mut budgets, Some("250.00")), Ok(true));
    assert_eq!(write(&mut budgets, Some("250")), Ok(false));
    assert_eq!(write(&mut budgets, Some("300.00")), Ok(true));
    assert_eq!(
        budgets.monthly_limit(id, &world.categories, dining, today()),
        Some(money("300.00"))
    );
    assert_eq!(write(&mut budgets, None), Ok(true));
    assert_eq!(
        budgets.monthly_limit(id, &world.categories, dining, today()),
        None
    );
}

#[test]
fn a_parents_monthly_limit_is_the_sum_of_its_budgeted_leaves() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "500.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::MonthOnly,
        "300.00",
        None,
    );
    let food = world.category("Food");
    assert_eq!(
        budgets.monthly_limit(id, &world.categories, food, today()),
        Some(money("800.00"))
    );
    assert_eq!(
        budgets.monthly_limit(id, &world.categories, world.category("Housing"), today()),
        None
    );
}

#[test]
fn a_leaf_that_gains_a_child_is_stopped_from_the_current_month() {
    let mut world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Transport",
        sep(),
        Span::Onward,
        "600.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        id,
        "Transport",
        sep().next(),
        Span::Onward,
        "650.00",
        None,
    );
    let transport = world.category("Transport");
    let child = categories::insert_category(
        &mut world.categories,
        "Fuel".to_string(),
        Some(transport),
        CategoryTypes::Expense,
    );
    assert!(child.is_ok());
    assert_eq!(
        budgets.stop_new_parents(&world.categories, today()),
        vec![transport]
    );
    let chain = budgets
        .get(id)
        .map(|b| b.chain(transport).to_vec())
        .unwrap_or_default();
    assert_eq!(amount_in(&chain, sep()), None);
    assert_eq!(amount_in(&chain, sep().next()), None);
    // A second pass finds nothing left to stop.
    assert!(
        budgets
            .stop_new_parents(&world.categories, today())
            .is_empty()
    );
}

#[test]
fn a_deleted_categorys_records_go_with_it_in_every_budget() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    let dining = world.category("Dining");
    assert_eq!(budgets.limit_count(dining), 1);
    budgets.remove_category(dining);
    assert_eq!(budgets.limit_count(dining), 0);
}

// -- containers ----------------------------------------------------------------------------------------

fn new_budget(world: &World, name: &str, start_from: StartFrom) -> NewBudget {
    NewBudget {
        name: name.to_string(),
        unit: "aud".to_string(),
        account_ids: vec![world.account("ANZ Everyday")],
        start_from,
    }
}

#[test]
fn the_first_budget_is_the_default_and_later_ones_are_not() {
    let world = World::new();
    let (mut budgets, first) = world.budgets();
    let second = budgets.create(
        &new_budget(&world, "Second", StartFrom::Empty),
        &world.ledger(),
        today(),
    );
    assert_eq!(budgets.default_budget().map(|b| b.id), Some(first));
    assert_eq!(
        second
            .ok()
            .and_then(|id| budgets.get(id))
            .map(|b| b.is_default),
        Some(false)
    );
}

#[test]
fn a_budget_needs_a_unique_name_and_same_unit_accounts_that_take_transactions() {
    let world = World::new();
    let (mut budgets, _) = world.budgets();
    let ledger = world.ledger();
    let create = |budgets: &mut Budgets, new: NewBudget| budgets.create(&new, &ledger, today());
    assert_eq!(
        create(&mut budgets, new_budget(&world, "  ", StartFrom::Empty)),
        Err(BudgetError::NameRequired)
    );
    assert_eq!(
        create(&mut budgets, new_budget(&world, "test", StartFrom::Empty)),
        Err(BudgetError::NameTaken)
    );
    let mut wrong_unit = new_budget(&world, "Crypto", StartFrom::Empty);
    wrong_unit.account_ids = vec![world.account("Bitcoin")];
    assert_eq!(
        create(&mut budgets, wrong_unit),
        Err(BudgetError::AccountNotInUnit)
    );
    let mut loan = new_budget(&world, "Loans", StartFrom::Empty);
    loan.account_ids = vec![world.account("Home Loan")];
    assert_eq!(
        create(&mut budgets, loan),
        Err(BudgetError::AccountNotInUnit)
    );
    let mut missing = new_budget(&world, "Missing", StartFrom::Empty);
    missing.account_ids = vec![9_999];
    assert_eq!(
        create(&mut budgets, missing),
        Err(BudgetError::AccountNotFound)
    );
}

#[test]
fn start_from_copy_takes_the_sources_current_amounts_and_rollover_as_onward() {
    let world = World::new();
    let (mut budgets, source) = world.budgets();
    set(
        &mut budgets,
        &world,
        source,
        "Dining",
        sep(),
        Span::MonthOnly,
        "250.00",
        Some(Rollover::CarryBoth),
    );
    set(
        &mut budgets,
        &world,
        source,
        "Water",
        sep().next(),
        Span::Onward,
        "60.00",
        None,
    );
    let copy = budgets.create(
        &new_budget(&world, "Copy", StartFrom::Copy(source)),
        &world.ledger(),
        today(),
    );
    let copied = copy
        .ok()
        .and_then(|id| budgets.get(id))
        .cloned()
        .unwrap_or_else(unreachable_budget);
    let dining = copied.chain(world.category("Dining"));
    assert_eq!(amount_in(dining, sep()), Some(money("250.00")));
    assert_eq!(amount_in(dining, sep().next()), Some(money("250.00")));
    assert_eq!(
        applied(dining, sep()).map(|a| a.rollover),
        Some(Rollover::CarryBoth)
    );
    // A future-only amount isn't in the source's current month.
    assert!(copied.chain(world.category("Water")).is_empty());
}

#[test]
fn start_from_last_three_months_leaves_a_zero_average_unbudgeted() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-303.00");
    world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-300.00");
    let (mut budgets, _) = world.budgets();
    let created = budgets.create(
        &new_budget(&world, "Averages", StartFrom::LastThreeMonths),
        &world.ledger(),
        today(),
    );
    let made = created
        .ok()
        .and_then(|id| budgets.get(id))
        .cloned()
        .unwrap_or_else(unreachable_budget);
    // 603 over 3 months is 201, rounded to 200.
    assert_eq!(
        amount_in(made.chain(world.category("Dining")), sep()),
        Some(money("200.00"))
    );
    assert!(made.chain(world.category("Water")).is_empty());
    assert_eq!(
        applied(made.chain(world.category("Dining")), sep()).map(|a| a.rollover),
        Some(Rollover::None)
    );
}

#[test]
fn edit_renames_and_changes_accounts_but_keeps_the_unit() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    let accounts = vec![world.account("Wallet")];
    assert_eq!(
        budgets.edit(id, " Renamed ", accounts.clone(), &world.accounts),
        Ok(())
    );
    let edited = budgets.get(id);
    assert_eq!(edited.map(|b| b.name.as_str()), Some("Renamed"));
    assert_eq!(edited.map(|b| b.account_ids.clone()), Some(accounts));
    assert_eq!(
        budgets.edit(id, "", vec![], &world.accounts),
        Err(BudgetError::NameRequired)
    );
    assert_eq!(
        budgets.edit(id, "X", vec![world.account("Bitcoin")], &world.accounts),
        Err(BudgetError::AccountNotInUnit)
    );
}

#[test]
fn duplicating_copies_the_chains_and_numbers_a_taken_name() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        Some(Rollover::CarryUnspent),
    );
    let first = budgets.duplicate(id).unwrap_or_default();
    let second = budgets.duplicate(id).unwrap_or_default();
    assert_eq!(
        budgets.get(first).map(|b| b.name.as_str()),
        Some("Test copy")
    );
    assert_eq!(
        budgets.get(second).map(|b| b.name.as_str()),
        Some("Test copy 2")
    );
    let copy = budgets
        .get(first)
        .cloned()
        .unwrap_or_else(unreachable_budget);
    assert!(!copy.is_default && !copy.is_archived());
    assert_eq!(copy.unit, "aud");
    assert_eq!(
        copy.chain(world.category("Dining")),
        budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")))
            .unwrap_or_default()
    );
    assert_eq!(budgets.duplicate(99), Err(BudgetError::NotFound));
}

#[test]
fn the_default_cant_be_archived_but_another_can_be_archived_and_restored() {
    let world = World::new();
    let (mut budgets, first) = world.budgets();
    let second = budgets
        .create(
            &new_budget(&world, "Second", StartFrom::Empty),
            &world.ledger(),
            today(),
        )
        .unwrap_or_default();
    assert_eq!(
        budgets.archive(first, today()),
        Err(BudgetError::DefaultCannotBeArchived)
    );
    assert_eq!(budgets.archive(second, today()), Ok(()));
    assert_eq!(
        budgets.get(second).and_then(|b| b.archived_at),
        Some(today())
    );
    assert_eq!(budgets.active().count(), 1);
    assert_eq!(budgets.archived().count(), 1);
    assert_eq!(budgets.restore(second), Ok(()));
    assert_eq!(budgets.active().count(), 2);
}

#[test]
fn set_default_moves_the_flag_and_refuses_an_archived_budget() {
    let world = World::new();
    let (mut budgets, first) = world.budgets();
    let second = budgets
        .create(
            &new_budget(&world, "Second", StartFrom::Empty),
            &world.ledger(),
            today(),
        )
        .unwrap_or_default();
    let third = budgets
        .create(
            &new_budget(&world, "Third", StartFrom::Empty),
            &world.ledger(),
            today(),
        )
        .unwrap_or_default();
    assert_eq!(budgets.archive(third, today()), Ok(()));
    assert_eq!(budgets.set_default(third), Err(BudgetError::Archived));
    assert_eq!(budgets.set_default(second), Ok(()));
    assert_eq!(budgets.all().iter().filter(|b| b.is_default).count(), 1);
    assert_eq!(budgets.default_budget().map(|b| b.id), Some(second));
    // The old default can now be archived.
    assert_eq!(budgets.archive(first, today()), Ok(()));
}

#[test]
fn an_archived_budget_is_read_only_but_still_computed() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-40.00");
    let (mut budgets, first) = world.budgets();
    set(
        &mut budgets,
        &world,
        first,
        "Dining",
        sep(),
        Span::Onward,
        "250.00",
        None,
    );
    let second = budgets
        .create(
            &new_budget(&world, "Second", StartFrom::Empty),
            &world.ledger(),
            today(),
        )
        .unwrap_or_default();
    set(
        &mut budgets,
        &world,
        second,
        "Dining",
        sep(),
        Span::Onward,
        "100.00",
        None,
    );
    assert_eq!(budgets.archive(second, today()), Ok(()));
    let refused = budgets.set_amount(
        second,
        &world.categories,
        world.category("Dining"),
        sep(),
        Span::Onward,
        money("1.00"),
        None,
        today(),
    );
    assert_eq!(refused, Err(BudgetError::Archived));
    assert_eq!(
        budgets.edit(second, "New name", vec![], &world.accounts),
        Err(BudgetError::Archived)
    );
    let archived = budgets
        .get(second)
        .cloned()
        .unwrap_or_else(unreachable_budget);
    let figures = period_figures(&archived, &world.ledger(), sep(), today());
    assert_eq!(figures.budgeted, money("100.00"));
}

#[test]
fn overlapping_budgets_count_the_same_split_independently() {
    let mut world = World::new();
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 4), "-80.00");
    let (mut budgets, first) = world.budgets();
    let second = budgets
        .create(
            &new_budget(&world, "Second", StartFrom::Empty),
            &world.ledger(),
            today(),
        )
        .unwrap_or_default();
    set(
        &mut budgets,
        &world,
        first,
        "Groceries",
        sep(),
        Span::Onward,
        "500.00",
        None,
    );
    set(
        &mut budgets,
        &world,
        second,
        "Groceries",
        sep(),
        Span::Onward,
        "200.00",
        None,
    );
    let ledger = world.ledger();
    for (id, left) in [(first, "420.00"), (second, "120.00")] {
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        assert_eq!(
            period_figures(&budget, &ledger, sep(), today()).left,
            money(left)
        );
    }
}

#[test]
fn cents_money_is_exact() {
    assert_eq!(cents_money(-1850), money("-18.50"));
}

#[test]
fn is_over_budget_reads_a_negative_spend_against_the_amount() {
    assert!(is_over_budget(&money("-318.40"), &money("250.00")));
    assert!(!is_over_budget(&money("-100.00"), &money("250.00")));
}

// -- category detail (9d) -------------------------------------------------------------------

#[test]
fn category_detail_lists_splits_largest_first_and_reads_the_track_record() {
    let mut world = World::new();
    let (budgets, id) = world.budgets();
    let dining = world.category("Dining");
    let Some(mut budget) = budgets.get(id).cloned() else {
        panic!("the Budget was created");
    };
    budget.limits.insert(
        dining,
        vec![LimitRecord {
            month: Period::of(date(2026, 6, 1)),
            limit: Limit::Onward {
                amount: money("250.00"),
                rollover: Rollover::None,
            },
        }],
    );
    // Over in August, under in July.
    world.post("ANZ Everyday", "Dining", date(2026, 8, 5), "-300.00");
    world.post("ANZ Everyday", "Dining", date(2026, 7, 5), "-100.00");
    world.post("ANZ Everyday", "Dining", date(2026, 9, 2), "-40.00");
    world.post("ANZ Everyday", "Dining", date(2026, 9, 9), "-278.40");
    // Off-budget Account: never counted.
    world.post("Amex Platinum", "Dining", date(2026, 9, 9), "-999.00");
    let detail = category_detail(&budget, &world.ledger(), dining, sep(), today())
        .expect("Dining is an Expense Category");

    assert_eq!(detail.figures.spent, money("318.40"));
    assert!(detail.figures.over);
    let amounts: Vec<Money> = detail.lines.iter().map(|l| l.amount.clone()).collect();
    assert_eq!(amounts, vec![money("278.40"), money("40.00")]);
    let track = detail.track.expect("earlier budgeted months exist");
    assert_eq!((track.months, track.over_months), (3, 1));
    assert_eq!(track.average, money("133.33"));
    assert_eq!(detail.rollover, Some(Rollover::None));
}

#[test]
fn category_detail_of_a_parent_rolls_up_and_has_no_rollover() {
    let mut world = World::new();
    let (mut budgets, id) = world.budgets();
    set(
        &mut budgets,
        &world,
        id,
        "Groceries",
        sep(),
        Span::Onward,
        "500.00",
        None,
    );
    world.post("ANZ Everyday", "Groceries", date(2026, 9, 3), "-120.00");
    let Some(budget) = budgets.get(id) else {
        panic!("the Budget was created");
    };
    let parent = categories::path(&world.categories, world.category("Groceries"))
        .and_then(|_| {
            world
                .categories
                .iter()
                .find(|c| {
                    !categories::is_leaf(&world.categories, c.id)
                        && categories::descendants_inclusive(&world.categories, c.id)
                            .contains(&world.category("Groceries"))
                })
                .map(|c| c.id)
        })
        .expect("Groceries sits under a parent");
    let detail = category_detail(budget, &world.ledger(), parent, sep(), today())
        .expect("a parent Expense Category");
    assert!(detail.figures.is_parent);
    assert_eq!(detail.figures.spent, money("120.00"));
    assert_eq!(detail.rollover, None);
}

#[test]
fn category_detail_is_none_for_a_non_expense_category() {
    let world = World::new();
    let (budgets, id) = world.budgets();
    let Some(budget) = budgets.get(id) else {
        panic!("the Budget was created");
    };
    let income = world
        .categories
        .iter()
        .find(|c| c.category_type == CategoryTypes::Income)
        .map(|c| c.id)
        .unwrap_or_default();
    assert_eq!(
        category_detail(budget, &world.ledger(), income, sep(), today()),
        None
    );
}

// The Plan grid (9b)

fn save(
    budgets: &mut Budgets,
    world: &World,
    id: u32,
    category: &str,
    month: Period,
    span: Span,
    text: &str,
) -> Result<bool, BudgetError> {
    budgets.save_cell(
        id,
        &world.categories,
        world.category(category),
        month,
        span,
        text,
        today(),
    )
}

fn chain_of(budgets: &Budgets, world: &World, id: u32, category: &str) -> Vec<LimitRecord> {
    budgets
        .get(id)
        .map(|budget| budget.chain(world.category(category)).to_vec())
        .unwrap_or_default()
}

#[test]
fn saving_a_cell_onward_runs_until_the_next_record() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250"
        ),
        Ok(true)
    );
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "300"
        ),
        Ok(true)
    );
    let chain = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
    assert_eq!(
        amount_in(&chain, sep().next().next()),
        Some(money("300.00"))
    );
}

#[test]
fn saving_a_cell_month_only_leaves_the_next_month_on_the_onward_value() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250",
    )
    .ok();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::MonthOnly,
        "400",
    )
    .ok();
    let chain = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(amount_in(&chain, sep().next()), Some(money("400.00")));
    assert_eq!(
        amount_in(&chain, sep().next().next()),
        Some(money("250.00"))
    );
}

#[test]
fn typing_zero_writes_a_real_zero_and_not_unbudgeted() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(&mut budgets, &world, id, "Dining", sep(), Span::Onward, "0").ok();
    let chain = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(amount_in(&chain, sep()), Some(money("0.00")));
}

#[test]
fn clearing_a_cell_onward_writes_a_stop_from_that_month() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250",
    )
    .ok();
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            ""
        ),
        Ok(true)
    );
    let chain = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
    assert_eq!(amount_in(&chain, sep().next()), None);
    assert_eq!(amount_in(&chain, sep().next().next()), None);
}

#[test]
fn clearing_a_cell_month_only_removes_that_months_own_month_only_record() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250",
    )
    .ok();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep().next(),
        Span::MonthOnly,
        "400",
    )
    .ok();
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            ""
        ),
        Ok(true)
    );
    let chain = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
    // With no Month-only record there, a second clear has nothing to remove.
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            ""
        ),
        Ok(false)
    );
}

#[test]
fn saving_an_unchanged_amount_onward_writes_nothing() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Dining",
        sep(),
        Span::Onward,
        "250",
    )
    .ok();
    let before = chain_of(&budgets, &world, id, "Dining");
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "250.00"
        ),
        Ok(false)
    );
    assert_eq!(chain_of(&budgets, &world, id, "Dining"), before);
}

#[test]
fn a_closed_month_or_a_bad_amount_is_refused() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().prev(),
            Span::Onward,
            "250"
        ),
        Err(BudgetError::ClosedMonth)
    );
    assert_eq!(
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "12x"
        ),
        Err(BudgetError::InvalidAmount)
    );
}

#[test]
fn the_first_key_replaces_the_prefilled_amount_and_later_keys_edit_it() {
    let mut edit = PlanEdit::new(1, sep(), Some(&money("250.00")));
    assert_eq!(edit.text, "250.00");
    edit.type_char('3');
    edit.type_char('x');
    edit.type_char('0');
    edit.type_char('.');
    edit.type_char('.');
    edit.type_char('5');
    assert_eq!(edit.text, "30.5");
    edit.backspace();
    assert_eq!(edit.text, "30.");
}

#[test]
fn backspace_on_a_prefilled_cell_clears_it_at_once() {
    let mut edit = PlanEdit::new(1, sep(), Some(&money("250.00")));
    edit.backspace();
    assert_eq!(edit.text, "");
}

#[test]
fn the_plan_groups_leaves_under_their_parent_and_totals_each_month() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Rent",
        sep(),
        Span::Onward,
        "1000",
    )
    .ok();
    save(
        &mut budgets,
        &world,
        id,
        "Electricity",
        sep(),
        Span::Onward,
        "90",
    )
    .ok();
    save(&mut budgets, &world, id, "Water", sep(), Span::Onward, "60").ok();
    save(
        &mut budgets,
        &world,
        id,
        "Transport",
        sep().next(),
        Span::Onward,
        "40",
    )
    .ok();
    let Some(budget) = budgets.get(id) else {
        panic!("the Budget was created");
    };
    let start = default_plan_start(today());
    let grid = plan(budget, &world.ledger(), start, today());
    assert_eq!(grid.months.len(), PLAN_MONTHS);
    assert_eq!(grid.months.get(2), Some(&sep()));
    let labels: Vec<Option<&str>> = grid
        .sections
        .iter()
        .map(|section| section.parent.as_deref())
        .collect();
    // A top-level leaf (Transport) closes the grid as the unlabelled group.
    assert_eq!(labels, vec![Some("Housing"), Some("Utilities"), None]);
    assert_eq!(grid.row_count(), 4);
    // Sep is the third month; Oct the fourth.
    assert_eq!(grid.totals.get(2), Some(&money("1150.00")));
    assert_eq!(grid.totals.get(3), Some(&money("1190.00")));
    assert_eq!(grid.totals.first(), Some(&money("0")));
    let cell = grid
        .row(0)
        .and_then(|row| row.cells.get(2))
        .expect("Rent's September cell");
    assert!(cell.own_record && !cell.closed);
    assert!(
        grid.row(0)
            .is_some_and(|row| row.cells.first().is_some_and(|c| c.closed))
    );
    // Unallocated is average income less the month's total.
    assert_eq!(
        grid.unallocated.get(2).map(|left| left.0.clone()),
        Some(grid.average_income.0.clone() - money("1150.00").0)
    );
}

#[test]
fn a_cleared_row_stays_in_the_grid_while_a_record_starts_in_the_range() {
    let world = World::new();
    let (mut budgets, id) = world.budgets();
    save(
        &mut budgets,
        &world,
        id,
        "Rent",
        sep().next(),
        Span::Onward,
        "1000",
    )
    .ok();
    save(
        &mut budgets,
        &world,
        id,
        "Rent",
        sep().next(),
        Span::Onward,
        "",
    )
    .ok();
    let Some(budget) = budgets.get(id) else {
        panic!("the Budget was created");
    };
    let grid = plan(
        budget,
        &world.ledger(),
        default_plan_start(today()),
        today(),
    );
    assert_eq!(grid.row_count(), 1);
    assert!(
        grid.row(0)
            .is_some_and(|row| row.cells.iter().all(|c| c.amount.is_none()))
    );
}

#[test]
fn the_plan_range_is_bounded_by_the_first_amount_and_a_year_ahead() {
    let world = World::new();
    let (budgets, id) = world.budgets();
    let Some(budget) = budgets.get(id) else {
        panic!("the Budget was created");
    };
    let (earliest, latest) = plan_start_bounds(budget, today());
    assert_eq!(earliest, default_plan_start(today()));
    assert_eq!(latest, (0..12).fold(earliest, |month, _| month.next()));
}
