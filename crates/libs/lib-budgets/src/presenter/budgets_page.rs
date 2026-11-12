//! The Budgets page (9a–9c, 11b–11f's entry points): the header (the Budget's name as the
//! Switcher's title, its method tag, the meta line and the primary action), the Progress, Plan and
//! History tabs, and the status line's legend and right-hand text. It holds the Budget on show,
//! each tab's month or range and cursor, and the Plan cell being typed into, and sends each event
//! to the tab it belongs to, showing that tab first when another is shown (a palette Command of
//! another tab). Its keymap is the tab shown's, and none while a Plan cell is being typed into:
//! that is its Insert mode, whose keys it types through `typed`.
//!
//! The Budget on show is held here in memory, opening on the default (the last one opened is a
//! Client-scoped Preference not yet kept). The page reads the day from its Clock on every event
//! and catches up when the month turns: a month or range that sat on the old current month moves
//! to the new one, so the page never strands itself in last month.

use chrono::NaiveDate;
use lib_config::KeyBindingConfig;
use lib_core::{BudgetId, Clock, DateStyle, Period};
use lib_list_cursor::Step;
use lib_locale::Segment;
use lib_toast::Notice;
use lib_viper::display::PageDisplay;
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::{PageMode, PagePresenter, Presenter, TextKey};
use lib_viper::view::Hint;

use super::history::{HistoryEvent, HistoryState, HistoryViewModel};
use super::plan::{PlanEvent, PlanState, PlanViewModel};
use super::progress::{ProgressEvent, ProgressState, ProgressViewModel};
use super::{error_text, method_label, plain};
use crate::interactor::{ArchiveBudget, DuplicateBudget, RestoreBudget, SetDefaultBudget};
use crate::keymap::{
    BUDGET_CATEGORY, CLEAR, COLUMN_LEFT, COLUMN_RIGHT, EXPORT, FILL, NEXT_PERIOD, NEXT_TAB,
    PREVIOUS_PERIOD, ROLLOVER, STOP, SWITCH, budgets_page_keymap, tab_keymap,
};
use crate::protocols::{BudgetRepository, BudgetsOutcome, BudgetsRouter, SharedPorts};
use crate::{Budget, Error, PERSONAL_SPENDING_ID, Result, Span, default_plan_start, msg};

/// Which tab of the page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetsTab {
    #[default]
    Progress,
    Plan,
    History,
}

impl BudgetsTab {
    pub const ALL: [BudgetsTab; 3] = [BudgetsTab::Progress, BudgetsTab::Plan, BudgetsTab::History];

    /// `tab` cycles forward, wrapping.
    pub fn next(self) -> Self {
        match self {
            BudgetsTab::Progress => BudgetsTab::Plan,
            BudgetsTab::Plan => BudgetsTab::History,
            BudgetsTab::History => BudgetsTab::Progress,
        }
    }

    fn label(self) -> String {
        match self {
            BudgetsTab::Progress => msg::budgets_tab_progress(),
            BudgetsTab::Plan => msg::budgets_tab_plan(),
            BudgetsTab::History => msg::budgets_tab_history(),
        }
    }
}

/// What the page is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetsPageEvent {
    /// A movement on the tab shown.
    Move(Step),
    /// `Enter` on the tab shown: the Category detail on Progress and History, the cell's edit on
    /// Plan.
    Open,
    /// `tab`: the next tab.
    NextTab,
    /// `1`/`2`/`3`, a click on a tab, or **Edit plan**.
    ShowTab(BudgetsTab),
    /// `B` and a click on the title: the Switcher.
    Switch,
    /// `n`: New budget.
    New,
    /// The Budget-wide verbs, on the Budget on show.
    EditBudget,
    Manage,
    Duplicate,
    SetDefault,
    Archive,
    Restore,
    Progress(ProgressEvent),
    Plan(PlanEvent),
    History(HistoryEvent),
    /// The page became the destination.
    Entered,
    /// The calendar month turned while the page was open.
    MonthTurned,
    /// The open Switcher or Manage budgets closed, with what it hands the page, `None` when it
    /// closed with nothing to show.
    DialogClosed(Option<BudgetsOutcome>),
}

