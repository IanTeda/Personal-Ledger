//! The Categories ports as the Desktop holds them (#858, #859): the store `shell::router::build` is
//! given, shared by the Categories module, Category delete (`crate::category_deletion`, #1005), the
//! Bills ports (`BillCategories`) and the Payees ports and the Import's
//! Category select (`PayeeCategories`, `ImportCategories`). The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! The Categories page writes the repository through `lib-categories`' Interactors. What the
//! module reaches of other domains goes through [`LedgerPorts`]: the usage its Dialogs quote (the
//! Splits from the Transactions store, #926, the Budgets from the Budgets store, #952), the
//! Monthly budget the Add and Edit Dialogs read and save in the Personal spending Budget, on the
//! Budgets store (#952), and the handover of a leaf's rows to its first child, written to the
//! Transactions, Bills and Payees stores ([`LedgerPorts`]'s `hand_over`). What names a Category,
//! the holdings, the handover and the type-change blockers' Bill Plans, is `lib-database`'s
//! `StoredCategoryRows`, the one selection Category delete reads too (#1034). The Open Ledger
//! hands the page these stores as [`CategoryStores`] (#1057). The Ledger keeps no read copy of the
//! Categories (#1055): each write to the store is counted ([`CategoriesWrites`]), and [`refresh`]
//! announces `Domain::Categories` once per counted write.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::announcer::Domain;
use chrono::{Datelike as _, NaiveDate};
use gpui::{App, Entity};
use lib_budgets::{Budgets, Error};
use lib_categories::{
    Category, CategoryBudget, CategoryHandover, CategoryRepository, CategoryUsage, CategoryUse,
    Holdings, MonthlyBudget, MonthlyBudgetRefusal, ParentStop, SharedCategories,
    TypeChangeBlockers,
};
use lib_core::{CategoryId, Clock, Money};
use lib_database::{
    CategoriesStore, SharedBudgetsStore, SharedCategoriesStore, StoredCategoryRows,
};

use crate::announcer::Announcer;
use crate::budgets::PERSONAL_SPENDING_ID;
use crate::write_count::WriteCount;

/// The Categories store with each write counted, whoever makes it (the Categories page, Category
/// delete), so an input's announcement needs no copy to compare against.
struct CountedCategories {
    store: Box<dyn CategoriesStore>,
    writes: WriteCount,
}

impl CategoryRepository for CountedCategories {
    fn categories(&self) -> Vec<Category> {
        self.store.categories().categories()
    }

    fn upsert_category(&mut self, category: Category) {
        self.writes.bump();
        self.store.categories_mut().upsert_category(category);
    }
}

impl CategoriesStore for CountedCategories {
    fn categories(&self) -> &dyn CategoryRepository {
        self
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        self
    }

