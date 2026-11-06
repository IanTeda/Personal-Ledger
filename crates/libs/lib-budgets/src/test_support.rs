//! The fixtures the Budgets model's tests share: a small hand-built Ledger over the sample Accounts
//! and Categories (`World`), so figures are exact, as of 21 September 2026, and the chain and
//! Budget builders the tests across the Entity and the calculation modules use.

pub(crate) use std::collections::BTreeMap;

pub(crate) use chrono::NaiveDate;
pub(crate) use lib_core::{AccountId, BudgetId, CategoryId, CategoryTypes, Money, Period, UnitId};

pub(crate) use crate::fake::InMemoryBudgetReads;
pub(crate) use crate::*;

pub(crate) fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap_or_default()
}

pub(crate) fn today() -> NaiveDate {
    date(2026, 9, 21)
}

pub(crate) fn sep() -> Period {
    Period::of(today())
}

pub(crate) fn money(text: &str) -> Money {
    text.parse().expect("a valid test amount")
}

pub(crate) fn category_id(categories: &[BudgetCategory], name: &str) -> CategoryId {
    tree::find_by_name(categories, name).expect("a seeded Category")
}

/// A tiny hand-built Ledger over the sample Accounts and Categories, so figures are exact rather
/// than random.
pub(crate) struct World {
    pub(crate) reads: InMemoryBudgetReads,
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            reads: InMemoryBudgetReads::sample(),
        }
    }

    pub(crate) fn ledger(&self) -> &InMemoryBudgetReads {
        &self.reads
    }

    pub(crate) fn account(&self, name: &str) -> AccountId {
        self.reads
            .accounts
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id)
            .expect("a seeded Account")
    }

    pub(crate) fn category(&self, name: &str) -> CategoryId {
        category_id(&self.reads.categories, name)
    }

    /// A single-Split Transaction; `amount` follows the ledger's sign (expenses negative).
    pub(crate) fn post(&mut self, account: &str, category: &str, on: NaiveDate, amount: &str) {
        let split = BudgetSplit {
            date: on,
            account_id: self.account(account),
            category_id: self.category(category),
            amount: money(amount),
            payee_id: None,
        };
        self.reads.splits.push(split);
    }

    /// A one-off Bill Plan, active, with its one open entry due `due`.
    pub(crate) fn bill(&mut self, account: &str, category: &str, due: NaiveDate, amount: &str) {
        let (account_id, category_id) = (self.account(account), self.category(category));
        self.reads.open_bills.push(BudgetBill {
            due,
            category_id,
            account_id,
            amount: money(amount),
        });
        self.reads.active_bill_plans.push(BudgetBillPlan {
            name: "A bill plan".into(),
            category_id,
            account_id,
        });
    }

    /// Nests a new Expense leaf `name` under `parent`, which stops being a leaf.
    pub(crate) fn add_child(&mut self, name: &str, parent: CategoryId) {
        let categories = &mut self.reads.categories;
        let Some(at) = categories.iter().position(|c| c.id == parent) else {
            panic!("a seeded parent Category");
        };
        categories[at].is_leaf = false;
        let path = format!("{} \u{203a} {name}", categories[at].path);
        categories.insert(
            at + 1,
            BudgetCategory {
                id: CategoryId::new(),
                name: name.to_string(),
                path,
                parent: Some(parent),
                category_type: CategoryTypes::Expense,
                is_leaf: true,
            },
        );
    }

    /// An empty Budget on the two ANZ accounts.
    pub(crate) fn budgets(&self) -> (Budgets, BudgetId) {
        let mut budgets = Budgets::default();
        let id = budgets
            .create(
                &NewBudget {
                    name: "Test".to_string(),
                    unit: UnitId::SEED_AUD,
                    account_ids: vec![self.account("ANZ Everyday"), self.account("ANZ Offset")],
                    start_from: StartFrom::Empty,
                },
                self.ledger(),
                today(),
            )
            .expect("a valid test Budget");
        (budgets, id)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a test shorthand for one set_amount call"
)]
pub(crate) fn set(
    budgets: &mut Budgets,
    world: &World,
    id: BudgetId,
    category: &str,
    month: Period,
    span: Span,
    amount: &str,
    rollover: Option<Rollover>,
) {
    let done = budgets.set_amount(
        id,
        world.ledger(),
        world.category(category),
        month,
        span,
        money(amount),
        rollover,
        today(),
    );
    assert_eq!(done, Ok(()));
}

