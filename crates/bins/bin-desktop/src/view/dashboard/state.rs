//! The Dashboard's Entity (ADR-0032, ADR-0037). It reads the Ledger through [`LedgerStore`],
//! renders itself (`impl Render` below), scrolls itself, and reaches `Shell` only through
//! [`ViewEvent`]. It has no keys of its own beyond scrolling, so most of the View interface is a
//! no-op.

use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription, Window};
use gpui::{div, prelude::*, px};
use lib_core::DateStyle;
use lib_ledger::Domain;

use super::{AttentionBill, BudgetBar, BudgetList, Dashboard, OnBillClick, empty_state};
use crate::{
    bills, budgets,
    chrome::statusline::PageStatus,
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    period::Period,
    settings::display::StatusGlyphs,
    view::{
        event::{Reveal, ViewEvent},
        format, scroll_by_movement,
    },
};

impl EventEmitter<ViewEvent> for DashboardView {}

/// The store Entity the Dashboard reads.
#[derive(Clone)]
pub struct DashboardSources {
    pub ledger: Entity<LedgerStore>,
}

/// One Needs Attention row as a test reads it: its Plan and the `debug_selector` of its target.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardBillSnapshot {
    pub plan: String,
    pub selector: String,
}

/// The Dashboard Entity.
pub struct DashboardView {
    sources: DashboardSources,
    today: NaiveDate,
    /// The page's own scroll, reset as it becomes the destination.
    scroll: ScrollHandle,
    date_style: Option<DateStyle>,
    glyphs: StatusGlyphs,
    /// Whether a Ledger is open: without one the page is the cold-start empty state.
    ledger_open: bool,
    _bills_changed: Subscription,
    _budgets_changed: Subscription,
    _transactions_changed: Subscription,
    _categories_changed: Subscription,
    _units_changed: Subscription,
}

impl DashboardView {
    pub fn new(sources: DashboardSources, today: NaiveDate, cx: &mut Context<'_, Self>) -> Self {
        Self {
            today,
            scroll: ScrollHandle::new(),
            date_style: None,
            glyphs: StatusGlyphs::default(),
            ledger_open: true,
            _bills_changed: notify_on(&sources.ledger, Domain::Bills, cx),
            _budgets_changed: notify_on(&sources.ledger, Domain::Budgets, cx),
            _transactions_changed: notify_on(&sources.ledger, Domain::Transactions, cx),
            _categories_changed: notify_on(&sources.ledger, Domain::Categories, cx),
            _units_changed: notify_on(&sources.ledger, Domain::Units, cx),
            sources,
        }
    }

    /// Moves the day Needs Attention and the budget list are dated by.
    pub fn set_today(&mut self, today: NaiveDate, cx: &mut Context<'_, Self>) {
        self.today = today;
        cx.notify();
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(
        &mut self,
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
        cx: &mut Context<'_, Self>,
    ) {
        self.date_style = date_style;
        self.glyphs = glyphs;
        cx.notify();
    }

    /// Records whether a Ledger is open, as `Shell` draws the page.
    pub fn set_ledger_open(&mut self, open: bool) {
        self.ledger_open = open;
    }

    /// No key is the Dashboard's ahead of the router.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// No key is the Dashboard's once the router has left it.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no list, so movement scrolls it.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        scroll_by_movement(&self.scroll, movement);
        cx.notify();
    }