    fn delete_category(&mut self, id: CategoryId) {
        self.writes.bump();
        self.store.delete_category(id);
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// The writes made to the Categories store, and the count the last announcement saw, so each
/// write is announced once. A clone shares both.
#[derive(Clone, Default)]
pub struct CategoriesWrites {
    writes: WriteCount,
    announced: Rc<Cell<u64>>,
}

impl CategoriesWrites {
    /// Whether the store was written since the last ask.
    fn take_written(&self) -> bool {
        let writes = self.writes.get();
        writes != self.announced.replace(writes)
    }
}

/// `store` shared, with the count of the writes made to it.
pub fn counted(store: Box<dyn CategoriesStore>) -> (SharedCategoriesStore, CategoriesWrites) {
    let writes = CategoriesWrites::default();
    let shared = Rc::new(RefCell::new(CountedCategories {
        store,
        writes: writes.writes.clone(),
    }));
    (shared, writes)
}

/// The stub Categories in memory, with the count of the writes made to them, for a test that
/// needs no storage.
pub fn in_memory_counted() -> (SharedCategoriesStore, CategoriesWrites) {
    counted(Box::new(lib_categories::fake::InMemoryCategories {
        categories: lib_categories::default_categories(),
    }))
}

/// The stub Categories in memory, for a test that needs no storage and no count.
pub fn in_memory() -> SharedCategoriesStore {
    in_memory_counted().0
}

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreCategories(SharedCategoriesStore);

impl CategoryRepository for StoreCategories {
    fn categories(&self) -> Vec<Category> {
        self.0.borrow().categories().categories()
    }

    fn upsert_category(&mut self, category: Category) {
        self.0
            .borrow_mut()
            .categories_mut()
            .upsert_category(category);
    }
}

/// `store` as the Categories module's Presenters share it.
pub fn shared(store: &SharedCategoriesStore) -> SharedCategories {
    Rc::new(RefCell::new(StoreCategories(store.clone())))
}

/// Announces `Domain::Categories` when the store was written since the last announcement. Run
/// before and after every Categories event and Dialog confirm, so Transactions, Budgets, Import and
/// the Dashboard see a write at once.
pub fn refresh(writes: &CategoriesWrites, announcer: &Entity<Announcer>, cx: &mut App) {
    if writes.take_written() {
        announce(announcer, cx);
    }
}

/// Announces `Domain::Categories`, so every reader redraws.
fn announce(announcer: &Entity<Announcer>, cx: &mut App) {
    announcer.update(cx, |announcer, cx| {
        announcer.announce(Domain::Categories, cx);
    });
}

/// Puts the Categories back as stored, after a write that named them failed, and announces
/// `Domain::Categories` so every reader redraws (ADR-0046).
///
/// # Errors
/// Returns the error reading them back; the store is left as it was.
pub fn reload(
    stores: &CategoryStores,
    announcer: &Entity<Announcer>,
    cx: &mut App,
) -> lib_database::Result<()> {
    stores.categories.borrow_mut().reload()?;
    // The announcement below covers any write not yet announced.
    stores.writes.take_written();
    announce(announcer, cx);
    Ok(())
}

/// What the Categories ports reach beyond the page's own repository: the Categories store and its
/// write count, the Budgets store and the rows naming a Category in the Transactions, Bills and
/// Payees stores. The Open Ledger builds it once ([`crate::shell::router::LedgerStores`]).
#[derive(Clone)]
pub struct CategoryStores {
    pub categories: SharedCategoriesStore,
    pub writes: CategoriesWrites,
    pub budgets: SharedBudgetsStore,
    pub rows: StoredCategoryRows,
}

impl CategoryStores {
    /// The Categories store's tree as the Budgets rules read it.
    fn budget_categories(&self) -> Vec<lib_budgets::BudgetCategory> {
        use lib_budgets::BudgetCategories as _;
        lib_database::StoredBudgetCategories {
            categories: self.categories.clone(),
        }
        .budget_categories()
    }
}

/// The Categories ports the Desktop answers from its stores, built for one call around its gpui
/// context: what names a Category, the Monthly budget in the Personal spending Budget as of
/// `today` (the day the page was built), and the handover, each write announced through
/// `announcer`.
pub struct LedgerPorts<'a> {
    pub stores: &'a CategoryStores,
    pub announcer: &'a Entity<Announcer>,
    pub today: NaiveDate,
    /// The Shell's Clock, read on each call by `type_change_blockers` so a month turn is seen.
    /// #783 moves the calls reading `today` to it, and `today` goes.
    pub clock: Clock,
    pub cx: &'a mut App,
}

impl LedgerPorts<'_> {
    fn announce(&mut self, domain: Domain) {
        self.announcer
            .update(self.cx, |announcer, cx| announcer.announce(domain, cx));
    }

    /// The Budgets as the Budgets store holds them.
    fn budgets(&self) -> Budgets {
        Budgets::from_stored(self.stores.budgets.borrow().budgets().budgets())
    }
}

impl CategoryUsage for LedgerPorts<'_> {
    /// The Splits counted by the Transactions store (#926), and the Budgets holding a Category
    /// Limit for it by the Budgets store (#952).
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        CategoryUse {
            splits: self.stores.rows.splits(id),
            budgets: self.stores.budgets.borrow().category_budgets(id),
        }
    }

    /// The active Budgets using the subtree this month or later, and every Bill Plan naming it.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        TypeChangeBlockers {
            budgets: self.budgets().using(subtree, self.clock.today()),
            bill_plans: self.stores.rows.bill_plans_naming(subtree),
        }
    }
}

