//! The Budgets destination's Entities (ADR-0032, ADR-0037). The Budgets are read through
//! [`LedgerStore`], which holds the only copy. Categories 5c and the Dashboard read them
//! showing. It renders itself (`impl Render` in `super`), handles its own keys, `Esc` and
//! movement, and reaches `Shell` only through [`ViewEvent`]. It builds the Budgets dialogs, and
//! `Shell` asks it for the ones the palette and the other dialogs open.

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription};
use lib_core::CategoryTypes;
use lib_ledger::{CycleBudgetRollover, SaveBudgetCell};
use lib_toast::ToastKind;

use super::hints::{
    budgets_history_hints, budgets_plan_hints, budgets_plan_insert_hints, budgets_progress_hints,
};
use crate::{
    budgets::{
        self, BudgetsDialog, BudgetsTab, PlanEdit, default_plan_start,
        limit_form::{LimitForm, LimitOptions, StopForm},
    },
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    period::Period,
    view::event::{Reveal, ViewEvent, ViewMode},
};
use lib_ledger::Domain;

/// The file name the History export's save dialog suggests.
const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

impl EventEmitter<ViewEvent> for BudgetsView {}

/// The Budgets page's own state, owned by its Entity.
#[derive(Debug)]
pub struct BudgetsState {
    /// The Budget the Budgets surface shows. The last one opened is a Client-scoped Preference, so
    /// this starts on the default Budget; the stub doesn't persist it.
    pub current: u32,
    pub tab: BudgetsTab,
    /// The Progress tab's calendar month; starts at today's.
    pub period: Period,
    /// The selected Progress row (a position in `PeriodFigures::rows`).
    pub selected: usize,
    /// The Plan grid's first month; it shows six from here.
    pub plan_start: Period,
    /// The Plan grid's cursor: a row across sections, and a column (the last is ROLLOVER).
    pub plan_cursor: (usize, usize),
    /// The Plan cell being typed into; `InputMode::Insert` is on for exactly as long as this is set.
    pub plan_edit: Option<PlanEdit>,
    /// The last month of the History tab's range (9c).
    pub history_end: Period,
    /// The History cursor: a row and a month column.
    pub history_cursor: (usize, usize),
}

impl BudgetsState {
    pub fn new(current: u32, today: NaiveDate) -> Self {
        Self {
            current,
            tab: BudgetsTab::default(),
            period: Period::of(today),
            selected: 0,
            plan_start: default_plan_start(today),
            plan_cursor: (0, 0),
            plan_edit: None,
            history_end: Period::of(today),
            history_cursor: (0, 0),
        }
    }
}

/// The store Entities the Budgets page reads, and the Ledger its Plan cells write through.
#[derive(Clone)]
pub struct BudgetsSources {
    pub ledger: Entity<LedgerStore>,
}

/// The Budgets Entity.
pub struct BudgetsView {
    pub(super) sources: BudgetsSources,
    pub(super) today: NaiveDate,
    /// The page's own scroll, reset as it becomes the destination, as its tab changes and as
    /// another Budget is shown.
    pub(super) scroll: ScrollHandle,
    pub(super) state: BudgetsState,
    /// Repaints the page when the Accounts its rows and filters name change.
    _accounts_changed: Subscription,
    _categories_changed: Subscription,
}

impl BudgetsView {
    /// The page on `current`, as of `today`.
    pub fn new(
        sources: BudgetsSources,
        current: u32,
        today: NaiveDate,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let _accounts_changed = notify_on(&sources.ledger, Domain::Accounts, cx);
        let _categories_changed = notify_on(&sources.ledger, Domain::Categories, cx);
        Self {
            sources,
            today,
            scroll: ScrollHandle::new(),
            state: BudgetsState::new(current, today),
            _accounts_changed,
            _categories_changed,
        }
    }

    pub fn state(&self) -> &BudgetsState {
        &self.state
    }

    /// The Budget on show's id.
    pub fn current(&self) -> u32 {
        self.state.current
    }

