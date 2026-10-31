//! The contract every stored [`BudgetRepository`] keeps (ADR-0046), run against both the in-memory
//! repository here and the `lib-database` adapter (`adapter/budgets.rs`), so the fakes the
//! Presenter tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Budgets rules, as a Client does, reloads as a restart would, and expects the same rows back.
//! The other domains' rows the rules read are the stub's ([`InMemoryBudgetReads::stub`]).

use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, Period, UnitId};

use crate::fake::{InMemoryBudgetReads, InMemoryBudgets};
use crate::{
    Budget, BudgetError, BudgetReads, BudgetRepository, Budgets, Limit, LimitRecord, NewBudget,
    PERSONAL_SPENDING_ID, Rollover, Span, StartFrom, cents_money,
};

const JOINT_HOUSEHOLD: BudgetId = BudgetId::seed(2);
const GROCERIES: CategoryId = CategoryId::seed(7);
const DINING: CategoryId = CategoryId::seed(8);
const RENT: CategoryId = CategoryId::seed(2);
const ANZ_EVERYDAY: AccountId = AccountId::seed(2);

/// The day every check runs on.
pub fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 21).expect("a real date")
}

/// The stub Budgets as of [`today`], which every check starts from.
pub fn stub_budgets() -> Vec<Budget> {
    InMemoryBudgets::stub(today()).budgets
}

/// Builds the repository a contract check runs on.
pub trait PortsFactory {
    type Ports: BudgetRepository;

    /// A repository holding `seed`'s Budgets, as if stored and loaded, over the stub's Units,
    /// Accounts and Categories.
    fn create(&mut self, seed: Vec<Budget>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh repository from wherever they were
    /// stored, as a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Deletes Account `id` from storage, as the Accounts domain's delete does.
    fn delete_account(&mut self, ports: &mut Self::Ports, id: AccountId);

    /// Deletes Category `id` from storage, as Category reassign's delete does, and returns whether
    /// storage let it.
    fn delete_category(&mut self, ports: &mut Self::Ports, id: CategoryId) -> bool;

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Budgets domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

/// The in-memory repository is its own storage, so a reload hands it back as it is. It keeps a
/// copy as of the last commit, which a failed write falls back to, and stands in for the file's
/// keys: an Account delete takes the Account from every Budget, and a Category a chain names can't
/// be deleted.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryBudgets>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryBudgets;

    fn create(&mut self, seed: Vec<Budget>) -> InMemoryBudgets {
        let ports = InMemoryBudgets { budgets: seed };
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryBudgets) -> InMemoryBudgets {
        self.commit(&mut ports);
        self.committed.clone().expect("create ran first")
    }

    fn commit(&mut self, ports: &mut InMemoryBudgets) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    /// Another domain's write, not a Budgets one, so it lands in the committed copy too.
    fn delete_account(&mut self, ports: &mut InMemoryBudgets, id: AccountId) {
        let drop_account = |budgets: &mut InMemoryBudgets| {
            for budget in &mut budgets.budgets {
                budget.account_ids.retain(|account| *account != id);
            }
        };
        drop_account(ports);
        if let Some(committed) = self.committed.as_mut() {
            drop_account(committed);
        }
    }

    fn delete_category(&mut self, _ports: &mut InMemoryBudgets, id: CategoryId) -> bool {
        let committed = self.committed.as_ref().expect("create ran first");
        !committed
            .budgets
            .iter()
            .any(|budget| !budget.chain(id).is_empty())
    }

    fn break_writes(&mut self, _ports: &mut InMemoryBudgets) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryBudgets) -> (InMemoryBudgets, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

/// The Budgets a repository holds, in creation order, without an empty chain: storage keeps a
/// Category's records, not that it once had a chain with none.
pub fn snapshot(ports: &impl BudgetRepository) -> Vec<Budget> {
    let mut budgets = ports.budgets();
    for budget in &mut budgets {
        budget.limits.retain(|_, chain| !chain.is_empty());
    }
    budgets
}

/// Runs `rule` on the Budgets `ports` holds and writes each Budget it changed, as an Interactor
/// does.
fn write<R>(
    ports: &mut impl BudgetRepository,
    rule: impl FnOnce(&mut Budgets, &dyn BudgetReads) -> Result<R, BudgetError>,
) -> R {
    let before = Budgets::from_stored(ports.budgets());
    let mut after = before.clone();
    let outcome = rule(&mut after, &InMemoryBudgetReads::stub()).expect("the rules allow it");
    for budget in after.changed_since(&before) {
        ports.upsert_budget(&budget);
    }
    outcome
}

fn budget(ports: &impl BudgetRepository, id: BudgetId) -> Budget {
    ports.budget(id).expect("the Budget is stored")
}

fn defaults(ports: &impl BudgetRepository) -> Vec<BudgetId> {
    ports
        .budgets()
        .iter()
        .filter(|budget| budget.is_default)
        .map(|budget| budget.id)
        .collect()
}

fn current() -> Period {
    Period::of(today())
}

/// Writes every kind of change through the rules, reloads, and gets the same rows: a new Budget
/// copied from another, a Month-only amount with a Rollover, a Stop, a cycled Rollover and an
/// archived Budget.
pub fn written_rows_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());

