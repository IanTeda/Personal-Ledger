//! The Bills Schedule tab's filters and the history figures behind its stat callout
//! (`docs/ux/desktop-mockups/12-bills/README.md`'s 8a, which absorbed 8f's History tab in #381) --
//! `gpui`-free and unit-tested, shared by the Desktop and the TUI.
//!
//! The rules are the Desktop Bills Surface map's History decisions (#369), as carried into the
//! Schedule by #381:
//!
//! - Status chips are multi-select (all five on by default); Bill Plan, Category and Account are
//!   single-select scopes over the rows [`crate::schedule_rows`] gives for the viewed month or All.
//! - The financial year starts in [`FINANCIAL_YEAR_START_MONTH`] until Settings' "Financial year
//!   starts" is wired to a real Preference.
//! - The stat callout reads only its one Bill Plan, never the other filters: Last paid is the Paid
//!   entry with the latest Matched-Transaction date (a later due date breaks a tie); Average is the
//!   mean of the previous complete financial year's Paid entries (by due date), Skipped ones left
//!   out rather than counted as zero; Same period last year looks back a year from Last paid's due
//!   date, by the Plan's Recurrence.

use bigdecimal::{BigDecimal, RoundingMode};
use chrono::{Datelike, Months, NaiveDate};
use lib_core::{Money, Period};
use lib_transactions::Transaction;

use crate::{BillPlan, BillScheduleEntry, BillStatus, Recurrence, Resolution, ScheduleRow, bills};

/// The month the financial year starts in: July, the Australian year, until Settings' "Financial
/// year starts" becomes a real Preference.
pub const FINANCIAL_YEAR_START_MONTH: u32 = 7;

/// The status chips, in the filter row's order.
pub const STATUS_CHIPS: [BillStatus; 5] = [
    BillStatus::Paid,
    BillStatus::Skipped,
    BillStatus::Overdue,
    BillStatus::Due,
    BillStatus::Upcoming,
];

/// The calendar year the financial year holding `date` starts in.
pub fn financial_year_start(date: NaiveDate) -> i32 {
    if date.month() >= FINANCIAL_YEAR_START_MONTH {
        date.year()
    } else {
        date.year() - 1
    }
}

/// The inclusive bounds of the financial year starting in `start_year`.
pub fn financial_year(start_year: i32) -> (NaiveDate, NaiveDate) {
    let first = Period {
        year: start_year,
        month: FINANCIAL_YEAR_START_MONTH,
    };
    let last = Period {
        year: start_year + 1,
        month: FINANCIAL_YEAR_START_MONTH,
    }
    .prev();
    (first.first_day(), last.last_day())
}

/// The Schedule tab's filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillFilters {
    /// The toggled-on status chips.
    pub statuses: Vec<BillStatus>,
    /// Scoped to one Bill Plan: the stat callout shows.
    pub plan_id: Option<u32>,
    pub category_id: Option<u32>,
    pub account_id: Option<u32>,
}

impl Default for BillFilters {
    fn default() -> Self {
        Self {
            statuses: STATUS_CHIPS.to_vec(),
            plan_id: None,
            category_id: None,
            account_id: None,
        }
    }
}

impl BillFilters {
    /// Turns a status chip on or off.
    pub fn toggle(&mut self, status: BillStatus) {
        if let Some(position) = self.statuses.iter().position(|s| *s == status) {
            self.statuses.remove(position);
        } else {
            self.statuses.push(status);
        }
    }

    pub fn shows(&self, status: BillStatus) -> bool {
        self.statuses.contains(&status)
    }

    /// Whether any filter narrows the rows.
    pub fn is_narrowed(&self) -> bool {
        *self != Self::default()
    }

    /// The `rows` these filters let through, in their order.
    pub fn apply(&self, rows: &[ScheduleRow], plans: &[BillPlan]) -> Vec<ScheduleRow> {
        rows.iter()
            .filter(|row| self.shows(row.status))
            .filter(|row| {
                crate::get(plans, row.id.plan_id).is_some_and(|plan| {
                    self.plan_id.is_none_or(|id| plan.id == id)
                        && self.category_id.is_none_or(|id| plan.category_id == id)
                        && self.account_id.is_none_or(|id| plan.account_id == id)
                })
            })
            .copied()
            .collect()
    }
}

/// The stat callout's Same-period-last-year figure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamePeriod {
    /// A One-shot Plan has no last year.
    Hidden,
    /// Nothing was paid in that period (or nothing has been paid at all).
    None,
    /// A Monthly or Quarterly Plan's row in that month, or an Annually Plan's in that year.
    Payment(Money),
    /// A Weekly or Fortnightly Plan's total paid in that calendar month, over `count` payments.
    Month { total: Money, count: usize },
}

/// The stat callout's figures for one Bill Plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStats {
    /// The latest payment's amount and Matched-Transaction date.
    pub last_paid: Option<(Money, NaiveDate)>,
    /// The mean over the previous complete financial year, rounded to the cent.
    pub average: Option<Money>,
    /// The calendar year that financial year starts in, for the Average's label.
    pub average_year: i32,
    pub same_period: SamePeriod,
    /// The Plan has no Paid entries at all.
    pub never_paid: bool,
}

