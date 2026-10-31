//! The Progress tab (9a): the meta line, the stat strip and the Category table, all read from one
//! [`period_figures`], with the month it shows and the selected row.
//!
//! A row's bar is Spent, then Known Costs, both as shares of its budget, with the period's elapsed
//! share as a tick; an over-budget row's LEFT reads `68.40 over`.

use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use chrono::NaiveDate;
use lib_core::{CategoryId, Money, Period};
use lib_list_cursor::Step;
use lib_locale::Segment;

use super::budgets_page::BudgetsPagePresenter;
use super::{EMPTY_CELL, category_name, count, depth, money, plain};
use crate::protocols::{BudgetReads, BudgetsRouter};
use crate::{Budget, CategoryFigures, PeriodFigures, msg, period_figures};

/// What the Progress tab is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
    /// `[`/`]` and the nav's arrows: a calendar month back or on.
    PreviousPeriod,
    NextPeriod,
    /// `e`: Edit budget on the selected row.
    EditAmount,
    /// `s`: Stop budgeting the selected row.
    Stop,
    /// `c`: Budget a category, with its Category picker.
    BudgetCategory,
    /// A click on row `index`: selects it, and opens its detail when it was already selected.
    Select(usize),
    /// A row's `edit` or `set`.
    Action(usize),
    /// The KNOWN COSTS stat's link: the Bills Schedule on the month shown.
    KnownCosts,
}

/// The tab's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProgressState {
    /// The calendar month shown; starts at today's.
    pub period: Period,
    /// The selected row, a position in the figures' rows.
    pub selected: usize,
}

impl ProgressState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            period: Period::of(today),
            selected: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressViewModel {
    /// Budgeted, Spent, Known costs and Left to spend.
    pub stats: Vec<StatView>,
    /// The legend beside the tabs: spent, known costs, elapsed and over.
    pub legend: [String; 4],
    pub columns: ProgressColumns,
    pub rows: Vec<ProgressRow>,
    /// The selected row, while there are rows.
    pub selected: Option<usize>,
    /// What shows in place of the table when nothing is budgeted or spent.
    pub empty: Option<String>,
    /// The period's elapsed share, 0 to 1: each bar's tick.
    pub elapsed: f32,
}

/// One cell of the stat strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatView {
    pub label: String,
    pub figure: String,
    /// The figure is over budget.
    pub alert: bool,
    pub details: Vec<String>,
    /// The KNOWN COSTS stat's `Schedule →`.
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressColumns {
    pub category: String,
    pub budget: String,
    pub spent: String,
    pub known: String,
    pub left: String,
    pub progress: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressRow {
    /// A parent's name carries its `▾`.
    pub name: String,
    pub depth: usize,
    pub is_parent: bool,
    /// `rollup` on a parent, `incl. 12.00 carried` on a leaf with a carry.
    pub note: Option<String>,
    pub budget: String,
    /// The row has a budget; `unbudgeted` is muted.
    pub budgeted: bool,
    pub spent: String,
    pub known: String,
    pub left: String,
    /// LEFT is over or at risk, drawn in the accent.
    pub left_alert: bool,
    pub bar: Option<BarView>,
    /// `edit` or `set`; a parent has none.
    pub action: Option<String>,
}

/// A row's bar: Spent and Known Costs as shares of its budget, 0 to 1, Known after Spent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarView {
    pub spent: f32,
    pub known: f32,
    pub over: bool,
}

/// Whole percent of `part` in `whole`, `0` when there is no whole.
fn percent(part: &Money, whole: &Money) -> i64 {
    if whole.0.is_zero() {
        return 0;
    }
    let ratio: BigDecimal = part.0.clone() * 100 / whole.0.clone();
    ratio.round(0).to_i64().unwrap_or(0)
}

