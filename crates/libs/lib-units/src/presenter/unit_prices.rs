//! The Unit prices view: one Unit's PRICE figures over its WEEKLY CLOSE chart and its WEEKLY PRICES
//! table, in a 12-month window `[` steps back and `]` forward by a month, never past the current
//! week. Opened from the Units page ([`UnitsRouter::open_prices`]) in place of its tables, so
//! closing it ([`UnitPricesEvent::Back`]) finds the page's selection as it was.
//!
//! Prices show at the quote Unit's Precision. The chart's values are plain numbers, for drawing
//! only; every figure a person reads is formatted here.

use bigdecimal::{BigDecimal, Signed as _, ToPrimitive as _, Zero as _};
use chrono::{DateTime, Duration, Local, Months, NaiveDate, Utc};
use lib_core::{Money, UnitId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::format::{AmountStyle, format_amount, format_date, format_month_day};
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};

use super::units_page::Hint;
use super::units_pane::{Fact, amount};
use super::{Clock, ago};
use crate::keymap::{EARLIER, LATER, SET_PRICE};
use crate::protocols::{SharedPorts, UnitsDialog, UnitsRouter};
use crate::{MANUAL_ENTRY, UnitRow, WeeklyPrice, msg};

/// The weeks a window holds: a year of them, ending with the window's last week.
const WINDOW_WEEKS: i64 = 52;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitPricesEvent {
    /// Walks the table's weeks, newest first.
    Move(Step),
    /// A click on a week's row.
    Select(NaiveDate),
    /// `[`: the window a month earlier.
    Earlier,
    /// `]`: the window a month later, stopping at the current week.
    Later,
    /// `p`: Set price, for a Unit on Manual entry.
    SetPrice,
    /// `esc`: back to the Units page.
    Back,
}

/// The view as drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitPricesViewModel {
    pub unit: UnitId,
    /// `btc · Bitcoin · prices`.
    pub title: String,
    /// The PRICE figures: last price and its age, week change, 52w range, symbol · priced in.
    pub facts: Vec<Fact>,
    pub chart_heading: String,
    pub table_heading: String,
    /// The window's first and last weeks: `27 Oct 2025 – 19 Oct 2026`.
    pub window: String,
    /// Whether `]` has a later window to step to.
    pub can_go_later: bool,
    /// The window's priced weeks, oldest first.
    pub chart: Vec<ChartWeek>,
    pub columns: PriceColumns,
    /// The window's priced weeks, newest first.
    pub rows: Vec<PriceRow>,
    /// What shows in place of the chart and table while the window has no prices.
    pub empty: Option<String>,
    /// The status-line legend, as `(chord, action)`.
    pub hints: Vec<Hint>,
    /// The action of the Client's own back key, which the view's keymap doesn't hold.
    pub back: String,
}

/// One week as the chart draws it: a candle when it has an open, high and low, otherwise a tick
/// at its close, whose open, high and low then repeat the close.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartWeek {
    pub week: NaiveDate,
    /// Its Monday without the year, for the axis.
    pub label: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub candle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceColumns {
    pub week: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub change: String,
}

/// One week of the table, each price at the quote Unit's Precision, — where the week has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    pub week: NaiveDate,
    /// Its Monday, in the Locale's medium form.
    pub label: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    /// The close's change from the week before, `+1.25%`, or — without one to compare.
    pub change: String,
    /// Whether the close fell from the week before, which the Clients mark.
    pub falling: bool,
    pub selected: bool,
}

pub struct UnitPricesPresenter {
    ports: SharedPorts,
    clock: Clock,
    keymap: Keymap<UnitPricesEvent>,
    unit: UnitId,
    /// How many months before the Clock's day the window ends. Counted rather than stored as a
    /// day, so `]` after `[` lands back on the same week whatever the month's length.
    months_back: u32,
    /// It may name a week out of the window, so it is read through [`resolve`].
    selected: Option<NaiveDate>,
}

