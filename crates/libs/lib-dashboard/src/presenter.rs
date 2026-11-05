//! The Dashboard's Presenter: Needs Attention's Bill rows and the default Budget's budget list,
//! built into a plain-data view model with every figure already worded, so a View only draws.
//!
//! It reads today from the Shell's Clock on every view model, so a day or month turn shows on the
//! next draw with nothing to catch up. The Display Preferences are pushed to it
//! ([`DashboardEvent::DisplayChanged`]): the due dates follow the date style and the flag the
//! Status glyphs. Row density isn't among them, since the Dashboard draws no table rows.

use std::rc::Rc;

use bigdecimal::{ToPrimitive, Zero};
use lib_core::{Clock, DateStyle, Money};
use lib_locale::format::{AmountStyle, format_amount, format_date, format_year_month};
use lib_viper::display::{StatusGlyphs, flag_glyph};

use crate::msg;
use crate::protocols::{
    DashboardBill, DashboardBudgetBar, DashboardEntryId, DashboardReads, DashboardRouter,
};

/// What the Dashboard is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardEvent {
    /// A click on a Needs Attention row.
    OpenBill(DashboardEntryId),
    /// The Display Preferences the page draws with.
    DisplayChanged {
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
    },
}

/// One Needs Attention row, worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionRow {
    pub id: DashboardEntryId,
    pub plan: String,
    pub overdue: bool,
    /// The due date in the date style.
    pub due: String,
    /// The amount, with its Unit code when that isn't the base Unit.
    pub amount: String,
    /// The whole row: `Netflix overdue since 18 Sept, 22.99 →`.
    pub text: String,
}

/// One bar of the budget list, worded.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetBarView {
    pub category: String,
    /// `640.00 / 1,000.00`.
    pub figures: String,
    /// Spent as a share of the budget, 0 to 1.
    pub fraction: f32,
    pub over: bool,
}

/// The default Budget's list for the current month.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BudgetListView {
    /// `Sep 2026 · day 21/30 · 70% elapsed`, empty with no default Budget.
    pub period: String,
    /// The month's elapsed share, 0 to 1: the tick on every bar.
    pub elapsed: f32,
    pub bars: Vec<BudgetBarView>,
    /// The line shown in place of the bars when there are none.
    pub none: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardViewModel {
    pub bills: Vec<AttentionRow>,
    /// The Flagged mark each Needs Attention row leads with, in the Status glyphs.
    pub flag: &'static str,
    pub budgets: BudgetListView,
}

pub struct DashboardPresenter {
    reads: Rc<dyn DashboardReads>,
    clock: Clock,
    date_style: Option<DateStyle>,
    glyphs: StatusGlyphs,
}

impl DashboardPresenter {
    /// The Dashboard over `reads`, dated by `clock`, in the default Display Preferences.
    pub fn new(reads: Rc<dyn DashboardReads>, clock: Clock) -> Self {
        Self {
            reads,
            clock,
            date_style: None,
            glyphs: StatusGlyphs::default(),
        }
    }

    pub fn handle(&mut self, event: DashboardEvent, router: &mut dyn DashboardRouter) {
        match event {
            DashboardEvent::OpenBill(id) => router.reveal_bill_entry(id),
            DashboardEvent::DisplayChanged { date_style, glyphs } => {
                self.date_style = date_style;
                self.glyphs = glyphs;
            }
        }
    }

    pub fn view_model(&self) -> DashboardViewModel {
        let today = self.clock.today();
        let base_unit = self.reads.base_unit_code();
        DashboardViewModel {
            bills: self
                .reads
                .attention_bills(today)
                .into_iter()
                .map(|bill| self.attention_row(bill, base_unit.as_deref()))
                .collect(),
            flag: flag_glyph(self.glyphs),
            budgets: self.budget_list(),
        }
    }
}

impl DashboardPresenter {
    fn attention_row(&self, bill: DashboardBill, base_unit: Option<&str>) -> AttentionRow {
        let due = format_date(bill.id.due, self.date_style);
        let amount = with_unit(money(&bill.amount), &bill.unit_code, base_unit);
        let text = if bill.overdue {
            msg::dashboard_bill_overdue(&bill.plan, &due, &amount)
        } else {
            msg::dashboard_bill_due(&bill.plan, &due, &amount)
        };
        AttentionRow {
            id: bill.id,
            plan: bill.plan,
            overdue: bill.overdue,
            due,
            amount,
            text,
        }
    }