pub(crate) fn row(figures: &PeriodFigures, category_id: CategoryId) -> Option<&CategoryFigures> {
    figures.rows.iter().find(|r| r.category_id == category_id)
}

pub(crate) fn onward(month: Period, amount: &str) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::Onward {
            amount: money(amount),
            rollover: Rollover::None,
        },
    }
}

pub(crate) fn month_only(month: Period, amount: &str) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::MonthOnly {
            amount: money(amount),
            rollover: Rollover::None,
        },
    }
}

pub(crate) fn stop(month: Period) -> LimitRecord {
    LimitRecord {
        month,
        limit: Limit::Stop,
    }
}

pub(crate) fn amount_in(chain: &[LimitRecord], month: Period) -> Option<Money> {
    applied(chain, month).map(|a| a.amount)
}

pub(crate) fn unreachable_budget() -> Budget {
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

/// A Budget whose Electricity chain starts in July with the given Rollover.
pub(crate) fn electricity_budget(world: &World, rollover: Rollover) -> Budget {
    let (budgets, id) = world.budgets();
    let mut budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
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

pub(crate) fn carried_into_september(world: &World, rollover: Rollover) -> Option<Money> {
    let budget = electricity_budget(world, rollover);
    effective_budget(
        &budget,
        world.ledger(),
        world.category("Electricity"),
        sep(),
        today(),
    )
    .map(|e| e.carried_in)
}

/// Dining budgeted 250 and Groceries 500 from September; Dining overspent by 68.40.
pub(crate) fn september() -> (World, Budgets, BudgetId) {
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

pub(crate) fn history_world() -> (World, Budget) {
    let mut world = World::new();
    // June under, July over, August under, September so far over.
    world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
    world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-350.00");
    world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-200.00");
    world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-400.00");
    let (budgets, id) = world.budgets();
    let mut budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
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

/// Dining 250 Onward with a Month-only 400 in September, Groceries 500 Onward, Rent 1000 with
/// its own October record, Transport budgeted from October only.
pub(crate) fn fill_world() -> (World, Budgets, BudgetId) {
    let mut world = World::new();
    for month in [6, 7, 8] {
        world.post("ANZ Everyday", "Dining", date(2026, month, 4), "-312.00");
        world.post("ANZ Everyday", "Groceries", date(2026, month, 4), "-500.00");
    }
    let (mut budgets, id) = world.budgets();
    let reads = world.reads.clone();
    let categories = &reads.categories;
    let jun = Period::of(date(2026, 6, 1));
    let mut set = |name: &str, month: Period, span: Span, amount: &str| {
        budgets
            .set_amount(
                id,
                &reads,
                category_id(categories, name),
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

pub(crate) fn new_budget(world: &World, name: &str, start_from: StartFrom) -> NewBudget {
    NewBudget {
        name: name.to_string(),
        unit: UnitId::SEED_AUD,
        account_ids: vec![world.account("ANZ Everyday")],
        start_from,
    }
}

pub(crate) fn save(
    budgets: &mut Budgets,
    world: &World,
    id: BudgetId,
    category: &str,
    month: Period,
    span: Span,
    text: &str,
) -> Result<bool> {
    budgets.save_cell(
        id,
        world.ledger(),
        world.category(category),
        month,
        span,
        text,
        today(),
    )
}

pub(crate) fn chain_of(
    budgets: &Budgets,
    world: &World,
    id: BudgetId,
    category: &str,
) -> Vec<LimitRecord> {
    budgets
        .get(id)
        .map(|budget| budget.chain(world.category(category)).to_vec())
        .unwrap_or_default()
}
