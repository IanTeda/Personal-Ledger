//! One Presenter per Bills View: the page (its header, tabs and status line), the Schedule and
//! Planner tabs it holds, and each Dialog the page opens (the Plan form and Skip). Each takes events
//! through `handle` and builds a plain-data view model through `view_model`, its text already
//! localised from [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`BillsRouter`](crate::protocols::BillsRouter) the Client passes in.

mod bills_page;
mod plan_form;
mod planner;
mod schedule;
mod skip;

use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{Money, UnitId};
use lib_locale::format::{AmountStyle, format_amount};
use lib_transactions::Transaction;

pub use bills_page::{
    BillsPageEvent, BillsPagePresenter, BillsPageViewModel, BillsTab, Hint, TabView,
};
pub use plan_form::{ActiveView, PlanField, PlanFormEvent, PlanFormPresenter, PlanFormViewModel};
pub use planner::{PlanLine, PlannerColumns, PlannerEvent, PlannerPresenter, PlannerViewModel};
pub use schedule::{
    ChipView, FilterChoice, FilterField, FilterSelectView, FiltersView, LineActions, MetaClause,
    ScheduleColumns, ScheduleEvent, ScheduleLine, SchedulePresenter, ScheduleViewModel, Stat,
    StatsView,
};
pub use skip::{SkipEvent, SkipPresenter, SkipViewModel};

use crate::protocols::BillsPorts;
use crate::{BillScheduleEntry, BillStatus, Recurrence, Resolution, msg};

/// Where a Presenter reads today from: the local date in a Client, a fixed day in a test. Bills is
/// dated by the calendar day, so it reads no time.
pub type Clock = Rc<dyn Fn() -> NaiveDate>;

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// An amount as the Bills tables show it: its own places, and its Unit's code after it unless it
/// is the base Unit.
fn money(ports: &dyn BillsPorts, amount: &Money, unit: UnitId) -> String {
    let text = format_amount(amount, AmountStyle::own()).text;
    let units = ports.units();
    if units.base_unit() == Some(unit) {
        return text;
    }
    match units.unit_code(unit) {
        Some(code) => format!("{text} {code}"),
        None => text,
    }
}

/// A Recurrence as the Planner and the Plan form name it. Public for the Documents picker, which
/// names a Bill Plan's cadence until Documents is a module of its own.
pub fn recurrence_label(recurrence: Recurrence) -> String {
    match recurrence {
        Recurrence::Weekly => msg::bills_recurrence_weekly(),
        Recurrence::Fortnightly => msg::bills_recurrence_fortnightly(),
        Recurrence::Monthly => msg::bills_recurrence_monthly(),
        Recurrence::Quarterly => msg::bills_recurrence_quarterly(),
        Recurrence::Annually => msg::bills_recurrence_annually(),
        Recurrence::OneShot => msg::bills_recurrence_one_shot(),
    }
}

/// A status as its filter chip names it.
fn status_chip_label(status: BillStatus) -> String {
    match status {
        BillStatus::Paid => msg::bills_filter_status_paid(),
        BillStatus::Skipped => msg::bills_filter_status_skipped(),
        BillStatus::Overdue => msg::bills_filter_status_overdue(),
        BillStatus::Due => msg::bills_filter_status_due(),
        BillStatus::Upcoming => msg::bills_filter_status_upcoming(),
    }
}

/// The `n`th field of `order` after (or, `backward`, before) `current`, skipping any `shows`
/// refuses, wrapping at either end.
fn cycle<T: Copy + PartialEq>(
    order: &[T],
    current: T,
    backward: bool,
    shows: impl Fn(T) -> bool,
) -> T {
    let count = order.len();
    let mut index = order
        .iter()
        .position(|field| *field == current)
        .unwrap_or(0);
    for _ in 0..count {
        index = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        if shows(order[index]) {
            return order[index];
        }
    }
    current
}

/// The Transactions `entries`' Matches name, read through the port, for the figures that need a
/// Paid entry's amount and date.
fn paid_transactions<'a>(
    ports: &dyn BillsPorts,
    entries: impl IntoIterator<Item = &'a BillScheduleEntry>,
) -> Vec<Transaction> {
    let mut found: Vec<Transaction> = Vec::new();
    for entry in entries {
        if let Resolution::Paid(split) = entry.resolution
            && !found.iter().any(|t| t.id == split.transaction_id)
            && let Some(transaction) = ports.transactions().paid_transaction(split.transaction_id)
        {
            found.push(transaction);
        }
    }
    found
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use chrono::NaiveDate;

    use super::Clock;
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
        date(2026, 9, 19)
    }

    /// A clock a test can move on, starting at [`today`].
    pub fn clock() -> (Rc<Cell<NaiveDate>>, Clock) {
        let day = Rc::new(Cell::new(today()));
        let read = day.clone();
        (day, Rc::new(move || read.get()))
    }

    /// The stub seed, shared, with the handle the test reads it back through.
    pub fn stub() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::stub(today()).shared()
    }
}
