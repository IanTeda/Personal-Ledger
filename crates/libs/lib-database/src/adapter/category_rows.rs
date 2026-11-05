//! The rows that name a Category, answered once over the Transactions, Bills and Payees stores
//! (#1034): the Splits filed under it, the Bill Plans naming it and the Payees defaulting to it.
//!
//! Category delete (the [`CategoryDeletionAdapter`]'s reads), the first-child handover
//! (`lib-categories`' `CategoryHandover`, which this answers, writes included), the Dialogs'
//! holdings and the type-change blockers' Bill Plans all select through here, so a rule one gains
//! (skipping a superseded Bill Plan, say) is gained by all four, and the counts a Dialog quotes
//! stay the rows a confirm writes. The selection itself is `lib-category-deletion`'s
//! `protocols` functions, which its in-memory port answers with too; the Splits count is the
//! Transactions store's own, counted in the file (#926), by the same rule.
//!
//! The handover is a Categories port write over the shared stores, not a cross-domain module's
//! atomic port: it runs only inside `send`, whose one commit lands its writes with the Category
//! save, as `CategoryBudget`'s do (#1010).
//!
//! [`CategoryDeletionAdapter`]: crate::CategoryDeletionAdapter

use lib_bills::BillPlan;
use lib_categories::{CategoryHandover, Holdings};
use lib_category_deletion::protocols::{
    bill_plans_using, payees_defaulting_to, transactions_using,
};
use lib_core::CategoryId;
use lib_payees::Payee;
use lib_transactions::Transaction;

use super::store::{SharedBillsStore, SharedPayeesStore, SharedTransactionsStore};

/// The rows naming a Category in the stores the Transactions, Bills and Payees modules also hold,
/// each borrowed for the one read or write.
#[derive(Clone)]
pub struct StoredCategoryRows {
    pub transactions: SharedTransactionsStore,
    pub bills: SharedBillsStore,
    pub payees: SharedPayeesStore,
}

impl StoredCategoryRows {
    /// The Transactions with a Split naming Category `id`, in the store's order.
    pub fn transactions_using(&self, id: CategoryId) -> Vec<Transaction> {
        transactions_using(self.transactions.borrow().all_transactions(), id)
    }

    /// The Bill Plans naming Category `id`, active or not.
    pub fn bill_plans_using(&self, id: CategoryId) -> Vec<BillPlan> {
        bill_plans_using(self.bills.borrow().plans().bill_plans(), id)
    }

    /// The Payees whose Default category is `id`.
    pub fn payees_defaulting_to(&self, id: CategoryId) -> Vec<Payee> {
        payees_defaulting_to(self.payees.borrow().payees().payees(), id)
    }

    /// The Splits filed under Category `id`, as the Transactions store counts them.
    pub fn splits(&self, id: CategoryId) -> usize {
        self.transactions.borrow().category_splits(id)
    }

    /// How many Bill Plans, active or not, name any of `categories`: the Bills half of what keeps
    /// a Category's type from changing. A Bill Plan names one Category, so the sum over distinct
    /// ids counts each once.
    pub fn bill_plans_naming(&self, categories: &[CategoryId]) -> usize {
        let mut categories = categories.to_vec();
        categories.sort_unstable();
        categories.dedup();
        categories
            .into_iter()
            .map(|id| self.bill_plans_using(id).len())
            .sum()
    }
}

impl CategoryHandover for StoredCategoryRows {
    fn holdings(&self, category: CategoryId) -> Holdings {
        Holdings {
            splits: self.splits(category),
            bill_plans: self.bill_plans_using(category).len(),
            payee_defaults: self.payees_defaulting_to(category).len(),
        }
    }

