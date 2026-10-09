//! The Bills destination's Entity (ADR-0032). [`BillsView`] owns the page's own state: the tab,
//! the selected row, the Schedule's period (or All), its filters and which filter select has
//! focus. The Plans and entries are read through [`crate::bills::BillsStore`]. The view reports
//! what needs `Shell` (the dialog host, navigation and the filter choices drawn from Categories
//! and Accounts are `Shell`'s) as [`BillsEvent`]s.

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter};
use lib_accounts::step_selection;

use super::filters::FilterField;
use crate::{
    bills::{self, BillsStore, BillsTab, EntryId, ScheduleRow, history::BillFilters},
    form::select::SelectState,
    navigation::key_router::Movement,
    period::Period,
    view::accounts::state::ACCOUNTS_HALF_PAGE,
};

/// What the Bills page asks `Shell` to do. The page never opens a dialog or navigates itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillsEvent {
    /// `n`, or **+ Add bill plan**: open the Add bill plan dialog.
    AddPlan,
    /// `e` or `enter` on a Planner row: edit Bill Plan `id`.
    EditPlan(u32),
    /// `p` on a Schedule row: open the Pay dialog for it.
    Pay(ScheduleRow),
    /// `s` on a Schedule row: open the Skip dialog for it.
    Skip(ScheduleRow),
    /// `enter` on a Schedule row: open the Transaction that paid it.
    OpenTransaction(EntryId),
    /// `f`: step focus to the next filter select. `Shell` builds the select, since its choices
    /// come from Categories and Accounts.
    FocusNextFilter,
    /// The tab changed, so the Chrome's status message and the page scroll reset.
    TabChanged,
}

impl EventEmitter<BillsEvent> for BillsView {}

/// The Bills page's own state, owned by its Entity.
#[derive(Debug)]
pub struct BillsState {
    pub tab: BillsTab,
    /// The selected row, a position in the active tab's order; clamped wherever it is read.
    pub selected: usize,
    /// The Schedule tab's calendar month; starts at today's. Kept while `all` shows every entry,
    /// so leaving All returns to it.
    pub period: Period,
    pub all: bool,
    /// The Schedule tab's status chips and scope selects.
    pub filters: BillFilters,
    /// The filter select `f` has focused, with its open/highlight state.
    pub filter_focus: Option<(FilterField, SelectState)>,
}

impl BillsState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            tab: BillsTab::default(),
            selected: 0,
            period: Period::of(today),
            all: false,
            filters: BillFilters::default(),
            filter_focus: None,
        }
    }
}

/// The Bills Entity. Its state is edited through [`Self::edit`], which notifies so the page
/// re-renders.
pub struct BillsView {
    store: Entity<BillsStore>,
    today: NaiveDate,
    state: BillsState,
}

impl BillsView {
    pub fn new(store: Entity<BillsStore>, today: NaiveDate) -> Self {
        Self {
            store,
            today,
            state: BillsState::new(today),
        }
    }

    pub fn state(&self) -> &BillsState {
        &self.state
    }

