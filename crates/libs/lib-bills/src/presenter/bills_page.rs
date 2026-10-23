//! The Bills page (8a–8e): the header's Add button, the Schedule and Planner tabs, and the status
//! line's legend and right-hand text. It holds the two tab Presenters and sends each event to the
//! tab it belongs to, ignoring one for the tab not shown.
//!
//! The page keeps the Bill Schedule generated: when a running Client crosses into a new calendar
//! month, the next event the page handles runs [`PopulateSchedule`] first, so the new month's
//! entries are there before anything reads them.

use chrono::NaiveDate;
use lib_core::{BillPlanId, DateStyle, Period};
use lib_list_cursor::Step;
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};

use super::Clock;
use super::planner::{PlannerEvent, PlannerPresenter, PlannerViewModel};
use super::schedule::{FilterField, ScheduleEvent, SchedulePresenter, ScheduleViewModel};
use crate::interactor::PopulateSchedule;
use crate::keymap::{
    ALL, CHIP_PAID, CHIP_UPCOMING, FILTERS, NEXT_PERIOD, NEXT_TAB, PAY, PREVIOUS_PERIOD, SKIP,
};
use crate::protocols::{BillsOutcome, BillsRouter, SharedPorts};
use crate::{EntryId, msg};

/// Which tab of the page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillsTab {
    #[default]
    Schedule,
    Planner,
}

impl BillsTab {
    pub const ALL: [BillsTab; 2] = [BillsTab::Schedule, BillsTab::Planner];

    fn next(self) -> Self {
        match self {
            BillsTab::Schedule => BillsTab::Planner,
            BillsTab::Planner => BillsTab::Schedule,
        }
    }
}

