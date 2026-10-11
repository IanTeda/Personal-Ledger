//! The Bills destination's Entity (ADR-0032, ADR-0037). [`BillsView`] owns the page's own state:
//! the tab, the selected row, the Schedule's period (or All), its filters and which filter select
//! has focus. It reads the Plans and entries through [`BillsStore`], and the Categories, Accounts,
//! Payees, Transactions and Units its filter choices, dialogs and columns draw on through theirs.
//! It renders itself (`impl Render` in `super`), handles its own keys, `Esc` and movement, and
//! reaches `Shell` only through [`ViewEvent`].

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle};
use lib_core::DateStyle;

use super::{
    filters::FilterField,
    hints::{bills_planner_hints, bills_schedule_hints},
};
use crate::{
    bills::{
        self, BillsDialog, BillsStore, BillsTab, EntryId, ScheduleRow,
        form::{BillPlanForm, BillPlanSource},
        history::BillFilters,
        pay_form::PayForm,
    },
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    form::select::SelectState,
    navigation::key_router::Movement,
    payees::PayeesStore,
    period::Period,
    settings::display::StatusGlyphs,
    transactions::TransactionsStore,
    units::UnitsStore,
    view::{
        accounts::state::AccountsStore,
        categories::state::CategoriesStore,
        event::{Reveal, ViewEvent},
    },
};

impl EventEmitter<ViewEvent> for BillsView {}

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

/// The store Entities the Bills page reads. Shared with the rest of the Desktop: the page writes
/// none of them, since a confirmed dialog is applied by the Dialog host.
#[derive(Clone)]
pub struct BillsSources {
    pub bills: Entity<BillsStore>,
    pub transactions: Entity<TransactionsStore>,
    pub categories: Entity<CategoriesStore>,
    pub accounts: Entity<AccountsStore>,
    pub payees: Entity<PayeesStore>,
    pub units: Entity<UnitsStore>,
}

/// The Bills Entity.
pub struct BillsView {
    pub(super) sources: BillsSources,
    pub(super) today: NaiveDate,
    /// The Display Preferences the dialogs' dates and the Schedule's status glyphs follow. Pushed
    /// by `Shell`, which owns the Settings page that changes them.
    date_style: Option<DateStyle>,
    pub(super) glyphs: StatusGlyphs,
    /// The page's own scroll, reset as it becomes the destination and as its tab changes.
    pub(super) scroll: ScrollHandle,
    pub(super) state: BillsState,
}

impl BillsView {
    pub fn new(sources: BillsSources, today: NaiveDate) -> Self {
        Self {
            sources,
            today,
            date_style: None,
            glyphs: StatusGlyphs::default(),
            scroll: ScrollHandle::new(),
            state: BillsState::new(today),
        }
    }