    /// Edits the page's state, then asks for a re-render.
    pub fn edit<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut BillsState) -> R,
    ) -> R {
        let result = change(&mut self.state);
        cx.notify();
        result
    }

    /// The Schedule tab's rows for the shown period (or All), before its filters.
    pub fn unfiltered_rows(&self, cx: &App) -> Vec<ScheduleRow> {
        let store = self.store.read(cx);
        bills::schedule_rows(
            store.plans(),
            store.entries(),
            (!self.state.all).then_some(self.state.period),
            self.today,
        )
    }

    /// The Schedule tab's rows as shown: the period's (or All's), through its filters.
    pub fn schedule_rows(&self, cx: &App) -> Vec<ScheduleRow> {
        self.state
            .filters
            .apply(&self.unfiltered_rows(cx), self.store.read(cx).plans())
    }

    /// The selected Schedule row, its stored position clamped to the rows now shown.
    pub fn selected_row(&self, cx: &App) -> Option<ScheduleRow> {
        let rows = self.schedule_rows(cx);
        rows.get(self.state.selected.min(rows.len().saturating_sub(1)))
            .copied()
    }

    /// The Planner tab's selected Bill Plan's id, its stored position clamped to the Plans.
    pub fn selected_plan(&self, cx: &App) -> Option<u32> {
        let plans = bills::planner_order(self.store.read(cx).plans());
        plans
            .get(self.state.selected.min(plans.len().saturating_sub(1)))
            .map(|plan| plan.id)
    }

    /// Switches the tab, dropping the selection and any filter focus.
    pub fn set_tab(&mut self, tab: BillsTab, cx: &mut Context<'_, Self>) {
        self.state.tab = tab;
        self.state.selected = 0;
        self.state.filter_focus = None;
        cx.emit(BillsEvent::TabChanged);
        cx.notify();
    }

    /// Steps the Schedule tab's period a calendar month; from All, returns to the month last viewed.
    pub fn shift_period(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        let state = &mut self.state;
        if !std::mem::take(&mut state.all) {
            state.period = if forward {
                state.period.next()
            } else {
                state.period.prev()
            };
        }
        state.selected = 0;
        cx.notify();
    }

    /// `0` toggles the Schedule between its month and All.
    pub fn toggle_all(&mut self, cx: &mut Context<'_, Self>) {
        self.state.all = !self.state.all;
        self.state.selected = 0;
        cx.notify();
    }

    pub fn toggle_filter_chip(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(status) = bills::history::STATUS_CHIPS.get(index) {
            self.state.filters.toggle(*status);
            self.state.selected = 0;
            cx.notify();
        }
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the active tab's row selection; `Enter` on a
    /// Schedule row asks for its Transaction, and on a Planner row asks to edit its Bill Plan.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let len = match self.state.tab {
            BillsTab::Schedule => self.schedule_rows(cx).len(),
            BillsTab::Planner => self.store.read(cx).plans().len(),
        };
        let selected = self.state.selected.min(len.saturating_sub(1));
        self.state.selected = match movement {
            Movement::Next => step_selection(selected, len, 1),
            Movement::Prev => step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                match self.state.tab {
                    BillsTab::Planner => {
                        if let Some(id) = self.selected_plan(cx) {
                            cx.emit(BillsEvent::EditPlan(id));
                        }
                    }
                    BillsTab::Schedule => {
                        if let Some(row) = self.selected_row(cx) {
                            cx.emit(BillsEvent::OpenTransaction(row.id));
                        }
                    }
                }
                selected
            }
        };
        cx.notify();
    }

    /// The page's own `p`/`s`/`e`/`n`/`f`/`[`/`]`/`0`/`1`–`5`, `shift` saying whether it was held.
    /// Returns whether the key was one of them.
    pub fn handle_key(&mut self, key: &str, shift: bool, cx: &mut Context<'_, Self>) -> bool {
        let schedule = self.state.tab == BillsTab::Schedule;
        let planner = self.state.tab == BillsTab::Planner;
        if schedule
            && let Some(chip) = key
                .parse::<usize>()
                .ok()
                .and_then(|digit| digit.checked_sub(1))
                .filter(|index| *index < bills::history::STATUS_CHIPS.len())
        {
            self.toggle_filter_chip(chip, cx);
            return true;
        }
        match key {
            "n" if !shift => cx.emit(BillsEvent::AddPlan),
            "f" if schedule && !shift => cx.emit(BillsEvent::FocusNextFilter),
            "0" if schedule => self.toggle_all(cx),
            "e" if planner && !shift => {
                if let Some(id) = self.selected_plan(cx) {
                    cx.emit(BillsEvent::EditPlan(id));
                }
            }
            "p" if schedule && !shift => {
                if let Some(row) = self.selected_row(cx) {
                    cx.emit(BillsEvent::Pay(row));
                }
            }
            "s" if schedule && !shift => {
                if let Some(row) = self.selected_row(cx) {
                    cx.emit(BillsEvent::Skip(row));
                }
            }
            "[" if schedule => self.shift_period(false, cx),
            "]" if schedule => self.shift_period(true, cx),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{
        accounts::default_accounts, categories::default_categories, payees::default_payees,
        tags::default_tags, transactions::default_transactions,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a valid date")
    }

    /// A `BillsView` over the seeded Bills, with every event it emits collected.
    fn seeded_view(cx: &mut TestAppContext) -> (Entity<BillsView>, Rc<RefCell<Vec<BillsEvent>>>) {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let mut transactions =
            default_transactions(&accounts, &categories, &payees, &default_tags(), today());
        let seed =
            bills::default_bills(&accounts, &categories, &payees, &mut transactions, today());
        let store = cx.new(|_| BillsStore::new(seed.plans, seed.entries));
        let view = cx.new(|_| BillsView::new(store, today()));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&view, move |_, event: &BillsEvent, _| {
                sink.borrow_mut().push(*event)
            })
            .detach();
        });
        (view, events)
    }

    #[gpui::test]
    fn page_keys_that_need_shell_are_emitted_as_events(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);
        view.update(cx, |view, cx| {
            assert!(view.handle_key("n", false, cx));
            assert!(view.handle_key("f", false, cx));
            assert!(view.handle_key("p", false, cx));
            assert!(!view.handle_key("x", false, cx));
        });
        cx.run_until_parked();
        let row = view.read_with(cx, |view, cx| view.selected_row(cx));
        let row = row.expect("the seeded Schedule has a row this month");
        assert_eq!(
            *events.borrow(),
            vec![
                BillsEvent::AddPlan,
                BillsEvent::FocusNextFilter,
                BillsEvent::Pay(row)
            ]
        );
    }

    #[gpui::test]
    fn the_planner_edits_the_selected_plan(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);
        view.update(cx, |view, cx| {
            view.set_tab(BillsTab::Planner, cx);
            view.apply_movement(Movement::Enter, cx);
        });
        cx.run_until_parked();
        let id = view
            .read_with(cx, |view, cx| view.selected_plan(cx))
            .expect("the seed has Bill Plans");
        assert_eq!(
            *events.borrow(),
            vec![BillsEvent::TabChanged, BillsEvent::EditPlan(id)]
        );
    }

    #[gpui::test]
    fn stepping_the_period_from_all_returns_to_the_month_last_viewed(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        view.update(cx, |view, cx| {
            view.shift_period(true, cx);
            view.toggle_all(cx);
            view.shift_period(true, cx);
        });
        view.read_with(cx, |view, _| {
            assert!(!view.state().all);
            assert_eq!(view.state().period, Period::of(today()).next());
        });
    }
}