    /// The default Budget's month containing today, empty when no Budget is the default.
    fn budget_list(&self) -> BudgetListView {
        let Some(month) = self.reads.default_budget_month(self.clock.today()) else {
            return BudgetListView {
                none: Some(msg::dashboard_budgets_none()),
                ..BudgetListView::default()
            };
        };
        let bars: Vec<BudgetBarView> = month.bars.iter().map(bar_view).collect();
        BudgetListView {
            period: msg::dashboard_budgets_period(
                &format_year_month(month.month.year, month.month.month),
                &month.elapsed.day.to_string(),
                &month.elapsed.days_in_month.to_string(),
                &month.elapsed.percent.to_string(),
            ),
            elapsed: (month.elapsed.percent as f32 / 100.0).clamp(0.0, 1.0),
            none: bars.is_empty().then(msg::dashboard_budgets_none),
            bars,
        }
    }
}

fn bar_view(bar: &DashboardBudgetBar) -> BudgetBarView {
    // A 0.00 budget is full as soon as anything is spent against it, so the bar isn't empty while
    // the spending is over.
    let fraction = if bar.budget.0.is_zero() {
        if bar.over { 1.0 } else { 0.0 }
    } else {
        (bar.spent.0.clone() / bar.budget.0.clone())
            .to_f32()
            .unwrap_or(0.0)
    };
    BudgetBarView {
        category: bar.category.clone(),
        figures: msg::dashboard_budget_figures(&money(&bar.spent), &money(&bar.budget)),
        fraction: fraction.clamp(0.0, 1.0),
        over: bar.over,
    }
}

/// An amount at its own places, as the Dashboard shows every figure.
fn money(amount: &Money) -> String {
    format_amount(amount, AmountStyle::own()).text
}