/// What a refused Budget write means to the Categories Dialogs.
fn budget_refusal(error: Error) -> MonthlyBudgetRefusal {
    match error {
        Error::NotAnExpenseLeaf => MonthlyBudgetRefusal::NotAnExpenseLeaf,
        Error::Archived | Error::ClosedMonth => MonthlyBudgetRefusal::ReadOnly,
        _ => MonthlyBudgetRefusal::Unavailable,
    }
}

impl CategoryBudget for LedgerPorts<'_> {
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget {
        let budgets = self.budgets();
        MonthlyBudget {
            amount: category.and_then(|id| {
                budgets.monthly_limit(
                    PERSONAL_SPENDING_ID,
                    &lib_database::budget_categories(categories),
                    id,
                    self.today,
                )
            }),
            archived: budgets
                .get(PERSONAL_SPENDING_ID)
                .filter(|budget| budget.is_archived())
                .map(|budget| budget.name.clone()),
        }
    }

    /// Runs the save on a copy of the Budgets, so the check is the Budgets' own rules.
    fn check_monthly_budget(
        &self,
        categories: &[Category],
        category: CategoryId,
        amount: Option<&Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let mut budgets = self.budgets();
        budgets
            .set_monthly_limit(
                PERSONAL_SPENDING_ID,
                &lib_database::budget_categories(categories),
                category,
                amount.cloned(),
                self.today,
            )
            .map(|_| ())
            .map_err(budget_refusal)
    }

    /// Saves the amount to the Budgets store on the Categories store's tree, which holds the
    /// Category just written, then announces the Budgets.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        refresh(&self.stores.writes, self.announcer, self.cx);
        let saved = crate::budgets::ports::set_monthly_limit(
            &mut *self.stores.budgets.borrow_mut(),
            &self.stores.budget_categories(),
            PERSONAL_SPENDING_ID,
            category,
            amount,
            self.today,
        );
        self.announce(Domain::Budgets);
        saved.map(|_| ()).map_err(budget_refusal)
    }

    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop> {
        let budgets = self.budgets().budgeting(category, self.today);
        (budgets > 0).then_some(ParentStop {
            budgets,
            month: self.today.month(),
        })
    }

    /// Stops the parent in every active Budget in the Budgets store, in the input's one commit,
    /// then announces the Budgets.
    fn stop_new_parent(&mut self, parent: CategoryId) {
        crate::budgets::ports::stop_new_parent(
            &mut *self.stores.budgets.borrow_mut(),
            parent,
            self.today,
        );
        self.announce(Domain::Budgets);
    }
}

