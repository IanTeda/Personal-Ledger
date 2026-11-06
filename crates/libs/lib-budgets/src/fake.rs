//! In-memory ports and the sample rows, for tests and UI-first work. [`InMemoryBudgets`] and
//! [`InMemoryBudgetReads`] hold every row in a plain field so a test can set up state directly;
//! [`sample_accounts`] and [`sample_categories`] are the Accounts and Categories samples as Budgets reads
//! them, which `lib-database`'s seed tests hold to the samples themselves. [`InMemoryPorts`] holds both
//! as the Presenters share them, and [`RecordingRouter`] records what a Presenter asked for.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, CategoryTypes, PayeeId, Period, UnitId};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogRouter;

use crate::protocols::{
    BudgetAccount, BudgetAccounts, BudgetBill, BudgetBillPlan, BudgetBills, BudgetCategories,
    BudgetCategory, BudgetPayees, BudgetReads, BudgetRepository, BudgetSplit, BudgetSplits,
    BudgetUnits, BudgetsOutcome, BudgetsPorts, BudgetsRouter, ExportHistory, PortsMut, SharedPorts,
};
use crate::{Budget, sample_budgets};

/// The Budgets repository over a plain list, in creation order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBudgets {
    pub budgets: Vec<Budget>,
}

impl InMemoryBudgets {
    /// The sample Budgets as of `today`, over the sample Accounts and Categories.
    pub fn sample(today: NaiveDate) -> Self {
        Self {
            budgets: sample_budgets(&sample_accounts(), &sample_categories(), today)
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
    /// The sample Accounts, Categories and Unit codes, with no Splits or Bills.
    pub fn sample() -> Self {
        Self {
            categories: sample_categories(),
            accounts: sample_accounts(),
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

/// The Budgets and every read port in memory, as the Presenters share them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub budgets: InMemoryBudgets,
    pub reads: InMemoryBudgetReads,
}

impl InMemoryPorts {
    /// The sample Budgets as of `today`, over the sample reads.
    pub fn sample(today: NaiveDate) -> Self {
        Self {
            budgets: InMemoryBudgets::sample(today),
            reads: InMemoryBudgetReads::sample(),
        }
    }

    /// The ports shared, with the handle a test reads them back through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl BudgetsPorts for InMemoryPorts {
    fn budgets(&self) -> &dyn BudgetRepository {
        &self.budgets
    }

    fn reads(&self) -> &dyn BudgetReads {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            budgets: &mut self.budgets,
            reads: &self.reads,
        }
    }
}

/// What a Presenter asked its Router for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    Switcher(BudgetId),
    Manage(BudgetId),
    NewBudget(BudgetId),
    EditBudget(BudgetId),
    Limit(BudgetId, CategoryId, Period),
    LimitPicker(BudgetId),
    Stop(BudgetId, CategoryId, Period),
    Fill(BudgetId, Period),
    Detail(BudgetId, CategoryId, Period),
    Export(ExportHistory),
    BillsPeriod(Period),
    Transactions(BudgetId, CategoryId, Period),
    Closed(Option<BudgetsOutcome>),
    Notice(Notice),
}

/// A Router that records every request and opens every Dialog it is asked for, unless told not
/// to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
    /// Refuses to open any Dialog, as a Client does on a Category with nothing to edit.
    pub refuse: bool,
}

impl RecordingRouter {
    /// The last status-line message the Presenter set, if any.
    pub fn status(&self) -> Option<&str> {
        self.routed.iter().rev().find_map(|routed| match routed {
            Routed::Notice(Notice::Status(text)) => Some(text.as_str()),
            _ => None,
        })
    }

    fn open(&mut self, routed: Routed) -> bool {
        self.routed.push(routed);
        !self.refuse
    }
}

impl BudgetsRouter for RecordingRouter {
    fn open_switcher(&mut self, current: BudgetId) -> bool {
        self.open(Routed::Switcher(current))
    }

    fn open_manage(&mut self, current: BudgetId) -> bool {
        self.open(Routed::Manage(current))
    }

    fn open_new_budget(&mut self, current: BudgetId) -> bool {
        self.open(Routed::NewBudget(current))
    }

    fn open_edit_budget(&mut self, id: BudgetId) -> bool {
        self.open(Routed::EditBudget(id))
    }

    fn open_limit(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        self.open(Routed::Limit(budget, category, month))
    }

    fn open_limit_picker(&mut self, budget: BudgetId) -> bool {
        self.open(Routed::LimitPicker(budget))
    }

    fn open_stop(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        self.open(Routed::Stop(budget, category, month))
    }

    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool {
        self.open(Routed::Fill(budget, month))
    }

    fn open_detail(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        self.open(Routed::Detail(budget, category, month))
    }

    fn export_history(&mut self, export: ExportHistory) {
        self.routed.push(Routed::Export(export));
    }

    fn show_bills_period(&mut self, period: Period) {
        self.routed.push(Routed::BillsPeriod(period));
    }

    fn show_transactions(&mut self, budget: BudgetId, category: CategoryId, month: Period) {
        self.routed
            .push(Routed::Transactions(budget, category, month));
    }
}

/// A Dialog's close records as the page Router's does, so one recording reads either.
impl DialogRouter<BudgetsOutcome> for RecordingRouter {
    fn close(&mut self, outcome: Option<BudgetsOutcome>) {
        self.routed.push(Routed::Closed(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

/// The sample Accounts as Budgets reads them: the Accounts sample's ids, names, Units and whether each
/// takes Transactions.
pub fn sample_accounts() -> Vec<BudgetAccount> {
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

/// The sample Categories as Budgets reads them, in tree order.
pub fn sample_categories() -> Vec<BudgetCategory> {
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
