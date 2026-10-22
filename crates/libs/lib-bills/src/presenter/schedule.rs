//! The Schedule tab (8a, which absorbed 8f's History in #381): one calendar month's entries (or
//! All), through the status chips and the Bill, Category and Account scopes, with the stat callout
//! while scoped to one Bill Plan. Rows come from [`schedule_rows`]: the month's own entries, Overdue
//! and Due entries carried in from other months, and previews past the generation horizon,
//! unresolved first.
//!
//! Pay and Skip act on an actionable row (Due or Overdue, and stored); on any other they say so on
//! the status line. A Paid row hands off to its Transaction.

use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, CategoryId, Period, Total};
use lib_list_cursor::Step;
use lib_locale::Segment;
use lib_locale::format::{format_month_day, format_year_month, upper};
use lib_viper::view::{Choice, SelectView};

use super::{count, money, paid_transactions, status_chip_label};
use crate::history::{BillFilters, PlanStats, STATUS_CHIPS, SamePeriod, plan_stats};
use crate::interactor::MatchCandidates;
use crate::protocols::{BillsPorts, BillsRouter, PayRequest, SharedPorts};
use crate::{
    AmountKind, BillPlan, BillScheduleEntry, BillStatus, Recurrence, Resolution, ScheduleRow,
    amount, entry, get, msg, paid_on, period_summary, planner_order, schedule_period,
    schedule_rows,
};

/// The filter row's selects, in `f`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    Plan,
    Category,
    Account,
}

impl FilterField {
    pub const ORDER: [FilterField; 3] = [Self::Plan, Self::Category, Self::Account];

    fn next(self) -> Option<Self> {
        let at = Self::ORDER.iter().position(|field| *field == self)?;
        Self::ORDER.get(at + 1).copied()
    }
}

