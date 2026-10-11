//! PROTOTYPE (#593), throwaway: named `Ledger` commands for the two cross-entity deletes, to test
//! ADR-0038's shape against real rules. Lives on `prototype/ledger-commands` only; never merged.
//!
//! Each command checks everything first and writes only once nothing can refuse, so a refusal
//! leaves the Ledger untouched. On success it records `LedgerChange`s (ADR-0039), and the domains
//! a store announces in `LedgerChanged { what }` are derived from those changes, not declared.

use std::collections::BTreeSet;

use lib_bills::BillPlan;
use lib_budgets::Budget;
use lib_categories::Category;
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::Ledger;

/// The domains a command can touch: what `LedgerChanged { what }` carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    Payees,
    Categories,
    Transactions,
    Bills,
    Budgets,
}

/// One row a command wrote, as the repository will receive it (ADR-0039).
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerChange {
    UpsertPayee(Payee),
    DeletePayee(u32),
    UpsertCategory(Category),
    DeleteCategory(u32),
    UpsertTransaction(Transaction),
    UpsertBillPlan(BillPlan),
    UpsertBudget(Budget),
}

impl LedgerChange {
    pub fn domain(&self) -> Domain {
        match self {
            Self::UpsertPayee(_) | Self::DeletePayee(_) => Domain::Payees,
            Self::UpsertCategory(_) | Self::DeleteCategory(_) => Domain::Categories,
            Self::UpsertTransaction(_) => Domain::Transactions,
            Self::UpsertBillPlan(_) => Domain::Bills,
            Self::UpsertBudget(_) => Domain::Budgets,
        }
    }
}

/// The domains a batch of changes touched, deduplicated: `LedgerChanged { what }`.
pub fn domains(changes: &[LedgerChange]) -> BTreeSet<Domain> {
    changes.iter().map(LedgerChange::domain).collect()
}

/// Why `delete_payee` refused. Each variant maps to one Message in the Client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePayeeRefusal {
    NotFound,
    /// Referenced rows survive a delete, so the Payee can only be deactivated.
    InUse {
        splits: usize,
        bill_plans: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetPayeeActiveRefusal {
    NotFound,
}

/// What a Payee delete would meet: the Delete Dialog reads this to pick Delete or Deactivate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayeeUse {
    pub splits: usize,
    pub bill_plans: usize,
}

