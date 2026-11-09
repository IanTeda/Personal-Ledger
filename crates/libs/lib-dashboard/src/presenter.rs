//! The Dashboard's Presenter: Needs Attention's Bill rows and the default Budget's budget list,
//! built into a plain-data view model with every figure already worded, so a View only draws.
//!
//! It keeps its view model with the day it was built on and reads the ports again only after a
//! change ([`DashboardEvent::LedgerChanged`], [`DashboardEvent::DisplayChanged`]) or when the Shell's
//! Clock says the day moved, so a day or month turn shows on the next draw with nothing to catch up.
//!
//! The Display Preferences are pushed to it
//! ([`DashboardEvent::DisplayChanged`]): the due dates follow the date style and the flag the
//! Status glyphs. Row density isn't among them, since the Dashboard draws no table rows.
//!
//! It has no list, so its keymap's movements are scroll requests ([`DashboardEvent::Scroll`]) the
//! Client applies to the page's scroll. `/` puts it in a Search with no box to type into, which
//! takes no keys until `Esc` ends it: the mode and its badge are kept as they were before the
//! Dashboard was a module.

use std::cell::RefCell;
use std::rc::Rc;

use bigdecimal::{ToPrimitive, Zero};
use chrono::NaiveDate;
use lib_core::{Clock, DateStyle, Money};
use lib_list_cursor::Step;
use lib_locale::format::{AmountStyle, format_amount, format_date, format_year_month};
use lib_viper::display::{StatusGlyphs, flag_glyph};
use lib_viper::keymap::{KeyBindingConfig, Keymap};
use lib_viper::presenter::{PageMode, PagePresenter, Presenter};

use crate::msg;
use crate::protocols::{
    DashboardBill, DashboardBudgetBar, DashboardEntryId, DashboardReads, DashboardRouter,
};

/// What the Dashboard is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardEvent {
    /// A click on a Needs Attention row.
    OpenBill(DashboardEntryId),
    /// A domain the page reads changed in the Ledger: the kept view model is stale.
    LedgerChanged,
    /// The Display Preferences the page draws with.
    DisplayChanged {
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
    },
    /// A movement key: the Client scrolls the page by `Step`, since the page has no list.
    Scroll(Step),
    /// `/`: the page enters its Search.
    Search,
    /// `Esc` in the Search: the page leaves it.
    EndSearch,
    /// `[keybindings]` changed: the keymap is built afresh under it.
    KeysChanged(KeyBindingConfig),
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
    /// Whether the page is in its Search, which `Esc` ends.
    pub searching: bool,
}

pub struct DashboardPresenter {
    reads: Rc<dyn DashboardReads>,
    clock: Clock,
    date_style: Option<DateStyle>,
    glyphs: StatusGlyphs,
    keymap: Keymap<DashboardEvent>,
    /// The keymap while the Search is open: no chord, so the Search swallows every key.
    searching_keys: Keymap<DashboardEvent>,
    searching: bool,
    /// The view model with the day it was built on, kept until the Ledger changes, the Display
    /// Preferences change or the day moves, so a render or a scroll reads no port (#1029).
    kept: RefCell<Option<(NaiveDate, DashboardViewModel)>>,
}

impl DashboardPresenter {
    /// The Dashboard over `reads`, dated by `clock`, in the default Display Preferences, its keys
    /// looked up in `keymap`.
    pub fn new(
        reads: Rc<dyn DashboardReads>,
        clock: Clock,
        keymap: Keymap<DashboardEvent>,
    ) -> Self {
        Self {
            reads,
            clock,
            date_style: None,
            glyphs: StatusGlyphs::default(),
            searching_keys: keymap.scoped(|_| false),
            keymap,
            searching: false,
            kept: RefCell::new(None),
        }
    }

    fn build(&self, today: NaiveDate) -> DashboardViewModel {
        let base_unit = self.reads.base_unit_code();
        DashboardViewModel {
            bills: self
                .reads
                .attention_bills(today)
                .into_iter()
                .map(|bill| self.attention_row(bill, base_unit.as_deref()))
                .collect(),
            flag: flag_glyph(self.glyphs),
            budgets: self.budget_list(today),
            searching: false,
        }
    }
}

impl Presenter for DashboardPresenter {
    type Event = DashboardEvent;
    type ViewModel = DashboardViewModel;
    type Router<'r> = dyn DashboardRouter + 'r;

    fn handle(&mut self, event: DashboardEvent, router: &mut Self::Router<'_>) {
        match event {
            DashboardEvent::OpenBill(id) => router.reveal_bill_entry(id),
            DashboardEvent::LedgerChanged => *self.kept.get_mut() = None,
            DashboardEvent::DisplayChanged { date_style, glyphs } => {
                self.date_style = date_style;
                self.glyphs = glyphs;
                *self.kept.get_mut() = None;
            }
            // The scroll is the Client's to apply: the Presenter holds no widget state.
            DashboardEvent::Scroll(_) => {}
            DashboardEvent::Search => self.searching = true,
            DashboardEvent::EndSearch => self.searching = false,
            DashboardEvent::KeysChanged(config) => {
                self.keymap = crate::keymap::dashboard_keymap(&config);
                self.searching_keys = self.keymap.scoped(|_| false);
            }
        }
    }