/// One option of a scope select: everything, or the one Bill Plan, Category or Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterChoice {
    All,
    Plan(BillPlanId),
    Category(CategoryId),
    Account(AccountId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` over the rows shown.
    Move(Step),
    /// A click on row `index` of the rows shown.
    Select(usize),
    /// `Enter`: a Paid row's Transaction.
    Open,
    /// `p` and `s` on the selected row.
    Pay,
    Skip,
    /// A row's pay, skip and view transaction buttons: select it, then act.
    PayRow(usize),
    SkipRow(usize),
    ViewTransaction(usize),
    /// `[`/`]` and the arrows: a calendar month back or on; from All, the month last viewed.
    PrevPeriod,
    NextPeriod,
    /// `0` and the All button.
    ToggleAll,
    /// `1`–`5` and a chip's click.
    ToggleChip(BillStatus),
    /// `f`: the next scope select, then off the filter row.
    CycleFilter,
    /// A click on a scope select.
    FocusFilter(FilterField),
    /// `Esc` with no list open: off the filter row.
    LeaveFilters,
    /// A scope select's option chosen.
    Filter(FilterField, FilterChoice),
}

/// The tab as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleViewModel {
    pub period: Period,
    /// The month and year shown, `Sept 2026`.
    pub period_label: String,
    /// Every entry ever generated, rather than `period`'s.
    pub all: bool,
    pub all_label: String,
    /// The header's meta line, one clause each.
    pub meta: Vec<MetaClause>,
    pub filters: FiltersView,
    /// The stat callout, while the Bill select names one Plan.
    pub stats: Option<StatsView>,
    pub columns: ScheduleColumns,
    pub rows: Vec<ScheduleLine>,
    /// What shows instead of the table while there are no rows.
    pub empty: Option<String>,
    pub footnote: String,
    /// Under the footnote while the stat callout shows.
    pub history_footnote: Option<String>,
    /// The rows before the filters: the `M` in the status line's "N of M".
    pub total: usize,
}

/// One clause of a meta line. A `strong` segment is bold; on an `alert` clause it takes the accent
/// colour too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaClause {
    pub segments: Vec<Segment>,
    pub alert: bool,
}

impl MetaClause {
    fn plain(text: String) -> Self {
        Self {
            segments: vec![Segment {
                text,
                tag: None,
                token: None,
            }],
            alert: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltersView {
    /// In [`STATUS_CHIPS`]' order.
    pub chips: Vec<ChipView>,
    /// In [`FilterField::ORDER`].
    pub selects: Vec<FilterSelectView>,
    /// The select `f` has focused.
    pub focused: Option<FilterField>,
    /// Whether any filter narrows the rows.
    pub narrowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipView {
    pub status: BillStatus,
    pub label: String,
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterSelectView {
    pub field: FilterField,
    pub view: SelectView<FilterChoice>,
    /// The Bill select while it names a Plan, drawn in the accent.
    pub accent: bool,
}

/// The stat callout: Last paid, the Average and Same period last year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsView {
    pub stats: Vec<Stat>,
    /// `No payments yet`, for a Plan never paid.
    pub never_paid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stat {
    pub label: String,
    /// The figure, `—` when there is none.
    pub figure: String,
    pub note: Option<String>,
}

/// The table's headings, upper-cased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleColumns {
    pub bill: String,
    pub account: String,
    pub planned: String,
    pub actual: String,
    pub due: String,
    pub paid: String,
    pub status: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleLine {
    pub id: crate::EntryId,
    pub name: String,
    /// `estimated`, beside an Estimated Plan's name.
    pub estimated: Option<String>,
    pub account: String,
    /// The Planned Amount (a Skipped row's snapshot), `~`-prefixed while Estimated and unpaid.
    pub planned: String,
    /// A Paid row's Matched amount, `—` otherwise.
    pub actual: String,
    pub due: String,
    /// The Matched Transaction's date, `—` otherwise.
    pub paid: String,
    pub status: BillStatus,
    pub status_label: String,
    /// In Needs Attention, which flags the status.
    pub needs_attention: bool,
    pub actions: LineActions,
    /// Skipped or Upcoming: nothing to do, drawn faint.
    pub quiet: bool,
    /// Pay and Skip act on it.
    pub actionable: bool,
    pub selected: bool,
}

/// What a row's ACTIONS cell holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineActions {
    PayOrSkip {
        pay: String,
        skip: String,
    },
    ViewTransaction(String),
    /// Text in place of buttons: Skipped rows are left out of the history figures, and Upcoming
    /// ones aren't actionable yet.
    Note(String),
}

/// The tab's state; the rows are read from the ports each time.
pub struct SchedulePresenter {
    ports: SharedPorts,
    /// A position in the rows shown, clamped wherever it is read.
    selected: usize,
    /// Kept while `all` shows every entry, so leaving All returns to it.
    period: Period,
    all: bool,
    filters: BillFilters,
    filter_focus: Option<FilterField>,
}

impl SchedulePresenter {
    /// The tab on `today`'s month, every status on and no scope.
    pub fn new(ports: SharedPorts, today: NaiveDate) -> Self {
        Self {
            ports,
            selected: 0,
            period: Period::of(today),
            all: false,
            filters: BillFilters::default(),
            filter_focus: None,
        }
    }

    pub fn handle(&mut self, event: ScheduleEvent, today: NaiveDate, router: &mut dyn BillsRouter) {
        match event {
            ScheduleEvent::Move(step) => {
                let len = self.rows(today).len();
                self.selected = lib_list_cursor::step(self.clamped(len), len, step);
            }
            ScheduleEvent::Select(index) => self.selected = index,
            ScheduleEvent::Open => self.view_transaction(today, router),
            ScheduleEvent::Pay => self.pay(today, router),
            ScheduleEvent::Skip => self.skip(today, router),
            ScheduleEvent::PayRow(index) => {
                self.selected = index;
                self.pay(today, router);
            }
            ScheduleEvent::SkipRow(index) => {
                self.selected = index;
                self.skip(today, router);
            }
            ScheduleEvent::ViewTransaction(index) => {
                self.selected = index;
                self.view_transaction(today, router);
            }
            ScheduleEvent::PrevPeriod | ScheduleEvent::NextPeriod => {
                if !std::mem::take(&mut self.all) {
                    self.period = if event == ScheduleEvent::NextPeriod {
                        self.period.next()
                    } else {
                        self.period.prev()
                    };
                }
                self.selected = 0;
            }
            ScheduleEvent::ToggleAll => {
                self.all = !self.all;
                self.selected = 0;
            }
            ScheduleEvent::ToggleChip(status) => {
                self.filters.toggle(status);
                self.selected = 0;
            }
            ScheduleEvent::CycleFilter => {
                self.filter_focus = match self.filter_focus {
                    None => Some(FilterField::Plan),
                    Some(field) => field.next(),
                };
            }
            ScheduleEvent::FocusFilter(field) => self.filter_focus = Some(field),
            ScheduleEvent::LeaveFilters => self.filter_focus = None,
            ScheduleEvent::Filter(field, choice) => {
                self.set_filter(field, choice);
                self.filter_focus = Some(field);
                self.selected = 0;
            }
        }
    }

    /// The tab on `period` with every filter cleared, as a hand-off from another View shows it.
    pub(super) fn show_period(&mut self, period: Period) {
        self.period = period;
        self.all = false;
        self.filters = BillFilters::default();
        self.filter_focus = None;
        self.selected = 0;
    }

    /// Entry `id`, in the month that shows it and selected; nothing for an entry that is gone.
    pub(super) fn reveal(&mut self, id: crate::EntryId, today: NaiveDate) {
        let Some(found) = self.ports.borrow().bills().entry(id) else {
            return;
        };
        self.show_period(schedule_period(&found, today));
        self.selected = self
            .rows(today)
            .iter()
            .position(|row| row.id == id)
            .unwrap_or(0);
    }

    /// The scope select `f` has focused.
    pub fn filter_focus(&self) -> Option<FilterField> {
        self.filter_focus
    }

    /// Leaves the filter row, as switching tab does.
    pub(super) fn reset_selection(&mut self) {
        self.selected = 0;
        self.filter_focus = None;
    }

    fn clamped(&self, len: usize) -> usize {
        self.selected.min(len.saturating_sub(1))
    }

    /// The rows before the filters.
    fn unfiltered(&self, today: NaiveDate) -> Vec<ScheduleRow> {
        let ports = self.ports.borrow();
        let bills = ports.bills();
        schedule_rows(
            &bills.bill_plans(),
            &bills.entries(),
            (!self.all).then_some(self.period),
            today,
        )
    }

    /// The rows shown.
    fn rows(&self, today: NaiveDate) -> Vec<ScheduleRow> {
        let plans = self.ports.borrow().bills().bill_plans();
        self.filters.apply(&self.unfiltered(today), &plans)
    }

    fn selected_row(&self, today: NaiveDate) -> Option<ScheduleRow> {
        let rows = self.rows(today);
        rows.get(self.clamped(rows.len())).copied()
    }

    /// The selected row when Pay and Skip may act on it: actionable, with its Plan still there.
    fn actionable(&self, today: NaiveDate, router: &mut dyn BillsRouter) -> Option<ScheduleRow> {
        let row = self.selected_row(today).filter(|row| {
            row.is_actionable()
                && self
                    .ports
                    .borrow()
                    .bills()
                    .bill_plan(row.id.plan_id)
                    .is_some()
        });
        if row.is_none() {
            router.set_status(Some(msg::bills_status_not_actionable()));
        }
        row
    }

    fn pay(&mut self, today: NaiveDate, router: &mut dyn BillsRouter) {
        let Some(row) = self.actionable(today, router) else {
            return;
        };
        let (candidates, preselected) = {
            let ports = self.ports.borrow();
            MatchCandidates { entry: row.id }.run(
                ports.bills(),
                ports.transactions(),
                ports.accounts(),
                ports.categories(),
            )
        };
        router.open_pay(PayRequest {
            entry: row.id,
            candidates,
            preselected,
        });
    }

    fn skip(&mut self, today: NaiveDate, router: &mut dyn BillsRouter) {
        if let Some(row) = self.actionable(today, router) {
            router.open_skip(row.id);
        }
    }

    /// A Paid row's Matched Transaction; nothing for any other row.
    fn view_transaction(&mut self, today: NaiveDate, router: &mut dyn BillsRouter) {
        let Some(row) = self.selected_row(today) else {
            return;
        };
        let resolution = self
            .ports
            .borrow()
            .bills()
            .entry(row.id)
            .map(|found| found.resolution);
        if let Some(Resolution::Paid(split)) = resolution {
            router.show_transaction(split.transaction_id);
        }
    }

    fn set_filter(&mut self, field: FilterField, choice: FilterChoice) {
        let filters = &mut self.filters;
        match field {
            FilterField::Plan => {
                filters.plan_id = match choice {
                    FilterChoice::Plan(id) => Some(id),
                    _ => None,
                };
            }
            FilterField::Category => {
                filters.category_id = match choice {
                    FilterChoice::Category(id) => Some(id),
                    _ => None,
                };
            }
            FilterField::Account => {
                filters.account_id = match choice {
                    FilterChoice::Account(id) => Some(id),
                    _ => None,
                };
            }
        }
    }

    /// The status line's right-hand text: the period (or All) and the rows shown of the rows
    /// before the filters.
    pub fn status(&self, today: NaiveDate) -> String {
        let period = if self.all {
            msg::bills_period_all()
        } else {
            format_year_month(self.period.year, self.period.month)
        };
        msg::bills_status_period(
            &period,
            &self.rows(today).len().to_string(),
            count(self.unfiltered(today).len()),
        )
    }

    pub fn view_model(&self, today: NaiveDate) -> ScheduleViewModel {
        let ports = self.ports.borrow();
        let ports = &*ports;
        let (plans, entries) = (ports.bills().bill_plans(), ports.bills().entries());
        let unfiltered = schedule_rows(&plans, &entries, (!self.all).then_some(self.period), today);
        let rows = self.filters.apply(&unfiltered, &plans);
        let transactions = paid_transactions(ports, &entries);
        let selected = (!rows.is_empty()).then(|| self.clamped(rows.len()));
        let accounts = ports.accounts().bill_accounts();
        let lines = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let plan = get(&plans, row.id.plan_id)?;
                let stored = entry(&entries, row.id);
                Some(line(LineInput {
                    ports,
                    row,
                    plan,
                    stored,
                    plans: &plans,
                    transactions: &transactions,
                    account: accounts
                        .iter()
                        .find(|account| account.id == plan.account_id)
                        .map(|account| account.name.clone()),
                    selected: selected == Some(index),
                }))
            })
            .collect();
        let stats = self
            .filters
            .plan_id
            .and_then(|id| get(&plans, id))
            .map(|plan| {
                let stats = plan_stats(plan, &plans, &entries, &transactions, today);
                stats_view(ports, plan, &stats)
            });
        ScheduleViewModel {
            period: self.period,
            period_label: format_year_month(self.period.year, self.period.month),
            all: self.all,
            all_label: msg::bills_period_all(),
            meta: self.meta(ports, &rows, &plans, &entries, &transactions),
            filters: self.filters_view(ports, &plans),
            history_footnote: stats.is_some().then(msg::bills_history_footnote),
            stats,
            columns: ScheduleColumns {
                bill: upper(&msg::bills_column_bill()),
                account: upper(&msg::bills_column_account()),
                planned: upper(&msg::bills_column_planned()),
                actual: upper(&msg::bills_column_actual()),
                due: upper(&msg::bills_column_due()),
                paid: upper(&msg::bills_column_paid()),
                status: upper(&msg::bills_column_status()),
                actions: upper(&lib_locale::msg::column_actions()),
            },
            empty: rows.is_empty().then(|| {
                if self.filters.is_narrowed() {
                    msg::bills_filter_empty()
                } else {
                    msg::bills_schedule_empty()
                }
            }),
            rows: lines,
            footnote: msg::bills_schedule_footnote(),
            total: unfiltered.len(),
        }
    }

    /// `9 schedule entries this period · 2 overdue · 3 due · 1 paid · 842.50 planned`; a zero
    /// count's clause is left out.
    fn meta(
        &self,
        ports: &dyn BillsPorts,
        rows: &[ScheduleRow],
        plans: &[BillPlan],
        entries: &[BillScheduleEntry],
        transactions: &[lib_transactions::Transaction],
    ) -> Vec<MetaClause> {
        let units = ports.units();
        let summary = period_summary(rows, plans, entries, transactions, |id| {
            units.unit_code(id).unwrap_or_default()
        });
        let entries_clause = if self.all {
            msg::bills_schedule_entries_all(count(rows.len()))
        } else {
            msg::bills_schedule_entries(count(rows.len()))
        };
        let mut clauses = vec![MetaClause::plain(entries_clause)];
        if summary.overdue > 0 {
            clauses.push(MetaClause {
                segments: msg::bills_schedule_overdue(&summary.overdue.to_string()),
                alert: true,
            });
        }
        if summary.due > 0 {
            clauses.push(MetaClause {
                segments: msg::bills_schedule_due(&summary.due.to_string()),
                alert: false,
            });
        }
        if summary.paid > 0 {
            clauses.push(MetaClause::plain(msg::bills_schedule_paid(
                &summary.paid.to_string(),
            )));
        }
        let base = units.base_unit().and_then(|id| units.unit_code(id));
        let total = match summary.planned {
            Total::Empty => None,
            Total::Mixed => Some(msg::bills_total_mixed()),
            Total::Single { unit, amount } => {
                let text = lib_locale::format::format_amount(
                    &amount,
                    lib_locale::format::AmountStyle::own(),
                )
                .text;
                Some(if Some(&unit) == base.as_ref() {
                    text
                } else {
                    format!("{text} {unit}")
                })
            }
        };
        if let Some(total) = total {
            clauses.push(MetaClause::plain(msg::bills_schedule_planned(&total)));
        }
        clauses
    }

    fn filters_view(&self, ports: &dyn BillsPorts, plans: &[BillPlan]) -> FiltersView {
        let filters = &self.filters;
        FiltersView {
            chips: STATUS_CHIPS
                .iter()
                .map(|status| ChipView {
                    status: *status,
                    label: status_chip_label(*status),
                    on: filters.shows(*status),
                })
                .collect(),
            selects: FilterField::ORDER
                .iter()
                .map(|field| {
                    let (label, all, chosen) = match field {
                        FilterField::Plan => (
                            msg::bills_column_bill(),
                            msg::bills_filter_all_bills(),
                            filters.plan_id.map(FilterChoice::Plan),
                        ),
                        FilterField::Category => (
                            msg::bills_column_category(),
                            msg::bills_filter_all_categories(),
                            filters.category_id.map(FilterChoice::Category),
                        ),
                        FilterField::Account => (
                            msg::bills_column_account(),
                            msg::bills_filter_all_accounts(),
                            filters.account_id.map(FilterChoice::Account),
                        ),
                    };
                    let options: Vec<Choice<FilterChoice>> = std::iter::once(Choice {
                        value: FilterChoice::All,
                        label: all,
                    })
                    .chain(filter_choices(ports, plans, *field))
                    .collect();
                    // A scope whose row has gone reads as All.
                    let chosen = chosen
                        .filter(|value| options.iter().any(|choice| choice.value == *value))
                        .unwrap_or(FilterChoice::All);
                    FilterSelectView {
                        field: *field,
                        view: SelectView {
                            label,
                            options,
                            chosen: Some(chosen),
                            focused: self.filter_focus == Some(*field),
                        },
                        accent: *field == FilterField::Plan && filters.plan_id.is_some(),
                    }
                })
                .collect(),
            focused: self.filter_focus,
            narrowed: filters.is_narrowed(),
        }
    }
}

/// A scope select's choices after its All option: the Bill Plans in Planner order, and the
/// Categories and Accounts some Plan uses, by name.
fn filter_choices(
    ports: &dyn BillsPorts,
    plans: &[BillPlan],
    field: FilterField,
) -> Vec<Choice<FilterChoice>> {
    let mut choices: Vec<Choice<FilterChoice>> = match field {
        FilterField::Plan => {
            return planner_order(plans)
                .into_iter()
                .map(|plan| Choice {
                    value: FilterChoice::Plan(plan.id),
                    label: plan.name.clone(),
                })
                .collect();
        }
        FilterField::Category => ports
            .categories()
            .bill_categories()
            .into_iter()
            .filter(|c| plans.iter().any(|p| p.category_id == c.id))
            .map(|c| Choice {
                value: FilterChoice::Category(c.id),
                label: c.name,
            })
            .collect(),
        FilterField::Account => ports
            .accounts()
            .bill_accounts()
            .into_iter()
            .filter(|a| plans.iter().any(|p| p.account_id == a.id))
            .map(|a| Choice {
                value: FilterChoice::Account(a.id),
                label: a.name,
            })
            .collect(),
    };
    choices.sort_by_key(|choice| choice.label.to_lowercase());
    choices
}

struct LineInput<'a> {
    ports: &'a dyn BillsPorts,
    row: &'a ScheduleRow,
    plan: &'a BillPlan,
    stored: Option<&'a BillScheduleEntry>,
    plans: &'a [BillPlan],
    transactions: &'a [lib_transactions::Transaction],
    account: Option<String>,
    selected: bool,
}

fn line(input: LineInput<'_>) -> ScheduleLine {
    let LineInput {
        ports,
        row,
        plan,
        stored,
        plans,
        transactions,
        account,
        selected,
    } = input;
    let paid = row.status == BillStatus::Paid;
    let planned = match stored.map(|found| &found.resolution) {
        Some(Resolution::Skipped { planned }) => planned.clone(),
        _ => plan.planned_amount.clone(),
    };
    let planned = money(ports, &planned, plan.unit);
    let actual = stored
        .filter(|_| paid)
        .and_then(|found| amount(found, plans, transactions))
        .map_or_else(msg::bills_none, |figure| money(ports, &figure, plan.unit));
    let status_label = match row.status {
        BillStatus::Overdue => upper(&msg::bills_status_overdue()),
        BillStatus::Due => upper(&msg::bills_status_due()),
        BillStatus::Paid => upper(&msg::bills_status_paid()),
        BillStatus::Skipped => upper(&msg::bills_status_skipped()),
        BillStatus::Upcoming => msg::bills_status_upcoming(),
    };
    let actions = match row.status {
        BillStatus::Paid => LineActions::ViewTransaction(msg::bills_row_view_transaction()),
        BillStatus::Skipped => LineActions::Note(msg::bills_row_excluded()),
        _ if row.is_actionable() => LineActions::PayOrSkip {
            pay: msg::bills_row_pay(),
            skip: msg::bills_row_skip(),
        },
        _ => LineActions::Note(msg::bills_row_not_actionable()),
    };
    let estimated = plan.amount_kind == AmountKind::Estimated;
    ScheduleLine {
        id: row.id,
        name: plan.name.clone(),
        estimated: estimated.then(msg::bills_estimated),
        account: account.unwrap_or_else(msg::bills_none),
        planned: if estimated && !paid {
            format!("~{planned}")
        } else {
            planned
        },
        actual,
        due: format_month_day(row.id.due),
        paid: stored
            .and_then(|found| paid_on(found, transactions))
            .map_or_else(msg::bills_none, format_month_day),
        status: row.status,
        status_label,
        needs_attention: row.needs_attention,
        actions,
        quiet: matches!(row.status, BillStatus::Skipped | BillStatus::Upcoming),
        actionable: row.is_actionable(),
        selected,
    }
}

/// The three figures for the scoped Bill Plan; `—` where there is nothing to show.
fn stats_view(ports: &dyn BillsPorts, plan: &BillPlan, stats: &PlanStats) -> StatsView {
    let figure = |amount: Option<&lib_core::Money>| {
        amount.map_or_else(msg::bills_none, |amount| money(ports, amount, plan.unit))
    };
    let year = |y: i32| format!("{:02}", y.rem_euclid(100));
    let mut items = vec![
        Stat {
            label: msg::bills_history_last_paid(),
            figure: figure(stats.last_paid.as_ref().map(|(amount, _)| amount)),
            note: stats
                .last_paid
                .as_ref()
                .map(|(_, date)| format_month_day(*date)),
        },
        Stat {
            label: msg::bills_history_average(
                &year(stats.average_year),
                &year(stats.average_year + 1),
            ),
            figure: figure(stats.average.as_ref()),
            note: (stats.average.is_none() && !stats.never_paid)
                .then(msg::bills_history_no_payments_last_fy),
        },
    ];
    let same_period = match &stats.same_period {
        SamePeriod::Hidden => None,
        SamePeriod::None => Some((same_period_label(plan.recurrence, 0), None)),
        SamePeriod::Payment(amount) => Some((same_period_label(plan.recurrence, 1), Some(amount))),
        SamePeriod::Month { total, count } => {
            Some((same_period_label(plan.recurrence, *count), Some(total)))
        }
    };
    items.extend(same_period.map(|(label, amount)| Stat {
        label,
        figure: figure(amount),
        note: None,
    }));
    StatsView {
        stats: items,
        never_paid: stats.never_paid.then(msg::bills_history_no_payments_yet),
    }
}

fn same_period_label(recurrence: Recurrence, payments: usize) -> String {
    match recurrence {
        Recurrence::Weekly | Recurrence::Fortnightly => {
            msg::bills_history_same_month_payments(count(payments))
        }
        Recurrence::Annually => msg::bills_history_last_year(),
        Recurrence::Monthly | Recurrence::Quarterly | Recurrence::OneShot => {
            msg::bills_history_same_month()
        }
    }
}