impl PayeeUse {
    pub fn is_used(self) -> bool {
        self.splits > 0 || self.bill_plans > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteCategoryRefusal {
    NotFound,
    /// Only leaves are deleted; a parent's children go first.
    HasChildren,
    /// Uncategorised is where a deleted Category's rows go, so it can't be deleted while used.
    UncategorisedInUse,
}

/// What a Category delete did, for the Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDeleted {
    pub name: String,
    pub splits_moved: usize,
    pub bill_plans_moved: usize,
    pub payee_defaults_cleared: usize,
    pub budgets_touched: usize,
}

/// A command as a value, so a Dialog's `command()` can return one and the Dialog host's `Apply`
/// can run it without knowing which (ADR-0037). Each runs through its `Ledger` method.
pub trait LedgerCommand {
    type Outcome;
    type Refusal;
    fn run(self, ledger: &mut Ledger) -> Result<Self::Outcome, Self::Refusal>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePayee(pub u32);

impl LedgerCommand for DeletePayee {
    type Outcome = ();
    type Refusal = DeletePayeeRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), DeletePayeeRefusal> {
        ledger.delete_payee(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPayeeActive {
    pub id: u32,
    pub is_active: bool,
}

impl LedgerCommand for SetPayeeActive {
    type Outcome = ();
    type Refusal = SetPayeeActiveRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), SetPayeeActiveRefusal> {
        ledger.set_payee_active(self.id, self.is_active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCategory(pub u32);

impl LedgerCommand for DeleteCategory {
    type Outcome = CategoryDeleted;
    type Refusal = DeleteCategoryRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<CategoryDeleted, DeleteCategoryRefusal> {
        ledger.delete_category(self.0)
    }
}

impl Ledger {
    /// Everything that references Payee `id`.
    pub fn payee_use(&self, id: u32) -> PayeeUse {
        PayeeUse {
            splits: self
                .transactions
                .iter()
                .flat_map(|t| &t.splits)
                .filter(|s| s.payee_id == Some(id))
                .count(),
            bill_plans: self
                .bill_plans
                .iter()
                .filter(|p| p.payee_id == Some(id))
                .count(),
        }
    }

    /// Hard-deletes Payee `id` and its aliases, refused while a Split or a Bill Plan names it.
    pub fn delete_payee(&mut self, id: u32) -> Result<(), DeletePayeeRefusal> {
        if !self.payees.iter().any(|p| p.id == id) {
            return Err(DeletePayeeRefusal::NotFound);
        }
        let usage = self.payee_use(id);
        if usage.is_used() {
            return Err(DeletePayeeRefusal::InUse {
                splits: usage.splits,
                bill_plans: usage.bill_plans,
            });
        }
        self.payees.retain(|p| p.id != id);
        self.pending.push(LedgerChange::DeletePayee(id));
        Ok(())
    }

    /// Deactivates or reactivates Payee `id`. Single-domain, so it is here only to record the
    /// change; under ADR-0038 the rule is `Payee::set_active` in `lib-payees`.
    pub fn set_payee_active(
        &mut self,
        id: u32,
        is_active: bool,
    ) -> Result<(), SetPayeeActiveRefusal> {
        let payee = self
            .payees
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(SetPayeeActiveRefusal::NotFound)?;
        if payee.is_active != is_active {
            payee.is_active = is_active;
            self.pending.push(LedgerChange::UpsertPayee(payee.clone()));
        }
        Ok(())
    }

    /// Deletes leaf Category `id`. Its Splits and Bill Plans move to the Uncategorised Category of
    /// its type (created only if something moves), Payees that default to it lose the default,
    /// and every Budget drops its Category Limits.
    pub fn delete_category(&mut self, id: u32) -> Result<CategoryDeleted, DeleteCategoryRefusal> {
        let category = lib_categories::get(&self.categories, id)
            .ok_or(DeleteCategoryRefusal::NotFound)?
            .clone();
        if !lib_categories::is_leaf(&self.categories, id) {
            return Err(DeleteCategoryRefusal::HasChildren);
        }
        let splits_using = |id: u32| {
            self.transactions
                .iter()
                .flat_map(|t| &t.splits)
                .filter(|s| s.category_id == id)
                .count()
        };
        let plans_using = |id: u32| {
            self.bill_plans
                .iter()
                .filter(|p| p.category_id == id)
                .count()
        };
        let needs_home = splits_using(id) + plans_using(id) > 0;
        // Today's flow re-points Uncategorised's Splits onto itself and then deletes it, leaving
        // them dangling.
        let is_uncategorised = category.name == "Uncategorised" && category.parent.is_none();
        if is_uncategorised && needs_home {
            return Err(DeleteCategoryRefusal::UncategorisedInUse);
        }

        // Nothing can refuse past here.
        let mut changes = Vec::new();
        let home = if needs_home {
            let before = self.categories.len();
            let home = lib_categories::get_or_create_uncategorised(
                &mut self.categories,
                category.category_type.clone(),
            );
            if self.categories.len() > before
                && let Some(created) = lib_categories::get(&self.categories, home)
            {
                changes.push(LedgerChange::UpsertCategory(created.clone()));
            }
            Some(home)
        } else {
            None
        };

        let mut splits_moved = 0;
        let mut bill_plans_moved = 0;
        if let Some(home) = home {
            for transaction in &mut self.transactions {
                let mut touched = false;
                for split in transaction
                    .splits
                    .iter_mut()
                    .filter(|s| s.category_id == id)
                {
                    split.category_id = home;
                    splits_moved += 1;
                    touched = true;
                }
                if touched {
                    changes.push(LedgerChange::UpsertTransaction(transaction.clone()));
                }
            }
            for plan in self.bill_plans.iter_mut().filter(|p| p.category_id == id) {
                plan.category_id = home;
                bill_plans_moved += 1;
                changes.push(LedgerChange::UpsertBillPlan(plan.clone()));
            }
        }

        let mut payee_defaults_cleared = 0;
        for payee in self
            .payees
            .iter_mut()
            .filter(|p| p.default_category == Some(id))
        {
            payee.default_category = None;
            payee_defaults_cleared += 1;
            changes.push(LedgerChange::UpsertPayee(payee.clone()));
        }

        // `Budgets::remove_category` doesn't say which Budgets it touched, so diff ids first.
        let budgeted: Vec<u32> = self
            .budgets
            .all()
            .iter()
            .filter(|b| b.limits.contains_key(&id))
            .map(|b| b.id)
            .collect();
        self.budgets.remove_category(id);
        for budget in budgeted.iter().filter_map(|id| self.budgets.get(*id)) {
            changes.push(LedgerChange::UpsertBudget(budget.clone()));
        }

        self.categories.retain(|c| c.id != id);
        changes.push(LedgerChange::DeleteCategory(id));
        self.pending.extend(changes);

        Ok(CategoryDeleted {
            name: category.name,
            splits_moved,
            bill_plans_moved,
            payee_defaults_cleared,
            budgets_touched: budgeted.len(),
        })
    }

    /// The changes recorded since the last drain, for the store to hand to the repository.
    pub fn drain_changes(&mut self) -> Vec<LedgerChange> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lib_core::CategoryTypes;

    fn ledger() -> Ledger {
        Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date"))
    }

    fn payee_id(ledger: &Ledger, name: &str) -> u32 {
        lib_payees::find_by_name(ledger.payees(), name).expect("a seeded Payee")
    }

    fn category_id(ledger: &Ledger, name: &str) -> u32 {
        lib_categories::find_by_name(ledger.categories(), name).expect("a seeded Category")
    }

    fn snapshot(ledger: &Ledger) -> String {
        format!("{ledger:?}")
    }

    #[test]
    fn an_unused_payee_is_deleted_and_the_change_recorded() {
        let mut ledger = ledger();
        // Every stub Payee is named somewhere, so add one nothing names.
        let id = lib_payees::insert_payee(
            &mut ledger.payees,
            &lib_payees::PayeeDraft {
                name: "Unused".into(),
                ..Default::default()
            },
        )
        .expect("a fresh name");

        ledger.delete_payee(id).expect("unused");

        assert!(lib_payees::get(ledger.payees(), id).is_none());
        let changes = ledger.drain_changes();
        assert_eq!(changes, vec![LedgerChange::DeletePayee(id)]);
        assert_eq!(domains(&changes), BTreeSet::from([Domain::Payees]));
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_payee_on_splits_is_refused_with_counts_and_nothing_changes() {
        let mut ledger = ledger();
        let id = payee_id(&ledger, "Woolworths");
        let before = snapshot(&ledger);

        let refusal = ledger.delete_payee(id).unwrap_err();

        assert!(matches!(refusal, DeletePayeeRefusal::InUse { splits, .. } if splits > 0));
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    /// Today's rule checks only Splits, so the Desktop hard-deletes Ray White Rentals and leaves
    /// its Bill Plan naming a Payee that no longer exists. The Ledger sees both.
    #[test]
    fn a_payee_only_a_bill_plan_names_is_refused() {
        let mut ledger = ledger();
        let id = payee_id(&ledger, "Ray White Rentals");
        assert!(!lib_payees::is_referenced(ledger.transactions(), id));

        assert_eq!(
            ledger.delete_payee(id),
            Err(DeletePayeeRefusal::InUse {
                splits: 0,
                bill_plans: 1
            })
        );
    }

    #[test]
    fn deactivating_records_the_payee_once_and_a_no_op_records_nothing() {
        let mut ledger = ledger();
        let id = payee_id(&ledger, "Woolworths");

        ledger.set_payee_active(id, false).unwrap();
        ledger.set_payee_active(id, false).unwrap();

        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], LedgerChange::UpsertPayee(p) if !p.is_active));
    }

    #[test]
    fn deleting_a_category_repoints_splits_and_plans_and_clears_defaults_and_limits() {
        let mut ledger = ledger();
        let groceries = category_id(&ledger, "Groceries");
        let splits_before = ledger
            .transactions()
            .iter()
            .flat_map(|t| &t.splits)
            .filter(|s| s.category_id == groceries)
            .count();
        assert!(splits_before > 0);

        let deleted = ledger.delete_category(groceries).expect("a used leaf");

        assert_eq!(deleted.splits_moved, splits_before);
        let uncategorised = ledger
            .categories()
            .iter()
            .find(|c| c.name == "Uncategorised" && c.category_type == CategoryTypes::Expense)
            .expect("created for the moved rows")
            .id;
        // Nothing still names the deleted Category, anywhere.
        let all_splits = ledger.transactions().iter().flat_map(|t| &t.splits);
        assert!(all_splits.clone().all(|s| s.category_id != groceries));
        assert!(
            ledger
                .bill_plans()
                .iter()
                .all(|p| p.category_id != groceries)
        );
        assert!(
            ledger
                .payees()
                .iter()
                .all(|p| p.default_category != Some(groceries))
        );
        assert!(
            ledger
                .budgets()
                .all()
                .iter()
                .all(|b| !b.limits.contains_key(&groceries))
        );
        assert_eq!(
            all_splits
                .filter(|s| s.category_id == uncategorised)
                .count(),
            splits_before
        );
        assert!(
            deleted.payee_defaults_cleared > 0,
            "Woolworths defaults to Groceries"
        );

        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::DeleteCategory(groceries)));
        assert!(
            changes
                .iter()
                .any(|c| matches!(c, LedgerChange::UpsertCategory(c) if c.id == uncategorised))
        );
        let what = domains(&changes);
        assert!(what.contains(&Domain::Categories));
        assert!(what.contains(&Domain::Transactions));
        assert!(what.contains(&Domain::Payees));
    }

    #[test]
    fn every_change_matches_the_row_now_in_the_ledger() {
        let mut ledger = ledger();
        ledger
            .delete_category(category_id(&ledger, "Electricity"))
            .unwrap();
        for change in ledger.drain_changes() {
            match change {
                LedgerChange::UpsertTransaction(t) => {
                    assert!(ledger.transactions().contains(&t))
                }
                LedgerChange::UpsertBillPlan(p) => assert!(ledger.bill_plans().contains(&p)),
                LedgerChange::UpsertPayee(p) => assert!(ledger.payees().contains(&p)),
                LedgerChange::UpsertCategory(c) => assert!(ledger.categories().contains(&c)),
                LedgerChange::UpsertBudget(b) => {
                    assert_eq!(ledger.budgets().get(b.id), Some(&b))
                }
                LedgerChange::DeleteCategory(id) => {
                    assert!(lib_categories::get(ledger.categories(), id).is_none())
                }
                LedgerChange::DeletePayee(id) => {
                    assert!(lib_payees::get(ledger.payees(), id).is_none())
                }
            }
        }
    }

    #[test]
    fn an_unused_leaf_is_deleted_without_creating_uncategorised() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let id = lib_categories::insert_category(
            &mut ledger.categories,
            "Snacks".into(),
            Some(food),
            CategoryTypes::Expense,
        )
        .unwrap();
        let count = ledger.categories().len();

        ledger.delete_category(id).unwrap();

        assert_eq!(ledger.categories().len(), count - 1);
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::DeleteCategory(id)]
        );
    }

    #[test]
    fn a_parent_is_refused_and_nothing_changes() {
        let mut ledger = ledger();
        let before = snapshot(&ledger);
        assert_eq!(
            ledger.delete_category(category_id(&ledger, "Food")),
            Err(DeleteCategoryRefusal::HasChildren)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn uncategorised_in_use_is_refused_rather_than_left_dangling() {
        let mut ledger = ledger();
        ledger
            .delete_category(category_id(&ledger, "Dining"))
            .unwrap();
        ledger.drain_changes();
        let uncategorised = category_id(&ledger, "Uncategorised");

        assert_eq!(
            ledger.delete_category(uncategorised),
            Err(DeleteCategoryRefusal::UncategorisedInUse)
        );
    }
}