    /// Nothing is open within the page, so `Esc` is `Shell`'s.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// `/` enters Search mode although the page has no box to search: the mode, its badge and
    /// the `Esc` out of it are unchanged from before the Dashboard moved here, and a Search with
    /// nothing to type into takes no keys.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has just become the destination: it starts scrolled to the top.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// No other View hands a record to the Dashboard.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The Dashboard words no hints of its own.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        None
    }

    fn click_bill(&mut self, id: bills::EntryId, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::Reveal(Reveal::BillEntry(id)));
    }

    /// The Needs Attention rows, worded for the Display Preferences.
    fn attention_bills(&self, cx: &App) -> Vec<AttentionBill> {
        let ledger = self.sources.ledger.read(cx).ledger();
        let (plans, entries) = (ledger.bill_plans(), ledger.bill_entries());
        let units = ledger.units();
        let base_unit = ledger.base_unit_row().map(|unit| unit.code.as_str());
        let splits = ledger.bill_splits();
        bills::attention_entries(plans, entries, self.today)
            .into_iter()
            .filter_map(|id| {
                let entry = bills::entry(entries, id)?;
                let plan = bills::get(plans, id.plan_id)?;
                let amount = bills::amount(entry, plans, &splits)?;
                Some(AttentionBill {
                    id,
                    plan: plan.name.clone(),
                    overdue: bills::status(entry, self.today) == bills::BillStatus::Overdue,
                    due: format::date(id.due, self.date_style),
                    amount: format::with_unit(
                        format::amount(&amount).1,
                        format::unit_code(units, plan.unit),
                        base_unit,
                    ),
                })
            })
            .collect()
    }

    /// The default Budget's current-month list, empty when no Budget is the default.
    fn budget_list(&self, cx: &App) -> BudgetList {
        use bigdecimal::{ToPrimitive, Zero};
        let ledger = self.sources.ledger.read(cx).ledger();
        let Some(budget) = ledger.budgets().default_budget() else {
            return BudgetList::default();
        };
        let sources = budgets::BudgetSources {
            categories: ledger.categories(),
            accounts: ledger.accounts(),
            transactions: ledger.transactions(),
            plans: ledger.bill_plans(),
            entries: ledger.bill_entries(),
        };
        let figures = budgets::period_figures(budget, &sources, Period::of(self.today), self.today);
        let bars = budgets::dashboard_bars(&figures, ledger.categories())
            .into_iter()
            .map(|bar| BudgetBar {
                category: ledger
                    .categories()
                    .iter()
                    .find(|category| category.id == bar.category_id)
                    .map(|category| category.name.clone())
                    .unwrap_or_default(),
                figures: crate::msg::desktop_budgets_history_pair(
                    &format::amount(&bar.spent).1,
                    &format::amount(&bar.budget).1,
                ),
                // A 0.00 budget is full as soon as anything is spent against it.
                fraction: if bar.budget.0.is_zero() {
                    if bar.over { 1.0 } else { 0.0 }
                } else {
                    (bar.spent.0.clone() / bar.budget.0.clone())
                        .to_f32()
                        .unwrap_or(0.0)
                },
                over: bar.over,
            })
            .collect();
        BudgetList {
            period: crate::msg::desktop_dashboard_budgets_period(
                &crate::view::budgets::period_label(figures.month),
                &figures.elapsed.day.to_string(),
                &figures.elapsed.days_in_month.to_string(),
                &figures.elapsed.percent.to_string(),
            ),
            elapsed: figures.elapsed.percent as f32 / 100.0,
            bars,
        }
    }

    /// The Needs Attention rows for a test, in the order they draw.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &App) -> Vec<DashboardBillSnapshot> {
        let ledger = self.sources.ledger.read(cx).ledger();
        bills::attention_entries(ledger.bill_plans(), ledger.bill_entries(), self.today)
            .into_iter()
            .filter_map(|id| {
                let plan = bills::get(ledger.bill_plans(), id.plan_id)?;
                Some(DashboardBillSnapshot {
                    plan: plan.name.clone(),
                    selector: format!("dashboard-bill-{}-{}", id.plan_id, id.due),
                })
            })
            .collect()
    }
}

impl Render for DashboardView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let content = if self.ledger_open {
            let on_bill_click: OnBillClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |view, cx| view.click_bill(id, cx));
            });
            let cx: &App = cx;
            Dashboard::new(
                self.attention_bills(cx),
                format::flag_glyph(self.glyphs),
                on_bill_click,
            )
            .budgets(self.budget_list(cx))
            .into_any_element()
        } else {
            let on_command_click: super::OnEmptyStateCommandClick =
                Rc::new(move |name, _window, cx| {
                    entity.update(cx, |_, cx| cx.emit(ViewEvent::RunCommand(name)));
                });
            empty_state(on_command_click, cx)
        };
        div()
            .id("view")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(content)
    }
}