impl CategoryHandover for LedgerPorts<'_> {
    fn holdings(&self, category: CategoryId) -> Holdings {
        self.stores.rows.holdings(category)
    }

    /// Re-points the rows in the Transactions, Bills and Payees stores, in the input's one commit,
    /// then announces the three.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        self.stores.rows.clone().hand_over(from, to);
        for domain in [Domain::Transactions, Domain::Bills, Domain::Payees] {
            self.announce(domain);
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[test]
    fn a_write_to_the_store_is_counted_whoever_makes_it() {
        let (store, writes) = in_memory_counted();
        assert!(!writes.take_written(), "nothing written yet");
        let first = store.borrow().categories().categories()[0].clone();

        store
            .borrow_mut()
            .categories_mut()
            .upsert_category(first.clone());
        assert!(writes.take_written());
        assert!(!writes.take_written(), "a write is announced once");

        store.borrow_mut().delete_category(first.id);
        assert!(writes.take_written());
        assert!(store.borrow().categories().category(first.id).is_none());
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_budget_port_checks_on_a_copy_and_saves_through_the_ledger(cx: &mut TestAppContext) {
        let stores = crate::shell::router::LedgerStores::build(
            lib_database::Stores::fake(lib_database::FakeStores::sample(today())).0,
        )
        .stores
        .category_stores();
        let announcer = cx.new(|_| Announcer::default());
        let categories = lib_categories::default_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let salary = lib_categories::find_by_name(&categories, "Salary").expect("seeded");
        let amount: Money = "612".parse().expect("an amount");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                stores: &stores,
                announcer: &announcer,
                today: today(),
                clock: Clock::fixed(today()),
                cx,
            };
            assert_eq!(
                ports.check_monthly_budget(&categories, salary, Some(&amount)),
                Err(MonthlyBudgetRefusal::NotAnExpenseLeaf)
            );
            assert_eq!(
                ports.check_monthly_budget(&categories, groceries, Some(&amount)),
                Ok(())
            );
            assert_ne!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                Some(amount.clone()),
                "a check writes nothing"
            );
            ports
                .set_monthly_budget(groceries, Some(amount.clone()))
                .expect("a leaf Expense");
            assert_eq!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                Some(amount)
            );
        });
    }

    #[gpui::test]
    fn a_new_parent_is_stopped_in_every_active_budget_through_the_store(cx: &mut TestAppContext) {
        let stores = crate::shell::router::LedgerStores::build(
            lib_database::Stores::fake(lib_database::FakeStores::sample(today())).0,
        )
        .stores
        .category_stores();
        let announcer = cx.new(|_| Announcer::default());
        let categories = lib_categories::default_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                stores: &stores,
                announcer: &announcer,
                today: today(),
                clock: Clock::fixed(today()),
                cx,
            };
            let stop = ports.parent_stop(groceries).expect("budgeted in the seed");
            assert_eq!((stop.budgets, stop.month), (2, 10));
            assert!(
                ports
                    .monthly_budget(&categories, Some(groceries))
                    .amount
                    .is_some()
            );

            ports.stop_new_parent(groceries);

            assert_eq!(ports.parent_stop(groceries), None);
            assert_eq!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                None
            );
        });
    }

    #[gpui::test]
    fn the_handover_port_moves_splits_to_the_new_child_through_the_stores(cx: &mut TestAppContext) {
        let stores = crate::shell::router::LedgerStores::build(
            lib_database::Stores::fake(lib_database::FakeStores::sample(today())).0,
        )
        .stores
        .category_stores();
        let announcer = cx.new(|_| Announcer::default());
        let categories = lib_categories::default_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let child = lib_categories::find_by_name(&categories, "Dining")
            .expect("another seeded leaf stands in for a new child");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                stores: &stores,
                announcer: &announcer,
                today: today(),
                clock: Clock::fixed(today()),
                cx,
            };
            let held = ports.holdings(groceries);
            assert!(held.splits > 0, "the stub buys groceries");
            let child_before = ports.holdings(child);

            ports.hand_over(groceries, child);

            assert_eq!(ports.holdings(groceries), Holdings::default());
            assert_eq!(
                ports.holdings(child).splits,
                child_before.splits + held.splits
            );
        });
    }

    #[gpui::test]
    fn type_change_blockers_judge_the_budgets_by_the_clock_after_a_month_turn(
        cx: &mut TestAppContext,
    ) {
        let stores = crate::shell::router::LedgerStores::build(
            lib_database::Stores::fake(lib_database::FakeStores::sample(today())).0,
        )
        .stores
        .category_stores();
        let announcer = cx.new(|_| Announcer::default());
        let categories = lib_categories::default_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let next_month = NaiveDate::from_ymd_opt(2026, 11, 1).expect("a real date");
        let (clock, day) = Clock::settable(today());

        cx.update(|cx| {
            // Stop Groceries from next month in every active Budget.
            LedgerPorts {
                stores: &stores,
                announcer: &announcer,
                today: next_month,
                clock: Clock::fixed(next_month),
                cx,
            }
            .stop_new_parent(groceries);

            // The page was built in October and never rebuilt: only the Clock moves.
            let ports = LedgerPorts {
                stores: &stores,
                announcer: &announcer,
                today: today(),
                clock,
                cx,
            };
            assert!(ports.type_change_blockers(&[groceries]).budgets > 0);

            day.set(next_month);

            assert_eq!(ports.type_change_blockers(&[groceries]).budgets, 0);
        });
    }
}