/// An amount's text, naming its Unit only when that isn't the base Unit.
fn with_unit(text: String, unit: &str, base_unit: Option<&str>) -> String {
    if Some(unit) == base_unit {
        text
    } else {
        format!("{text} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use chrono::NaiveDate;
    use lib_core::{BillPlanId, Money};

    use super::*;
    use crate::protocols::{DashboardBill, DashboardBudgetBar};
    use crate::test_support::{InMemoryDashboardReads, RecordingRouter, Routed};

    fn locale() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a valid test amount")
    }

    fn bill(plan: u32, name: &str, due: NaiveDate, amount: &str, unit: &str) -> DashboardBill {
        DashboardBill {
            id: DashboardEntryId {
                plan_id: BillPlanId::seed(plan),
                due,
            },
            plan: name.to_string(),
            overdue: false,
            amount: money(amount),
            unit_code: unit.to_string(),
        }
    }

    /// A Presenter over `reads` on a Clock the test can move, as of 21 September 2026.
    fn presenter(reads: InMemoryDashboardReads) -> (Rc<Cell<NaiveDate>>, DashboardPresenter) {
        locale();
        let (clock, day) = Clock::settable(date(2026, 9, 21));
        (day, DashboardPresenter::new(Rc::new(reads), clock))
    }

    fn reads_with_bills(bills: Vec<DashboardBill>) -> InMemoryDashboardReads {
        let mut reads = InMemoryDashboardReads::default();
        reads.bills.bills = bills;
        reads.bills.base_unit_code = Some("aud".to_string());
        reads
    }

    #[test]
    fn needs_attention_keeps_the_ports_order_and_marks_the_overdue() {
        let (_, presenter) = presenter(reads_with_bills(vec![
            bill(1, "Netflix", date(2026, 9, 18), "22.99", "aud"),
            bill(2, "Rent", date(2026, 9, 24), "2400.00", "aud"),
        ]));

        let rows = presenter.view_model().bills;

        let shown: Vec<(&str, bool)> = rows
            .iter()
            .map(|row| (row.plan.as_str(), row.overdue))
            .collect();
        assert_eq!(shown, [("Netflix", true), ("Rent", false)]);
        assert_eq!(rows[0].text, "Netflix overdue since Sep 18, 2026, 22.99 →");
        assert_eq!(rows[1].text, "Rent due Sep 24, 2026, 2,400.00 →");
    }

    #[test]
    fn an_amount_names_its_unit_only_when_it_is_not_the_base_unit() {
        let (_, presenter) = presenter(reads_with_bills(vec![
            bill(1, "Rent", date(2026, 9, 24), "2400.00", "aud"),
            bill(2, "Vanguard", date(2026, 9, 25), "12.5000", "vas"),
        ]));

        let amounts: Vec<String> = presenter
            .view_model()
            .bills
            .into_iter()
            .map(|row| row.amount)
            .collect();

        assert_eq!(amounts, ["2,400.00", "12.5000 vas"]);
    }

    fn bar(category: &str, spent: &str, budget: &str) -> DashboardBudgetBar {
        DashboardBudgetBar {
            category: category.to_string(),
            spent: money(spent),
            budget: money(budget),
            over: money(spent).0 > money(budget).0,
        }
    }

    fn reads_with_bars(bars: Vec<DashboardBudgetBar>) -> InMemoryDashboardReads {
        let mut reads = InMemoryDashboardReads::default();
        reads.budgets.default_bars = Some(bars);
        reads
    }

    #[test]
    fn the_budget_list_words_each_bar_and_ticks_the_elapsed_share() {
        let (_, presenter) = presenter(reads_with_bars(vec![
            bar("Dining", "300.00", "200.00"),
            bar("Groceries", "150.00", "600.00"),
        ]));

        let budgets = presenter.view_model().budgets;

        assert_eq!(budgets.period, "Sep 2026 · day 21/30 · 70% elapsed");
        assert_eq!(budgets.elapsed, 0.7);
        assert_eq!(
            budgets.bars,
            [
                BudgetBarView {
                    category: "Dining".to_string(),
                    figures: "300.00 / 200.00".to_string(),
                    fraction: 1.0,
                    over: true,
                },
                BudgetBarView {
                    category: "Groceries".to_string(),
                    figures: "150.00 / 600.00".to_string(),
                    fraction: 0.25,
                    over: false,
                },
            ]
        );
        assert_eq!(budgets.none, None);
    }

    #[test]
    fn a_zero_budget_is_empty_until_anything_is_spent_then_full() {
        let (_, presenter) = presenter(reads_with_bars(vec![
            bar("Gifts", "0.00", "0.00"),
            bar("Water", "12.00", "0.00"),
        ]));

        let fractions: Vec<(f32, bool)> = presenter
            .view_model()
            .budgets
            .bars
            .iter()
            .map(|bar| (bar.fraction, bar.over))
            .collect();

        assert_eq!(fractions, [(0.0, false), (1.0, true)]);
    }

    #[test]
    fn with_no_default_budget_the_list_is_empty() {
        let (_, presenter) = presenter(InMemoryDashboardReads::default());

        let budgets = presenter.view_model().budgets;

        assert_eq!(budgets.period, "");
        assert!(budgets.bars.is_empty());
        assert_eq!(
            budgets.none.as_deref(),
            Some("No budget amounts this month.")
        );
    }

    #[test]
    fn a_day_turn_marks_a_bill_due_today_overdue_on_the_next_view_model() {
        let (day, presenter) = presenter(reads_with_bills(vec![bill(
            1,
            "Netflix",
            date(2026, 9, 21),
            "22.99",
            "aud",
        )]));
        assert!(!presenter.view_model().bills[0].overdue);

        day.set(date(2026, 9, 22));

        assert!(presenter.view_model().bills[0].overdue);
    }

    #[test]
    fn a_month_turn_moves_the_budget_list_to_the_new_month() {
        let (day, presenter) = presenter(reads_with_bars(vec![bar("Dining", "0.00", "200.00")]));

        day.set(date(2026, 10, 1));

        let budgets = presenter.view_model().budgets;
        assert_eq!(budgets.period, "Oct 2026 · day 1/31 · 3% elapsed");
        assert_eq!(budgets.elapsed, 0.03);
    }

    #[test]
    fn the_display_preferences_reach_the_view_model() {
        let (_, mut presenter) = presenter(reads_with_bills(vec![bill(
            1,
            "Netflix",
            date(2026, 9, 18),
            "22.99",
            "aud",
        )]));
        let before = presenter.view_model();
        assert_eq!(before.flag, "\u{2691}");

        presenter.handle(
            DashboardEvent::DisplayChanged {
                date_style: Some(DateStyle::Iso),
                glyphs: StatusGlyphs::AsciiFallback,
            },
            &mut RecordingRouter::default(),
        );

        let after = presenter.view_model();
        assert_eq!(after.flag, "!");
        assert_eq!(after.bills[0].due, "2026-09-18");
        assert_eq!(
            after.bills[0].text,
            "Netflix overdue since 2026-09-18, 22.99 →"
        );
    }

    #[test]
    fn a_click_on_a_bill_reveals_its_entry() {
        let (_, mut presenter) = presenter(InMemoryDashboardReads::default());
        let mut router = RecordingRouter::default();
        let id = DashboardEntryId {
            plan_id: BillPlanId::seed(1),
            due: date(2026, 9, 18),
        };

        presenter.handle(DashboardEvent::OpenBill(id), &mut router);

        assert_eq!(router.routed, [Routed::BillEntry(id)]);
    }
}