/// `part` as a share of `whole`, held between 0 and 1.
fn share(part: &Money, whole: &Money) -> f32 {
    (part.0.clone() / whole.0.clone())
        .to_f32()
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

impl BudgetsPagePresenter {
    pub(super) fn progress_event(&mut self, event: ProgressEvent, router: &mut dyn BudgetsRouter) {
        match event {
            ProgressEvent::PreviousPeriod => {
                self.progress.period = self.progress.period.prev();
                self.progress.selected = 0;
            }
            ProgressEvent::NextPeriod => {
                self.progress.period = self.progress.period.next();
                self.progress.selected = 0;
            }
            ProgressEvent::EditAmount => {
                if let Some(category) = self.progress_leaf(self.progress.selected) {
                    router.open_limit(self.current, category, self.progress.period);
                }
            }
            ProgressEvent::Stop => {
                if let Some(category) = self.progress_leaf(self.progress.selected) {
                    router.open_stop(self.current, category, self.progress.period);
                }
            }
            ProgressEvent::BudgetCategory => {
                router.open_limit_picker(self.current);
            }
            ProgressEvent::Select(index) => {
                let was_selected = self.progress.selected == index;
                self.progress.selected = index;
                if was_selected {
                    self.open_progress_detail(index, router);
                }
            }
            ProgressEvent::Action(index) => {
                self.progress.selected = index;
                if let Some(category) = self.progress_leaf(index) {
                    router.open_limit(self.current, category, self.progress.period);
                }
            }
            ProgressEvent::KnownCosts => router.show_bills_period(self.progress.period),
        }
    }

    pub(super) fn progress_move(&mut self, step: Step) {
        let len = self
            .progress_figures()
            .map_or(0, |figures| figures.rows.len());
        let selected = self.progress.selected.min(len.saturating_sub(1));
        self.progress.selected = lib_list_cursor::step(selected, len, step);
    }

    /// The Budget on show's figures for the month shown.
    fn progress_figures(&self) -> Option<PeriodFigures> {
        let budget = self.budget()?;
        let ports = self.ports.borrow();
        Some(period_figures(
            &budget,
            ports.reads(),
            self.progress.period,
            self.today(),
        ))
    }

    /// Row `index`'s Category, when it is a leaf: a parent only rolls up.
    fn progress_leaf(&self, index: usize) -> Option<CategoryId> {
        let figures = self.progress_figures()?;
        let row = figures.rows.get(index)?;
        (!row.is_parent).then_some(row.category_id)
    }

    /// The Category detail on row `index`, for the month shown.
    pub(super) fn open_progress_detail(&mut self, index: usize, router: &mut dyn BudgetsRouter) {
        let len = self
            .progress_figures()
            .map_or(0, |figures| figures.rows.len());
        let index = index.min(len.saturating_sub(1));
        self.progress.selected = index;
        let category = self
            .progress_figures()
            .and_then(|figures| figures.rows.get(index).map(|row| row.category_id));
        if let Some(category) = category {
            router.open_detail(self.current, category, self.progress.period);
        }
    }

    /// The tab's view model, its meta line's clauses and the status line's right-hand text.
    pub(super) fn progress_view(
        &self,
        budget: &Budget,
        reads: &dyn BudgetReads,
    ) -> (ProgressViewModel, Vec<Vec<Segment>>, String) {
        let figures = period_figures(budget, reads, self.progress.period, self.today());
        let categories = reads.budget_categories();
        let rows: Vec<ProgressRow> = figures
            .rows
            .iter()
            .map(|row| progress_row(row, &categories))
            .collect();
        let selected =
            (!rows.is_empty()).then(|| self.progress.selected.min(rows.len().saturating_sub(1)));
        let status = msg::budgets_status_period(
            &super::period_label(self.progress.period),
            count(rows.len()),
        );
        let view = ProgressViewModel {
            stats: stats(&figures),
            legend: [
                msg::budgets_legend_spent(),
                msg::budgets_legend_known(),
                msg::budgets_legend_elapsed(),
                msg::budgets_legend_over(),
            ],
            columns: ProgressColumns {
                category: lib_locale::msg::column_category(),
                budget: msg::budgets_column_budget(),
                spent: msg::budgets_column_spent(),
                known: msg::budgets_column_known(),
                left: msg::budgets_column_left(),
                progress: msg::budgets_column_progress(),
            },
            empty: rows.is_empty().then(msg::budgets_progress_empty),
            rows,
            selected,
            elapsed: f32::from(u16::try_from(figures.elapsed.percent).unwrap_or(100)) / 100.0,
        };
        (view, meta(&figures, self.progress.period), status)
    }
}

/// `September 2026 · day 21 of 30 · 1 over budget · 58.00 unbudgeted`. The day is the current
/// month's only, and a zero clause is left out.
fn meta(figures: &PeriodFigures, period: Period) -> Vec<Vec<Segment>> {
    let mut clauses = vec![plain(msg::budgets_meta_period(&super::period_label(
        period,
    )))];
    let elapsed = figures.elapsed;
    if elapsed.day > 0 && elapsed.days_left > 0 {
        clauses.push(plain(msg::budgets_meta_day(
            &elapsed.day.to_string(),
            &elapsed.days_in_month.to_string(),
        )));
    }
    if figures.over_count > 0 {
        clauses.push(msg::budgets_meta_over(&figures.over_count.to_string()));
    }
    if figures.at_risk_count > 0 {
        clauses.push(plain(msg::budgets_meta_at_risk(
            &figures.at_risk_count.to_string(),
        )));
    }
    if figures.unbudgeted_spent.0.is_positive() {
        clauses.push(plain(msg::budgets_meta_unbudgeted(&money(
            &figures.unbudgeted_spent,
        ))));
    }
    clauses
}

fn stats(figures: &PeriodFigures) -> Vec<StatView> {
    let budgeted_leaves = figures
        .rows
        .iter()
        .filter(|row| !row.is_parent && row.budget.is_some())
        .count();
    let mut budgeted_details = vec![msg::budgets_stat_budgeted_detail(count(budgeted_leaves))];
    if figures.carried_in.0.is_positive() {
        budgeted_details.push(msg::budgets_stat_carried(&money(&figures.carried_in)));
    }
    let left_over = figures.left.0.is_negative();
    let left_detail = if left_over {
        msg::budgets_stat_left_over()
    } else {
        figures
            .per_day
            .as_ref()
            .map_or_else(String::new, |per_day| {
                msg::budgets_stat_left_per_day(
                    &money(per_day),
                    i64::from(figures.elapsed.days_left),
                )
            })
    };
    let stat = |label: String, figure: &Money, alert: bool, details: Vec<String>| StatView {
        label,
        figure: money(figure),
        alert,
        details,
        link: None,
    };
    vec![
        stat(
            msg::budgets_stat_budgeted(),
            &figures.budgeted,
            false,
            budgeted_details,
        ),
        stat(
            msg::budgets_stat_spent(),
            &figures.spent,
            false,
            vec![msg::budgets_stat_spent_detail(
                &percent(&figures.spent, &figures.budgeted).to_string(),
                &figures.elapsed.percent.to_string(),
            )],
        ),
        StatView {
            link: Some(msg::budgets_stat_known_link()),
            ..stat(
                msg::budgets_stat_known(),
                &figures.known,
                false,
                vec![msg::budgets_stat_known_detail(count(figures.known_count))],
            )
        },
        stat(
            msg::budgets_stat_left(),
            &figures.left,
            left_over,
            vec![left_detail],
        ),
    ]
}

/// A row's bar, when it has a positive budget to measure against. Public for the Category
/// detail, which draws the same bar until it is a Presenter of its own (#955).
pub fn bar_view(row: &CategoryFigures) -> Option<BarView> {
    let budget = row
        .budget
        .as_ref()
        .filter(|budget| budget.0.is_positive())?;
    let spent = if row.spent.0.is_negative() {
        0.0
    } else {
        share(&row.spent, budget)
    };
    Some(BarView {
        spent,
        known: share(&row.known, budget).min(1.0 - spent),
        over: row.over,
    })
}

fn progress_row(row: &CategoryFigures, categories: &[crate::BudgetCategory]) -> ProgressRow {
    let name = category_name(categories, row.category_id);
    let note = if row.is_parent {
        Some(msg::budgets_row_rollup())
    } else if row.carried_in.0.is_positive() {
        Some(msg::budgets_row_carried(&money(&row.carried_in)))
    } else {
        None
    };
    let (budget, budgeted, left, left_alert) = match (&row.budget, &row.left) {
        (Some(budget), Some(left)) => {
            let (left, alert) = if row.over {
                let over = Money(row.spent.0.clone() - budget.0.clone());
                (msg::budgets_row_over(&money(&over)), true)
            } else if row.at_risk {
                (
                    format!("{} {}", money(left), msg::budgets_row_at_risk()),
                    true,
                )
            } else {
                (money(left), false)
            };
            (money(budget), true, left, alert)
        }
        _ => (
            msg::budgets_row_unbudgeted(),
            false,
            EMPTY_CELL.to_string(),
            false,
        ),
    };
    let bar = bar_view(row);
    ProgressRow {
        name: if row.is_parent {
            format!("\u{25be} {name}")
        } else {
            name
        },
        depth: depth(categories, row.category_id),
        is_parent: row.is_parent,
        note,
        budget,
        budgeted,
        spent: money(&row.spent),
        known: if row.known.0.is_zero() {
            EMPTY_CELL.to_string()
        } else {
            money(&row.known)
        },
        left,
        left_alert,
        bar,
        action: (!row.is_parent).then(|| {
            if row.budget.is_some() {
                msg::budgets_row_edit()
            } else {
                msg::budgets_row_set()
            }
        }),
    }
}