    /// Re-points the rows [`Self::holdings`] counted, each written whole to its store; the Client
    /// commits them with the save that asked.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        let transactions = self.transactions_using(from);
        let bill_plans = self.bill_plans_using(from);
        let payees = self.payees_defaulting_to(from);
        {
            let mut store = self.transactions.borrow_mut();
            for mut transaction in transactions {
                for split in &mut transaction.splits {
                    if split.category_id == from {
                        split.category_id = to;
                    }
                }
                store.upsert_transaction(transaction);
            }
        }
        {
            let mut store = self.bills.borrow_mut();
            for mut plan in bill_plans {
                plan.category_id = to;
                store.plans_mut().upsert_bill_plan(plan);
            }
        }
        let mut store = self.payees.borrow_mut();
        for mut payee in payees {
            payee.default_category = Some(to);
            store.payees_mut().upsert_payee(payee);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_category_deletion::CategoryDeletion as _;
    use lib_category_deletion::fake::world::{
        DINING, GROCERIES, GROCERIES_AND_DINING, GROCERY_BOX, LEASE, RENT, WATER, world,
    };

    use super::*;

    /// The world `lib-category-deletion`'s tests delete from, in the stores a Client shares.
    fn rows() -> StoredCategoryRows {
        let world = world();
        StoredCategoryRows {
            transactions: Rc::new(RefCell::new(world.transactions)),
            bills: Rc::new(RefCell::new(world.bills)),
            payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts {
                payees: world.payees,
                ..Default::default()
            })),
        }
    }

    /// The handover's notice counts the rows the delete would move: one selection.
    #[test]
    fn holdings_count_the_rows_the_delete_reads() {
        let deletion = world();

        let held = rows().holdings(GROCERIES);

        assert_eq!(
            held,
            Holdings {
                splits: 2,
                bill_plans: 1,
                payee_defaults: deletion.payees_defaulting_to(GROCERIES).len(),
            }
        );
        assert!(held.payee_defaults > 0, "the world's Payees default to it");
        assert_eq!(held.bill_plans, deletion.bill_plans_using(GROCERIES).len());
    }

    #[test]
    fn the_handover_moves_every_row_naming_the_category_and_no_other() {
        let mut rows = rows();
        let dining_before = rows.holdings(DINING);
        let held = rows.holdings(GROCERIES);

        rows.hand_over(GROCERIES, DINING);

        assert_eq!(rows.holdings(GROCERIES), Holdings::default());
        assert_eq!(
            rows.holdings(DINING),
            Holdings {
                splits: dining_before.splits + held.splits,
                bill_plans: dining_before.bill_plans + held.bill_plans,
                payee_defaults: dining_before.payee_defaults + held.payee_defaults,
            }
        );
        let shared = rows
            .transactions
            .borrow()
            .all_transactions()
            .into_iter()
            .find(|transaction| transaction.id == GROCERIES_AND_DINING)
            .expect("in the world");
        assert!(
            shared
                .splits
                .iter()
                .all(|split| split.category_id == DINING)
        );
        let plans = rows.bills.borrow().plans().bill_plans();
        let category_of = |id| {
            plans
                .iter()
                .find(|plan| plan.id == id)
                .map(|plan| plan.category_id)
        };
        assert_eq!(category_of(GROCERY_BOX), Some(DINING));
        assert_eq!(category_of(LEASE), Some(RENT), "the Lease names Rent");
    }

    #[test]
    fn a_category_holding_nothing_hands_over_nothing() {
        let mut rows = rows();
        let before = rows.transactions.borrow().all_transactions();

        rows.hand_over(WATER, DINING);

        assert_eq!(rows.transactions.borrow().all_transactions(), before);
    }

    /// The type-change blockers count each Bill Plan naming the subtree once.
    #[test]
    fn bill_plans_naming_a_subtree_count_each_plan_once() {
        let rows = rows();

        assert_eq!(rows.bill_plans_naming(&[GROCERIES, RENT, WATER]), 2);
        assert_eq!(rows.bill_plans_naming(&[GROCERIES, GROCERIES]), 1);
        assert_eq!(rows.bill_plans_naming(&[WATER]), 0);
    }
}