    let holiday = write(&mut ports, |budgets, reads| {
        budgets.create(
            &NewBudget {
                name: "Holiday".into(),
                unit: UnitId::SEED_AUD,
                account_ids: vec![AccountId::seed(1), AccountId::seed(4)],
                start_from: StartFrom::Copy(PERSONAL_SPENDING_ID),
            },
            reads,
            today(),
        )
    });
    factory.commit(&mut ports);
    write(&mut ports, |budgets, reads| {
        budgets.set_amount(
            PERSONAL_SPENDING_ID,
            reads,
            GROCERIES,
            current().next(),
            Span::MonthOnly,
            cents_money(12_345),
            Some(Rollover::CarryBoth),
            today(),
        )
    });
    factory.commit(&mut ports);
    write(&mut ports, |budgets, reads| {
        budgets.stop(
            PERSONAL_SPENDING_ID,
            reads,
            DINING,
            current().shift(2),
            today(),
        )
    });
    factory.commit(&mut ports);
    let rollover = write(&mut ports, |budgets, _| {
        budgets.cycle_rollover(PERSONAL_SPENDING_ID, RENT, today())
    });
    factory.commit(&mut ports);
    write(&mut ports, |budgets, _| {
        budgets.archive(JOINT_HOUSEHOLD, today())
    });
    factory.commit(&mut ports);

    let before = snapshot(&ports);
    let ports = factory.reload(ports);
    assert_eq!(snapshot(&ports), before);

    let holiday = budget(&ports, holiday);
    assert_eq!(holiday.name, "Holiday");
    assert_eq!(
        holiday.account_ids,
        [AccountId::seed(1), AccountId::seed(4)]
    );
    assert!(!holiday.is_default);
    let personal = budget(&ports, PERSONAL_SPENDING_ID);
    assert_eq!(
        personal.chain(GROCERIES).last(),
        Some(&LimitRecord {
            month: current().next(),
            limit: Limit::MonthOnly {
                amount: cents_money(12_345),
                rollover: Rollover::CarryBoth,
            },
        })
    );
    assert_eq!(
        personal.chain(DINING).last().map(|record| &record.limit),
        Some(&Limit::Stop)
    );
    assert_eq!(rollover, Rollover::CarryUnspent);
    assert_eq!(
        budget(&ports, JOINT_HOUSEHOLD).archived_at,
        Some(today()),
        "archived, not gone"
    );
}