impl BudgetsPageEvent {
    /// The tab this event acts on, which the page shows first when another is shown; `None` for
    /// an event every tab takes.
    pub fn tab(&self) -> Option<BudgetsTab> {
        match self {
            Self::Progress(_) => Some(BudgetsTab::Progress),
            Self::Plan(_) => Some(BudgetsTab::Plan),
            Self::History(_) => Some(BudgetsTab::History),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetsPageViewModel {
    /// `None` once the Budget on show has gone: the View draws the empty page.
    pub header: Option<HeaderView>,
    pub tabs: Vec<TabView>,
    pub tab: BudgetsTab,
    /// The `‹ › ` nav beside the tabs: the Progress month, or the Plan or History range.
    pub nav: String,
    pub body: TabBody,
    /// The status line's legend for the tab shown, or the Plan cell editor's.
    pub hints: Vec<Hint>,
    /// The status line's right-hand text for the tab shown.
    pub status: String,
    /// A Plan cell is being typed into.
    pub editing: bool,
}

/// The header: the title (the Switcher), its method tag and meta line, and the primary action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderView {
    pub name: String,
    pub method: String,
    /// The meta line's clauses, which the View separates; a `strong` run is the accent.
    pub meta: Vec<Vec<Segment>>,
    /// `Edit plan`, `+ Budget a category` or `Export CSV`, and what a click on it sends.
    pub action: String,
    pub action_event: BudgetsPageEvent,
    /// The Plan tab's `Fill October from…`, worded for the month Fill would target.
    pub fill: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabView {
    pub tab: BudgetsTab,
    pub label: String,
    pub active: bool,
}

/// The tab shown's body.
#[derive(Debug, Clone, PartialEq)]
pub enum TabBody {
    Progress(ProgressViewModel),
    Plan(PlanViewModel),
    History(HistoryViewModel),
    /// The Budget on show has gone.
    Gone,
}

/// The page's keymap as each state reads it: a tab's own chords, with every command still reached
/// by name, and the Plan cell editor's, which binds no chord so that every key types.
struct Keys {
    all: Keymap<BudgetsPageEvent>,
    progress: Keymap<BudgetsPageEvent>,
    plan: Keymap<BudgetsPageEvent>,
    history: Keymap<BudgetsPageEvent>,
    editing: Keymap<BudgetsPageEvent>,
}

impl Keys {
    fn new(all: Keymap<BudgetsPageEvent>) -> Self {
        Self {
            progress: tab_keymap(&all, BudgetsTab::Progress),
            plan: tab_keymap(&all, BudgetsTab::Plan),
            history: tab_keymap(&all, BudgetsTab::History),
            editing: all.scoped(|_| false),
            all,
        }
    }
}

pub struct BudgetsPagePresenter {
    pub(super) ports: SharedPorts,
    clock: Clock,
    keys: Keys,
    pub(super) current: BudgetId,
    tab: BudgetsTab,
    /// The month the page last caught up to.
    month: Period,
    date_style: Option<DateStyle>,
    pub(super) progress: ProgressState,
    pub(super) plan: PlanState,
    pub(super) history: HistoryState,
}

impl BudgetsPagePresenter {
    /// The page on the default Budget's Progress tab, on today's month. Its keys are built from
    /// `config`, as [`PagePresenter::rebind`] builds them later.
    pub fn new(ports: SharedPorts, clock: Clock, config: &KeyBindingConfig) -> Self {
        let today = clock.today();
        let current = {
            let budgets = ports.borrow().budgets().budgets();
            budgets
                .iter()
                .find(|budget| budget.is_default)
                .or_else(|| budgets.first())
                .map_or(PERSONAL_SPENDING_ID, |budget| budget.id)
        };
        Self {
            ports,
            clock,
            keys: Keys::new(budgets_page_keymap(config)),
            current,
            tab: BudgetsTab::default(),
            month: Period::of(today),
            date_style: None,
            progress: ProgressState::new(today),
            plan: PlanState::new(today),
            history: HistoryState::new(today),
        }
    }

    /// The Budget on show's id.
    pub fn current(&self) -> BudgetId {
        self.current
    }

    pub fn tab(&self) -> BudgetsTab {
        self.tab
    }

    /// Whether a Plan cell is being typed into: the Client's Insert mode.
    pub fn editing(&self) -> bool {
        self.plan.edit.is_some()
    }

    /// The date style the Switcher's dates are written in.
    pub fn date_style(&self) -> Option<DateStyle> {
        self.date_style
    }

    /// The Progress month shown and its selected row.
    pub fn period(&self) -> Period {
        self.progress.period
    }

    pub fn selected(&self) -> usize {
        self.progress.selected
    }

    /// The Plan's first month and its cursor, as (row, column).
    pub fn plan_start(&self) -> Period {
        self.plan.start
    }

    pub fn plan_cursor(&self) -> (usize, usize) {
        self.plan.cursor
    }

    /// The Plan cell being typed into, as its draft text.
    pub fn plan_draft(&self) -> Option<&str> {
        self.plan.edit.as_ref().map(|edit| edit.text.as_str())
    }

    /// The History range's last month.
    pub fn history_end(&self) -> Period {
        self.history.end
    }

    pub(super) fn today(&self) -> NaiveDate {
        self.clock.today()
    }

    /// The Budget on show, or `None` once it has gone.
    pub(super) fn budget(&self) -> Option<Budget> {
        self.ports.borrow().budgets().budget(self.current)
    }

    /// The Budget on show when it takes edits: an archived one is read-only.
    pub(super) fn editable(&self) -> Option<Budget> {
        self.budget().filter(|budget| !budget.is_archived())
    }

    /// Runs `write` on the repository, borrowed for one Interactor.
    pub(super) fn with_repo<T>(&self, write: impl FnOnce(&mut dyn BudgetRepository) -> T) -> T {
        write(self.ports.borrow_mut().split_mut().budgets)
    }

    /// A Budget-wide verb's refusal, on the status line.
    fn report(done: Result<()>, router: &mut dyn BudgetsRouter) {
        if let Err(error) = done {
            router.notify(Notice::Status(error_text(&error)));
        }
    }

    /// Moves what sat on the month the page last saw onto today's, once the month has turned.
    fn catch_up(&mut self) {
        let today = self.today();
        let now = Period::of(today);
        if now == self.month {
            return;
        }
        let was = self.month;
        if self.progress.period == was {
            self.progress.period = now;
        }
        if self.plan.start == was.prev().prev() {
            self.plan.start = default_plan_start(today);
        }
        if self.history.end == was {
            self.history.end = now;
        }
        self.month = now;
    }

    /// Switches the tab, putting its selection back at the top. The Shell clears a stale status
    /// message on the key that switched it (ADR-0048), and the Client on a click.
    fn show_tab(&mut self, tab: BudgetsTab) {
        self.tab = tab;
        self.progress.selected = 0;
    }

    /// Shows Budget `id`, keeping the tab (every method built has all three) and putting each
    /// tab's cursor and range back at its start. A Budget that has gone leaves the page as it was.
    fn show_budget(&mut self, id: BudgetId) {
        if self.ports.borrow().budgets().budget(id).is_none() {
            return;
        }
        let today = self.today();
        self.current = id;
        self.progress.selected = 0;
        self.plan = PlanState::new(today);
        self.history = HistoryState::new(today);
    }

    /// The tab shown's legend, its chords read from the keymap so an override shows. The Plan
    /// cell editor's keys are its own, not the keymap's.
    fn hints(&self) -> Vec<Hint> {
        let chord = |command: &str| self.keys.all.hint_chord(command).to_string();
        let pair = |a: &str, b: &str| format!("{}/{}", chord(a), chord(b));
        let hint = |chord: String, action: String| Hint { chord, action };
        let rows = hint(pair(MOVE_DOWN, MOVE_UP), msg::budgets_hint_row());
        let switch = hint(chord(NEXT_TAB), msg::budgets_hint_switch_view());
        let range = hint(
            pair(PREVIOUS_PERIOD, NEXT_PERIOD),
            msg::budgets_hint_range(),
        );
        let budget_category = hint(chord(BUDGET_CATEGORY), msg::budgets_hint_budget_category());
        match self.tab {
            BudgetsTab::Progress => vec![
                rows,
                hint(chord(SELECT), msg::budgets_hint_open()),
                hint(chord(EDIT), msg::budgets_hint_edit_budget()),
                budget_category,
                hint(chord(STOP), msg::budgets_hint_stop_budgeting()),
                hint(chord(SWITCH), msg::budgets_hint_switch_budget()),
                hint(
                    pair(PREVIOUS_PERIOD, NEXT_PERIOD),
                    msg::budgets_hint_period(),
                ),
                switch,
            ],
            BudgetsTab::Plan if self.editing() => vec![
                hint("enter".to_string(), msg::budgets_hint_save_cell()),
                hint("esc".to_string(), msg::budgets_hint_cancel()),
                hint("tab".to_string(), msg::budgets_hint_next_month()),
                hint(
                    "shift+enter".to_string(),
                    msg::budgets_hint_this_month_only(),
                ),
            ],
            BudgetsTab::Plan => vec![
                hint(
                    format!(
                        "{}/{}/{}/{}",
                        chord(COLUMN_LEFT),
                        chord(MOVE_DOWN),
                        chord(MOVE_UP),
                        chord(COLUMN_RIGHT)
                    ),
                    msg::budgets_hint_cell(),
                ),
                hint(chord(SELECT), msg::budgets_hint_edit()),
                hint(chord(ROLLOVER), msg::budgets_hint_rollover()),
                hint(chord(CLEAR), msg::budgets_hint_clear()),
                hint(chord(FILL), msg::budgets_hint_fill()),
                budget_category,
                range,
                switch,
            ],
            BudgetsTab::History => vec![
                rows,
                hint(pair(COLUMN_LEFT, COLUMN_RIGHT), msg::budgets_hint_month()),
                hint(chord(SELECT), msg::budgets_hint_open()),
                hint(chord(EXPORT), msg::budgets_hint_export()),
                range,
                switch,
            ],
        }
    }
}

impl Presenter for BudgetsPagePresenter {
    type Event = BudgetsPageEvent;
    type ViewModel = BudgetsPageViewModel;
    type Router<'r> = dyn BudgetsRouter + 'r;

    fn handle(&mut self, event: BudgetsPageEvent, router: &mut Self::Router<'_>) {
        self.catch_up();
        // Anything but the cell editor's own keys leaves the cell being typed into: a Dialog, a
        // tab or another Budget never opens over it.
        if !matches!(
            event,
            BudgetsPageEvent::Plan(
                PlanEvent::Type(_) | PlanEvent::Backspace | PlanEvent::Save { .. }
            )
        ) {
            self.plan.edit = None;
        }
        match event {
            BudgetsPageEvent::Move(step) => match self.tab {
                BudgetsTab::Progress => self.progress_move(step),
                BudgetsTab::Plan => self.plan_move(step),
                BudgetsTab::History => self.history_move(step),
            },
            BudgetsPageEvent::Open => match self.tab {
                BudgetsTab::Progress => self.open_progress_detail(self.progress.selected, router),
                BudgetsTab::Plan => self.start_plan_edit(),
                BudgetsTab::History => self.open_history_detail(router),
            },
            BudgetsPageEvent::NextTab => self.show_tab(self.tab.next()),
            BudgetsPageEvent::ShowTab(tab) => self.show_tab(tab),
            BudgetsPageEvent::Switch => {
                router.open_switcher(self.current);
            }
            BudgetsPageEvent::New => {
                router.open_new_budget(self.current);
            }
            BudgetsPageEvent::EditBudget => {
                if self.editable().is_some() {
                    router.open_edit_budget(self.current);
                } else {
                    router.notify(Notice::Status(error_text(&Error::Archived)));
                }
            }
            BudgetsPageEvent::Manage => {
                router.open_manage(self.current);
            }
            BudgetsPageEvent::Duplicate => {
                let copy = self.with_repo(|repo| DuplicateBudget(self.current).run(repo));
                match copy {
                    Ok(copy) => {
                        self.show_budget(copy);
                        router.open_edit_budget(copy);
                    }
                    Err(error) => router.notify(Notice::Status(error_text(&error))),
                }
            }
            BudgetsPageEvent::SetDefault => {
                let done = self.with_repo(|repo| SetDefaultBudget(self.current).run(repo));
                Self::report(done, router);
            }
            BudgetsPageEvent::Archive => {
                let done = self.with_repo(|repo| {
                    ArchiveBudget {
                        id: self.current,
                        today: self.today(),
                    }
                    .run(repo)
                });
                Self::report(done, router);
            }
            BudgetsPageEvent::Restore => {
                let done = self.with_repo(|repo| RestoreBudget(self.current).run(repo));
                Self::report(done, router);
            }
            BudgetsPageEvent::Progress(event) => self.progress_event(event, router),
            BudgetsPageEvent::Plan(event) => self.plan_event(event, router),
            BudgetsPageEvent::History(event) => self.history_event(event, router),
            BudgetsPageEvent::Entered | BudgetsPageEvent::MonthTurned => {}
            BudgetsPageEvent::DialogClosed(outcome) => match outcome {
                Some(BudgetsOutcome::Show(id)) => self.show_budget(id),
                Some(BudgetsOutcome::ShowAndEdit(id)) => {
                    self.show_budget(id);
                    router.open_edit_budget(id);
                }
                Some(BudgetsOutcome::NewBudget) => {
                    router.open_new_budget(self.current);
                }
                Some(BudgetsOutcome::EditBudget(id)) => {
                    router.open_edit_budget(id);
                }
                Some(BudgetsOutcome::Manage) => {
                    router.open_manage(self.current);
                }
                Some(BudgetsOutcome::Limit(category, month)) => {
                    router.open_limit(self.current, category, month);
                }
                Some(BudgetsOutcome::Stop(category, month)) => {
                    router.open_stop(self.current, category, month);
                }
                Some(BudgetsOutcome::Transactions(category, month)) => {
                    router.show_transactions(self.current, category, month);
                }
                None => {}
            },
        }
    }

    fn view_model(&self) -> BudgetsPageViewModel {
        let tabs = BudgetsTab::ALL
            .iter()
            .map(|tab| TabView {
                tab: *tab,
                label: tab.label(),
                active: *tab == self.tab,
            })
            .collect();
        let Some(budget) = self.budget() else {
            return BudgetsPageViewModel {
                header: None,
                tabs,
                tab: self.tab,
                nav: String::new(),
                body: TabBody::Gone,
                hints: self.hints(),
                status: String::new(),
                editing: false,
            };
        };
        let ports = self.ports.borrow();
        let reads = ports.reads();
        let (meta, action, action_event, fill, nav, body, status) = match self.tab {
            BudgetsTab::Progress => {
                let (view, meta, status) = self.progress_view(&budget, reads);
                (
                    meta,
                    msg::budgets_edit_plan(),
                    BudgetsPageEvent::ShowTab(BudgetsTab::Plan),
                    None,
                    super::period_label(self.progress.period),
                    TabBody::Progress(view),
                    status,
                )
            }
            BudgetsTab::Plan => {
                let (view, nav, fill, status) = self.plan_view(&budget, reads);
                (
                    vec![plain(msg::budgets_plan_meta())],
                    msg::budgets_plan_add(),
                    BudgetsPageEvent::Plan(PlanEvent::BudgetCategory),
                    Some(fill),
                    nav,
                    TabBody::Plan(view),
                    status,
                )
            }
            BudgetsTab::History => {
                let (view, nav, status) = self.history_view(&budget, reads);
                (
                    vec![plain(msg::budgets_history_meta())],
                    msg::budgets_history_export(),
                    BudgetsPageEvent::History(HistoryEvent::Export),
                    None,
                    nav,
                    TabBody::History(view),
                    status,
                )
            }
        };
        BudgetsPageViewModel {
            header: Some(HeaderView {
                name: budget.name.clone(),
                method: method_label(budget.method),
                meta,
                action,
                action_event,
                fill,
            }),
            tabs,
            tab: self.tab,
            nav,
            body,
            hints: self.hints(),
            status,
            editing: self.editing(),
        }
    }
}

impl PagePresenter for BudgetsPagePresenter {
    type Outcome = BudgetsOutcome;

    fn dialog_closed(outcome: Option<BudgetsOutcome>) -> Option<BudgetsPageEvent> {
        Some(BudgetsPageEvent::DialogClosed(outcome))
    }

    /// The tab shown's keys; none while a Plan cell is being typed into, so every key types.
    fn keymap(&self) -> &Keymap<BudgetsPageEvent> {
        if self.editing() {
            return &self.keys.editing;
        }
        match self.tab {
            BudgetsTab::Progress => &self.keys.progress,
            BudgetsTab::Plan => &self.keys.plan,
            BudgetsTab::History => &self.keys.history,
        }
    }

    /// Every tab's keys, and the legend's chords, follow the new bindings.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        self.keys = Keys::new(budgets_page_keymap(keys));
    }

    /// The date style the Switcher's archived dates are written in.
    fn display(&mut self, display: &PageDisplay) {
        self.date_style = display.date_style;
    }

    /// The Plan cell editor's keys (9b): `enter` saves onward and `shift+enter` this month only,
    /// `tab`/`shift+tab` save and move to the next or previous month, and a character types.
    fn typed(&self, key: TextKey) -> Option<BudgetsPageEvent> {
        if !self.editing() {
            return None;
        }
        let event = match key {
            TextKey::Char(c) => PlanEvent::Type(c),
            TextKey::Backspace => PlanEvent::Backspace,
            TextKey::Enter { shift } => PlanEvent::Save {
                span: if shift { Span::MonthOnly } else { Span::Onward },
                step: 0,
            },
            TextKey::Tab { shift } => PlanEvent::Save {
                span: Span::Onward,
                step: if shift { -1 } else { 1 },
            },
        };
        Some(BudgetsPageEvent::Plan(event))
    }

    /// A palette Command of the other tab shows that tab first.
    fn shows(&self, event: &BudgetsPageEvent) -> Option<BudgetsPageEvent> {
        event
            .tab()
            .filter(|tab| *tab != self.tab)
            .map(BudgetsPageEvent::ShowTab)
    }

    /// A Plan cell being typed into is the page's Insert mode.
    fn mode(&self) -> Option<PageMode> {
        self.editing().then_some(PageMode::Insert)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use lib_core::CategoryId;

    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, Routed};
    use crate::presenter::test_support::{clock, date, sample, today};
    use crate::{Span, applied};

    pub(crate) struct Page {
        pub ports: Rc<RefCell<InMemoryPorts>>,
        pub day: Rc<Cell<NaiveDate>>,
        pub presenter: BudgetsPagePresenter,
        pub router: RecordingRouter,
    }

    impl Page {
        pub fn new() -> Self {
            let (ports, shared) = sample();
            let (day, clock) = clock();
            Self {
                ports,
                day,
                presenter: BudgetsPagePresenter::new(shared, clock, &KeyBindingConfig::default()),
                router: RecordingRouter::default(),
            }
        }

        pub fn send(&mut self, event: BudgetsPageEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn budget(&self, id: BudgetId) -> Budget {
            self.ports
                .borrow()
                .budgets
                .budget(id)
                .expect("the Budget is there")
        }
    }

    #[test]
    fn the_page_opens_on_the_default_budgets_progress_tab() {
        let page = Page::new();

        let view = page.presenter.view_model();
        assert_eq!(page.presenter.current(), PERSONAL_SPENDING_ID);
        assert_eq!(view.tab, BudgetsTab::Progress);
        assert_eq!(
            view.header.as_ref().map(|h| h.name.as_str()),
            Some("Personal spending")
        );
        assert_eq!(view.nav, "Jun 2026");
        assert!(matches!(view.body, TabBody::Progress(_)));
        assert_eq!(view.hints[0].chord, "j/k");
    }

    #[test]
    fn tab_and_the_digits_switch_the_tab_and_the_header_follows_it() {
        let mut page = Page::new();

        page.send(BudgetsPageEvent::NextTab);
        let plan = page.presenter.view_model();
        assert_eq!(plan.tab, BudgetsTab::Plan);
        let header = plan.header.expect("a header");
        assert_eq!(header.action, "+ Budget a category");
        assert_eq!(
            header.fill.as_deref(),
            Some("Fill Jul from…"),
            "a closed month fills next month"
        );

        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::History));
        let history = page.presenter.view_model();
        assert_eq!(
            history.header.map(|h| h.action_event),
            Some(BudgetsPageEvent::History(HistoryEvent::Export))
        );
        page.send(BudgetsPageEvent::NextTab);
        assert_eq!(page.presenter.tab(), BudgetsTab::Progress);
    }

    #[test]
    fn an_event_for_another_tab_names_the_event_that_shows_that_tab() {
        let mut page = Page::new();
        let export = BudgetsPageEvent::History(HistoryEvent::Export);

        let shown = page.presenter.shows(&export).expect("Export is History's");
        assert_eq!(shown, BudgetsPageEvent::ShowTab(BudgetsTab::History));
        page.send(shown);
        assert_eq!(page.presenter.shows(&export), None, "History shows");
        page.send(export);

        assert_eq!(page.presenter.tab(), BudgetsTab::History);
        assert!(matches!(page.router.routed.as_slice(), [Routed::Export(_)]));
    }

    #[test]
    fn the_keymap_is_the_tab_shown_s_and_names_every_command() {
        let mut page = Page::new();
        assert_eq!(
            page.presenter.keymap().event("["),
            Some(BudgetsPageEvent::Progress(ProgressEvent::PreviousPeriod))
        );

        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));