/// What the page is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillsPageEvent {
    /// A movement on the tab shown.
    Move(Step),
    /// `Enter` on the tab shown: a Paid row's Transaction, or a Plan's Edit form.
    Open,
    /// `tab`: the other tab.
    NextTab,
    /// A click on a tab.
    ShowTab(BillsTab),
    /// `n` and **+ Add bill plan**, on either tab.
    New,
    Schedule(ScheduleEvent),
    Planner(PlannerEvent),
    /// The page became the destination.
    Entered,
    /// The Display Preference the Plan form's dates are typed in.
    SetDateStyle(Option<DateStyle>),
    /// A hand-off from another View: an entry in the month that shows it, a Plan on the Planner,
    /// or a month of the Schedule, with no filter hiding it.
    RevealEntry(EntryId),
    RevealPlan(BillPlanId),
    RevealPeriod(Period),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<BillsOutcome>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillsPageViewModel {
    /// `+ Add bill plan`.
    pub add_button: String,
    pub tabs: Vec<TabView>,
    pub tab: BillsTab,
    pub schedule: ScheduleViewModel,
    pub planner: PlannerViewModel,
    /// The status line's legend for the tab shown.
    pub hints: Vec<Hint>,
    /// The status line's right-hand text for the tab shown.
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabView {
    pub tab: BillsTab,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

pub struct BillsPagePresenter {
    ports: SharedPorts,
    clock: Clock,
    keymap: Keymap<BillsPageEvent>,
    tab: BillsTab,
    schedule: SchedulePresenter,
    planner: PlannerPresenter,
    /// The month generation last ran for.
    generated: Period,
    date_style: Option<DateStyle>,
}

impl BillsPagePresenter {
    /// The page on the Schedule tab, on today's month. The Client generated the Schedule when it
    /// started, so the page generates again only once the month turns. `keymap` is the one the
    /// Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, clock: Clock, keymap: Keymap<BillsPageEvent>) -> Self {
        let today = clock();
        Self {
            schedule: SchedulePresenter::new(ports.clone(), today),
            planner: PlannerPresenter::new(ports.clone()),
            ports,
            clock,
            keymap,
            tab: BillsTab::default(),
            generated: Period::of(today),
            date_style: None,
        }
    }

    /// Today, as the page dates its rows and its Dialogs.
    pub fn today(&self) -> NaiveDate {
        (self.clock)()
    }

    /// The date style the Plan form's dates are typed in.
    pub fn date_style(&self) -> Option<DateStyle> {
        self.date_style
    }

    /// The tab shown.
    pub fn tab(&self) -> BillsTab {
        self.tab
    }

    /// The Schedule's focused scope select, while the Schedule shows.
    pub fn filter_focus(&self) -> Option<FilterField> {
        (self.tab == BillsTab::Schedule)
            .then(|| self.schedule.filter_focus())
            .flatten()
    }

    pub fn handle(&mut self, event: BillsPageEvent, router: &mut dyn BillsRouter) {
        self.catch_up();
        let today = self.today();
        match event {
            BillsPageEvent::Move(step) => match self.tab {
                BillsTab::Schedule => {
                    self.schedule
                        .handle(ScheduleEvent::Move(step), today, router);
                }
                BillsTab::Planner => self.planner.handle(PlannerEvent::Move(step), router),
            },
            BillsPageEvent::Open => match self.tab {
                BillsTab::Schedule => self.schedule.handle(ScheduleEvent::Open, today, router),
                BillsTab::Planner => self.planner.handle(PlannerEvent::Edit, router),
            },
            BillsPageEvent::NextTab => self.show_tab(self.tab.next(), router),
            BillsPageEvent::ShowTab(tab) => self.show_tab(tab, router),
            BillsPageEvent::New => {
                router.open_plan_form(None);
            }
            BillsPageEvent::Schedule(event) => {
                if self.tab == BillsTab::Schedule {
                    self.schedule.handle(event, today, router);
                }
            }
            BillsPageEvent::Planner(event) => {
                if self.tab == BillsTab::Planner {
                    self.planner.handle(event, router);
                }
            }
            BillsPageEvent::Entered => {}
            BillsPageEvent::SetDateStyle(style) => self.date_style = style,
            // A hand-off to a row that has gone leaves the page as it was.
            BillsPageEvent::RevealEntry(id) => {
                if self.ports.borrow().bills().entry(id).is_some() {
                    self.show_tab(BillsTab::Schedule, router);
                    self.schedule.reveal(id, today);
                }
            }
            BillsPageEvent::RevealPlan(id) => {
                if self.ports.borrow().bills().bill_plan(id).is_some() {
                    self.show_tab(BillsTab::Planner, router);
                    self.planner.select_plan(id);
                }
            }
            BillsPageEvent::RevealPeriod(period) => {
                self.show_tab(BillsTab::Schedule, router);
                self.schedule.show_period(period);
            }
            BillsPageEvent::DialogClosed(outcome) => match outcome {
                Some(BillsOutcome::PlanSaved(id)) if self.tab == BillsTab::Planner => {
                    self.planner.select_plan(id);
                }
                Some(BillsOutcome::SkipRefused) => {
                    router.set_status(Some(msg::bills_skip_error_gone()));
                }
                Some(BillsOutcome::PlanSaved(_) | BillsOutcome::Skipped) | None => {}
            },
        }
    }

    /// Generates the Schedule for a month the Client has crossed into since it last did.
    fn catch_up(&mut self) {
        let today = self.today();
        if Period::of(today) == self.generated {
            return;
        }
        PopulateSchedule { today }.run(self.ports.borrow_mut().split_mut().bills);
        self.generated = Period::of(today);
    }

    /// Switches the tab, dropping the selection and any filter focus. A new tab starts at the top
    /// with no stale status message.
    fn show_tab(&mut self, tab: BillsTab, router: &mut dyn BillsRouter) {
        self.tab = tab;
        self.schedule.reset_selection();
        self.planner.reset_selection();
        router.set_status(None);
    }

    pub fn view_model(&self) -> BillsPageViewModel {
        let today = self.today();
        BillsPageViewModel {
            add_button: msg::bills_add_button("+"),
            tabs: BillsTab::ALL
                .iter()
                .map(|tab| TabView {
                    tab: *tab,
                    label: match tab {
                        BillsTab::Schedule => msg::bills_tab_schedule(),
                        BillsTab::Planner => msg::bills_tab_planner(),
                    },
                    active: *tab == self.tab,
                })
                .collect(),
            tab: self.tab,
            schedule: self.schedule.view_model(today),
            planner: self.planner.view_model(),
            hints: self.hints(),
            status: match self.tab {
                BillsTab::Schedule => self.schedule.status(today),
                BillsTab::Planner => self.planner.status(),
            },
        }
    }

    /// The tab shown's legend, its chords read from the keymap so an override shows.
    fn hints(&self) -> Vec<Hint> {
        let chord = |command: &str| self.keymap.chord(command).unwrap_or_default().to_string();
        let pair = |a: &str, b: &str, joint: &str| format!("{}{joint}{}", chord(a), chord(b));
        let hint = |chord: String, action: String| Hint { chord, action };
        let rows = hint(pair(MOVE_DOWN, MOVE_UP, "/"), msg::bills_hint_row());
        let switch = hint(chord(NEXT_TAB), msg::bills_hint_switch_view());
        match self.tab {
            BillsTab::Schedule => vec![
                rows,
                hint(chord(PAY), msg::bills_hint_pay()),
                hint(chord(SKIP), msg::bills_hint_skip()),
                hint(
                    pair(PREVIOUS_PERIOD, NEXT_PERIOD, "/"),
                    msg::bills_hint_period(),
                ),
                hint(chord(ALL), msg::bills_hint_all()),
                hint(
                    pair(CHIP_PAID, CHIP_UPCOMING, "\u{2013}"),
                    msg::bills_hint_status_chips(),
                ),
                hint(chord(FILTERS), msg::bills_hint_filters()),
                switch,
            ],
            BillsTab::Planner => vec![
                rows,
                hint(chord(EDIT), msg::bills_hint_edit()),
                hint(chord(NEW), msg::bills_hint_new()),
                switch,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_config::KeyBindingConfig;
    use lib_core::TransactionId;

    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, Routed};
    use crate::keymap::bills_page_keymap;
    use crate::presenter::test_support::{clock, date, stub, today};
    use crate::presenter::{FilterChoice, FilterField, LineActions};
    use crate::{BillStatus, Resolution, SplitRef};

    struct Page {
        ports: Rc<RefCell<InMemoryPorts>>,
        day: Rc<std::cell::Cell<NaiveDate>>,
        presenter: BillsPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new() -> Self {
            let (ports, shared) = stub();
            let (day, clock) = clock();
            let keymap = bills_page_keymap(&KeyBindingConfig::default());
            Self {
                ports,
                day,
                presenter: BillsPagePresenter::new(shared, clock, keymap),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: BillsPageEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn schedule(&mut self, event: ScheduleEvent) {
            self.send(BillsPageEvent::Schedule(event));
        }

        fn view(&self) -> BillsPageViewModel {
            self.presenter.view_model()
        }

        /// The Schedule rows shown, as `(name, day, status)`.
        fn rows(&self) -> Vec<(String, u32, BillStatus)> {
            use chrono::Datelike;
            self.view()
                .schedule
                .rows
                .iter()
                .map(|row| (row.name.clone(), row.id.due.day(), row.status))
                .collect()
        }

        fn select_named(&mut self, name: &str) {
            let index = self
                .view()
                .schedule
                .rows
                .iter()
                .position(|row| row.name == name)
                .unwrap_or_else(|| panic!("{name} shows"));
            self.schedule(ScheduleEvent::Select(index));
        }
    }

    #[test]
    fn the_september_schedule_is_unresolved_first_then_latest_resolved() {
        let page = Page::new();
        use BillStatus::*;
        let expected = [
            ("Gym \u{2014} Fitness First", 15, Overdue),
            ("Netflix", 18, Overdue),
            ("Telstra Internet", 22, Due),
            ("Origin Energy", 24, Due),
            ("Car Insurance \u{2014} AAMI", 30, Due),
            ("Car Wash Membership", 10, Skipped),
            ("Streaming Bundle", 5, Paid),
        ];
        let expected: Vec<_> = expected
            .iter()
            .map(|(name, day, status)| (name.to_string(), *day, *status))
            .collect();
        assert_eq!(page.rows(), expected);
    }

    #[test]
    fn a_row_shows_its_figures_status_and_actions_as_text() {
        let page = Page::new();
        let view = page.view().schedule;
        let gym = &view.rows[0];
        assert_eq!(gym.status_label, "OVERDUE");
        assert!(gym.needs_attention);
        assert_eq!(gym.due, format_month_day(date(2026, 9, 15)));
        assert_eq!(gym.paid, "—");
        assert!(
            matches!(&gym.actions, LineActions::PayOrSkip { pay, skip } if pay == "pay" && skip == "skip")
        );
        let paid = &view.rows[6];
        assert_eq!(paid.status_label, "PAID");
        assert_eq!(
            paid.actions,
            LineActions::ViewTransaction("view transaction \u{2192}".into())
        );
        assert_ne!(paid.actual, "—");
        let skipped = &view.rows[5];
        assert_eq!(
            skipped.actions,
            LineActions::Note("excluded from history".into())
        );
        assert!(skipped.quiet);
        assert_eq!(view.columns.bill, "BILL");
    }

    use lib_locale::format::format_month_day;

    #[test]
    fn the_meta_line_counts_the_month_and_totals_it() {
        let page = Page::new();
        let text: Vec<String> = page
            .view()
            .schedule
            .meta
            .iter()
            .map(|clause| clause.segments.iter().map(|s| s.text.as_str()).collect())
            .collect();
        assert_eq!(
            text,
            [
                "7 schedule entries this period",
                "2 overdue",
                "3 due",
                "1 paid",
                "1,560.98 planned",
            ]
        );
        assert!(page.view().schedule.meta[1].alert);
    }

    #[test]
    fn pay_on_an_actionable_row_asks_for_the_pay_dialog_with_its_candidates() {
        let mut page = Page::new();
        let telstra = page.view().schedule.rows[2].id;
        let payee = page
            .ports
            .borrow()
            .bills
            .plans
            .iter()
            .find(|plan| plan.id == telstra.plan_id)
            .and_then(|plan| plan.payee_id);
        let mut spend = page.ports.borrow().reads.splits[0].clone();
        spend.split = SplitRef {
            transaction_id: TransactionId::seed(90_001),
            split_index: 0,
        };
        spend.date = date(2026, 9, 21);
        spend.account_id = lib_core::AccountId::seed(2);
        spend.payee_id = payee;
        spend.category_id = lib_core::CategoryId::seed(5);
        page.ports.borrow_mut().reads.splits.push(spend);

        page.schedule(ScheduleEvent::Select(2));
        page.schedule(ScheduleEvent::Pay);

        let split = SplitRef {
            transaction_id: TransactionId::seed(90_001),
            split_index: 0,
        };
        let [Routed::Pay(request)] = page.router.routed.as_slice() else {
            panic!(
                "Pay asks for the Pay Dialog alone: {:?}",
                page.router.routed
            );
        };
        assert_eq!(request.entry, telstra);
        assert_eq!(
            request.candidates.first(),
            Some(&split),
            "the Payee match leads"
        );
        assert_eq!(request.preselected, Some(split));
    }

    #[test]
    fn pay_and_skip_on_a_row_with_nothing_to_settle_say_so() {
        let mut page = Page::new();
        page.send(BillsPageEvent::Move(Step::Last));

        page.schedule(ScheduleEvent::Pay);
        page.schedule(ScheduleEvent::Skip);

        let not_actionable = Routed::Status(Some("this bill isn't actionable".into()));
        assert_eq!(page.router.routed, [not_actionable.clone(), not_actionable]);
    }

    #[test]
    fn skip_on_an_actionable_row_asks_for_the_skip_dialog() {
        let mut page = Page::new();
        let gym = page.view().schedule.rows[0].id;

        page.schedule(ScheduleEvent::SkipRow(0));

        assert_eq!(page.router.routed, [Routed::Skip(gym)]);
    }

    #[test]
    fn enter_on_a_paid_row_shows_its_transaction_and_on_another_does_nothing() {
        let mut page = Page::new();
        page.send(BillsPageEvent::Open);
        assert!(page.router.routed.is_empty());

        page.select_named("Streaming Bundle");
        page.send(BillsPageEvent::Open);

        let paid = page.view().schedule.rows[6].id;
        let Some(Resolution::Paid(split)) =
            crate::entry(&page.ports.borrow().bills.entries, paid).map(|e| e.resolution.clone())
        else {
            panic!("the stub pays Streaming Bundle");
        };
        assert_eq!(
            page.router.routed,
            [Routed::Transaction(split.transaction_id)]
        );
    }

    #[test]
    fn the_period_steps_and_all_returns_to_the_month_last_viewed() {
        let mut page = Page::new();
        page.schedule(ScheduleEvent::NextPeriod);
        assert_eq!(page.view().schedule.period, Period::of(today()).next());

        page.schedule(ScheduleEvent::ToggleAll);
        assert!(page.view().schedule.all);
        assert!(page.view().schedule.rows.len() > 7);
        page.schedule(ScheduleEvent::NextPeriod);

        let view = page.view().schedule;
        assert!(!view.all);
        assert_eq!(view.period, Period::of(today()).next());
    }

    #[test]
    fn a_chip_hides_its_status_and_the_status_line_counts_what_shows() {
        let mut page = Page::new();
        page.schedule(ScheduleEvent::ToggleChip(BillStatus::Paid));

        let view = page.view();
        assert!(!view.schedule.filters.chips[0].on);
        assert!(
            view.schedule
                .rows
                .iter()
                .all(|row| row.status != BillStatus::Paid)
        );
        assert!(view.status.ends_with("6 of 7 entries"), "{}", view.status);
    }

    #[test]
    fn f_cycles_the_scope_selects_and_a_plan_scope_shows_its_stats() {
        let mut page = Page::new();
        for field in FilterField::ORDER {
            page.schedule(ScheduleEvent::CycleFilter);
            assert_eq!(page.view().schedule.filters.focused, Some(field));
        }
        page.schedule(ScheduleEvent::CycleFilter);
        assert_eq!(page.view().schedule.filters.focused, None);

        let telstra = page.view().schedule.rows[2].id.plan_id;
        page.schedule(ScheduleEvent::Filter(
            FilterField::Plan,
            FilterChoice::Plan(telstra),
        ));

        let view = page.view().schedule;
        assert!(view.rows.iter().all(|row| row.name == "Telstra Internet"));
        assert!(view.filters.selects[0].accent);
        let stats = view.stats.expect("a Plan scope shows its stats");
        let labels: Vec<&str> = stats.stats.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Last paid", "Average FY25–26", "Same month last year"]
        );
        assert_eq!(stats.stats[0].figure, "89.00");
        assert!(view.history_footnote.is_some());
    }

    #[test]
    fn a_narrowed_schedule_with_no_rows_says_the_filters_hide_them() {
        let mut page = Page::new();
        for status in crate::history::STATUS_CHIPS {
            page.schedule(ScheduleEvent::ToggleChip(status));
        }
        assert_eq!(
            page.view().schedule.empty.as_deref(),
            Some("No schedule rows match these filters.")
        );
    }

    #[test]
    fn switching_tab_resets_the_selection_clears_the_status_and_swaps_the_legend() {
        let mut page = Page::new();
        page.send(BillsPageEvent::Move(Step::Next));
        let schedule_keys: Vec<String> = page
            .view()
            .hints
            .into_iter()
            .map(|hint| hint.chord)
            .collect();
        assert_eq!(
            schedule_keys,
            ["j/k", "p", "s", "[/]", "0", "1\u{2013}5", "f", "tab"]
        );

        page.send(BillsPageEvent::NextTab);

        let view = page.view();
        assert_eq!(view.tab, BillsTab::Planner);
        assert_eq!(page.router.routed, [Routed::Status(None)]);
        let planner_keys: Vec<String> = view.hints.into_iter().map(|hint| hint.chord).collect();
        assert_eq!(planner_keys, ["j/k", "e", "n", "tab"]);
        assert_eq!(view.status, "10 bill plans");
        page.send(BillsPageEvent::NextTab);
        assert!(page.view().schedule.rows[0].selected);
    }

    #[test]
    fn the_planner_lists_active_plans_by_name_then_inactive_ones() {
        let mut page = Page::new();
        page.send(BillsPageEvent::ShowTab(BillsTab::Planner));

        let view = page.view().planner;
        assert_eq!(view.meta_plans, "10 bill plans");
        let inactive: String = view
            .meta_inactive
            .expect("one Plan is inactive")
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(inactive, "1 inactive");
        let last = view.rows.last().expect("the stub has Plans");
        assert_eq!(last.name, "Basic Fitness (closed)");
        assert!(last.inactive);
        assert_eq!(last.active, "no");
        assert!(view.rows.windows(2).take(8).all(|w| w[0].name <= w[1].name));
    }

    #[test]
    fn enter_and_e_on_the_planner_open_the_selected_plans_edit_form() {
        let mut page = Page::new();
        page.send(BillsPageEvent::NextTab);
        page.send(BillsPageEvent::Move(Step::Next));
        let second = page.view().planner.rows[1].id;

        page.send(BillsPageEvent::Open);
        page.send(BillsPageEvent::Planner(PlannerEvent::Edit));
        page.send(BillsPageEvent::New);

        assert_eq!(
            page.router.routed[1..],
            [
                Routed::PlanForm(Some(second)),
                Routed::PlanForm(Some(second)),
                Routed::PlanForm(None),
            ]
        );
    }

    #[test]
    fn a_schedule_key_does_nothing_on_the_planner() {
        let mut page = Page::new();
        page.send(BillsPageEvent::NextTab);
        page.router.routed.clear();

        page.schedule(ScheduleEvent::Pay);

        assert!(page.router.routed.is_empty());
    }

    #[test]
    fn a_saved_plan_is_selected_on_the_planner_and_a_refused_skip_says_why() {
        let mut page = Page::new();
        page.send(BillsPageEvent::NextTab);
        let last = page.view().planner.rows[9].id;

        page.send(BillsPageEvent::DialogClosed(Some(BillsOutcome::PlanSaved(
            last,
        ))));
        assert!(page.view().planner.rows[9].selected);

        page.send(BillsPageEvent::DialogClosed(Some(
            BillsOutcome::SkipRefused,
        )));
        assert_eq!(
            page.router.status(),
            Some(&Some("This bill can no longer be skipped.".to_string()))
        );
    }

    #[test]
    fn revealing_an_entry_shows_its_month_selected_with_no_filter() {
        let mut page = Page::new();
        page.schedule(ScheduleEvent::ToggleChip(BillStatus::Due));
        page.send(BillsPageEvent::NextTab);
        let rent = crate::interactor::test_support::entry_of(
            &page.ports.borrow(),
            "Rent",
            date(2026, 10, 1),
        );

        page.send(BillsPageEvent::RevealEntry(rent));

        let view = page.view();
        assert_eq!(view.tab, BillsTab::Schedule);
        assert_eq!(view.schedule.period, Period::of(today()).next());
        assert!(!view.schedule.filters.narrowed);
        let selected = view.schedule.rows.iter().find(|row| row.selected);
        assert_eq!(selected.map(|row| row.id), Some(rent));
    }

    #[test]
    fn a_hand_off_to_a_row_that_has_gone_leaves_the_page_as_it_was() {
        let mut page = Page::new();
        page.send(BillsPageEvent::RevealPlan(BillPlanId::seed(9_999)));
        page.send(BillsPageEvent::RevealEntry(EntryId {
            plan_id: BillPlanId::seed(9_999),
            due: today(),
        }));

        assert_eq!(page.view().tab, BillsTab::Schedule);
        assert!(page.router.routed.is_empty());
    }

    #[test]
    fn crossing_into_a_new_month_generates_its_entries_on_the_next_event() {
        let mut page = Page::new();
        let rent = crate::interactor::test_support::entry_of(
            &page.ports.borrow(),
            "Rent",
            date(2026, 11, 1),
        );
        let stored = |page: &Page| {
            page.ports
                .borrow()
                .bills
                .entries
                .iter()
                .any(|e| e.id() == rent)
        };
        page.send(BillsPageEvent::Entered);
        assert!(!stored(&page), "November is past September's horizon");

        page.day.set(date(2026, 10, 1));
        page.send(BillsPageEvent::Entered);

        assert!(stored(&page));
        assert_eq!(page.presenter.today(), date(2026, 10, 1));
    }
}