/// A write replaces a Budget's chains whole: a record or a chain it no longer holds is gone after a
/// reload.
pub fn a_chain_is_replaced_whole<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());
    let mut personal = budget(&ports, PERSONAL_SPENDING_ID);
    let records = [
        (
            current(),
            Limit::Onward {
                amount: cents_money(40_000),
                rollover: Rollover::None,
            },
        ),
        (
            current().next(),
            Limit::MonthOnly {
                amount: cents_money(0),
                rollover: Rollover::CarryUnspent,
            },
        ),
        (current().shift(2), Limit::Stop),
    ];
    let chain = personal.limits.entry(GROCERIES).or_default();
    for (month, limit) in records {
        crate::put(chain, LimitRecord { month, limit });
    }
    ports.upsert_budget(&personal);
    let mut ports = factory.reload(ports);
    assert_eq!(budget(&ports, PERSONAL_SPENDING_ID), personal);

    let groceries = vec![LimitRecord {
        month: current(),
        limit: Limit::Onward {
            amount: cents_money(45_000),
            rollover: Rollover::None,
        },
    }];
    personal.limits.insert(GROCERIES, groceries.clone());
    personal.limits.remove(&DINING);
    ports.upsert_budget(&personal);
    let ports = factory.reload(ports);

    let reloaded = budget(&ports, PERSONAL_SPENDING_ID);
    assert_eq!(reloaded.chain(GROCERIES), groceries.as_slice());
    assert!(
        reloaded.chain(DINING).is_empty(),
        "the dropped chain is gone"
    );
    assert_eq!(reloaded, personal);
}

/// Storage holds one default: moving it to another Budget and back leaves exactly one each time.
pub fn one_budget_is_the_default<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());
    assert_eq!(defaults(&ports), [PERSONAL_SPENDING_ID]);

    write(&mut ports, |budgets, _| {
        budgets.set_default(JOINT_HOUSEHOLD)
    });
    let mut ports = factory.reload(ports);
    assert_eq!(defaults(&ports), [JOINT_HOUSEHOLD]);

    write(&mut ports, |budgets, _| {
        budgets.set_default(PERSONAL_SPENDING_ID)
    });
    let ports = factory.reload(ports);
    assert_eq!(defaults(&ports), [PERSONAL_SPENDING_ID]);
}

/// Deleting an Account takes it from every Budget that counted it, once the Budgets reload; the
/// rest of each Budget stays.
pub fn a_deleted_account_leaves_every_budget<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());
    let before = snapshot(&ports);
    assert!(
        before
            .iter()
            .all(|budget| budget.account_ids.contains(&ANZ_EVERYDAY)),
        "every stub Budget counts ANZ Everyday"
    );

    factory.delete_account(&mut ports, ANZ_EVERYDAY);
    let ports = factory.reload(ports);

    let mut expected = before;
    for budget in &mut expected {
        budget.account_ids.retain(|id| *id != ANZ_EVERYDAY);
    }
    assert_eq!(snapshot(&ports), expected);
}

/// A Category a chain names can't be deleted from storage; once no chain names it, it can.
pub fn a_category_a_chain_names_cannot_be_deleted<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());
    assert!(
        !factory.delete_category(&mut ports, GROCERIES),
        "both stub Budgets budget Groceries"
    );

    write(&mut ports, |budgets, _| {
        budgets.remove_category(GROCERIES);
        Ok(())
    });
    factory.commit(&mut ports);
    assert!(factory.delete_category(&mut ports, GROCERIES));
    let ports = factory.reload(ports);
    assert!(
        ports
            .budgets()
            .iter()
            .all(|budget| budget.chain(GROCERIES).is_empty())
    );
}

/// A write storage refuses is reported and reloads to the last committed rows.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(stub_budgets());
    let before = snapshot(&ports);

    factory.break_writes(&mut ports);
    write(&mut ports, |budgets, reads| {
        budgets.edit(JOINT_HOUSEHOLD, "Shared house", vec![ANZ_EVERYDAY], reads)
    });
    assert_eq!(budget(&ports, JOINT_HOUSEHOLD).name, "Shared house");
    factory.commit(&mut ports);
    let (ports, failures) = factory.recover(ports);

    assert_eq!(failures, 1, "the failure is reported once");
    assert_eq!(snapshot(&ports), before);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_written_rows_reload_the_same() {
        written_rows_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_chain_is_replaced_whole() {
        a_chain_is_replaced_whole(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_one_budget_is_the_default() {
        one_budget_is_the_default(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_deleted_account_leaves_every_budget() {
        a_deleted_account_leaves_every_budget(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_category_a_chain_names_cannot_be_deleted() {
        a_category_a_chain_names_cannot_be_deleted(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
