//! One Presenter per Budgets View: the page (its header, its Progress, Plan and History tabs and
//! the status line), the Switcher popover and Manage budgets. Each takes events through `handle`
//! and builds a plain-data view model through `view_model`, its text already localised from
//! [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`BudgetsRouter`](crate::protocols::BudgetsRouter) the Client passes in.
//!
//! The Budget form, Edit budget, Stop budgeting, Fill and the Category detail are Presenters too
//! (#955): the page asks its Router to open each, and a Dialog hands what follows back to the page
//! through [`BudgetsOutcome`](crate::protocols::BudgetsOutcome).

mod budget_form;
mod budgets_page;
mod detail;
mod fill;
mod history;
mod limit_form;
mod manage;
mod plan;
mod progress;
mod stop;
mod switcher;

use lib_core::{CategoryId, Money, Period};
use lib_locale::Segment;
use lib_locale::format::{AmountStyle, format_amount, format_month, format_year_month};

pub use budget_form::{
    AccountChip, BudgetField, BudgetFormEvent, BudgetFormPresenter, BudgetFormViewModel,
    MethodCard, StartChoice,
};
pub use budgets_page::{
    BudgetsPageEvent, BudgetsPagePresenter, BudgetsPageViewModel, BudgetsTab, HeaderView, TabBody,
    TabView,
};
pub use detail::{DetailEvent, DetailPresenter, DetailStat, DetailViewModel, LISTED, LineRow};
pub use fill::{ChangeRow, FillEvent, FillPresenter, FillViewModel, SourceRow};
pub use history::{
    HistoryBarView, HistoryCellView, HistoryColumns, HistoryEvent, HistoryRowView, HistoryViewModel,
};
pub use limit_form::{
    LimitField, LimitFormEvent, LimitFormPresenter, LimitFormViewModel, LimitPreviewView,
};
pub use manage::{
    ManageColumns, ManageEvent, ManagePresenter, ManageRow, ManageViewModel, actions,
};
pub use plan::{
    PlanCellView, PlanEvent, PlanFooterView, PlanMonthView, PlanRowView, PlanSectionView,
    PlanViewModel,
};
pub use progress::{
    BarView, ProgressColumns, ProgressEvent, ProgressRow, ProgressViewModel, StatView, bar_view,
};
pub use stop::{StopEvent, StopPresenter, StopViewModel};
pub use switcher::{SwitcherEvent, SwitcherPresenter, SwitcherRow, SwitcherViewModel};

use crate::protocols::BudgetCategory;
use crate::{Error, Method, msg};

/// A cell with no figure.
const EMPTY_CELL: &str = "\u{2014}";

/// An amount as the Budgets tables show it: its own places, no Unit, since every figure on a
/// Budget is in its one Unit.
fn money(amount: &Money) -> String {
    format_amount(amount, AmountStyle::own()).text
}

/// A month as a Dialog's select words it: `September 2026 (current)` for the current month.
fn month_label(month: Period, current: Period) -> String {
    let label = period_label(month);
    if month == current {
        msg::budgets_limit_month_current(&label)
    } else {
        label
    }
}

/// An amount with a `+` in front of a positive one: a change.
fn signed_money(amount: &Money) -> String {
    format_amount(amount, AmountStyle::own().with_plus()).text
}

/// A month as the Locale writes a month and year (`September 2026`). Public for the Dashboard,
/// which names the default Budget's month the same way.
pub fn period_label(period: Period) -> String {
    format_year_month(period.year, period.month)
}

/// A Method as its tag names it.
pub fn method_label(method: Method) -> String {
    match method {
        Method::Limits => msg::budgets_method_limits(),
    }
}

/// `Jul – Dec 2026`, or with both years when the range crosses one.
fn range_label(first: Period, last: Period) -> String {
    if first.year == last.year {
        msg::budgets_plan_range(
            &format_month(first.month),
            &format_month(last.month),
            &last.year.to_string(),
        )
    } else {
        msg::budgets_plan_range_years(
            &format_month(first.month),
            &first.year.to_string(),
            &format_month(last.month),
            &last.year.to_string(),
        )
    }
}

/// Category `id`'s name, or the empty cell once it has gone.
fn category_name(categories: &[BudgetCategory], id: CategoryId) -> String {
    categories
        .iter()
        .find(|category| category.id == id)
        .map_or_else(|| EMPTY_CELL.to_string(), |category| category.name.clone())
}

/// How many ancestors Category `id` has: its indent in the tables.
fn depth(categories: &[BudgetCategory], id: CategoryId) -> usize {
    let mut depth = 0;
    let mut parent = categories
        .iter()
        .find(|category| category.id == id)
        .and_then(|category| category.parent);
    while let Some(id) = parent {
        depth += 1;
        // A tree is never deeper than its rows, so a cycle can't loop forever.
        if depth > categories.len() {
            break;
        }
        parent = categories
            .iter()
            .find(|category| category.id == id)
            .and_then(|category| category.parent);
    }
    depth
}

/// Text with no markup, as one run.
fn plain(text: String) -> Vec<Segment> {
    vec![Segment {
        text,
        tag: None,
        token: None,
    }]
}

/// A refused Budget-wide change, as the status line or Manage budgets words it.
fn error_text(error: &Error) -> String {
    match error {
        Error::DefaultCannotBeArchived => msg::budgets_manage_error_default(),
        Error::Archived => msg::budgets_manage_error_archived(),
        // Budgets are archived, never deleted, so a Budget-wide verb can't find its Budget gone,
        // and the other refusals belong to a Dialog's fields.
        refusal => {
            debug_assert!(false, "a Budget-wide verb doesn't return {refusal:?}");
            msg::budgets_manage_error_refused()
        }
    }
}

/// A refused Dialog write, as the Dialog words it under its fields.
fn dialog_error_text(error: &Error) -> String {
    match error {
        Error::NameRequired => msg::budgets_new_error_name_required(),
        Error::NameTaken => msg::budgets_new_error_name(),
        Error::AccountNotFound => msg::budgets_new_error_account_gone(),
        Error::AccountNotInUnit => msg::budgets_new_error_account_unit(),
        Error::Archived => msg::budgets_limit_error_archived(),
        Error::ClosedMonth => msg::budgets_limit_error_closed_month(),
        Error::NotAnExpenseLeaf => msg::budgets_limit_error_not_leaf(),
        Error::NotBudgeted => msg::budgets_limit_error_not_budgeted(),
        Error::NegativeAmount | Error::InvalidAmount => msg::budgets_limit_error_amount(),
        // Budgets are archived, never deleted, and a Dialog's Budget is the one open on the page.
        refusal @ (Error::NotFound | Error::DefaultCannotBeArchived) => {
            debug_assert!(false, "a Dialog write doesn't return {refusal:?}");
            msg::budgets_limit_error_refused()
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use chrono::NaiveDate;
    use lib_core::Clock;

    use crate::fake::InMemoryPorts;
    use crate::protocols::SharedPorts;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// The day the tests' sample is seeded on.
    pub fn today() -> NaiveDate {
        date(2026, 6, 15)
    }

    /// A clock a test can move on, starting at [`today`].
    pub fn clock() -> (Rc<Cell<NaiveDate>>, Clock) {
        let (clock, day) = Clock::settable(today());
        (day, clock)
    }

    /// The sample seed, shared, with the handle the test reads it back through.
    pub fn sample() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::sample(today()).shared()
    }
}
