//! The Budgets Dialogs still on the legacy host (#955): the Budget form, Edit budget, Stop
//! budgeting, Fill and the Category detail. The page's Router opens them when its Presenter asks,
//! and `Root` opens them from each other (9d's **Edit budget**, 9e's Stop link). Each builder only
//! reads, from the Budgets store and the other domains' stores, and answers `None` when there is
//! nothing to open: an archived Budget is read-only, and a parent Category only rolls up.

use chrono::NaiveDate;
use lib_budgets::{
    Budget, BudgetReads, BudgetRepository, CategoryDetail, FillSource, applied, category_detail,
    unbudgeted_leaves,
};
use lib_categories::Category;
use lib_core::{BudgetId, CategoryId, CategoryTypes};

use super::form::BudgetForm;
use super::limit_form::{self, LimitForm, LimitOptions, StopForm};
use super::ports::DesktopPorts;
use super::{BudgetsDialog, Detail};
use crate::period::Period;

/// The legacy Dialogs' builders over the Budgets ports, as of `today`.
pub struct Legacy<'a> {
    pub ports: &'a DesktopPorts,
    pub today: NaiveDate,
}

impl Legacy<'_> {
    /// Budget `id`, or `None` once it has gone.
    pub fn budget(&self, id: BudgetId) -> Option<Budget> {
        self.ports.shared.borrow().budgets.budget(id)
    }

    /// Budget `id` when it takes edits: an archived one is read-only.
    pub fn editable(&self, id: BudgetId) -> Option<Budget> {
        self.budget(id).filter(|budget| !budget.is_archived())
    }

    /// The Categories as their store holds them (#952).
    pub fn categories(&self) -> Vec<Category> {
        self.ports
            .reads()
            .categories
            .borrow()
            .categories()
            .categories()
    }

    /// The Accounts and the Units the Budget form offers, as their stores hold them (#952).
    fn accounts_and_units(&self) -> (Vec<lib_accounts::Account>, Vec<lib_units::UnitRow>) {
        let reads = self.ports.reads();
        let accounts = reads.accounts.borrow().accounts().accounts();
        let units = reads.units.borrow().units().units();
        (accounts, units)
    }

    /// The Starting / From options for `budget`: `count` months from the current one.
    fn limit_options(&self, budget: &Budget, count: usize) -> LimitOptions {
        LimitOptions::new(
            budget,
            &self.categories(),
            Period::of(self.today),
            count,
            |month, current| {
                let label = lib_budgets::presenter::period_label(month);
                if current {
                    crate::msg::desktop_budgets_limit_month_current(&label)
                } else {
                    label
                }
            },
        )
    }

    /// 11c, with Budget `current` as what "Copy categories from" names.
    pub fn new_budget(&self, current: BudgetId) -> BudgetsDialog {
        let (accounts, units) = self.accounts_and_units();
        BudgetsDialog::Budget(BudgetForm::new(
            &accounts,
            &units,
            self.budget(current).as_ref(),
        ))
    }

    /// 11c's edit mode on Budget `id`: rename it or change its Accounts.
    pub fn edit_budget(&self, id: BudgetId) -> Option<BudgetsDialog> {
        let budget = self.editable(id)?;
        let (accounts, units) = self.accounts_and_units();
        Some(BudgetsDialog::Budget(BudgetForm::from_budget(
            &budget, &accounts, &units,
        )))
    }

    /// 9e on a leaf Category of Budget `id`, starting in `month` when that is offered. A
    /// Category with no Budget Amount this month gets the picker with it preselected (9a's
    /// `set`).
    pub fn limit(
        &self,
        id: BudgetId,
        category: CategoryId,
        month: Period,
    ) -> Option<BudgetsDialog> {
        let budget = self.editable(id)?;
        let options = self.limit_options(&budget, limit_form::START_MONTHS);
        let current = Period::of(self.today);
        let form = if applied(budget.chain(category), current).is_some() {
            if !lib_categories::is_leaf(&self.categories(), category) {
                return None;
            }
            LimitForm::edit(&budget, category, month, &options)
        } else if unbudgeted_leaves(&budget, &*self.ports.reads(), current).contains(&category) {
            LimitForm::pick(Some(category), &options)
        } else {
            return None;
        };
        Some(BudgetsDialog::EditLimit(form))
    }

    /// **+ Budget a category**: 9e with its Category picker.
    pub fn limit_picker(&self, id: BudgetId) -> Option<BudgetsDialog> {
        let budget = self.editable(id)?;
        let options = self.limit_options(&budget, limit_form::START_MONTHS);
        Some(BudgetsDialog::EditLimit(LimitForm::pick(None, &options)))
    }

    /// 9g on a leaf Category that has a Budget Amount this month or in `month`.
    pub fn stop(&self, id: BudgetId, category: CategoryId, month: Period) -> Option<BudgetsDialog> {
        let budget = self.editable(id)?;
        let chain = budget.chain(category);
        if applied(chain, Period::of(self.today)).is_none() && applied(chain, month).is_none() {
            return None;
        }
        let options = self.limit_options(&budget, limit_form::STOP_MONTHS);
        Some(BudgetsDialog::Stop(StopForm::new(
            category, month, &options,
        )))
    }

    /// 9f on `month`.
    pub fn fill(&self, id: BudgetId, month: Period) -> Option<BudgetsDialog> {
        self.editable(id)?;
        Some(BudgetsDialog::Fill {
            month,
            source: FillSource::default(),
        })
    }

    /// 9d, counting the Transactions it lists now so its `j`/`k` never need the stores.
    pub fn detail(&self, id: BudgetId, category: CategoryId, month: Period) -> BudgetsDialog {
        let listed = self
            .category_detail(id, category, month)
            .map_or(0, |detail| {
                detail
                    .lines
                    .len()
                    .min(crate::view::budgets::detail_dialog::LISTED)
            });
        BudgetsDialog::CategoryDetail(Detail::new(category, month, listed))
    }

    /// 9d's figures for `category` in `month`, or `None` once its Budget or Category is gone.
    pub fn category_detail(
        &self,
        id: BudgetId,
        category: CategoryId,
        month: Period,
    ) -> Option<CategoryDetail> {
        let budget = self.budget(id)?;
        let reads = self.ports.reads();
        let reads: &dyn BudgetReads = &*reads;
        category_detail(&budget, reads, category, month, self.today)
    }

    /// Whether 9d offers Edit budget: a leaf Expense Category in a Budget that takes edits.
    pub fn detail_editable(&self, id: BudgetId, category: CategoryId) -> bool {
        let categories = self.categories();
        self.editable(id).is_some()
            && lib_categories::is_leaf(&categories, category)
            && categories
                .iter()
                .any(|each| each.id == category && each.category_type == CategoryTypes::Expense)
    }
}