/// One Paid entry of the Plan: due date, paid date and amount.
struct Payment {
    due: NaiveDate,
    paid_on: NaiveDate,
    amount: Money,
}

pub fn plan_stats(
    plan: &BillPlan,
    plans: &[BillPlan],
    entries: &[BillScheduleEntry],
    transactions: &[Transaction],
    today: NaiveDate,
) -> PlanStats {
    let payments: Vec<Payment> = entries
        .iter()
        .filter(|e| e.plan_id == plan.id && !e.superseded)
        .filter(|e| matches!(e.resolution, Resolution::Paid(_)))
        .filter_map(|e| {
            Some(Payment {
                due: e.due,
                paid_on: bills::paid_on(e, transactions)?,
                amount: bills::amount(e, plans, transactions)?,
            })
        })
        .collect();

    let last = payments.iter().max_by_key(|p| (p.paid_on, p.due));

    let average_year = financial_year_start(today) - 1;
    let (from, to) = financial_year(average_year);
    let in_year: Vec<&Payment> = payments
        .iter()
        .filter(|p| (from..=to).contains(&p.due))
        .collect();
    let average = (!in_year.is_empty()).then(|| {
        let sum: BigDecimal = in_year.iter().map(|p| p.amount.0.clone()).sum();
        Money(
            (sum / BigDecimal::from(in_year.len() as u64))
                .with_scale_round(2, RoundingMode::HalfEven),
        )
    });

    let same_period = match (plan.recurrence, last) {
        (Recurrence::OneShot, _) => SamePeriod::Hidden,
        (_, None) => SamePeriod::None,
        (recurrence, Some(last)) => last
            .due
            .checked_sub_months(Months::new(12))
            .map_or(SamePeriod::None, |anchor| {
                same_period(recurrence, anchor, &payments)
            }),
    };

    PlanStats {
        last_paid: last.map(|p| (p.amount.clone(), p.paid_on)),
        average,
        average_year,
        same_period,
        never_paid: payments.is_empty(),
    }
}

fn same_period(recurrence: Recurrence, anchor: NaiveDate, payments: &[Payment]) -> SamePeriod {
    let month = Period::of(anchor);
    match recurrence {
        Recurrence::Weekly | Recurrence::Fortnightly => {
            let paid: Vec<&Payment> = payments.iter().filter(|p| month.contains(p.due)).collect();
            if paid.is_empty() {
                SamePeriod::None
            } else {
                SamePeriod::Month {
                    total: Money(paid.iter().map(|p| p.amount.0.clone()).sum()),
                    count: paid.len(),
                }
            }
        }
        Recurrence::Monthly | Recurrence::Quarterly | Recurrence::Annually => payments
            .iter()
            .filter(|p| match recurrence {
                Recurrence::Annually => p.due.year() == anchor.year(),
                _ => month.contains(p.due),
            })
            .max_by_key(|p| p.due)
            .map_or(SamePeriod::None, |p| SamePeriod::Payment(p.amount.clone())),
        Recurrence::OneShot => SamePeriod::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn cents(value: i64) -> Money {
        Money(BigDecimal::new(value.into(), 2))
    }

    #[test]
    fn financial_years_start_in_july() {
        assert_eq!(financial_year_start(date(2026, 6, 30)), 2025);
        assert_eq!(financial_year_start(date(2026, 7, 1)), 2026);
        assert_eq!(financial_year(2025), (date(2025, 7, 1), date(2026, 6, 30)));
    }

    #[test]
    fn toggling_a_chip_adds_or_removes_it() {
        let mut filters = BillFilters::default();
        assert!(STATUS_CHIPS.iter().all(|s| filters.shows(*s)));
        assert!(!filters.is_narrowed());
        filters.toggle(BillStatus::Paid);
        assert!(!filters.shows(BillStatus::Paid) && filters.is_narrowed());
        filters.toggle(BillStatus::Paid);
        assert!(filters.shows(BillStatus::Paid));
    }

    #[test]
    fn weekly_plans_total_last_years_month() {
        let pay = |due, amount: i64| Payment {
            due,
            paid_on: due,
            amount: cents(amount),
        };
        let payments = vec![
            pay(date(2025, 9, 3), 1_000),
            pay(date(2025, 9, 10), 1_200),
            pay(date(2025, 10, 1), 900),
        ];
        assert_eq!(
            same_period(Recurrence::Weekly, date(2025, 9, 24), &payments),
            SamePeriod::Month {
                total: cents(2_200),
                count: 2
            }
        );
        assert_eq!(
            same_period(Recurrence::Annually, date(2025, 1, 1), &payments),
            SamePeriod::Payment(cents(900))
        );
        assert_eq!(
            same_period(Recurrence::Monthly, date(2025, 8, 1), &payments),
            SamePeriod::None
        );
    }
}
