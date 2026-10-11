//! The Budgets destination's Entities (ADR-0032). [`BudgetsStore`] reads the Budgets through
//! [`LedgerStore`], which holds the only copy. Categories 5c and the Dashboard read them too, and
//! [`BudgetsView`] owns the page's own state: which
//! Budget, tab, month and cell it is showing. The view reports what needs `Shell` (the
//! dialog host, navigation and the save dialog are `Shell`'s) as [`BudgetsEvent`]s.

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_budgets::Budgets;
use lib_ledger::Domain;

use crate::budgets::{BudgetsTab, PlanEdit, default_plan_start};
use crate::ledger_store::{LedgerChanged, LedgerStore};
use crate::period::Period;

/// The shared Budgets. Every reader (the Budgets page, Categories 5c and the Dashboard) reads
/// them from here, so a change is seen everywhere on the next render.
pub struct BudgetsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl BudgetsStore {
    /// A store over `ledger`'s Budgets.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Budgets) {
                cx.notify();
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn budgets<'a>(&self, cx: &'a App) -> &'a Budgets {
        self.ledger.read(cx).ledger().budgets()
    }
}

/// What the Budgets page asks `Shell` to do. The page never navigates or opens a dialog itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetsEvent {
    /// `x` on the History tab: export the table. `Shell` runs it, since the save dialog needs a
    /// `Context` a key handler doesn't have.
    ExportHistory,
    /// The KNOWN COSTS stat's link: open the Bills Schedule on the period being shown.
    OpenSchedule,
}

impl EventEmitter<BudgetsEvent> for BudgetsView {}

/// The Budgets page's own state, owned by its Entity.
pub struct BudgetsView {
    state: BudgetsState,
}

impl BudgetsView {
    pub fn new(state: BudgetsState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &BudgetsState {
        &self.state
    }

    /// Edits the page's state, then asks for a re-render.
    pub fn edit<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut BudgetsState) -> R,
    ) -> R {
        let result = change(&mut self.state);
        cx.notify();
        result
    }

    pub fn request_export(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(BudgetsEvent::ExportHistory);
    }

    pub fn request_schedule(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(BudgetsEvent::OpenSchedule);
    }
}

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