        let keymap = page.presenter.keymap();
        assert_eq!(
            keymap.event("["),
            Some(BudgetsPageEvent::Plan(PlanEvent::PreviousRange))
        );
        assert_eq!(
            keymap.event_for(EXPORT),
            Some(BudgetsPageEvent::History(HistoryEvent::Export)),
            "a command of another tab is still reached by name"
        );
    }

    #[test]
    fn a_plan_cell_being_typed_into_is_insert_mode_whose_keys_type() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        assert_eq!(page.presenter.mode(), None);
        assert_eq!(page.presenter.typed(TextKey::Char('1')), None);

        page.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(0, 2)));

        assert_eq!(page.presenter.mode(), Some(PageMode::Insert));
        assert_eq!(page.presenter.keymap().event("j"), None, "every key types");
        assert_eq!(
            page.presenter.typed(TextKey::Char('1')),
            Some(BudgetsPageEvent::Plan(PlanEvent::Type('1')))
        );
        assert_eq!(
            page.presenter.typed(TextKey::Enter { shift: true }),
            Some(BudgetsPageEvent::Plan(PlanEvent::Save {
                span: Span::MonthOnly,
                step: 0,
            }))
        );
        assert_eq!(
            page.presenter.typed(TextKey::Tab { shift: true }),
            Some(BudgetsPageEvent::Plan(PlanEvent::Save {
                span: Span::Onward,
                step: -1,
            }))
        );
        page.send(BudgetsPageEvent::Plan(PlanEvent::Abandon));
        assert_eq!(page.presenter.mode(), None);
    }

    #[test]
    fn rebound_keys_reach_the_keymap_and_the_legend() {
        let mut page = Page::new();
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(SWITCH.to_string(), "ctrl+b".to_string());

        page.presenter.rebind(&config);

        assert_eq!(
            page.presenter.keymap().event("ctrl+b"),
            Some(BudgetsPageEvent::Switch)
        );
        let hints = page.presenter.view_model().hints;
        assert!(hints.iter().any(|hint| hint.chord == "ctrl+b"));
    }

    #[test]
    fn the_switcher_and_new_budget_open_on_the_budget_shown() {
        let mut page = Page::new();

        page.send(BudgetsPageEvent::Switch);
        page.send(BudgetsPageEvent::New);
        page.send(BudgetsPageEvent::Manage);

        assert_eq!(
            page.router.routed,
            [
                Routed::Switcher(PERSONAL_SPENDING_ID),
                Routed::NewBudget(PERSONAL_SPENDING_ID),
                Routed::Manage(PERSONAL_SPENDING_ID),
            ]
        );
    }

    #[test]
    fn a_closed_switcher_shows_its_budget_or_opens_what_it_asked_for() {
        let mut page = Page::new();
        let other = page.ports.borrow().budgets.budgets[1].id;

        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        page.send(BudgetsPageEvent::DialogClosed(Some(BudgetsOutcome::Show(
            other,
        ))));
        assert_eq!(page.presenter.current(), other);
        assert_eq!(page.presenter.tab(), BudgetsTab::Plan, "the tab is kept");

        page.send(BudgetsPageEvent::DialogClosed(Some(
            BudgetsOutcome::NewBudget,
        )));
        page.send(BudgetsPageEvent::DialogClosed(Some(BudgetsOutcome::Manage)));
        assert_eq!(
            page.router.routed,
            [Routed::NewBudget(other), Routed::Manage(other)]
        );
    }

    #[test]
    fn duplicate_shows_the_copy_and_opens_its_edit_mode() {
        let mut page = Page::new();
        let before = page.ports.borrow().budgets.budgets.len();

        page.send(BudgetsPageEvent::Duplicate);

        let copy = page.presenter.current();
        assert_ne!(copy, PERSONAL_SPENDING_ID);
        assert_eq!(page.ports.borrow().budgets.budgets.len(), before + 1);
        assert_eq!(page.router.routed, [Routed::EditBudget(copy)]);
    }

    #[test]
    fn archiving_the_default_is_refused_on_the_status_line() {
        let mut page = Page::new();

        page.send(BudgetsPageEvent::Archive);

        assert_eq!(
            page.router.status(),
            Some("The default budget can't be archived. Set another default first.")
        );
        assert!(!page.budget(PERSONAL_SPENDING_ID).is_archived());
    }

    #[test]
    fn set_default_and_archive_run_on_the_budget_shown() {
        let mut page = Page::new();
        let other = page.ports.borrow().budgets.budgets[1].id;
        page.send(BudgetsPageEvent::DialogClosed(Some(BudgetsOutcome::Show(
            other,
        ))));

        page.send(BudgetsPageEvent::SetDefault);
        assert!(page.budget(other).is_default);
        page.send(BudgetsPageEvent::DialogClosed(Some(BudgetsOutcome::Show(
            PERSONAL_SPENDING_ID,
        ))));
        page.send(BudgetsPageEvent::Archive);
        assert!(page.budget(PERSONAL_SPENDING_ID).is_archived());

        page.send(BudgetsPageEvent::EditBudget);
        assert_eq!(
            page.router.status(),
            Some("An archived budget is read-only.")
        );
        page.send(BudgetsPageEvent::Restore);
        assert!(!page.budget(PERSONAL_SPENDING_ID).is_archived());
    }

    #[test]
    fn a_turned_month_moves_what_sat_on_the_old_one() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));

        page.day.set(date(2026, 7, 1));
        page.send(BudgetsPageEvent::MonthTurned);

        assert_eq!(page.presenter.progress.period, Period::of(date(2026, 7, 1)));
        assert_eq!(
            page.presenter.plan.start,
            default_plan_start(date(2026, 7, 1))
        );
        assert_eq!(page.presenter.history.end, Period::of(date(2026, 7, 1)));
    }

    #[test]
    fn a_month_stepped_away_from_stays_where_it_was_when_the_month_turns() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::Progress(ProgressEvent::PreviousPeriod));

        page.day.set(date(2026, 7, 1));
        page.send(BudgetsPageEvent::MonthTurned);

        assert_eq!(page.presenter.progress.period, Period::of(date(2026, 5, 1)));
    }

    #[test]
    fn a_plan_cell_edit_saves_through_the_interactor_and_steps_on() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        // The range opens two months back, so column 2 is the current month.
        page.send(BudgetsPageEvent::Plan(PlanEvent::ColumnRight));
        page.send(BudgetsPageEvent::Plan(PlanEvent::ColumnRight));
        page.send(BudgetsPageEvent::Plan(PlanEvent::StartEdit));
        assert!(page.presenter.editing());
        let category = page
            .presenter
            .plan
            .edit
            .as_ref()
            .map(|edit| edit.category_id)
            .expect("an edit");
        for c in "0123".chars() {
            page.send(BudgetsPageEvent::Plan(PlanEvent::Type(c)));
        }
        page.send(BudgetsPageEvent::Plan(PlanEvent::Backspace));
        page.send(BudgetsPageEvent::Plan(PlanEvent::Save {
            span: Span::Onward,
            step: 1,
        }));

        assert_eq!(
            applied(
                page.budget(PERSONAL_SPENDING_ID).chain(category),
                Period::of(today())
            )
            .map(|applied| applied.amount),
            Some(crate::cents_money(1200))
        );
        assert!(
            page.presenter.editing(),
            "tab moves on to the next month's cell"
        );
        assert_eq!(page.presenter.plan.cursor.1, 3);
        page.send(BudgetsPageEvent::Plan(PlanEvent::Abandon));
        assert!(!page.presenter.editing());
    }

    #[test]
    fn text_that_isnt_an_amount_keeps_the_cell_open() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        page.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(0, 2)));
        assert!(page.presenter.editing());
        if let Some(edit) = page.presenter.plan.edit.as_mut() {
            edit.text = "x".to_string();
        }

        page.send(BudgetsPageEvent::Plan(PlanEvent::Save {
            span: Span::Onward,
            step: 0,
        }));

        assert!(page.presenter.editing());
    }

    #[test]
    fn the_plan_legend_turns_into_the_cell_editors_while_typing() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        assert_eq!(page.presenter.view_model().hints[0].chord, "h/j/k/l");

        page.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(0, 2)));

        let view = page.presenter.view_model();
        assert!(view.editing);
        assert_eq!(view.hints[0].chord, "enter");
    }

    #[test]
    fn a_closed_month_is_read_only_and_the_rollover_column_cycles() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::Plan));
        page.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(0, 0)));
        assert!(!page.presenter.editing(), "two months back is closed");

        let category: CategoryId = page.presenter.plan_row_category(0).expect("a row");
        let before = crate::inherited_rollover(
            page.budget(PERSONAL_SPENDING_ID).chain(category),
            Period::of(today()).next(),
        );
        page.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(
            0,
            crate::PLAN_ROLLOVER_COLUMN,
        )));
        let after = crate::inherited_rollover(
            page.budget(PERSONAL_SPENDING_ID).chain(category),
            Period::of(today()).next(),
        );
        assert_eq!(after, before.next());
    }

    #[test]
    fn the_export_hands_the_router_the_history_as_csv() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::ShowTab(BudgetsTab::History));

        page.send(BudgetsPageEvent::History(HistoryEvent::Export));

        let Some(Routed::Export(export)) = page.router.routed.last() else {
            panic!("an export, got {:?}", page.router.routed);
        };
        assert!(export.csv.starts_with("Category,"));
        assert!(export.csv.contains("Jun 2026 Spent"));
    }

    #[test]
    fn known_costs_shows_the_bills_period_shown() {
        let mut page = Page::new();
        page.send(BudgetsPageEvent::Progress(ProgressEvent::PreviousPeriod));

        page.send(BudgetsPageEvent::Progress(ProgressEvent::KnownCosts));

        assert_eq!(
            page.router.routed,
            [Routed::BillsPeriod(Period::of(today()).prev())]
        );
    }

    #[test]
    fn enter_and_a_second_click_open_the_category_detail() {
        let mut page = Page::new();

        page.send(BudgetsPageEvent::Progress(ProgressEvent::Select(1)));
        assert!(page.router.routed.is_empty(), "a first click selects");
        page.send(BudgetsPageEvent::Progress(ProgressEvent::Select(1)));
        page.send(BudgetsPageEvent::Open);

        assert!(matches!(
            page.router.routed.as_slice(),
            [Routed::Detail(..), Routed::Detail(..)]
        ));
    }

    #[test]
    fn an_override_shows_in_the_legend() {
        let (_, shared) = sample();
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(SWITCH.to_string(), "ctrl+b".to_string());
        let presenter = BudgetsPagePresenter::new(shared, Clock::fixed(today()), &config);

        let hints = presenter.view_model().hints;
        assert!(hints.iter().any(|hint| hint.chord == "ctrl+b"));
    }
}
