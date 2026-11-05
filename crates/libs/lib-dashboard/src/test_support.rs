//! In-memory ports and a recording Router, for the Presenter's tests and a Client's UI-first work.
//! Each fake holds its rows in a plain field so a test sets up state directly, and dates them by
//! the day it is asked about, so a test that moves the Clock sees the rows move with it.

use chrono::{Datelike, NaiveDate};
use lib_core::Period;
use lib_toast::{Notice, Notify};

use crate::protocols::{
    DashboardBill, DashboardBills, DashboardBudgetBar, DashboardBudgetMonth, DashboardBudgets,
    DashboardElapsed, DashboardEntryId, DashboardRouter,
};

/// The Needs Attention rows over a plain list, in the order given. An entry is overdue once its due
/// date is behind the day asked about, as `lib-bills`' status reads it; the `overdue` each row is
/// set up with is ignored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryDashboardBills {
    pub bills: Vec<DashboardBill>,
    pub base_unit_code: Option<String>,
}

impl DashboardBills for InMemoryDashboardBills {
    fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill> {
        self.bills
            .iter()
            .map(|bill| DashboardBill {
                overdue: bill.id.due < today,
                ..bill.clone()
            })
            .collect()
    }

    fn base_unit_code(&self) -> Option<String> {
        self.base_unit_code.clone()
    }
}

/// The default Budget's bars over a plain list, the same every month; `None` when no Budget is the
/// default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryDashboardBudgets {
    pub default_bars: Option<Vec<DashboardBudgetBar>>,
}

impl DashboardBudgets for InMemoryDashboardBudgets {
    fn default_budget_month(&self, today: NaiveDate) -> Option<DashboardBudgetMonth> {
        let bars = self.default_bars.clone()?;
        let month = Period::of(today);
        let days_in_month = month.last_day().day();
        Some(DashboardBudgetMonth {
            month,
            elapsed: DashboardElapsed {
                day: today.day(),
                days_in_month,
                percent: today.day() * 100 / days_in_month,
            },
            bars,
        })
    }
}

/// Both fakes, as the one value the Presenter reads through.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryDashboardReads {
    pub bills: InMemoryDashboardBills,
    pub budgets: InMemoryDashboardBudgets,
}

impl DashboardBills for InMemoryDashboardReads {
    fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill> {
        self.bills.attention_bills(today)
    }

    fn base_unit_code(&self) -> Option<String> {
        self.bills.base_unit_code()
    }
}

impl DashboardBudgets for InMemoryDashboardReads {
    fn default_budget_month(&self, today: NaiveDate) -> Option<DashboardBudgetMonth> {
        self.budgets.default_budget_month(today)
    }
}

/// What a Presenter asked its Router for.
#[derive(Debug, Clone, PartialEq)]
pub enum Routed {
    BillEntry(DashboardEntryId),
    Notice(Notice),
}

/// A Router that records each request, in order.
#[derive(Debug, Default)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

impl DashboardRouter for RecordingRouter {
    fn reveal_bill_entry(&mut self, id: DashboardEntryId) {
        self.routed.push(Routed::BillEntry(id));
    }
}