    pub fn state(&self) -> &BillsState {
        &self.state
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

    /// The Schedule tab's rows for the shown period (or All), before its filters.
    pub fn unfiltered_rows(&self, cx: &App) -> Vec<ScheduleRow> {
        let store = self.sources.bills.read(cx);
        bills::schedule_rows(
            store.plans(cx),
            store.entries(cx),
            (!self.state.all).then_some(self.state.period),
            self.today,
        )
    }

    /// The Schedule tab's rows as shown: the period's (or All's), through its filters.
    pub fn schedule_rows(&self, cx: &App) -> Vec<ScheduleRow> {
        self.state.filters.apply(
            &self.unfiltered_rows(cx),
            self.sources.bills.read(cx).plans(cx),
        )
    }

    /// The selected Schedule row, its stored position clamped to the rows now shown.
    pub fn selected_row(&self, cx: &App) -> Option<ScheduleRow> {
        let rows = self.schedule_rows(cx);
        rows.get(self.state.selected.min(rows.len().saturating_sub(1)))
            .copied()
    }

    /// The Planner tab's selected Bill Plan's id, its stored position clamped to the Plans.
    pub fn selected_plan(&self, cx: &App) -> Option<u32> {
        let plans = bills::planner_order(self.sources.bills.read(cx).plans(cx));
        plans
            .get(self.state.selected.min(plans.len().saturating_sub(1)))
            .map(|plan| plan.id)
    }

    /// Keys the page claims before the router: `tab` switches its tab (the handoff's `tab switch
    /// view`) rather than cycling focus. `shift-tab` still cycles focus, so the View zone can
    /// always be left.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key != "tab" || keystroke.modifiers.shift {
            return false;
        }
        self.set_tab(self.state.tab.next(), cx);
        true
    }

    /// The page's own `p`/`s`/`e`/`n`/`f`/`[`/`]`/`0`/`1`–`5`. Returns whether the key was one of
    /// them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let shift = keystroke.modifiers.shift;
        let schedule = self.state.tab == BillsTab::Schedule;
        let planner = self.state.tab == BillsTab::Planner;
        let key = keystroke.key.as_str();
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
            "n" if !shift => self.open_add_plan(cx),
            "f" if schedule && !shift => self.cycle_filter_focus(cx),
            "0" if schedule => self.toggle_all(cx),
            "e" if planner && !shift => {
                if let Some(id) = self.selected_plan(cx) {
                    self.open_edit_plan(id, cx);
                }
            }
            "p" if schedule && !shift => {
                if let Some(row) = self.selected_row(cx) {
                    self.open_pay(row, cx);
                }
            }
            "s" if schedule && !shift => {
                if let Some(row) = self.selected_row(cx) {
                    self.open_skip(row, cx);
                }
            }
            "[" if schedule => self.shift_period(false, cx),
            "]" if schedule => self.shift_period(true, cx),
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the active tab's row selection; `Enter` on a Paid
    /// Schedule row opens its Transaction, and on a Planner row edits its Bill Plan. While `f` has
    /// a Schedule filter select focused, they drive the select instead.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        if self.state.tab == BillsTab::Schedule && self.state.filter_focus.is_some() {
            self.apply_filter_select_movement(movement, cx);
            return;
        }
        let len = match self.state.tab {
            BillsTab::Schedule => self.schedule_rows(cx).len(),
            BillsTab::Planner => self.sources.bills.read(cx).plans(cx).len(),
        };
        let selected = self.state.selected.min(len.saturating_sub(1));
        self.state.selected = match movement.step() {
            Some(step) => lib_list_cursor::step(selected, len, step),
            None => {
                match self.state.tab {
                    BillsTab::Planner => {
                        if let Some(id) = self.selected_plan(cx) {
                            self.open_edit_plan(id, cx);
                        }
                    }
                    BillsTab::Schedule => {
                        if let Some(row) = self.selected_row(cx) {
                            self.open_transaction(row.id, cx);
                        }
                    }
                }
                selected
            }
        };
        cx.notify();
    }

    /// The Schedule tab's filter selects: the first `Esc` closes an open list, the next leaves the
    /// filter row. Returns whether it consumed the `Esc`.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let Some((_, select)) = self.state.filter_focus.as_mut() else {
            return false;
        };
        if select.is_open() {
            select.cancel();
        } else {
            self.state.filter_focus = None;
        }
        cx.notify();
        true
    }

    /// The Bills page has no Search, so `/` is inert on it.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The Bills page has no Search box to type into.
    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page becomes the destination: it starts scrolled to the top.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// Shows a record handed off from another View: a Schedule entry in the period that shows
    /// it, a Bill Plan on the Planner, or a period of the Schedule. No filter is left hiding the
    /// record.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        match reveal {
            Reveal::BillEntry(id) => {
                let Some(entry) = bills::entry(self.sources.bills.read(cx).entries(cx), id) else {
                    return;
                };
                let period = bills::schedule_period(entry, self.today);
                self.show_schedule_period(period, cx);
                self.state.selected = self
                    .schedule_rows(cx)
                    .iter()
                    .position(|row| row.id == id)
                    .unwrap_or(0);
            }
            Reveal::BillPlan(id) => {
                let Some(at) = bills::planner_order(self.sources.bills.read(cx).plans(cx))
                    .iter()
                    .position(|plan| plan.id == id)
                else {
                    return;
                };
                self.set_tab(BillsTab::Planner, cx);
                self.state.selected = at;
            }
            Reveal::BillsPeriod(period) => self.show_schedule_period(period, cx),
            Reveal::Transaction(_) | Reveal::Account(_) | Reveal::Payee(_) => return,
        }
        cx.notify();
    }

    /// The Schedule tab on `period`, with the filters cleared.
    fn show_schedule_period(&mut self, period: Period, cx: &mut Context<'_, Self>) {
        self.set_tab(BillsTab::Schedule, cx);
        self.state.period = period;
        self.state.all = false;
        self.state.filters = BillFilters::default();
    }

    /// A saved Bill Plan, selected if the Planner is on show.
    pub fn show_saved_plan(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        if self.state.tab != BillsTab::Planner {
            return;
        }
        self.state.selected = bills::planner_order(self.sources.bills.read(cx).plans(cx))
            .iter()
            .position(|plan| plan.id == id)
            .unwrap_or(0);
        cx.notify();
    }

    /// The status line's hints and right-hand text for the active tab.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let plans_count = || {
            crate::msg::desktop_bills_status_plans(
                i64::try_from(self.sources.bills.read(cx).plans(cx).len()).unwrap_or(i64::MAX),
            )
        };
        Some(match self.state.tab {
            BillsTab::Schedule => PageStatus {
                hints: bills_schedule_hints(),
                right: crate::msg::desktop_bills_status_period(
                    &if self.state.all {
                        crate::msg::desktop_bills_period_all()
                    } else {
                        super::period_label(self.state.period)
                    },
                    &self.schedule_rows(cx).len().to_string(),
                    i64::try_from(self.unfiltered_rows(cx).len()).unwrap_or(i64::MAX),
                ),
            },
            BillsTab::Planner => PageStatus {
                hints: bills_planner_hints(),
                right: plans_count(),
            },
        })
    }

    /// Switches the tab, dropping the selection and any filter focus. A new tab starts at the top
    /// with no stale status message.
    pub fn set_tab(&mut self, tab: BillsTab, cx: &mut Context<'_, Self>) {
        self.state.tab = tab;
        self.state.selected = 0;
        self.state.filter_focus = None;
        self.scroll.set_offset(gpui::Point::default());
        cx.emit(ViewEvent::SetStatus(None));
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

    /// Selects row `index` of the active tab.
    pub fn select(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.state.selected = index;
        cx.notify();
    }

    /// A Schedule filter select's options, and the index of its current value. Category and
    /// Account list only those some Bill Plan uses.
    pub(super) fn filter_options(&self, field: FilterField, cx: &App) -> (Vec<String>, usize) {
        let filters = &self.state.filters;
        let (all, current) = match field {
            FilterField::Plan => (
                crate::msg::desktop_bills_filter_all_bills(),
                filters.plan_id,
            ),
            FilterField::Category => (
                crate::msg::desktop_bills_filter_all_categories(),
                filters.category_id,
            ),
            FilterField::Account => (
                crate::msg::desktop_bills_filter_all_accounts(),
                filters.account_id,
            ),
        };
        let choices = self.filter_choices(field, cx);
        let index = current
            .and_then(|id| choices.iter().position(|(choice, _)| *choice == id))
            .map_or(0, |position| position + 1);
        let labels = std::iter::once(all)
            .chain(choices.into_iter().map(|(_, name)| name))
            .collect();
        (labels, index)
    }

    /// The Bill Plan, Category or Account choices behind a scope select (after its "All" option),
    /// as `(id, name)`.
    fn filter_choices(&self, field: FilterField, cx: &App) -> Vec<(u32, String)> {
        let plans = self.sources.bills.read(cx).plans(cx);
        let mut choices: Vec<(u32, String)> = match field {
            FilterField::Plan => {
                return bills::planner_order(plans)
                    .into_iter()
                    .map(|plan| (plan.id, plan.name.clone()))
                    .collect();
            }
            FilterField::Category => self
                .sources
                .categories
                .read(cx)
                .categories(cx)
                .iter()
                .filter(|c| plans.iter().any(|p| p.category_id == c.id))
                .map(|c| (c.id, c.name.clone()))
                .collect(),
            FilterField::Account => self
                .sources
                .accounts
                .read(cx)
                .accounts(cx)
                .iter()
                .filter(|a| plans.iter().any(|p| p.account_id == a.id))
                .map(|a| (a.id, a.name.clone()))
                .collect(),
        };
        choices.sort_by_key(|(_, name)| name.to_lowercase());
        choices
    }

    /// Sets a Schedule filter from its select's option `index`.
    fn apply_filter_option(&mut self, field: FilterField, index: usize, cx: &App) {
        let id = index
            .checked_sub(1)
            .and_then(|i| self.filter_choices(field, cx).get(i).map(|(id, _)| *id));
        let filters = &mut self.state.filters;
        match field {
            FilterField::Plan => filters.plan_id = id,
            FilterField::Category => filters.category_id = id,
            FilterField::Account => filters.account_id = id,
        }
        self.state.selected = 0;
    }

    /// A closed select state on a field's current value.
    pub(super) fn filter_select_state(&self, field: FilterField, cx: &App) -> SelectState {
        let (options, index) = self.filter_options(field, cx);
        SelectState::new(options.get(index).cloned())
    }

    /// `f` steps focus along the filter row's selects, then off it.
    fn cycle_filter_focus(&mut self, cx: &mut Context<'_, Self>) {
        let next = match self.state.filter_focus.as_ref().map(|(field, _)| *field) {
            None => Some(FilterField::Plan),
            Some(field) => FilterField::ORDER
                .iter()
                .position(|f| *f == field)
                .and_then(|i| FilterField::ORDER.get(i + 1))
                .copied(),
        };
        self.state.filter_focus = next.map(|field| (field, self.filter_select_state(field, cx)));
        cx.notify();
    }

    /// `j`/`k` on a focused select: step its value while closed (applying it at once), move the
    /// highlight while open; `Enter` opens it, or commits the highlight.
    fn apply_filter_select_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some((field, mut select)) = self.state.filter_focus.take() else {
            return;
        };
        let (options, _) = self.filter_options(field, cx);
        let delta = match movement {
            Movement::Next => Some(1),
            Movement::Prev => Some(-1),
            _ => None,
        };
        let mut chosen = false;
        match (movement, delta) {
            (_, Some(delta)) if select.is_open() => select.move_highlight(&options, delta),
            (_, Some(delta)) => {
                select.step(&options, delta);
                chosen = true;
            }
            (Movement::Enter, _) if select.is_open() => {
                select.commit(&options);
                chosen = true;
            }
            (Movement::Enter, _) => select.open(&options),
            _ => {}
        }
        if chosen
            && let Some(index) = select
                .value()
                .and_then(|value| options.iter().position(|option| option == value))
        {
            self.apply_filter_option(field, index, cx);
        }
        self.state.filter_focus = Some((field, select));
        cx.notify();
    }

    /// Clicking a select opens its list (closing any other), or closes its own open list.
    pub(super) fn click_filter_field(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        let open_here = self
            .state
            .filter_focus
            .as_ref()
            .is_some_and(|(focused, select)| *focused == field && select.is_open());
        let mut select = self.filter_select_state(field, cx);
        if !open_here {
            let (options, _) = self.filter_options(field, cx);
            select.open(&options);
        }
        self.state.filter_focus = Some((field, select));
        cx.notify();
    }

    pub(super) fn click_filter_option(
        &mut self,
        field: FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.apply_filter_option(field, index, cx);
        self.state.filter_focus = Some((field, self.filter_select_state(field, cx)));
        cx.notify();
    }

    /// What the Add and Edit bill plan selects choose from, copied into the form as it opens. On
    /// Edit the Plan's own Payee stays listed even if it has since been deactivated.
    fn plan_source(&self, keep_payee: Option<u32>, cx: &App) -> BillPlanSource {
        BillPlanSource::new(
            self.sources.categories.read(cx).categories(cx),
            self.sources.accounts.read(cx).accounts(cx),
            self.sources.payees.read(cx).payees(cx),
            keep_payee,
            crate::msg::desktop_payees_category_none(),
            super::planner::recurrence_label,
            self.today,
            self.date_style,
        )
    }

    fn open_dialog(dialog: BillsDialog, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Bills(
            Box::new(dialog),
        ))));
    }

    /// `n`, or **+ Add bill plan**: the Add bill plan dialog.
    pub(super) fn open_add_plan(&mut self, cx: &mut Context<'_, Self>) {
        let form = BillPlanForm::new(self.plan_source(None, cx));
        Self::open_dialog(BillsDialog::Add(form), cx);
    }

    /// The Edit dialog, pre-filled from Bill Plan `id`.
    fn open_edit_plan(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(plan) = bills::get(self.sources.bills.read(cx).plans(cx), id) else {
            return;
        };
        let form = BillPlanForm::from_plan(plan, self.plan_source(plan.payee_id, cx));
        Self::open_dialog(BillsDialog::Edit(id, form), cx);
    }

    /// The Pay dialog (8d) on an open Schedule row, with its Match candidates worked out now; a
    /// row with nothing to pay says so instead.
    fn open_pay(&mut self, row: ScheduleRow, cx: &mut Context<'_, Self>) {
        match self.pay_form(row, cx) {
            Some(form) => Self::open_dialog(BillsDialog::Pay(form), cx),
            None => Self::nothing_to_settle(cx),
        }
    }

    /// Pay or Skip on a row with nothing to settle says so on the status line.
    fn nothing_to_settle(cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_bill_not_actionable(),
        )));
    }

    /// The Pay form for `row`, or `None` when the row has nothing to pay.
    fn pay_form(&self, row: ScheduleRow, cx: &App) -> Option<PayForm> {
        let store = self.sources.bills.read(cx);
        let plan = bills::get(store.plans(cx), row.id.plan_id).filter(|_| row.is_actionable())?;
        let transactions = self.sources.transactions.read(cx).transactions(cx);
        let candidates = bills::match_candidates(
            store.plans(cx),
            store.entries(cx),
            transactions,
            self.sources.accounts.read(cx).accounts(cx),
            self.sources.categories.read(cx).categories(cx),
            row.id,
        );
        let preselected = bills::preselected_candidate(plan, &candidates, transactions);
        Some(PayForm::new(
            row.id,
            plan,
            candidates,
            preselected,
            self.today,
            self.date_style,
        ))
    }

    /// The Skip dialog (8e) on an open Schedule row; a row with nothing to skip says so instead.
    fn open_skip(&mut self, row: ScheduleRow, cx: &mut Context<'_, Self>) {
        if !row.is_actionable()
            || bills::get(self.sources.bills.read(cx).plans(cx), row.id.plan_id).is_none()
        {
            Self::nothing_to_settle(cx);
            return;
        }
        Self::open_dialog(BillsDialog::Skip(row.id), cx);
    }

    /// A Paid Schedule row's hand-off: its Matched Transaction. A no-op for any other row.
    fn open_transaction(&mut self, id: EntryId, cx: &mut Context<'_, Self>) {
        let Some(bills::Resolution::Paid(split)) =
            bills::entry(self.sources.bills.read(cx).entries(cx), id)
                .map(|entry| entry.resolution.clone())
        else {
            return;
        };
        cx.emit(ViewEvent::Reveal(Reveal::Transaction(split.transaction_id)));
    }

    pub(super) fn click_edit_plan(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.select(index, cx);
        if let Some(id) = self.selected_plan(cx) {
            self.open_edit_plan(id, cx);
        }
    }

    pub(super) fn click_pay(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.select(index, cx);
        if let Some(row) = self.selected_row(cx) {
            self.open_pay(row, cx);
        }
    }

    pub(super) fn click_skip(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.select(index, cx);
        if let Some(row) = self.selected_row(cx) {
            self.open_skip(row, cx);
        }
    }

    pub(super) fn click_view_transaction(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.select(index, cx);
        if let Some(row) = self.selected_row(cx) {
            self.open_transaction(row.id, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, TestAppContext};

    use super::*;
    use lib_ledger::Ledger;

    use crate::ledger_store::LedgerStore;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a valid date")
    }

    /// The kind of each event, so a test can compare them without building the Dialogs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Seen {
        AddPlan,
        EditPlan(u32),
        Pay(EntryId),
        Skip(EntryId),
        Status(Option<String>),
        Reveal(Reveal),
        Other,
    }

    fn seen(event: &ViewEvent) -> Seen {
        match event {
            ViewEvent::OpenDialog(dialog) => match &**dialog {
                OpenDialog::Bills(dialog) => match &**dialog {
                    BillsDialog::Add(_) => Seen::AddPlan,
                    BillsDialog::Edit(id, _) => Seen::EditPlan(*id),
                    BillsDialog::Pay(form) => Seen::Pay(form.entry),
                    BillsDialog::Skip(entry) => Seen::Skip(*entry),
                },
                _ => Seen::Other,
            },
            ViewEvent::SetStatus(text) => Seen::Status(text.clone()),
            ViewEvent::Reveal(reveal) => Seen::Reveal(*reveal),
            _ => Seen::Other,
        }
    }

    fn key(key: &str) -> Keystroke {
        Keystroke::parse(key).expect("a valid keystroke")
    }

    /// A `BillsView` over the seeded stores, with every event it emits collected.
    fn seeded_view(cx: &mut TestAppContext) -> (Entity<BillsView>, Rc<RefCell<Vec<Seen>>>) {
        crate::locale::init_for_tests();
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let sources = BillsSources {
            bills: cx.new(|cx| BillsStore::new(ledger.clone(), cx)),
            transactions: cx.new(|cx| TransactionsStore::new(ledger.clone(), cx)),
            categories: cx.new(|cx| CategoriesStore::new(ledger.clone(), cx)),
            accounts: cx.new(|cx| AccountsStore::new(ledger.clone(), cx)),
            payees: cx.new(|cx| PayeesStore::new(ledger.clone(), cx)),
            units: cx.new(|cx| UnitsStore::new(ledger, cx)),
        };
        let view = cx.new(|_| BillsView::new(sources, today()));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&view, move |_, event: &ViewEvent, _| {
                sink.borrow_mut().push(seen(event))
            })
            .detach();
        });
        (view, events)
    }

    #[gpui::test]
    fn page_keys_that_need_shell_are_emitted_as_events(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);
        view.update(cx, |view, cx| {
            assert!(view.handle_key(&key("n"), cx));
            assert!(view.handle_key(&key("p"), cx));
            assert!(!view.handle_key(&key("x"), cx));
        });
        cx.run_until_parked();
        let row = view.read_with(cx, |view, cx| view.selected_row(cx));
        let row = row.expect("the seeded Schedule has a row this month");
        assert_eq!(*events.borrow(), vec![Seen::AddPlan, Seen::Pay(row.id)]);
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
            vec![Seen::Status(None), Seen::EditPlan(id)]
        );
    }

    #[gpui::test]
    fn tab_is_claimed_ahead_of_the_router_but_shift_tab_is_not(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        view.update(cx, |view, cx| {
            assert!(view.key_ahead(&key("tab"), cx));
            assert_eq!(view.state().tab, BillsTab::Planner);
            assert!(!view.key_ahead(&key("shift-tab"), cx));
            assert_eq!(view.state().tab, BillsTab::Planner);
        });
    }

    #[gpui::test]
    fn escape_closes_the_open_list_then_leaves_the_filter_row(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        view.update(cx, |view, cx| {
            assert!(!view.escape(cx), "nothing to close yet");
            view.handle_key(&key("f"), cx);
            view.apply_movement(Movement::Enter, cx);
            assert!(view.escape(cx));
            assert!(view.state().filter_focus.is_some());
            assert!(view.escape(cx));
            assert!(view.state().filter_focus.is_none());
        });
    }

    #[gpui::test]
    fn revealing_a_plan_selects_it_on_the_planner(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        let last = view.read_with(cx, |view, cx| {
            bills::planner_order(view.sources.bills.read(cx).plans(cx))
                .last()
                .map(|plan| plan.id)
                .expect("the seed has Bill Plans")
        });
        view.update(cx, |view, cx| view.reveal(Reveal::BillPlan(last), cx));
        view.read_with(cx, |view, cx| {
            assert_eq!(view.state().tab, BillsTab::Planner);
            assert_eq!(view.selected_plan(cx), Some(last));
        });
    }

    #[gpui::test]
    fn page_status_follows_the_tab(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        let keys = |view: &Entity<BillsView>, cx: &mut TestAppContext| {
            view.read_with(cx, |view, cx| {
                view.page_status(cx)
                    .expect("Bills always has a page status")
                    .hints
                    .into_iter()
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>()
            })
        };
        assert!(keys(&view, cx).contains(&"p"), "the Schedule offers Pay");
        view.update(cx, |view, cx| view.set_tab(BillsTab::Planner, cx));
        let planner = keys(&view, cx);
        assert!(planner.contains(&"e") && !planner.contains(&"p"));
    }

    #[gpui::test]
    fn a_bills_dialog_brings_its_own_hints(cx: &mut TestAppContext) {
        use crate::chrome::dialog_host::Dialog;
        let (view, _) = seeded_view(cx);
        let row = view
            .read_with(cx, |view, cx| view.selected_row(cx))
            .expect("the seeded Schedule has a row this month");
        let skip = OpenDialog::Bills(Box::new(BillsDialog::Skip(row.id)));
        let keys: Vec<_> = skip
            .hints()
            .expect("a Bills dialog has hints")
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, vec!["esc", "enter"]);
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