impl UnitPricesPresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(
        ports: SharedPorts,
        clock: Clock,
        keymap: Keymap<UnitPricesEvent>,
        unit: UnitId,
    ) -> Self {
        Self {
            ports,
            clock,
            keymap,
            unit,
            months_back: 0,
            selected: None,
        }
    }

    /// The Unit it shows.
    pub fn unit(&self) -> UnitId {
        self.unit
    }

    pub fn handle(&mut self, event: UnitPricesEvent, router: &mut dyn UnitsRouter) {
        match event {
            UnitPricesEvent::Move(step) => {
                let weeks = self.row_weeks();
                if let Some(week) = step_id(&weeks, resolve(weeks.clone(), self.selected), step) {
                    self.selected = Some(week);
                }
            }
            UnitPricesEvent::Select(week) => self.selected = Some(week),
            UnitPricesEvent::Earlier => {
                self.months_back = self.months_back.saturating_add(1);
            }
            UnitPricesEvent::Later => self.months_back = self.months_back.saturating_sub(1),
            UnitPricesEvent::SetPrice => {
                if self.unit_row().is_some_and(|unit| manual(&unit)) {
                    router.open_dialog(UnitsDialog::SetPrice(self.unit));
                }
            }
            UnitPricesEvent::Back => router.close_prices(),
        }
    }

    pub fn view_model(&self) -> UnitPricesViewModel {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        let unit = units.iter().find(|unit| unit.id == self.unit);
        let quote = unit.and_then(|unit| units.iter().find(|quote| quote.id == unit.link.quote));
        let (first, last) = self.window();
        let in_window: Vec<&WeeklyPrice> = unit
            .map(|unit| {
                unit.prices
                    .iter()
                    .filter(|price| (first..=last).contains(&price.week))
                    .collect()
            })
            .unwrap_or_default();
        let weeks: Vec<NaiveDate> = in_window.iter().rev().map(|price| price.week).collect();
        let selected = resolve(weeks, self.selected);
        let places = |money: &Money| format_amount(money, quoted(quote)).text;
        let optional = |money: Option<&Money>| money.map_or_else(msg::units_none, places);

        let rows = in_window
            .iter()
            .rev()
            .map(|price| {
                let change = unit.and_then(|unit| change(unit, price));
                PriceRow {
                    week: price.week,
                    label: format_date(price.week, None),
                    open: optional(price.open.as_ref()),
                    high: optional(price.high.as_ref()),
                    low: optional(price.low.as_ref()),
                    close: places(&price.close),
                    falling: change.as_ref().is_some_and(|change| change.0.is_negative()),
                    change: change.map_or_else(msg::units_none, |change| {
                        msg::units_prices_change(&percent(&change))
                    }),
                    selected: selected == Some(price.week),
                }
            })
            .collect();
        let chart = in_window
            .iter()
            .map(|price| {
                let close = number(&price.close);
                let candle = price.open.is_some() && price.high.is_some() && price.low.is_some();
                let or_close = |money: Option<&Money>| money.map_or(close, number);
                ChartWeek {
                    week: price.week,
                    label: format_month_day(price.week),
                    open: or_close(price.open.as_ref()),
                    high: or_close(price.high.as_ref()),
                    low: or_close(price.low.as_ref()),
                    close,
                    candle,
                }
            })
            .collect();
        let empty = in_window.is_empty().then(|| match unit {
            Some(unit) if !unit.prices.is_empty() => msg::units_prices_window_empty(),
            _ => msg::units_prices_empty(unit.map_or("", |unit| unit.code.as_str())),
        });

        UnitPricesViewModel {
            unit: self.unit,
            title: unit.map_or_else(String::new, |unit| {
                msg::units_prices_title(&unit.code, &unit.name)
            }),
            facts: unit.map_or_else(Vec::new, |unit| self.facts(unit, quote)),
            chart_heading: msg::units_prices_chart_heading(),
            table_heading: msg::units_prices_table_heading(),
            window: msg::units_prices_window(&format_date(first, None), &format_date(last, None)),
            can_go_later: self.months_back > 0,
            chart,
            columns: PriceColumns {
                week: msg::units_prices_column_week(),
                open: msg::units_prices_column_open(),
                high: msg::units_prices_column_high(),
                low: msg::units_prices_column_low(),
                close: msg::units_prices_column_close(),
                change: msg::units_prices_column_change(),
            },
            rows,
            empty,
            hints: self.hints(unit.is_some_and(manual)),
            back: msg::units_prices_hint_back(),
        }
    }

    /// The PRICE figures, which read the Unit's latest prices whatever the window, then its
    /// symbol and the Unit it is priced in.
    fn facts(&self, unit: &UnitRow, quote: Option<&UnitRow>) -> Vec<Fact> {
        let code = quote.map_or("", |quote| quote.code.as_str());
        let symbol = if unit.link.symbol.is_empty() {
            msg::units_none()
        } else {
            unit.link.symbol.clone()
        };
        let mut facts = price_figures(unit, quote, (self.clock)());
        facts.push(Fact::plain(
            msg::units_prices_symbol(),
            msg::units_prices_symbol_value(&symbol, code),
        ));
        facts
    }

    fn hints(&self, manual: bool) -> Vec<Hint> {
        let chord = |command| self.keymap.chord(command).unwrap_or_default().to_string();
        let hint = |chord: String, action: String| Hint { chord, action };
        let mut hints = vec![
            hint(
                format!("{}/{}", chord(MOVE_DOWN), chord(MOVE_UP)),
                msg::units_prices_hint_week(),
            ),
            hint(chord(EARLIER), msg::units_prices_hint_earlier()),
            hint(chord(LATER), msg::units_prices_hint_later()),
        ];
        if manual {
            hints.push(hint(chord(SET_PRICE), msg::units_hint_set_price()));
        }
        hints
    }

    fn unit_row(&self) -> Option<UnitRow> {
        self.ports.borrow().units().unit(self.unit)
    }

    fn today(&self) -> NaiveDate {
        (self.clock)().with_timezone(&Local).date_naive()
    }

    /// A day in the window's last week: the Clock's day, `months_back` months earlier.
    fn end_day(&self) -> NaiveDate {
        let today = self.today();
        today
            .checked_sub_months(Months::new(self.months_back))
            .unwrap_or(today)
    }

    /// The window's first and last weeks' Mondays.
    fn window(&self) -> (NaiveDate, NaiveDate) {
        let last = WeeklyPrice::week_of(self.end_day());
        (last - Duration::weeks(WINDOW_WEEKS - 1), last)
    }

    /// The table's weeks in drawn order, newest first.
    fn row_weeks(&self) -> Vec<NaiveDate> {
        let (first, last) = self.window();
        self.unit_row()
            .map(|unit| {
                unit.prices
                    .iter()
                    .rev()
                    .map(|price| price.week)
                    .filter(|week| (first..=last).contains(week))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A Unit's PRICE figures, which the Units page's docked pane shows too: its last price and that
/// price's age, the week change from the week before's close, and the 52-week range, the lowest low
/// or close and the highest high or close of the year of weeks ending with the last price's.
pub(super) fn price_figures(
    unit: &UnitRow,
    quote: Option<&UnitRow>,
    now: DateTime<Utc>,
) -> Vec<Fact> {
    let code = quote.map_or("", |quote| quote.code.as_str());
    let last = unit.last_price();
    let last_price = last.map_or_else(msg::units_none, |price| {
        msg::units_prices_last_price_value(
            &amount(&price.close, quote),
            &ago(price.written_at, now),
        )
    });
    let week_change = last
        .and_then(|price| {
            let before = previous(unit, price)?;
            let difference = Money(&price.close.0 - &before.close.0);
            let style = quoted(quote);
            Some(msg::units_prices_week_change_value(
                &format!(
                    "{} {code}",
                    format_amount(&difference, style.with_plus()).text
                ),
                &percent(&change(unit, price)?),
            ))
        })
        .unwrap_or_else(msg::units_none);
    let range = last
        .and_then(|price| {
            let from = price.week - Duration::weeks(WINDOW_WEEKS - 1);
            let year = unit.prices.iter().filter(|price| price.week >= from);
            let low = year
                .clone()
                .map(|price| price.low.as_ref().unwrap_or(&price.close))
                .min()?;
            let high = year
                .map(|price| price.high.as_ref().unwrap_or(&price.close))
                .max()?;
            let style = quoted(quote);
            Some(msg::units_prices_range_value(
                &format_amount(low, style).text,
                &format_amount(high, style).text,
                code,
            ))
        })
        .unwrap_or_else(msg::units_none);
    vec![
        Fact::plain(msg::units_prices_last_price(), last_price),
        Fact::plain(msg::units_prices_week_change(), week_change),
        Fact::plain(msg::units_prices_range(), range),
    ]
}

/// A price's style: the quote Unit's Precision, or the price's own places when the quote is gone.
fn quoted(quote: Option<&UnitRow>) -> AmountStyle {
    quote.map_or(AmountStyle::own(), |quote| {
        AmountStyle::fixed(quote.precision.places())
    })
}

fn manual(unit: &UnitRow) -> bool {
    unit.link.source_id == MANUAL_ENTRY
}

/// The Weekly Price of the week before `price`'s, if `unit` has one.
fn previous<'a>(unit: &'a UnitRow, price: &WeeklyPrice) -> Option<&'a WeeklyPrice> {
    let week = price.week - Duration::weeks(1);
    unit.prices.iter().find(|before| before.week == week)
}

/// `price`'s close against the week before's, as a percentage; `None` without a week before, or
/// one that closed at zero.
fn change(unit: &UnitRow, price: &WeeklyPrice) -> Option<Money> {
    let before = &previous(unit, price)?.close.0;
    if before.is_zero() {
        return None;
    }
    Some(Money(
        (&price.close.0 - before) * BigDecimal::from(100) / before,
    ))
}

/// A percentage to two places, signed: `+1.25`, `−0.40`.
fn percent(change: &Money) -> String {
    format_amount(change, AmountStyle::fixed(2).with_plus()).text
}

fn number(money: &Money) -> f64 {
    money.0.to_f64().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_config::KeyBindingConfig;

    use super::super::test_support::{clock, stub};
    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::keymap::unit_prices_keymap;
    use crate::protocols::UnitRepository as _;

    struct View {
        ports: std::rc::Rc<std::cell::RefCell<InMemoryPorts>>,
        presenter: UnitPricesPresenter,
        router: RecordingRouter,
    }

    impl View {
        fn of(unit: UnitId) -> Self {
            let (ports, shared) = stub();
            Self {
                ports,
                presenter: UnitPricesPresenter::new(
                    shared,
                    clock(),
                    unit_prices_keymap(&KeyBindingConfig::default()),
                    unit,
                ),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: UnitPricesEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> UnitPricesViewModel {
            self.presenter.view_model()
        }
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid date")
    }

    fn fact(view: &UnitPricesViewModel, label: &str) -> String {
        view.facts
            .iter()
            .find(|fact| fact.label == label)
            .map(|fact| fact.value.clone())
            .unwrap_or_else(|| panic!("no {label} in {:?}", view.facts))
    }

    #[test]
    fn seeded_btc_shows_a_year_of_weeks_newest_first_in_the_table_and_oldest_first_in_the_chart() {
        let view = View::of(UnitId::SEED_BTC).view();

        assert_eq!(view.title, "btc · Bitcoin · prices");
        assert_eq!(view.rows.len(), 52);
        assert_eq!(view.chart.len(), 52);
        // The tests' clock reads Wednesday 21 Oct 2026, in the week of Monday 19 Oct.
        assert_eq!(view.rows[0].week, day(2026, 10, 19));
        assert_eq!(view.rows[51].week, day(2025, 10, 27));
        assert_eq!(view.chart[0].week, day(2025, 10, 27));
        assert_eq!(view.window, "Oct 27, 2025 – Oct 19, 2026");
        assert!(view.rows[0].selected, "the newest week starts selected");
        assert!(
            view.chart.iter().all(|week| week.candle),
            "seeded weeks have OHLC"
        );
        assert_eq!(view.empty, None);
        assert!(!view.can_go_later);
    }

    #[test]
    fn a_row_shows_its_prices_at_the_quote_units_precision_and_its_change() {
        let view = View::of(UnitId::SEED_BTC).view();
        let newest = &view.rows[0];

        assert_eq!(newest.label, "Oct 19, 2026");
        assert_eq!(newest.close, "148,212.40");
        assert!(newest.open.ends_with(|ch: char| ch.is_ascii_digit()));
        assert!(newest.open.contains('.'), "two places: {}", newest.open);
        assert!(newest.change.ends_with('%'), "{}", newest.change);
        assert_eq!(
            view.rows[51].change, "—",
            "the oldest has no week before it"
        );
    }

    #[test]
    fn the_header_repeats_the_price_figures() {
        let view = View::of(UnitId::SEED_BTC).view();

        assert_eq!(fact(&view, "Last price"), "148,212.40 aud · 5 minutes ago");
        let change = fact(&view, "Week change");
        assert!(
            change.ends_with("%)") && change.contains(" aud ("),
            "{change}"
        );
        let range = fact(&view, "52w range");
        assert!(range.ends_with(" aud") && range.contains(" – "), "{range}");
        assert_eq!(fact(&view, "Symbol · priced in"), "bitcoin · aud");
    }

    #[test]
    fn earlier_steps_the_window_back_a_month_and_later_forward_stopping_at_the_current_week() {
        let mut view = View::of(UnitId::SEED_BTC);

        view.send(UnitPricesEvent::Earlier);
        let earlier = view.view();
        // 21 Sep 2026 is a Monday, so the window ends in its week.
        assert_eq!(earlier.rows[0].week, day(2026, 9, 21));
        assert_eq!(earlier.rows.len(), 48, "the seed starts on 27 Oct 2025");
        assert!(earlier.can_go_later);

        view.send(UnitPricesEvent::Later);
        assert_eq!(view.view().rows[0].week, day(2026, 10, 19));
        assert!(!view.view().can_go_later);

        view.send(UnitPricesEvent::Later);
        let still = view.view();
        assert_eq!(
            still.rows[0].week,
            day(2026, 10, 19),
            "never past the current week"
        );
        assert_eq!(still.rows.len(), 52);
    }

    #[test]
    fn earlier_then_later_from_a_months_last_day_lands_back_on_the_current_week() {
        let (_ports, shared) = stub();
        // Wednesday 31 Mar 2027: a month back is 28 Feb, and a month on from that is 28 Mar.
        let clock: Clock = std::rc::Rc::new(|| {
            chrono::TimeZone::with_ymd_and_hms(&Utc, 2027, 3, 31, 9, 30, 0)
                .single()
                .expect("a valid instant")
        });
        let mut presenter = UnitPricesPresenter::new(
            shared,
            clock,
            unit_prices_keymap(&KeyBindingConfig::default()),
            UnitId::SEED_BTC,
        );
        let window = |presenter: &UnitPricesPresenter| presenter.view_model().window;
        let current = window(&presenter);
        let mut router = RecordingRouter::default();

        presenter.handle(UnitPricesEvent::Earlier, &mut router);
        assert_ne!(window(&presenter), current);
        presenter.handle(UnitPricesEvent::Later, &mut router);

        assert_eq!(window(&presenter), current);
        assert!(!presenter.view_model().can_go_later);
    }

    #[test]
    fn a_unit_with_no_weekly_prices_shows_the_empty_state() {
        let view = View::of(UnitId::SEED_VAS).view();

        assert_eq!(view.empty.as_deref(), Some("vas has no weekly prices yet."));
        assert!(view.rows.is_empty() && view.chart.is_empty());
        assert_eq!(fact(&view, "Last price"), "—");
        assert_eq!(fact(&view, "Symbol · priced in"), "— · aud");
    }

    #[test]
    fn a_window_before_the_first_price_says_so() {
        let mut view = View::of(UnitId::SEED_BTC);
        for _ in 0..12 {
            view.send(UnitPricesEvent::Earlier);
        }
        assert_eq!(
            view.view().empty.as_deref(),
            Some("No weekly prices in this window.")
        );
    }

    #[test]
    fn a_close_only_week_draws_as_a_tick_at_its_close() {
        let view = View::of(UnitId::SEED_VAS);
        let mut vas = view
            .ports
            .borrow()
            .units
            .unit(UnitId::SEED_VAS)
            .expect("vas");
        let at = super::super::test_support::now();
        vas.set_price(
            Money(BigDecimal::from(42)),
            day(2026, 10, 12),
            day(2026, 10, 21),
            at,
        );
        view.ports.borrow_mut().units.upsert_unit(vas);

        let model = view.view();
        assert_eq!(model.chart.len(), 1);
        let week = &model.chart[0];
        assert!(!week.candle);
        assert_eq!(
            (week.open, week.high, week.low, week.close),
            (42.0, 42.0, 42.0, 42.0)
        );
        assert_eq!(model.rows[0].open, "—");
        assert_eq!(model.rows[0].close, "42.00");
    }

    #[test]
    fn the_moves_walk_the_weeks() {
        let mut view = View::of(UnitId::SEED_BTC);
        view.send(UnitPricesEvent::Move(Step::Next));
        assert!(view.view().rows[1].selected);
        view.send(UnitPricesEvent::Move(Step::Last));
        assert!(view.view().rows[51].selected);
        view.send(UnitPricesEvent::Select(day(2026, 10, 12)));
        assert!(view.view().rows[1].selected);
    }

    #[test]
    fn set_price_opens_its_dialog_only_for_a_manual_entry_unit() {
        let mut btc = View::of(UnitId::SEED_BTC);
        assert_eq!(btc.send(UnitPricesEvent::SetPrice), []);
        assert!(!btc.view().hints.iter().any(|hint| hint.chord == "p"));

        let mut vas = View::of(UnitId::SEED_VAS);
        assert_eq!(
            vas.send(UnitPricesEvent::SetPrice),
            [RouterCall::OpenDialog(UnitsDialog::SetPrice(
                UnitId::SEED_VAS
            ))]
        );
        assert!(vas.view().hints.iter().any(|hint| hint.chord == "p"));
    }

    #[test]
    fn back_closes_the_view() {
        let mut view = View::of(UnitId::SEED_BTC);
        assert_eq!(view.send(UnitPricesEvent::Back), [RouterCall::ClosePrices]);
    }

    #[test]
    fn the_legend_names_the_keymaps_chords() {
        let view = View::of(UnitId::SEED_BTC).view();
        let hints: Vec<_> = view
            .hints
            .iter()
            .map(|hint| (hint.chord.as_str(), hint.action.as_str()))
            .collect();
        assert_eq!(hints, [("j/k", "week"), ("[", "earlier"), ("]", "later")]);
        assert_eq!(view.back, "back");
    }
}
