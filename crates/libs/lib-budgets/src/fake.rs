//! In-memory ports and the stub rows, for tests and UI-first work. [`InMemoryBudgets`] and
//! [`InMemoryBudgetReads`] hold every row in a plain field so a test can set up state directly;
//! [`stub_accounts`] and [`stub_categories`] are the Accounts and Categories stubs as Budgets reads
//! them, which `lib-ledger`'s seed tests hold to the stubs themselves.

use std::collections::HashMap;

use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, CategoryTypes, PayeeId, UnitId};

use crate::protocols::{
    BudgetAccount, BudgetAccounts, BudgetBill, BudgetBillPlan, BudgetBills, BudgetCategories,
    BudgetCategory, BudgetPayees, BudgetRepository, BudgetSplit, BudgetSplits, BudgetUnits,
};
use crate::{Budget, default_budgets};

/// The Budgets repository over a plain list, in creation order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBudgets {
    pub budgets: Vec<Budget>,
}

impl InMemoryBudgets {
    /// The stub Budgets as of `today`, over the stub Accounts and Categories.
    pub fn stub(today: NaiveDate) -> Self {
        Self {
            budgets: default_budgets(&stub_accounts(), &stub_categories(), today)
                .all()
                .to_vec(),
        }
    }
}

impl BudgetRepository for InMemoryBudgets {
    fn budgets(&self) -> Vec<Budget> {
        self.budgets.clone()
    }

    fn upsert_budget(&mut self, budget: &Budget) {
        match self.budgets.iter_mut().find(|row| row.id == budget.id) {
            Some(row) => *row = budget.clone(),
            None => self.budgets.push(budget.clone()),
        }
    }
}

/// Every Budgets read port over rows a test sets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBudgetReads {
    /// In tree order, as [`BudgetCategories`] returns them.
    pub categories: Vec<BudgetCategory>,
    pub accounts: Vec<BudgetAccount>,
    /// In the Transactions' order.
    pub splits: Vec<BudgetSplit>,
    pub open_bills: Vec<BudgetBill>,
    pub active_bill_plans: Vec<BudgetBillPlan>,
    pub unit_codes: HashMap<UnitId, String>,
    pub payee_names: HashMap<PayeeId, String>,
}

impl InMemoryBudgetReads {
    /// The stub Accounts, Categories and Unit codes, with no Splits or Bills.
    pub fn stub() -> Self {
        Self {
            categories: stub_categories(),
            accounts: stub_accounts(),
            unit_codes: HashMap::from([
                (UnitId::SEED_AUD, "aud".to_string()),
                (UnitId::SEED_VAS, "vas".to_string()),
                (UnitId::SEED_BTC, "btc".to_string()),
            ]),
            ..Self::default()
        }
    }
}

impl BudgetCategories for InMemoryBudgetReads {
    fn budget_categories(&self) -> Vec<BudgetCategory> {
        self.categories.clone()
    }
}

impl BudgetAccounts for InMemoryBudgetReads {
    fn budget_accounts(&self) -> Vec<BudgetAccount> {
        self.accounts.clone()
    }
}

impl BudgetSplits for InMemoryBudgetReads {
    fn budget_splits(&self, from: NaiveDate, to: NaiveDate) -> Vec<BudgetSplit> {
        self.splits
            .iter()
            .filter(|split| (from..=to).contains(&split.date))
            .cloned()
            .collect()
    }
}

impl BudgetBills for InMemoryBudgetReads {
    fn open_bills(&self) -> Vec<BudgetBill> {
        self.open_bills.clone()
    }

    fn active_bill_plans(&self) -> Vec<BudgetBillPlan> {
        self.active_bill_plans.clone()
    }
}

impl BudgetUnits for InMemoryBudgetReads {
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.unit_codes.get(&id).cloned()
    }
}

impl BudgetPayees for InMemoryBudgetReads {
    fn payee_name(&self, id: PayeeId) -> Option<String> {
        self.payee_names.get(&id).cloned()
    }
}

/// The stub Accounts as Budgets reads them: the Accounts stub's ids, names, Units and whether each
/// takes Transactions.
pub fn stub_accounts() -> Vec<BudgetAccount> {
    let account = |n, name: &str, unit, takes_transactions| BudgetAccount {
        id: AccountId::seed(n),
        name: name.to_string(),
        unit,
        takes_transactions,
    };
    vec![
        account(1, "Wallet", UnitId::SEED_AUD, true),
        account(2, "ANZ Everyday", UnitId::SEED_AUD, true),
        account(3, "ANZ Offset", UnitId::SEED_AUD, true),
        account(4, "Amex Platinum", UnitId::SEED_AUD, true),
        account(5, "Home Loan", UnitId::SEED_AUD, false),
        account(6, "Vanguard VAS", UnitId::SEED_VAS, false),
        account(7, "Bitcoin", UnitId::SEED_BTC, false),
    ]
}

/// The stub Categories as Budgets reads them, in tree order.
pub fn stub_categories() -> Vec<BudgetCategory> {
    let category =
        |n, name: &str, path: &str, parent: Option<u32>, is_expense, is_leaf| BudgetCategory {
            id: CategoryId::seed(n),
            name: name.to_string(),
            path: path.to_string(),
            parent: parent.map(CategoryId::seed),
            category_type: if is_expense {
                CategoryTypes::Expense
            } else {
                CategoryTypes::Income
            },
            is_leaf,
        };
    let (expense, income) = (true, false);
    vec![
        category(1, "Housing", "Housing", None, expense, false),
        category(2, "Rent", "Housing \u{203a} Rent", Some(1), expense, true),
        category(
            3,
            "Utilities",
            "Housing \u{203a} Utilities",
            Some(1),
            expense,
            false,
        ),
        category(
            4,
            "Electricity",
            "Housing \u{203a} Utilities \u{203a} Electricity",
            Some(3),
            expense,
            true,
        ),
        category(
            5,
            "Water",
            "Housing \u{203a} Utilities \u{203a} Water",
            Some(3),
            expense,
            true,
        ),
        category(6, "Food", "Food", None, expense, false),
        category(
            7,
            "Groceries",
            "Food \u{203a} Groceries",
            Some(6),
            expense,
            true,
        ),
        category(8, "Dining", "Food \u{203a} Dining", Some(6), expense, true),
        category(9, "Transport", "Transport", None, expense, true),
        category(10, "Household", "Household", None, expense, true),
        category(11, "Salary", "Salary", None, income, true),
        category(12, "Interest", "Interest", None, income, true),
    ]
}