    pub fn budgets<'a>(&self, cx: &'a App) -> &'a budgets::Budgets {
        self.sources.ledger.read(cx).ledger().budgets()
    }

    /// The Budget on show, or `None` once it has gone.
    pub fn budget<'a>(&self, cx: &'a App) -> Option<&'a budgets::Budget> {
        self.budgets(cx).get(self.state.current)
    }

    pub fn categories<'a>(&self, cx: &'a App) -> &'a [lib_categories::Category] {
        self.sources.ledger.read(cx).ledger().categories()
    }

    /// Everything the Budgets figures are worked out from.
    pub fn budget_sources<'a>(&self, cx: &'a App) -> budgets::BudgetSources<'a> {
        let ledger = self.sources.ledger.read(cx).ledger();
        budgets::BudgetSources {
            categories: self.categories(cx),
            accounts: self.sources.ledger.read(cx).ledger().accounts(),
            transactions: self.sources.ledger.read(cx).ledger().transactions(),
            plans: ledger.bill_plans(),
            entries: ledger.bill_entries(),
        }
    }

    /// The Budget on show and its figures for the Progress month, as of today.
    pub fn figures<'a>(
        &self,
        cx: &'a App,
    ) -> Option<(&'a budgets::Budget, budgets::PeriodFigures)> {
        let budget = self.budget(cx)?;
        let figures = budgets::period_figures(
            budget,
            &self.budget_sources(cx),
            self.state.period,
            self.today,
        );
        Some((budget, figures))
    }

    /// The Plan tab's grid for the range in view.
    pub fn plan_data(&self, cx: &App) -> Option<budgets::Plan> {
        let budget = self.budget(cx)?;
        Some(budgets::plan(
            budget,
            &self.budget_sources(cx),
            self.state.plan_start,
            self.today,
        ))
    }

    /// The cursor held inside the grid, so a row that vanished can't leave it dangling.
    pub fn plan_cursor_in(&self, plan: &budgets::Plan) -> (usize, usize) {
        let cursor = self.state.plan_cursor;
        (
            cursor.0.min(plan.row_count().saturating_sub(1)),
            cursor.1.min(budgets::PLAN_ROLLOVER_COLUMN),
        )
    }

    /// The History tab's chart and table for the range in view.
    pub fn history_data(&self, cx: &App) -> Option<budgets::History> {
        let budget = self.budget(cx)?;
        let (first, last) = budgets::history_range(budget, self.state.history_end, self.today)?;
        Some(budgets::history(
            budget,
            &self.budget_sources(cx),
            first,
            last,
            self.today,
        ))
    }

    /// The cursor held inside the table, so a shorter range can't leave it dangling.
    pub fn history_cursor_in(&self, history: &budgets::History) -> (usize, usize) {
        let cursor = self.state.history_cursor;
        (
            cursor.0.min(history.rows.len().saturating_sub(1)),
            cursor.1.min(history.months.len().saturating_sub(1)),
        )
    }

    /// The Budget on show when it takes edits: an archived one is read-only.
    pub fn editable<'a>(&self, cx: &'a App) -> Option<&'a budgets::Budget> {
        self.budgets(cx)
            .get(self.state.current)
            .filter(|budget| !budget.is_archived())
    }

    /// Whether 9d offers Edit budget: a leaf Expense Category in a Budget that takes edits.
    pub fn detail_editable(&self, category_id: u32, cx: &App) -> bool {
        let categories = self.categories(cx);
        self.editable(cx).is_some()
            && lib_categories::is_leaf(categories, category_id)
            && categories.iter().any(|category| {
                category.id == category_id && category.category_type == CategoryTypes::Expense
            })
    }

    /// The month Fill would target from the Plan cursor: the first open month at or after its
    /// column.
    pub fn fill_target(&self, cx: &App) -> Period {
        let cursor = self.plan_data(cx).and_then(|plan| {
            let column = self.plan_cursor_in(&plan).1;
            plan.months.get(column).copied()
        });
        budgets::fill_target(cursor, self.today)
    }

    /// The Category under the cursor on Progress or Plan, with the month its dialog starts in.
    pub fn cursor_category(&self, cx: &App) -> Option<(u32, Period)> {
        match self.state.tab {
            BudgetsTab::Progress => {
                let (_, figures) = self.figures(cx)?;
                let row = figures.rows.get(self.state.selected)?;
                (!row.is_parent).then_some((row.category_id, self.state.period))
            }
            BudgetsTab::Plan => {
                let plan = self.plan_data(cx)?;
                let (row_index, column) = self.plan_cursor_in(&plan);
                let month = plan
                    .months
                    .get(column)
                    .copied()
                    .unwrap_or_else(|| Period::of(self.today));
                Some((plan.row(row_index)?.category_id, month))
            }
            BudgetsTab::History => None,
        }
    }

    /// 9d's figures for `category_id` in `month`, or `None` once its Budget or Category is gone.
    pub fn category_detail(
        &self,
        category_id: u32,
        month: Period,
        cx: &App,
    ) -> Option<budgets::CategoryDetail> {
        let budget = self.budget(cx)?;
        budgets::category_detail(
            budget,
            &self.budget_sources(cx),
            category_id,
            month,
            self.today,
        )
    }

    /// The Starting / From options for the Budget on show: `count` months from the current one.
    fn limit_options(&self, count: usize, cx: &App) -> Option<LimitOptions> {
        let budget = self.budget(cx)?;
        Some(LimitOptions::new(
            budget,
            self.categories(cx),
            Period::of(self.today),
            count,
            |month, current| {
                let label = super::period_label(month);
                if current {
                    crate::msg::desktop_budgets_limit_month_current(&label)
                } else {
                    label
                }
            },
        ))
    }

    // The dialogs. Each builder only reads, so `Shell` can open the same dialog from the palette
    // or from another dialog; the page opens them through `ViewEvent::OpenDialog`.

    /// 11b, with the Budget on show highlighted.
    pub fn switcher_dialog(&self, cx: &App) -> BudgetsDialog {
        BudgetsDialog::Switcher(budgets::Switcher::new(self.budgets(cx), self.state.current))
    }

    /// 11f, with the Budget on show under the cursor.
    pub fn manage_dialog(&self, cx: &App) -> BudgetsDialog {
        BudgetsDialog::Manage(budgets::Manage::new(self.budgets(cx), self.state.current))
    }

    /// 11c, with the Budget on show as what "Copy categories from" names.
    pub fn new_budget_dialog(&self, cx: &App) -> BudgetsDialog {
        BudgetsDialog::Budget(budgets::form::BudgetForm::new(
            self.sources.ledger.read(cx).ledger().accounts(),
            self.sources.ledger.read(cx).ledger().units(),
            self.budget(cx),
        ))
    }

    /// 11c's edit mode on Budget `id`: rename it or change its Accounts. An archived Budget is
    /// read-only, so it has none.
    pub fn edit_budget_dialog(&self, id: u32, cx: &App) -> Option<BudgetsDialog> {
        let budget = self
            .budgets(cx)
            .get(id)
            .filter(|budget| !budget.is_archived())?;
        Some(BudgetsDialog::Budget(
            budgets::form::BudgetForm::from_budget(
                budget,
                self.sources.ledger.read(cx).ledger().accounts(),
                self.sources.ledger.read(cx).ledger().units(),
            ),
        ))
    }

    /// 9e on a leaf Category, starting in `month` when that is offered. A Category with no Budget
    /// Amount this month gets the picker with it preselected (9a's `set`); a parent only rolls up,
    /// so it gets none.
    pub fn limit_dialog(&self, category_id: u32, month: Period, cx: &App) -> Option<BudgetsDialog> {
        let budget = self.editable(cx)?;
        let options = self.limit_options(budgets::limit_form::START_MONTHS, cx)?;
        let current = Period::of(self.today);
        let categories = self.categories(cx);
        let form = if budgets::applied(budget.chain(category_id), current).is_some() {
            if !lib_categories::is_leaf(categories, category_id) {
                return None;
            }
            LimitForm::edit(budget, category_id, month, &options)
        } else if budgets::unbudgeted_leaves(budget, categories, current).contains(&category_id) {
            LimitForm::pick(Some(category_id), &options)
        } else {
            return None;
        };
        Some(BudgetsDialog::EditLimit(form))
    }

    /// **+ Budget a category**: 9e with its Category picker.
    pub fn limit_picker_dialog(&self, cx: &App) -> Option<BudgetsDialog> {
        self.editable(cx)?;
        let options = self.limit_options(budgets::limit_form::START_MONTHS, cx)?;
        Some(BudgetsDialog::EditLimit(LimitForm::pick(None, &options)))
    }

    /// 9g on a leaf Category that has a Budget Amount this month or in `month`.
    pub fn stop_dialog(&self, category_id: u32, month: Period, cx: &App) -> Option<BudgetsDialog> {
        let budget = self.editable(cx)?;
        let options = self.limit_options(budgets::limit_form::STOP_MONTHS, cx)?;
        let chain = budget.chain(category_id);
        let current = Period::of(self.today);
        if budgets::applied(chain, current).is_none() && budgets::applied(chain, month).is_none() {
            return None;
        }
        Some(BudgetsDialog::Stop(StopForm::new(
            category_id,
            month,
            &options,
        )))
    }

    /// 9f on the Plan cursor's target month.
    fn fill_dialog(&self, cx: &App) -> Option<BudgetsDialog> {
        self.editable(cx)?;
        Some(BudgetsDialog::Fill {
            month: self.fill_target(cx),
            source: budgets::FillSource::default(),
        })
    }

    /// 9d, counting the Transactions it lists now so its `j`/`k` never need the ledger.
    fn detail_dialog(&self, category_id: u32, month: Period, cx: &App) -> BudgetsDialog {
        let listed = self
            .category_detail(category_id, month, cx)
            .map_or(0, |detail| {
                detail.lines.len().min(super::detail_dialog::LISTED)
            });
        BudgetsDialog::CategoryDetail(budgets::Detail::new(category_id, month, listed))
    }

    /// Opens `dialog` when there is one. A Plan cell being typed into is abandoned first, so the
    /// Insert it opened never outlives it behind the Dialog.
    fn open_dialog(&mut self, dialog: Option<BudgetsDialog>, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = dialog {
            self.abandon_plan_edit(cx);
            cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Budgets(
                Box::new(dialog),
            ))));
        }
    }

    // The interface `Shell` forwards to (ADR-0037).

    /// Keys the page claims before the router: every key typed into a Plan cell (9b, Insert
    /// mode), bar `esc`, which comes back through [`Self::escape`]; and `tab`, which switches the
    /// tab (the handoff's `tab switch view`) rather than cycling focus. `shift-tab` still cycles
    /// focus, so the View zone can always be left.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if self.state.plan_edit.is_some() {
            return self.plan_edit_key(keystroke, cx);
        }
        if keystroke.key != "tab" || keystroke.modifiers.shift {
            return false;
        }
        self.set_tab(self.state.tab.next(), cx);
        true
    }

    /// `B` opens the Switcher and `n` New budget; `[`/`]` step the period and `1`/`2`/`3` pick the
    /// tab; `c` budgets a Category, and on Progress `e` edits the row's budget and `s` stops it.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        // Bare `b` stays the rail toggle, so the Switcher takes the shifted key (#400).
        if keystroke.modifiers.shift {
            if keystroke.key.eq_ignore_ascii_case("b") {
                self.open_dialog(Some(self.switcher_dialog(cx)), cx);
                return true;
            }
            return false;
        }
        if let Some(tab) = keystroke
            .key
            .chars()
            .next()
            .filter(|_| keystroke.key.chars().count() == 1)
            .and_then(BudgetsTab::from_digit)
        {
            self.set_tab(tab, cx);
            return true;
        }
        let tab = self.state.tab;
        match keystroke.key.as_str() {
            "c" if tab != BudgetsTab::History => {
                self.open_dialog(self.limit_picker_dialog(cx), cx);
                return true;
            }
            "n" => {
                self.open_dialog(Some(self.new_budget_dialog(cx)), cx);
                return true;
            }
            "e" | "s" if tab == BudgetsTab::Progress => {
                if let Some((category_id, month)) = self.cursor_category(cx) {
                    let dialog = if keystroke.key == "e" {
                        self.limit_dialog(category_id, month, cx)
                    } else {
                        self.stop_dialog(category_id, month, cx)
                    };
                    self.open_dialog(dialog, cx);
                }
                return true;
            }
            _ => {}
        }
        match tab {
            BudgetsTab::Plan => self.plan_key(keystroke.key.as_str(), cx),
            BudgetsTab::History => self.history_key(keystroke.key.as_str(), cx),
            BudgetsTab::Progress => match keystroke.key.as_str() {
                "[" => {
                    self.shift_period(false, cx);
                    true
                }
                "]" => {
                    self.shift_period(true, cx);
                    true
                }
                _ => false,
            },
        }
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the active tab's rows. `enter` opens the Category
    /// detail (9d) on Progress and History, and edits the cursor cell on Plan.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        match self.state.tab {
            BudgetsTab::Plan => {
                let Some(plan) = self.plan_data(cx) else {
                    return;
                };
                let selected = self.plan_cursor_in(&plan).0;
                match movement.step() {
                    Some(step) => {
                        self.state.plan_cursor.0 =
                            lib_list_cursor::step(selected, plan.row_count(), step);
                    }
                    None => {
                        self.state.plan_cursor.0 = selected;
                        self.start_plan_edit(cx);
                    }
                }
            }
            BudgetsTab::History => {
                let Some(history) = self.history_data(cx) else {
                    return;
                };
                let selected = self.history_cursor_in(&history).0;
                match movement.step() {
                    Some(step) => {
                        self.state.history_cursor.0 =
                            lib_list_cursor::step(selected, history.rows.len(), step);
                    }
                    None => self.open_history_detail(cx),
                }
            }
            BudgetsTab::Progress => {
                let len = self
                    .figures(cx)
                    .map_or(0, |(_, figures)| figures.rows.len());
                let selected = self.state.selected.min(len.saturating_sub(1));
                match movement.step() {
                    Some(step) => self.state.selected = lib_list_cursor::step(selected, len, step),
                    None => {
                        self.state.selected = selected;
                        self.open_detail_at(selected, cx);
                    }
                }
            }
        }
        cx.notify();
    }

    /// `Esc` abandons a Plan cell being typed into and leaves Insert. Nothing else on the page
    /// holds an `Esc` of its own.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let abandoned = self.abandon_plan_edit(cx);
        cx.notify();
        abandoned
    }

    /// Drops the Plan cell being typed into, if any, and leaves the Insert it opened. Returns
    /// whether there was one.
    fn abandon_plan_edit(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.state.plan_edit.take().is_none() {
            return false;
        }
        cx.emit(ViewEvent::ExitMode);
        true
    }

    /// Budgets has no Search, so `/` is inert.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Back to the top of the page as it becomes the destination.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        // An edit left open when the page was left doesn't come back with it.
        self.abandon_plan_edit(cx);
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// No other View hands a record to Budgets yet.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The status line's legend for the active tab, and its right-hand count.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(match self.state.tab {
            BudgetsTab::Progress => PageStatus {
                hints: budgets_progress_hints(),
                right: crate::msg::desktop_budgets_status_period(
                    &super::period_label(self.state.period),
                    self.figures(cx).map_or(0, |(_, figures)| {
                        i64::try_from(figures.rows.len()).unwrap_or(i64::MAX)
                    }),
                ),
            },
            BudgetsTab::Plan => PageStatus {
                hints: if self.state.plan_edit.is_some() {
                    budgets_plan_insert_hints()
                } else {
                    budgets_plan_hints()
                },
                right: self.plan_data(cx).map_or_else(String::new, |plan| {
                    crate::msg::desktop_budgets_status_plan(
                        &super::plan::range_label(&plan),
                        i64::try_from(plan.row_count()).unwrap_or(i64::MAX),
                    )
                }),
            },
            BudgetsTab::History => PageStatus {
                hints: budgets_history_hints(),
                right: self.history_data(cx).map_or_else(String::new, |history| {
                    crate::msg::desktop_budgets_status_history(
                        i64::try_from(history.leaves_ever_budgeted).unwrap_or(i64::MAX),
                        &history.leaves_shown.to_string(),
                    )
                }),
            },
        })
    }

    // What `Shell` asks of the page from outside it.

    /// Shows Budget `id`, keeping the active tab (every method built has all three) and putting
    /// each tab's cursor and range back at its start.
    pub fn switch_budget(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        if self.budgets(cx).get(id).is_none() {
            return;
        }
        self.abandon_plan_edit(cx);
        let today = self.today;
        let state = &mut self.state;
        state.current = id;
        state.selected = 0;
        state.plan_cursor = (0, 0);
        state.plan_start = default_plan_start(today);
        state.history_cursor = (0, 0);
        state.history_end = Period::of(today);
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    // The page's own actions, reached by keys and by clicks alike.

    pub(super) fn set_tab(&mut self, tab: BudgetsTab, cx: &mut Context<'_, Self>) {
        self.state.tab = tab;
        self.state.selected = 0;
        self.scroll.set_offset(gpui::Point::default());
        cx.emit(ViewEvent::SetStatus(None));
        cx.notify();
    }

    /// Steps the Progress month a calendar month.
    pub(super) fn shift_period(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        self.state.period = if forward {
            self.state.period.next()
        } else {
            self.state.period.prev()
        };
        self.state.selected = 0;
        cx.notify();
    }

    /// The `‹ ›` beside the tabs: the Plan or History range on those tabs.
    pub(super) fn shift_range(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        if self.state.tab == BudgetsTab::History {
            self.shift_history_range(forward, cx);
        } else {
            self.shift_plan_range(forward, cx);
        }
    }

    /// Steps the Plan range a month, inside the Budget's bounds.
    fn shift_plan_range(&mut self, forward: bool, cx: &mut Context<'_, Self>) -> bool {
        let Some(budget) = self.budget(cx) else {
            return false;
        };
        let (earliest, latest) = budgets::plan_start_bounds(budget, self.today);
        let target = if forward {
            self.state.plan_start.next()
        } else {
            self.state.plan_start.prev()
        };
        if target < earliest || target > latest {
            return false;
        }
        self.state.plan_start = target;
        cx.notify();
        true
    }

    /// Steps the History range a month, inside the Budget's bounds.
    fn shift_history_range(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        let Some(budget) = self.budget(cx) else {
            return;
        };
        // Step from where the range really ends, not from a stale out-of-bounds end.
        let Some((_, last)) = budgets::history_range(budget, self.state.history_end, self.today)
        else {
            return;
        };
        let target = if forward { last.next() } else { last.prev() };
        if let Some((_, held)) = budgets::history_range(budget, target, self.today) {
            self.state.history_end = held;
            cx.notify();
        }
    }

    /// `h`/`l` on the History tab: the month cursor, stopping at either end of the range.
    fn step_history_month(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        let Some(history) = self.history_data(cx) else {
            return;
        };
        let (row, column) = self.history_cursor_in(&history);
        let column = if forward {
            (column + 1).min(history.months.len().saturating_sub(1))
        } else {
            column.saturating_sub(1)
        };
        self.state.history_cursor = (row, column);
        cx.notify();
    }

    /// The History tab's Normal-mode keys. `false` for any it doesn't take.
    fn history_key(&mut self, key: &str, cx: &mut Context<'_, Self>) -> bool {
        match key {
            "[" => self.shift_history_range(false, cx),
            "]" => self.shift_history_range(true, cx),
            "h" | "left" => self.step_history_month(false, cx),
            "l" | "right" => self.step_history_month(true, cx),
            "x" => self.export_history(cx),
            _ => return false,
        }
        true
    }

    /// The Plan grid's Normal-mode keys. `false` for any it doesn't take.
    fn plan_key(&mut self, key: &str, cx: &mut Context<'_, Self>) -> bool {
        match key {
            "h" | "left" => {
                self.state.plan_cursor.1 = self.state.plan_cursor.1.saturating_sub(1);
            }
            "l" | "right" => {
                self.state.plan_cursor.1 =
                    (self.state.plan_cursor.1 + 1).min(budgets::PLAN_ROLLOVER_COLUMN);
            }
            "i" => self.start_plan_edit(cx),
            "r" => {
                if let Some(plan) = self.plan_data(cx)
                    && let Some(row) = plan.row(self.plan_cursor_in(&plan).0)
                {
                    self.cycle_rollover(row.category_id, cx);
                }
            }
            "f" => self.open_dialog(self.fill_dialog(cx), cx),
            "x" | "backspace" => self.write_plan_cell("", cx),
            "0" => self.write_plan_cell("0", cx),
            "[" => {
                self.shift_plan_range(false, cx);
            }
            "]" => {
                self.shift_plan_range(true, cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Keys typed into a Plan cell (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s 9b,
    /// Insert mode). `esc` is left to the router, which hands it back as [`Self::escape`].
    fn plan_edit_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let shift = keystroke.modifiers.shift;
        match keystroke.key.as_str() {
            "escape" => return false,
            "enter" => self.commit_plan_edit(
                if shift {
                    budgets::Span::MonthOnly
                } else {
                    budgets::Span::Onward
                },
                0,
                cx,
            ),
            "tab" => self.commit_plan_edit(budgets::Span::Onward, if shift { -1 } else { 1 }, cx),
            "backspace" => {
                if let Some(edit) = self.state.plan_edit.as_mut() {
                    edit.backspace();
                }
            }
            _ => {
                if let (Some(text), false, Some(edit)) = (
                    keystroke.key_char.as_deref(),
                    keystroke.modifiers.control,
                    self.state.plan_edit.as_mut(),
                ) {
                    for c in text.chars() {
                        edit.type_char(c);
                    }
                }
            }
        }
        cx.notify();
        true
    }

    /// Runs a Budgets command on the Ledger, which announces the change to every reader.
    fn apply<C>(&self, command: C, cx: &mut Context<'_, Self>) -> Result<C::Outcome, C::Refusal>
    where
        C: lib_ledger::LedgerCommand,
    {
        self.sources
            .ledger
            .update(cx, |store, cx| store.apply(cx, command))
    }

    fn cycle_rollover(&self, category_id: u32, cx: &mut Context<'_, Self>) {
        let _ = self.apply(
            CycleBudgetRollover {
                budget_id: self.state.current,
                category_id,
                today: self.today,
            },
            cx,
        );
    }

    /// `enter`/`i`/a click on the cursor cell: types into an open month, or cycles Rollover.
    fn start_plan_edit(&mut self, cx: &mut Context<'_, Self>) {
        let Some(plan) = self.plan_data(cx) else {
            return;
        };
        let (row_index, column) = self.plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        if column == budgets::PLAN_ROLLOVER_COLUMN {
            self.cycle_rollover(row.category_id, cx);
            return;
        }
        let Some(cell) = row.cells.get(column) else {
            return;
        };
        if cell.closed || self.editable(cx).is_none() {
            return;
        }
        self.state.plan_edit = Some(PlanEdit::new(
            row.category_id,
            cell.month,
            cell.amount.as_ref(),
        ));
        cx.emit(ViewEvent::EnterMode(ViewMode::Insert));
        cx.notify();
    }

    /// Saves the typed cell and leaves Insert; `step` moves on to the next (1) or previous (-1)
    /// month's cell and edits it, as `tab`/`shift+tab` do. Text that isn't an amount keeps the
    /// cell open.
    fn commit_plan_edit(&mut self, span: budgets::Span, step: i32, cx: &mut Context<'_, Self>) {
        let Some(edit) = self.state.plan_edit.take() else {
            return;
        };
        let saved = self.apply(
            SaveBudgetCell {
                budget_id: self.state.current,
                category_id: edit.category_id,
                month: edit.month,
                span,
                text: edit.text.clone(),
                today: self.today,
            },
            cx,
        );
        if saved == Err(budgets::BudgetError::InvalidAmount) {
            self.state.plan_edit = Some(edit);
            return;
        }
        cx.emit(ViewEvent::ExitMode);
        if step == 0 {
            return;
        }
        let column = self.state.plan_cursor.1;
        if step > 0 {
            if column + 1 < budgets::PLAN_MONTHS {
                self.state.plan_cursor.1 += 1;
            } else if !self.shift_plan_range(true, cx) {
                return;
            }
        } else if column > 0 {
            self.state.plan_cursor.1 -= 1;
        } else if !self.shift_plan_range(false, cx) {
            return;
        }
        self.start_plan_edit(cx);
    }

    /// Writes straight to the cursor's month cell in Normal mode: `x`/`backspace` clear it (a
    /// Stop from that month) and `0` writes an explicit 0.00 onward.
    fn write_plan_cell(&self, text: &str, cx: &mut Context<'_, Self>) {
        let Some(plan) = self.plan_data(cx) else {
            return;
        };
        let (row_index, column) = self.plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        let Some(cell) = row.cells.get(column).filter(|cell| !cell.closed) else {
            return;
        };
        let _ = self.apply(
            SaveBudgetCell {
                budget_id: self.state.current,
                category_id: row.category_id,
                month: cell.month,
                span: budgets::Span::Onward,
                text: text.to_string(),
                today: self.today,
            },
            cx,
        );
    }

    /// Opens 9d on the Progress row at `index`, for the month being shown.
    fn open_detail_at(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(category_id) = self
            .figures(cx)
            .and_then(|(_, figures)| figures.rows.get(index).map(|row| row.category_id))
        else {
            return;
        };
        self.open_dialog(
            Some(self.detail_dialog(category_id, self.state.period, cx)),
            cx,
        );
    }

    /// `enter` on a History cell: 9d for that Category and month (a parent shows its rollup).
    fn open_history_detail(&mut self, cx: &mut Context<'_, Self>) {
        let Some(history) = self.history_data(cx) else {
            return;
        };
        let (row, column) = self.history_cursor_in(&history);
        let (Some(row), Some(month)) = (history.rows.get(row), history.months.get(column)) else {
            return;
        };
        self.open_dialog(
            Some(self.detail_dialog(row.category_id, month.month, cx)),
            cx,
        );
    }

    /// **Export CSV** and `x` on History: asks where to save through the platform's save dialog,
    /// writes the visible History table there and raises a Toast naming the path. A cancelled
    /// dialog does nothing.
    pub(super) fn export_history(&mut self, cx: &mut Context<'_, Self>) {
        let Some(history) = self.history_data(cx) else {
            return;
        };
        let labels = budgets::HistoryCsvLabels {
            category: lib_locale::msg::column_category(),
            budget: crate::msg::desktop_budgets_column_budget(),
            spent: crate::msg::desktop_budgets_column_spent(),
            average: crate::msg::desktop_budgets_history_column_avg(),
            over: crate::msg::desktop_budgets_history_column_over(),
        };
        let csv = budgets::history_csv(&history, self.categories(cx), &labels, super::period_label);
        let directory = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        let chosen = cx.prompt_for_new_path(&directory, Some(BUDGET_HISTORY_FILE));
        cx.spawn(async move |this, cx| {
            let outcome = match chosen.await {
                Ok(Ok(Some(path))) => std::fs::write(&path, csv)
                    .map(|()| path.display().to_string())
                    .map_err(|error| error.to_string()),
                Ok(Err(error)) => Err(error.to_string()),
                // Cancelled, or the dialog went away.
                Ok(Ok(None)) | Err(_) => return,
            };
            let (kind, text) = match outcome {
                Ok(path) => (
                    ToastKind::Success,
                    crate::msg::desktop_budgets_history_exported(&path),
                ),
                Err(error) => (
                    ToastKind::Error,
                    crate::msg::desktop_budgets_history_export_failed(&error),
                ),
            };
            this.update(cx, |_, cx| cx.emit(ViewEvent::RaiseToast { kind, text }))
                .ok();
        })
        .detach();
    }

    // Clicks.

    /// The title is the Switcher.
    pub(super) fn click_title(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(Some(self.switcher_dialog(cx)), cx);
    }

    pub(super) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(self.limit_picker_dialog(cx), cx);
    }

    pub(super) fn click_fill(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(self.fill_dialog(cx), cx);
    }

    /// The KNOWN COSTS stat's link: the Bills Schedule on the period being shown.
    pub(super) fn click_known(&mut self, cx: &mut Context<'_, Self>) {
        self.abandon_plan_edit(cx);
        cx.emit(ViewEvent::Reveal(Reveal::BillsPeriod(self.state.period)));
    }

    /// A click selects a Progress row; clicking the selected row opens its detail.
    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let was_selected = self.state.selected == index;
        self.state.selected = index;
        if was_selected {
            self.open_detail_at(index, cx);
        }
        cx.notify();
    }

    /// A Progress row's `edit` or `set`.
    pub(super) fn click_action(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.state.selected = index;
        if let Some((category_id, month)) = self.cursor_category(cx) {
            self.open_dialog(self.limit_dialog(category_id, month, cx), cx);
        }
        cx.notify();
    }

    /// A click puts the cursor on the cell; on an open month it also starts typing, and on the
    /// ROLLOVER cell it cycles the mode. A click elsewhere abandons an edit in progress.
    pub(super) fn click_plan_cell(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.abandon_plan_edit(cx);
        self.state.plan_cursor = (row, column);
        self.start_plan_edit(cx);
        cx.notify();
    }

    /// A click on a History cell moves the cursor there; a second click opens its detail.
    pub(super) fn click_history_cell(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let was_here = self.state.history_cursor == (row, column);
        self.state.history_cursor = (row, column);
        if was_here {
            self.open_history_detail(cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, TestAppContext};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 15).expect("a valid date")
    }

    /// What the page asked of `Shell`, kept as plain values the tests can compare.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Seen {
        Dialog(String),
        EnterInsert,
        ExitInsert,
        Reveal(Reveal),
        Other,
    }

    fn seen(event: &ViewEvent) -> Seen {
        match event {
            ViewEvent::OpenDialog(dialog) => match &**dialog {
                OpenDialog::Budgets(dialog) => Seen::Dialog(
                    match &**dialog {
                        BudgetsDialog::Switcher(_) => "Switcher",
                        BudgetsDialog::Budget(_) => "Budget",
                        BudgetsDialog::Manage(_) => "Manage",
                        BudgetsDialog::CategoryDetail(_) => "Detail",
                        BudgetsDialog::EditLimit(_) => "EditLimit",
                        BudgetsDialog::Fill { .. } => "Fill",
                        BudgetsDialog::Stop(_) => "Stop",
                    }
                    .to_string(),
                ),
                _ => Seen::Other,
            },
            ViewEvent::EnterMode(ViewMode::Insert) => Seen::EnterInsert,
            ViewEvent::ExitMode => Seen::ExitInsert,
            ViewEvent::Reveal(reveal) => Seen::Reveal(*reveal),
            _ => Seen::Other,
        }
    }

    /// A keystroke as the platform delivers it: a printable key carries its character.
    fn key(key: &str) -> Keystroke {
        let mut keystroke = Keystroke::parse(key).expect("a valid keystroke");
        if keystroke.key.chars().count() == 1 && !keystroke.modifiers.shift {
            keystroke.key_char = Some(keystroke.key.clone());
        }
        keystroke
    }

    fn seeded_view(cx: &mut TestAppContext) -> (Entity<BudgetsView>, Rc<RefCell<Vec<Seen>>>) {
        crate::locale::init_for_tests();
        let ledger = cx.new(|_| LedgerStore::new(lib_ledger::Ledger::stub_on(today())));
        let current = ledger.read_with(cx, |store, _| {
            store
                .ledger()
                .budgets()
                .default_budget()
                .map_or(budgets::PERSONAL_SPENDING_ID, |budget| budget.id)
        });
        let sources = BudgetsSources {
            ledger: ledger.clone(),
        };
        let view = cx.new(|cx| BudgetsView::new(sources, current, today(), cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&view, move |_, event: &ViewEvent, _| {
                sink.borrow_mut().push(seen(event));
            })
            .detach();
        });
        (view, events)
    }

    fn press(view: &Entity<BudgetsView>, keystroke: &str, cx: &mut TestAppContext) -> bool {
        let keystroke = key(keystroke);
        let taken = view.update(cx, |view, cx| {
            view.key_ahead(&keystroke, cx) || view.handle_key(&keystroke, cx)
        });
        cx.run_until_parked();
        taken
    }

    #[gpui::test]
    fn tab_is_claimed_ahead_of_the_router_but_shift_tab_is_not(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);

        let tab = key("tab");
        assert!(view.update(cx, |view, cx| view.key_ahead(&tab, cx)));
        view.read_with(cx, |view, _| assert_eq!(view.state().tab, BudgetsTab::Plan));
        let shift_tab = key("shift-tab");
        assert!(!view.update(cx, |view, cx| view.key_ahead(&shift_tab, cx)));
    }

    #[gpui::test]
    fn the_page_opens_its_dialogs_through_view_events(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);

        press(&view, "shift-b", cx);
        press(&view, "n", cx);
        view.update(cx, |view, cx| view.apply_movement(Movement::Enter, cx));
        cx.run_until_parked();

        assert_eq!(
            *events.borrow(),
            vec![
                Seen::Dialog("Switcher".to_string()),
                Seen::Dialog("Budget".to_string()),
                Seen::Dialog("Detail".to_string()),
            ]
        );
    }

    #[gpui::test]
    fn a_plan_cell_edit_asks_for_insert_and_esc_abandons_it(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);
        let draft = |view: &Entity<BudgetsView>, cx: &mut TestAppContext| {
            view.read_with(cx, |view, _| {
                view.state()
                    .plan_edit
                    .as_ref()
                    .map(|edit| edit.text.clone())
            })
        };

        // The range opens two months back, so column 2 is the current month.
        press(&view, "2", cx);
        view.update(cx, |view, cx| view.apply_movement(Movement::Next, cx));
        for each in ["l", "l", "i", "9", "0", "0"] {
            assert!(press(&view, each, cx), "{each}");
        }
        assert_eq!(draft(&view, cx), Some("900".to_string()));
        // Picking the tab clears the status line first.
        assert_eq!(*events.borrow(), vec![Seen::Other, Seen::EnterInsert]);

        assert!(view.update(cx, |view, cx| view.escape(cx)));
        cx.run_until_parked();
        assert_eq!(draft(&view, cx), None);
        assert_eq!(events.borrow().last(), Some(&Seen::ExitInsert));
        // With nothing being typed, `Esc` is left to `Shell`.
        assert!(!view.update(cx, |view, cx| view.escape(cx)));
    }

    #[gpui::test]
    fn the_known_costs_link_reveals_the_bills_period_shown(cx: &mut TestAppContext) {
        let (view, events) = seeded_view(cx);

        press(&view, "[", cx);
        view.update(cx, |view, cx| view.click_known(cx));
        cx.run_until_parked();

        let shown = Period::of(today()).prev();
        assert_eq!(
            events.borrow().last(),
            Some(&Seen::Reveal(Reveal::BillsPeriod(shown)))
        );
    }

    #[gpui::test]
    fn page_status_follows_the_tab(cx: &mut TestAppContext) {
        let (view, _) = seeded_view(cx);
        let hint_keys = |view: &Entity<BudgetsView>, cx: &mut TestAppContext| {
            view.read_with(cx, |view, cx| {
                view.page_status(cx)
                    .expect("Budgets always has a status")
                    .hints
                    .into_iter()
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>()
            })
        };

        assert!(hint_keys(&view, cx).contains(&"e"));
        press(&view, "2", cx);
        assert!(hint_keys(&view, cx).contains(&"r"));
        press(&view, "3", cx);
        assert!(hint_keys(&view, cx).contains(&"x"));
    }
}