    /// The kept view model, rebuilt only when none is kept or the day has moved.
    fn view_model(&self) -> DashboardViewModel {
        let today = self.clock.today();
        let kept = match &*self.kept.borrow() {
            Some((day, kept)) if *day == today => Some(kept.clone()),
            _ => None,
        };
        let mut view = kept.unwrap_or_else(|| {
            let built = self.build(today);
            *self.kept.borrow_mut() = Some((today, built.clone()));
            built
        });
        view.searching = self.searching;
        view
    }
}

/// The Dashboard opens no Dialog, so no outcome ever lands on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardOutcome {}

impl PagePresenter for DashboardPresenter {
    type Outcome = DashboardOutcome;

    /// Never sent: the Dashboard opens no Dialog. A redraw is harmless if one ever is.
    fn dialog_closed(_outcome: Option<DashboardOutcome>) -> DashboardEvent {
        DashboardEvent::LedgerChanged
    }

    fn keymap(&self) -> &Keymap<DashboardEvent> {
        if self.searching {
            &self.searching_keys
        } else {
            &self.keymap
        }
    }

    fn mode(&self) -> Option<PageMode> {
        self.searching.then_some(PageMode::Search)
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
    fn budget_list(&self, today: NaiveDate) -> BudgetListView {
        let Some(month) = self.reads.default_budget_month(today) else {
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
        let keymap = crate::keymap::dashboard_keymap(&KeyBindingConfig::default());
        (day, DashboardPresenter::new(Rc::new(reads), clock, keymap))
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

    /// Reads that count each time the Presenter asks a port, so a test sees a kept view model.
    struct CountingReads {
        inner: InMemoryDashboardReads,
        asked: Cell<usize>,
    }

    impl crate::protocols::DashboardBills for CountingReads {
        fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill> {
            self.asked.set(self.asked.get() + 1);
            self.inner.attention_bills(today)
        }

        fn base_unit_code(&self) -> Option<String> {
            self.inner.base_unit_code()
        }
    }

    impl crate::protocols::DashboardBudgets for CountingReads {
        fn default_budget_month(
            &self,
            today: NaiveDate,
        ) -> Option<crate::protocols::DashboardBudgetMonth> {
            self.asked.set(self.asked.get() + 1);
            self.inner.default_budget_month(today)
        }
    }

    #[test]
    fn the_view_model_reads_the_ports_once_until_a_change_or_a_new_day() {
        locale();
        let reads = Rc::new(CountingReads {
            inner: reads_with_bills(vec![bill(1, "Rent", date(2026, 9, 24), "2400.00", "aud")]),
            asked: Cell::new(0),
        });
        let (clock, day) = Clock::settable(date(2026, 9, 21));
        let keymap = crate::keymap::dashboard_keymap(&KeyBindingConfig::default());
        let mut presenter = DashboardPresenter::new(reads.clone(), clock, keymap);
        let mut router = RecordingRouter::default();

        presenter.view_model();
        let once = reads.asked.get();
        assert!(once > 0);
        assert_eq!(presenter.view_model().bills.len(), 1);
        assert_eq!(reads.asked.get(), once, "a repeat reads no port");

        presenter.handle(DashboardEvent::LedgerChanged, &mut router);
        presenter.view_model();
        assert_eq!(reads.asked.get(), once * 2, "a change reads again");

        presenter.handle(
            DashboardEvent::DisplayChanged {
                date_style: None,
                glyphs: StatusGlyphs::default(),
            },
            &mut router,
        );
        presenter.view_model();
        assert_eq!(reads.asked.get(), once * 3, "a Display change rebuilds");

        day.set(date(2026, 9, 22));
        presenter.view_model();
        assert_eq!(reads.asked.get(), once * 4, "a new day reads again");
        presenter.view_model();
        assert_eq!(reads.asked.get(), once * 4);
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

    #[test]
    fn the_search_takes_no_key_until_it_ends() {
        let (_, mut presenter) = presenter(InMemoryDashboardReads::default());
        let mut router = RecordingRouter::default();
        assert_eq!(
            presenter.keymap().event("j"),
            Some(DashboardEvent::Scroll(Step::Next))
        );

        presenter.handle(DashboardEvent::Search, &mut router);
        assert_eq!(presenter.mode(), Some(PageMode::Search));
        assert!(presenter.view_model().searching);
        assert_eq!(
            presenter.keymap().event("j"),
            None,
            "the Search swallows it"
        );
        assert_eq!(
            presenter.typed(lib_viper::presenter::TextKey::Char('x')),
            None
        );

        presenter.handle(DashboardEvent::EndSearch, &mut router);
        assert_eq!(presenter.mode(), None);
        assert_eq!(
            presenter.keymap().event("j"),
            Some(DashboardEvent::Scroll(Step::Next))
        );
        assert!(router.routed.is_empty());
    }

    #[test]
    fn rebound_keys_reach_the_keymap() {
        let (_, mut presenter) = presenter(InMemoryDashboardReads::default());
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(lib_viper::keymap::MOVE_DOWN.to_string(), "n".to_string());

        presenter.handle(
            DashboardEvent::KeysChanged(config),
            &mut RecordingRouter::default(),
        );

        assert_eq!(
            presenter.keymap().event("n"),
            Some(DashboardEvent::Scroll(Step::Next))
        );
        assert_eq!(presenter.keymap().event("j"), None);
    }
}
