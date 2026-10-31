//! One Presenter per Budgets View: the page (its header, its Progress, Plan and History tabs and
//! the status line), the Switcher popover and Manage budgets. Each takes events through `handle`
//! and builds a plain-data view model through `view_model`, its text already localised from
//! [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`BudgetsRouter`](crate::protocols::BudgetsRouter) the Client passes in.
//!
//! The Budget form, Edit budget, Stop budgeting, Fill and the Category detail are still the
//! Client's own Dialogs (#955): the page asks its Router to open them.

mod budgets_page;
mod history;
mod manage;
mod plan;
mod progress;
mod switcher;

use lib_core::{CategoryId, Money, Period};
use lib_locale::Segment;
use lib_locale::format::{AmountStyle, format_amount, format_month, format_year_month};

pub use budgets_page::{
    BudgetsPageEvent, BudgetsPagePresenter, BudgetsPageViewModel, BudgetsTab, HeaderView, TabBody,
    TabView,
};
pub use history::{
    HistoryBarView, HistoryCellView, HistoryColumns, HistoryEvent, HistoryRowView, HistoryViewModel,
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
pub use switcher::{SwitcherEvent, SwitcherPresenter, SwitcherRow, SwitcherViewModel};

use crate::protocols::BudgetCategory;
use crate::{BudgetError, Method, msg};

/// A cell with no figure.
const EMPTY_CELL: &str = "\u{2014}";

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// An amount as the Budgets tables show it: its own places, no Unit, since every figure on a
/// Budget is in its one Unit.
fn money(amount: &Money) -> String {
    format_amount(amount, AmountStyle::own()).text
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
fn error_text(error: &BudgetError) -> String {
    match error {
        BudgetError::DefaultCannotBeArchived => msg::budgets_manage_error_default(),
        BudgetError::Archived => msg::budgets_manage_error_archived(),
        _ => msg::budgets_manage_error_refused(),
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

    /// The day the tests' stub is seeded on.
    pub fn today() -> NaiveDate {
        date(2026, 6, 15)
    }

    /// A clock a test can move on, starting at [`today`].
    pub fn clock() -> (Rc<Cell<NaiveDate>>, Clock) {
        let (clock, day) = Clock::settable(today());
        (day, clock)
    }

    /// The stub seed, shared, with the handle the test reads it back through.
    pub fn stub() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::stub(today()).shared()
    }
}
