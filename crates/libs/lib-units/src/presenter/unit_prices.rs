//! The Unit prices view: one Unit's PRICE figures over its chart and its prices table. At Week, a
//! 12-month window of its Weekly Prices that `[` steps back and `]` forward by a month, never past
//! the current week. `z` steps to the finer grains its source offers (Week → Day → Hour → Week):
//! Daily Prices in 13-week windows stepped a week, Hourly Prices in 7-day windows stepped a day,
//! asked of the source as the view shows them and never written (`GLOSSARY.md`). The figures stay
//! weekly at every grain. A state of the Units page, which holds it while it shows in place of the
//! tables and hands it its events (`UnitsPageEvent::Prices`). Closing it
//! ([`UnitPricesEvent::Back`]) is the page's to do, so it finds the page's selection as it was.
//!
//! Prices show at the quote Unit's Precision. The chart's values are plain numbers, for drawing
//! only; every figure a person reads is formatted here.

use bigdecimal::{BigDecimal, Signed as _, ToPrimitive as _, Zero as _};
use chrono::{DateTime, Duration, Local, Months, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use lib_core::{Money, UnitId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::format::{
    AmountStyle, format_amount, format_date, format_month_day, format_time_minutes,
};
use lib_toast::Notice;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};
use lib_viper::presenter::Presenter;
use lib_viper::view::Hint;

use super::fetch_text::{Asked, failure_detail};
use super::units_pane::{Fact, amount};
use super::{Clock, ago};
use crate::fetching::{FineCall, FineState, SharedFetching, SourceSettings, key_missing};
use crate::fine::{FineKey, FinePrice, window as fine_window};
use crate::history::history_request;
use crate::keymap::{EARLIER, GRAIN, LATER, SET_PRICE};
use crate::protocols::{SharedPorts, UnitsDialog, UnitsRouter};
use crate::{FetchFailure, Grain, MANUAL_ENTRY, UnitRow, WeeklyPrice, msg};

/// The weeks a window holds: a year of them, ending with the window's last week.
const WINDOW_WEEKS: i64 = 52;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitPricesEvent {
    /// Walks the table's rows, newest first.
    Move(Step),
    /// A click on a row, named by the start of its week, day or hour.
    Select(NaiveDateTime),
    /// `[`: the window a step earlier, stopping at the source's reach.
    Earlier,
    /// `]`: the window a step later, stopping at the current one.
    Later,
    /// `z`: the next grain the source offers.
    CycleGrain,
    /// A click on the Desktop's *Week · Day · Hour* switch.
    SetGrain(Grain),
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
    /// Weekly at every grain.
    pub facts: Vec<Fact>,
    /// The grain showing.
    pub grain: Grain,
    /// The grains the source offers, in cycle order; empty when it offers Week alone.
    pub grains: Vec<GrainChoice>,
    /// Names the grain: `Weekly close`, `Daily close`, `Hourly close`.
    pub chart_heading: String,
    pub table_heading: String,
    /// The window's first and last weeks or days: `27 Oct 2025 – 19 Oct 2026`.
    pub window: String,
    /// Whether `]` has a later window to step to.
    pub can_go_later: bool,
    /// The window's prices, oldest first.
    pub chart: Vec<ChartWeek>,
    pub columns: PriceColumns,
    /// The window's prices, newest first.
    pub rows: Vec<PriceRow>,
    /// What shows in place of the chart and table while the window has no prices to show: none
    /// yet, a finer grain still fetching, or why it failed.
    pub empty: Option<String>,
    /// The status-line legend, as `(chord, action)`.
    pub hints: Vec<Hint>,
    /// The action of the Client's own back key, which the view's keymap doesn't hold.
    pub back: String,
}

/// One grain on the Desktop's switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrainChoice {
    pub grain: Grain,
    pub label: String,
    pub active: bool,
}

/// One week, day or hour as the chart draws it: a candle when it has an open, high and low,
/// otherwise a tick at its close, whose open, high and low then repeat the close.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartWeek {
    /// The day it starts on: a week's Monday, or a day or hour's day.
    pub week: NaiveDate,
    /// When it starts.
    pub at: NaiveDateTime,
    /// Its start without the year, for the axis.
    pub label: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub candle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceColumns {
    /// Names the grain: WEEK, DAY or HOUR.
    pub week: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub change: String,
}

/// One row of the table, each price at the quote Unit's Precision, — where it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    /// The day it starts on: a week's Monday, or a day or hour's day.
    pub week: NaiveDate,
    /// When it starts, which a click selects it by.
    pub at: NaiveDateTime,
    /// Its start, in the Locale's medium form, with the time for an hour.
    pub label: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    /// The close's change from the row before, `+1.25%`, or — without one to compare.
    pub change: String,
    /// Whether the close fell from the row before, which the Clients mark.
    pub falling: bool,
    pub selected: bool,
}

/// One price as the table and chart read it, whatever its grain.
struct Shown {
    at: NaiveDateTime,
    open: Option<Money>,
    high: Option<Money>,
    low: Option<Money>,
    close: Money,
    /// The close of the point before, for CHANGE.
    before: Option<Money>,
}

pub struct UnitPricesPresenter {
    ports: SharedPorts,
    clock: Clock,
    keymap: Keymap<UnitPricesEvent>,
    unit: UnitId,
    /// The Client's fetching state, without which only Week is offered.
    fetching: Option<SharedFetching>,
    grain: Grain,
    /// How many months before the Clock's day the Week window ends. Counted rather than stored as
    /// a day, so `]` after `[` lands back on the same week whatever the month's length.
    months_back: u32,
    /// How many steps (a week at Day, a day at Hour) before the current window the finer one is.
    fine_back: u32,
    /// It may name a row out of the window, so it is read through [`resolve`].
    selected: Option<NaiveDateTime>,
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
            fetching: None,
            grain: Grain::Week,
            months_back: 0,
            fine_back: 0,
            selected: None,
        }
    }

    /// The view asking the Unit's source for its finer grains through `fetching`, the Client's.
    #[must_use]
    pub fn with_fetching(mut self, fetching: Option<SharedFetching>) -> Self {
        self.fetching = fetching;
        self
    }

    /// The Unit it shows.
    pub fn unit(&self) -> UnitId {
        self.unit
    }
}

impl Presenter for UnitPricesPresenter {
    type Event = UnitPricesEvent;
    type ViewModel = UnitPricesViewModel;
    type Router<'r> = dyn UnitsRouter + 'r;

    fn handle(&mut self, event: UnitPricesEvent, router: &mut Self::Router<'_>) {
        match event {
            UnitPricesEvent::Move(step) => {
                let rows = self.row_starts();
                if let Some(at) = step_id(&rows, resolve(rows.clone(), self.selected), step) {
                    self.selected = Some(at);
                }
            }
            UnitPricesEvent::Select(at) => self.selected = Some(at),
            UnitPricesEvent::Earlier => match self.grain {
                Grain::Week => self.months_back = self.months_back.saturating_add(1),
                Grain::Day | Grain::Hour => {
                    if let Some(refusal) = self.beyond_reach() {
                        router.notify(Notice::Status(refusal));
                    } else {
                        self.fine_back = self.fine_back.saturating_add(1);
                    }
                    self.ask();
                }
            },
            UnitPricesEvent::Later => match self.grain {
                Grain::Week => self.months_back = self.months_back.saturating_sub(1),
                Grain::Day | Grain::Hour => {
                    self.fine_back = self.fine_back.saturating_sub(1);
                    self.ask();
                }
            },
            UnitPricesEvent::CycleGrain => {
                let next = self.grain.next_in(&self.grains());
                self.set_grain(next);
            }
            UnitPricesEvent::SetGrain(grain) => {
                if self.grains().contains(&grain) {
                    self.set_grain(grain);
                }
            }
            UnitPricesEvent::SetPrice => {
                if self.unit_row().is_some_and(|unit| manual(&unit)) {
                    router.open_dialog(UnitsDialog::SetPrice(self.unit));
                }
            }
            // The page closes the view, which it holds.
            UnitPricesEvent::Back => {}
        }
    }

    fn view_model(&self) -> UnitPricesViewModel {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        drop(ports);
        let unit = units.iter().find(|unit| unit.id == self.unit);
        let quote = unit.and_then(|unit| units.iter().find(|quote| quote.id == unit.link.quote));
        let (first, last) = self.window();
        let (shown, empty) = match unit {
            Some(unit) => self.shown(unit, quote, (first, last)),
            None => (Vec::new(), Some(msg::units_prices_empty(""))),
        };
        let starts: Vec<NaiveDateTime> = shown.iter().rev().map(|price| price.at).collect();
        let selected = resolve(starts, self.selected);
        let places = |money: &Money| format_amount(money, quoted(quote)).text;
        let optional = |money: Option<&Money>| money.map_or_else(msg::units_none, places);

        let rows = shown
            .iter()
            .rev()
            .map(|price| {
                let change = price
                    .before
                    .as_ref()
                    .and_then(|before| change_from(before, &price.close));
                PriceRow {
                    week: price.at.date(),
                    at: price.at,
                    label: self.row_label(price.at),
                    open: optional(price.open.as_ref()),
                    high: optional(price.high.as_ref()),
                    low: optional(price.low.as_ref()),
                    close: places(&price.close),
                    falling: change.as_ref().is_some_and(|change| change.0.is_negative()),
                    change: change.map_or_else(msg::units_none, |change| {
                        msg::units_prices_change(&percent(&change))
                    }),
                    selected: selected == Some(price.at),
                }
            })
            .collect();
        let chart = shown
            .iter()
            .map(|price| {
                let close = number(&price.close);
                let candle = price.open.is_some() && price.high.is_some() && price.low.is_some();
                let or_close = |money: Option<&Money>| money.map_or(close, number);
                ChartWeek {
                    week: price.at.date(),
                    at: price.at,
                    label: self.axis_label(price.at),
                    open: or_close(price.open.as_ref()),
                    high: or_close(price.high.as_ref()),
                    low: or_close(price.low.as_ref()),
                    close,
                    candle,
                }
            })
            .collect();
        let offered = self.grains();
        let grains = if offered.len() > 1 {
            offered
                .iter()
                .map(|grain| GrainChoice {
                    grain: *grain,
                    label: match grain {
                        Grain::Week => msg::units_prices_grain_week(),
                        Grain::Day => msg::units_prices_grain_day(),
                        Grain::Hour => msg::units_prices_grain_hour(),
                    },
                    active: *grain == self.grain,
                })
                .collect()
        } else {
            Vec::new()
        };

        UnitPricesViewModel {
            unit: self.unit,
            title: unit.map_or_else(String::new, |unit| {
                msg::units_prices_title(&unit.code, &unit.name)
            }),
            facts: unit.map_or_else(Vec::new, |unit| self.facts(unit, quote)),
            grain: self.grain,
            grains,
            chart_heading: match self.grain {
                Grain::Week => msg::units_prices_chart_heading(),
                Grain::Day => msg::units_prices_chart_heading_day(),
                Grain::Hour => msg::units_prices_chart_heading_hour(),
            },
            table_heading: match self.grain {
                Grain::Week => msg::units_prices_table_heading(),
                Grain::Day => msg::units_prices_table_heading_day(),
                Grain::Hour => msg::units_prices_table_heading_hour(),
            },
            window: msg::units_prices_window(&format_date(first, None), &format_date(last, None)),
            can_go_later: match self.grain {
                Grain::Week => self.months_back > 0,
                Grain::Day | Grain::Hour => self.fine_back > 0,
            },
            chart,
            columns: PriceColumns {
                week: match self.grain {
                    Grain::Week => msg::units_prices_column_week(),
                    Grain::Day => msg::units_prices_column_day(),
                    Grain::Hour => msg::units_prices_column_hour(),
                },
                open: msg::units_prices_column_open(),
                high: msg::units_prices_column_high(),
                low: msg::units_prices_column_low(),
                close: msg::units_prices_column_close(),
                change: msg::units_prices_column_change(),
            },
            rows,
            empty,
            hints: self.hints(unit.is_some_and(manual), offered.len() > 1),
            back: msg::units_prices_hint_back(),
        }
    }
}

impl UnitPricesPresenter {
    /// The PRICE figures, which read the Unit's latest prices whatever the window, then its
    /// symbol and the Unit it is priced in.
    fn facts(&self, unit: &UnitRow, quote: Option<&UnitRow>) -> Vec<Fact> {
        let code = quote.map_or("", |quote| quote.code.as_str());
        let symbol = if unit.link.symbol.is_empty() {
            msg::units_none()
        } else {
            unit.link.symbol.clone()
        };
        let mut facts = price_figures(unit, quote, self.now());
        facts.push(Fact::plain(
            msg::units_prices_symbol(),
            msg::units_prices_symbol_value(&symbol, code),
        ));
        facts
    }

    fn hints(&self, manual: bool, grains: bool) -> Vec<Hint> {
        let chord = |command| self.keymap.hint_chord(command);
        let mut hints = vec![
            Hint::new(
                self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
                msg::units_prices_hint_week(),
            ),
            Hint::new(chord(EARLIER), msg::units_prices_hint_earlier()),
            Hint::new(chord(LATER), msg::units_prices_hint_later()),
        ];
        if grains {
            hints.push(Hint::new(chord(GRAIN), msg::units_prices_hint_grain()));
        }
        if manual {
            hints.push(Hint::new(chord(SET_PRICE), msg::units_hint_set_price()));
        }
        hints
    }

    fn unit_row(&self) -> Option<UnitRow> {
        self.ports.borrow().units().unit(self.unit)
    }

    fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    fn today(&self) -> NaiveDate {
        self.now().with_timezone(&Local).date_naive()
    }

    /// The grains the Unit's source offers, Week alone with no fetching.
    fn grains(&self) -> Vec<Grain> {
        let (Some(_), Some(unit)) = (&self.fetching, self.unit_row()) else {
            return vec![Grain::Week];
        };
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        catalogue
            .source(unit.link.source_id)
            .map_or_else(|| vec![Grain::Week], |source| source.grains())
    }

    /// Shows `grain` from its current window, asking for it if it is a finer one.
    fn set_grain(&mut self, grain: Grain) {
        self.grain = grain;
        self.fine_back = 0;
        self.selected = None;
        self.ask();
    }

    /// A day in the Week window's last week: the Clock's day, `months_back` months earlier.
    fn end_day(&self) -> NaiveDate {
        let today = self.today();
        today
            .checked_sub_months(Months::new(self.months_back))
            .unwrap_or(today)
    }

    /// The window's first and last days: at Week, its first and last weeks' Mondays.
    fn window(&self) -> (NaiveDate, NaiveDate) {
        match self.grain {
            Grain::Week => {
                let last = WeeklyPrice::week_of(self.end_day());
                (last - Duration::weeks(WINDOW_WEEKS - 1), last)
            }
            Grain::Day | Grain::Hour => fine_window(self.grain, self.today(), self.fine_back),
        }
    }

    /// Why `[` can't step a finer window earlier: the window a step back would start before the
    /// source's reach. `None` when it can.
    fn beyond_reach(&self) -> Option<String> {
        let unit = self.unit_row()?;
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(unit.link.source_id)?;
        let reach = source.reach(self.grain)?;
        let (first, _) = fine_window(self.grain, self.today(), self.fine_back.saturating_add(1));
        (first < self.today() - Duration::days(i64::from(reach))).then(|| match self.grain {
            Grain::Hour => msg::units_prices_no_older_hour(source.name),
            Grain::Week | Grain::Day => msg::units_prices_no_older_day(source.name),
        })
    }

    /// The series the finer window names.
    fn fine_key(&self) -> FineKey {
        FineKey {
            unit: self.unit,
            grain: self.grain,
            first: self.window().0,
        }
    }

    /// Asks the Unit's source for the finer window showing, unless it is cached fresh or already
    /// asked. A source needing a key this Client doesn't have fails in place without sending.
    fn ask(&self) {
        if self.grain == Grain::Week {
            return;
        }
        let (Some(fetching), Some(unit)) = (&self.fetching, self.unit_row()) else {
            return;
        };
        let now = self.now();
        let key = self.fine_key();
        let mut fetching = fetching.borrow_mut();
        if fetching.fine_fresh(key, &unit.link, now) {
            return;
        }
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let Some(source) = catalogue.source(unit.link.source_id) else {
            return;
        };
        let (Some(endpoint), Some(history)) = (source.endpoint(), source.fine_history(self.grain))
        else {
            return;
        };
        let Some(quote) = ports.units().unit(unit.link.quote) else {
            return;
        };
        let secret = ports.keys().key(source.id);
        if key_missing(endpoint, secret.as_ref()) {
            fetching.fine_refused(key, unit.link.clone(), FetchFailure::KeyMissing);
            return;
        }
        let (first, last) = self.window();
        let request = history_request(
            endpoint,
            history,
            &unit.link.symbol,
            &quote.code,
            (first, last),
            secret,
        );
        fetching.ask_fine(
            FineCall {
                key,
                last,
                link: unit.link.clone(),
                settings: SourceSettings::of(&source),
                history: history.clone(),
                quote: quote.code,
                request: Some(request),
            },
            now,
        );
    }

    /// The window's prices oldest first, each with the close before it, or what shows in their
    /// place.
    fn shown(
        &self,
        unit: &UnitRow,
        quote: Option<&UnitRow>,
        (first, last): (NaiveDate, NaiveDate),
    ) -> (Vec<Shown>, Option<String>) {
        if self.grain == Grain::Week {
            let shown: Vec<Shown> = unit
                .prices
                .iter()
                .filter(|price| (first..=last).contains(&price.week))
                .map(|price| Shown {
                    at: price.week.and_time(NaiveTime::MIN),
                    open: price.open.clone(),
                    high: price.high.clone(),
                    low: price.low.clone(),
                    close: price.close.clone(),
                    before: previous(unit, price).map(|before| before.close.clone()),
                })
                .collect();
            let empty = shown.is_empty().then(|| {
                if unit.prices.is_empty() {
                    msg::units_prices_empty(&unit.code)
                } else {
                    msg::units_prices_window_empty()
                }
            });
            return (shown, empty);
        }
        let name = {
            let ports = self.ports.borrow();
            let catalogue = ports.source_catalogue();
            catalogue
                .source(unit.link.source_id)
                .map_or_else(String::new, |source| source.name.to_string())
        };
        let window_empty = || match self.grain {
            Grain::Hour => msg::units_prices_window_empty_hour(),
            Grain::Week | Grain::Day => msg::units_prices_window_empty_day(),
        };
        let Some(fetching) = &self.fetching else {
            return (Vec::new(), Some(window_empty()));
        };
        let fetching = fetching.borrow();
        match fetching.fine_state(self.fine_key(), &unit.link, self.now()) {
            FineState::Fetching { waiting } if waiting > 0 => (
                Vec::new(),
                Some(msg::units_status_fetch_waiting(&name, &waiting.to_string())),
            ),
            FineState::Fetching { .. } => (
                Vec::new(),
                Some(match self.grain {
                    Grain::Hour => msg::units_prices_fetching_hour(&name),
                    Grain::Week | Grain::Day => msg::units_prices_fetching_day(&name),
                }),
            ),
            FineState::Failed(reason) => {
                let asked = Asked {
                    symbol: &unit.link.symbol,
                    quote: quote.map_or("", |quote| quote.code.as_str()),
                };
                let detail = failure_detail(reason, &name, Some(asked));
                let back = self.keymap.hint_chord(GRAIN);
                (Vec::new(), Some(msg::units_prices_failed(&detail, back)))
            }
            FineState::Unasked => (Vec::new(), Some(window_empty())),
            FineState::Shown(prices) => {
                let shown: Vec<Shown> = prices
                    .iter()
                    .enumerate()
                    .map(|(index, price)| {
                        fine_shown(price, index.checked_sub(1).and_then(|i| prices.get(i)))
                    })
                    .collect();
                let empty = shown.is_empty().then(window_empty);
                (shown, empty)
            }
        }
    }

    /// The table's row starts in drawn order, newest first.
    fn row_starts(&self) -> Vec<NaiveDateTime> {
        let units = self.ports.borrow().units().units();
        let Some(unit) = units.iter().find(|unit| unit.id == self.unit) else {
            return Vec::new();
        };
        let quote = units.iter().find(|quote| quote.id == unit.link.quote);
        let (shown, _) = self.shown(unit, quote, self.window());
        shown.iter().rev().map(|price| price.at).collect()
    }

    fn row_label(&self, at: NaiveDateTime) -> String {
        match self.grain {
            Grain::Hour => msg::units_prices_hour_label(
                &format_date(at.date(), None),
                &format_time_minutes(at.time()),
            ),
            Grain::Week | Grain::Day => format_date(at.date(), None),
        }
    }

    fn axis_label(&self, at: NaiveDateTime) -> String {
        match self.grain {
            Grain::Hour => msg::units_prices_hour_label(
                &format_month_day(at.date()),
                &format_time_minutes(at.time()),
            ),
            Grain::Week | Grain::Day => format_month_day(at.date()),
        }
    }
}

fn fine_shown(price: &FinePrice, before: Option<&FinePrice>) -> Shown {
    Shown {
        at: price.start,
        open: price.open.clone(),
        high: price.high.clone(),
        low: price.low.clone(),
        close: price.close.clone(),
        before: before.map(|before| before.close.clone()),
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
pub(super) fn quoted(quote: Option<&UnitRow>) -> AmountStyle {
    quote.map_or(AmountStyle::own(), |quote| {
        AmountStyle::fixed(quote.precision.places())
    })
}

pub(super) fn manual(unit: &UnitRow) -> bool {
    unit.link.source_id == MANUAL_ENTRY
}

/// The Weekly Price of the week before `price`'s, if `unit` has one.
fn previous<'a>(unit: &'a UnitRow, price: &WeeklyPrice) -> Option<&'a WeeklyPrice> {
    let week = price.week - Duration::weeks(1);
    unit.prices.iter().find(|before| before.week == week)
}

/// `price`'s close against the week before's, as a percentage; `None` without a week before, or
/// one that closed at zero.
pub(super) fn change(unit: &UnitRow, price: &WeeklyPrice) -> Option<Money> {
    change_from(&previous(unit, price)?.close, &price.close)
}

/// `close` against `before`, as a percentage; `None` when `before` is zero.
fn change_from(before: &Money, close: &Money) -> Option<Money> {
    let before = &before.0;
    if before.is_zero() {
        return None;
    }
    Some(Money((&close.0 - before) * BigDecimal::from(100) / before))
}

/// A percentage to two places, signed: `+1.25`, `−0.40`.
pub(super) fn percent(change: &Money) -> String {
    format_amount(change, AmountStyle::fixed(2).with_plus()).text
}

pub(super) fn number(money: &Money) -> f64 {
    money.0.to_f64().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_config::KeyBindingConfig;

    use super::super::test_support::{clock, sample};
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
            let (ports, shared) = sample();
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

        assert_eq!(view.title, "BTC · Bitcoin · prices");
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

        assert_eq!(fact(&view, "Last price"), "148,212.40 AUD · 5 minutes ago");
        let change = fact(&view, "Week change");
        assert!(
            change.ends_with("%)") && change.contains(" AUD ("),
            "{change}"
        );
        let range = fact(&view, "52w range");
        assert!(range.ends_with(" AUD") && range.contains(" – "), "{range}");
        assert_eq!(fact(&view, "Symbol · priced in"), "bitcoin · AUD");
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
        let (_ports, shared) = sample();
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

        assert_eq!(view.empty.as_deref(), Some("VAS has no weekly prices yet."));
        assert!(view.rows.is_empty() && view.chart.is_empty());
        assert_eq!(fact(&view, "Last price"), "—");
        assert_eq!(fact(&view, "Symbol · priced in"), "— · AUD");
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
            .expect("VAS");
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
        view.send(UnitPricesEvent::Select(
            day(2026, 10, 12).and_time(NaiveTime::MIN),
        ));
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
    fn back_is_left_to_the_page() {
        let mut view = View::of(UnitId::SEED_BTC);
        assert_eq!(view.send(UnitPricesEvent::Back), []);
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

    mod finer {
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;

        use lib_toast::Notice;

        use super::*;
        use crate::fake::FakeTransport;
        use crate::fetching::PriceFetching;
        use crate::interactor::FetchDue;
        use crate::presenter::FetchedPricesPresenter;
        use crate::protocols::UnitsPorts as _;
        use crate::{
            COINGECKO, FRANKFURTER, Precision, PriceLink, TransportError, TransportOutcome,
            UnitKind,
        };

        /// The Unit prices view fetching through a fake transport, its responses landing through the
        /// Client's fetched-prices Presenter, on a Clock a test moves.
        struct Finer {
            ports: Rc<RefCell<InMemoryPorts>>,
            fetching: SharedFetching,
            transport: FakeTransport,
            at: Rc<Cell<DateTime<Utc>>>,
            presenter: UnitPricesPresenter,
            landing: FetchedPricesPresenter,
            router: RecordingRouter,
            notices: Vec<Notice>,
        }

        impl Finer {
            fn of(unit: UnitId) -> Self {
                Self::with(unit, |_| {})
            }

            fn with(unit: UnitId, arrange: impl FnOnce(&mut InMemoryPorts)) -> Self {
                let (ports, shared) = sample();
                arrange(&mut ports.borrow_mut());
                let transport = FakeTransport::default();
                let fetching = PriceFetching::shared(Rc::new(transport.clone()));
                let at = Rc::new(Cell::new(super::super::super::test_support::now()));
                let clock: Clock = {
                    let at = at.clone();
                    Rc::new(move || at.get())
                };
                Self {
                    presenter: UnitPricesPresenter::new(
                        shared.clone(),
                        clock.clone(),
                        unit_prices_keymap(&KeyBindingConfig::default()),
                        unit,
                    )
                    .with_fetching(Some(fetching.clone())),
                    landing: FetchedPricesPresenter::new(shared, fetching.clone(), clock),
                    ports,
                    fetching,
                    transport,
                    at,
                    router: RecordingRouter::default(),
                    notices: Vec::new(),
                }
            }

            fn press(&mut self, event: UnitPricesEvent) {
                self.presenter.handle(event, &mut self.router);
            }

            fn view(&self) -> UnitPricesViewModel {
                self.presenter.view_model()
            }

            fn answer(&mut self, status: u16, body: &str) {
                let response = self.transport.answer_last(status, body);
                self.landing.handle(response, &mut self.notices);
            }

            fn answer_with(&mut self, outcome: TransportOutcome) {
                let (id, _) = self
                    .transport
                    .requests()
                    .last()
                    .cloned()
                    .expect("a request");
                let response = self.transport.answer(id, outcome);
                self.landing.handle(response, &mut self.notices);
            }

            fn sent(&self) -> usize {
                self.transport.urls().len()
            }

            fn last_url(&self) -> String {
                self.transport.urls().last().cloned().unwrap_or_default()
            }

            fn later(&self, by: Duration) {
                self.at.set(self.at.get() + by);
            }

            fn today(&self) -> NaiveDate {
                self.at.get().with_timezone(&Local).date_naive()
            }
        }

        /// A Unit on `source` priced in AUD.
        fn priced_on(
            ports: &mut InMemoryPorts,
            source: lib_core::PriceSourceId,
            code: &str,
        ) -> UnitId {
            let unit = UnitRow::new(
                code.into(),
                code.into(),
                UnitKind::Currency,
                Precision::of_kind(UnitKind::Currency),
                PriceLink {
                    source_id: source,
                    symbol: code.into(),
                    quote: UnitId::SEED_AUD,
                },
            );
            let id = unit.id;
            ports.units.units.push(unit);
            id
        }

        /// CoinGecko's `[ms, price]` pairs at 00:00 UTC on each of `days`.
        fn coingecko_days(days: &[(NaiveDate, &str)]) -> String {
            let pairs: Vec<String> = days
                .iter()
                .map(|(on, price)| {
                    let ms = on.and_time(NaiveTime::MIN).and_utc().timestamp_millis();
                    format!("[{ms},{price}]")
                })
                .collect();
            format!(r#"{{"prices":[{}]}}"#, pairs.join(","))
        }

        fn grains(view: &UnitPricesViewModel) -> Vec<(Grain, bool)> {
            view.grains
                .iter()
                .map(|choice| (choice.grain, choice.active))
                .collect()
        }

        #[test]
        fn z_cycles_week_day_hour_on_coingecko_and_each_finer_grain_asks_in_place() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            assert_eq!(btc.view().grain, Grain::Week, "the view opens at Week");
            assert_eq!(
                grains(&btc.view()),
                [
                    (Grain::Week, true),
                    (Grain::Day, false),
                    (Grain::Hour, false)
                ]
            );
            assert!(
                btc.view()
                    .hints
                    .iter()
                    .any(|hint| hint.chord == "z" && hint.action == "grain")
            );

            btc.press(UnitPricesEvent::CycleGrain);
            let day = btc.view();
            assert_eq!(day.grain, Grain::Day);
            assert_eq!(day.chart_heading, "Daily close");
            assert_eq!(day.columns.week, "Day");
            assert_eq!(
                day.empty.as_deref(),
                Some("Fetching daily prices from CoinGecko…")
            );
            assert!(
                btc.last_url()
                    .contains("/coins/bitcoin/market_chart/range?vs_currency=aud")
            );
            assert!(
                btc.last_url().ends_with("&interval=daily"),
                "{}",
                btc.last_url()
            );
            assert_eq!(
                fact(&day, "Last price"),
                "148,212.40 AUD · 5 minutes ago",
                "figures stay weekly"
            );

            btc.press(UnitPricesEvent::CycleGrain);
            assert_eq!(btc.view().grain, Grain::Hour);
            assert_eq!(btc.view().chart_heading, "Hourly close");
            assert_eq!(
                btc.view().empty.as_deref(),
                Some("Fetching hourly prices from CoinGecko…")
            );
            assert!(!btc.last_url().contains("interval"), "{}", btc.last_url());

            btc.press(UnitPricesEvent::CycleGrain);
            assert_eq!(btc.view().grain, Grain::Week);
            assert_eq!(btc.view().rows.len(), 52);
        }

        #[test]
        fn frankfurter_offers_day_alone_and_a_manual_unit_no_finer_grain() {
            let mut eur_id = None;
            let mut eur = Finer::with(UnitId::SEED_BTC, |ports| {
                eur_id = Some(priced_on(ports, FRANKFURTER, "EUR"));
            });
            eur.presenter = UnitPricesPresenter::new(
                eur.presenter.ports.clone(),
                eur.presenter.clock.clone(),
                unit_prices_keymap(&KeyBindingConfig::default()),
                eur_id.expect("added"),
            )
            .with_fetching(Some(eur.fetching.clone()));
            assert_eq!(
                grains(&eur.view()),
                [(Grain::Week, true), (Grain::Day, false)]
            );
            eur.press(UnitPricesEvent::CycleGrain);
            assert_eq!(eur.view().grain, Grain::Day);
            assert!(
                eur.last_url()
                    .starts_with("https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD")
            );
            eur.press(UnitPricesEvent::CycleGrain);
            assert_eq!(eur.view().grain, Grain::Week, "no Hour to step to");

            let mut vas = Finer::of(UnitId::SEED_VAS);
            vas.press(UnitPricesEvent::CycleGrain);
            assert_eq!(vas.view().grain, Grain::Week);
            assert!(vas.view().grains.is_empty());
            assert!(!vas.view().hints.iter().any(|hint| hint.chord == "z"));
            vas.press(UnitPricesEvent::SetGrain(Grain::Day));
            assert_eq!(vas.view().grain, Grain::Week);
            assert_eq!(vas.sent(), 0);
        }

        #[test]
        fn landed_daily_prices_show_newest_first_with_change_against_the_day_before_and_write_nothing()
         {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            let weeks_before = btc
                .ports
                .borrow()
                .units
                .unit(UnitId::SEED_BTC)
                .expect("btc")
                .prices;
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            let today = btc.today();
            let (two, one) = (today - Duration::days(2), today - Duration::days(1));
            btc.answer(200, &coingecko_days(&[(two, "100"), (one, "110")]));

            let view = btc.view();
            assert_eq!(view.empty, None);
            assert_eq!(view.rows.len(), 2, "a day with no price is absent");
            assert_eq!(view.rows[0].week, one);
            assert_eq!(view.rows[0].close, "110.00");
            assert_eq!(view.rows[0].change, "+10.00%");
            assert_eq!(view.rows[1].change, "—");
            assert!(view.rows[0].selected);
            assert_eq!(view.chart.len(), 2);
            assert!(!view.chart[0].candle, "a close-only day is a tick");
            assert_eq!(btc.notices, [], "no Toast");
            assert_eq!(
                btc.ports
                    .borrow()
                    .units
                    .unit(UnitId::SEED_BTC)
                    .expect("btc")
                    .prices,
                weeks_before,
                "nothing finer than a week is written"
            );

            btc.press(UnitPricesEvent::Move(Step::Next));
            assert!(btc.view().rows[1].selected);
        }

        #[test]
        fn hourly_prices_bucket_by_the_local_hour() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Hour));
            let noon = btc.today().and_hms_opt(12, 0, 0).expect("noon");
            let instant = |minutes: i64| {
                (noon + Duration::minutes(minutes))
                    .and_local_timezone(Local)
                    .earliest()
                    .expect("local noon exists")
                    .timestamp_millis()
            };
            btc.answer(
                200,
                &format!(
                    r#"{{"prices":[[{},10],[{},12],[{},11]]}}"#,
                    instant(5),
                    instant(40),
                    instant(65)
                ),
            );

            let view = btc.view();
            let starts: Vec<_> = view.rows.iter().map(|row| row.at.time()).collect();
            assert_eq!(
                starts,
                [
                    NaiveTime::from_hms_opt(13, 0, 0).expect("1 pm"),
                    NaiveTime::from_hms_opt(12, 0, 0).expect("noon")
                ]
            );
            assert_eq!(view.rows[1].open, "10.00");
            assert_eq!(view.rows[1].close, "12.00");
            assert!(
                view.rows[0].label.contains("1:00") || view.rows[0].label.contains("13:00"),
                "{}",
                view.rows[0].label
            );
        }

        #[test]
        fn a_day_window_steps_a_week_and_stops_at_coingeckos_365_days() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            let today = btc.today();
            let current = btc.view().window;
            assert_eq!(
                current,
                format!(
                    "{} – {}",
                    format_date(today - Duration::days(90), None),
                    format_date(today, None)
                )
            );
            assert!(!btc.view().can_go_later);

            btc.press(UnitPricesEvent::Earlier);
            let earlier = today - Duration::days(7);
            assert!(btc.view().window.ends_with(&format_date(earlier, None)));
            assert!(btc.view().can_go_later);
            btc.press(UnitPricesEvent::Later);
            assert_eq!(btc.view().window, current);

            // A window reaching past 365 days back is never asked for.
            for _ in 0..39 {
                btc.press(UnitPricesEvent::Earlier);
            }
            assert!(btc.router.calls.is_empty(), "{:?}", btc.router.calls);
            let furthest = btc.view().window;
            btc.press(UnitPricesEvent::Earlier);
            assert_eq!(btc.view().window, furthest, "[ stops at the reach");
            assert_eq!(
                btc.router.take(),
                [RouterCall::Notice(Notice::Status(
                    "No older daily prices from CoinGecko.".into()
                ))]
            );
        }

        #[test]
        fn an_hour_window_is_seven_days_stepped_a_day_and_stops_at_364_days() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Hour));
            let today = btc.today();
            assert!(
                btc.view()
                    .window
                    .starts_with(&format_date(today - Duration::days(6), None))
            );
            btc.press(UnitPricesEvent::Earlier);
            assert!(
                btc.view()
                    .window
                    .ends_with(&format_date(today - Duration::days(1), None))
            );
            for _ in 0..364 {
                btc.press(UnitPricesEvent::Earlier);
            }
            assert_eq!(
                btc.router.calls.last(),
                Some(&RouterCall::Notice(Notice::Status(
                    "No older hourly prices from CoinGecko.".into()
                )))
            );
            assert!(
                btc.view()
                    .window
                    .starts_with(&format_date(today - Duration::days(364), None))
            );
        }

        #[test]
        fn a_failure_shows_in_place_with_no_toast_and_no_held_failure_and_z_back_asks_again() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            btc.answer(404, "{}");

            assert_eq!(
                btc.view().empty.as_deref(),
                Some(
                    "Couldn't fetch: CoinGecko has no bitcoin in AUD. Press z to go back to weekly."
                )
            );
            assert_eq!(btc.notices, []);
            let ports = btc.ports.borrow();
            let catalogue = ports.source_catalogue();
            let coingecko = catalogue.source(COINGECKO).expect("built in");
            let unit = ports.units.unit(UnitId::SEED_BTC).expect("btc");
            let fetching = btc.fetching.borrow();
            assert_eq!(fetching.unit_fetch(&unit, &coingecko), None);
            assert_eq!(fetching.source_fetch(&coingecko), None);
            assert!(!fetching.source_backing_off(&coingecko, btc.at.get()));
            drop((fetching, catalogue, ports));

            let sent = btc.sent();
            btc.press(UnitPricesEvent::CycleGrain);
            btc.press(UnitPricesEvent::CycleGrain);
            btc.press(UnitPricesEvent::CycleGrain);
            assert_eq!(btc.view().grain, Grain::Day);
            assert_eq!(btc.sent(), sent + 2, "Hour, then Day again");
        }

        #[test]
        fn offline_and_rate_limited_back_the_source_off_quietly() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            btc.answer_with(TransportOutcome::Failed(TransportError::Offline));

            assert_eq!(
                btc.view().empty.as_deref(),
                Some("Couldn't fetch: couldn't reach CoinGecko. Press z to go back to weekly.")
            );
            let ports = btc.ports.borrow();
            let catalogue = ports.source_catalogue();
            let coingecko = catalogue.source(COINGECKO).expect("built in");
            assert!(
                btc.fetching
                    .borrow()
                    .source_backing_off(&coingecko, btc.at.get())
            );
            assert_eq!(btc.fetching.borrow().source_fetch(&coingecko), None);
            assert_eq!(btc.notices, []);
        }

        #[test]
        fn a_retry_after_is_waited_out_in_place_and_back_off_is_ignored() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            let response = btc.transport.answer_last_with(429, "{}", Some("40"));
            btc.landing.handle(response, &mut btc.notices);
            assert!(
                btc.view()
                    .empty
                    .as_deref()
                    .is_some_and(|text| text.contains("asked to be called less often"))
            );

            // Backing off doesn't refuse it, but the Retry-After holds it in the queue.
            btc.press(UnitPricesEvent::SetGrain(Grain::Week));
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            let sent = btc.sent();
            assert_eq!(
                btc.view().empty.as_deref(),
                Some("Waiting for CoinGecko, 40 s…")
            );
            btc.later(Duration::seconds(41));
            btc.landing.pump(&mut btc.notices);
            assert_eq!(btc.sent(), sent + 1);
            assert_eq!(
                btc.view().empty.as_deref(),
                Some("Fetching daily prices from CoinGecko…")
            );
        }

        #[test]
        fn a_request_waits_behind_its_sources_round_and_a_scheduled_check_skips_it_meanwhile() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            // Past CoinGecko's 15 minutes, so btc is due.
            btc.later(Duration::hours(1));
            let today = btc.today();
            let now = btc.at.get();
            let started =
                FetchDue { today, now }.run(&*btc.ports.borrow(), &mut btc.fetching.borrow_mut());
            assert_eq!(started, [COINGECKO]);
            let round = btc.sent();

            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            assert_eq!(btc.sent(), round, "queued behind the round");
            assert_eq!(
                btc.view().empty.as_deref(),
                Some("Fetching daily prices from CoinGecko…")
            );
            btc.answer(200, r#"{"bitcoin":{"aud":150000}}"#);
            assert_eq!(btc.sent(), round + 1, "sent once the round landed");
            assert!(btc.last_url().contains("market_chart"));

            btc.later(Duration::hours(2));
            let started = FetchDue {
                today,
                now: btc.at.get(),
            }
            .run(&*btc.ports.borrow(), &mut btc.fetching.borrow_mut());
            assert!(started.is_empty(), "the daily request counts as in flight");
        }

        #[test]
        fn a_past_window_is_cached_for_the_session_and_a_current_one_for_15_minutes() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            btc.answer(200, &coingecko_days(&[]));
            btc.press(UnitPricesEvent::Earlier);
            btc.answer(200, &coingecko_days(&[]));
            let sent = btc.sent();

            btc.later(Duration::minutes(10));
            btc.press(UnitPricesEvent::Later);
            btc.press(UnitPricesEvent::Earlier);
            assert_eq!(btc.sent(), sent, "both served from the cache");
            assert_eq!(
                btc.view().empty.as_deref(),
                Some("No daily prices in this window.")
            );

            btc.later(Duration::minutes(10));
            btc.press(UnitPricesEvent::Later);
            assert_eq!(
                btc.sent(),
                sent + 1,
                "the current window refetches after 15 minutes"
            );
            btc.press(UnitPricesEvent::Earlier);
            assert_eq!(
                btc.sent(),
                sent + 1,
                "the past window stays for the session"
            );
        }

        #[test]
        fn yahoo_asks_hourly_bars_back_to_700_days() {
            let mut bhp = None;
            let mut view = Finer::with(UnitId::SEED_BTC, |ports| {
                bhp = Some(priced_on(ports, crate::YAHOO_FINANCE, "BHP.AX"));
            });
            view.presenter = UnitPricesPresenter::new(
                view.presenter.ports.clone(),
                view.presenter.clock.clone(),
                unit_prices_keymap(&KeyBindingConfig::default()),
                bhp.expect("added"),
            )
            .with_fetching(Some(view.fetching.clone()));
            view.press(UnitPricesEvent::SetGrain(Grain::Hour));
            assert!(
                view.last_url().starts_with(
                    "https://query1.finance.yahoo.com/v8/finance/chart/BHP.AX?period1="
                ) && view.last_url().ends_with("&interval=1h"),
                "{}",
                view.last_url()
            );
            for _ in 0..694 {
                view.press(UnitPricesEvent::Earlier);
            }
            assert!(view.router.calls.is_empty(), "{:?}", view.router.calls);
            view.press(UnitPricesEvent::Earlier);
            assert_eq!(
                view.router.take(),
                [RouterCall::Notice(Notice::Status(
                    "No older hourly prices from Yahoo Finance.".into()
                ))]
            );
        }

        #[test]
        fn a_change_to_the_units_link_drops_its_series() {
            let mut btc = Finer::of(UnitId::SEED_BTC);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            btc.answer(200, &coingecko_days(&[(btc.today(), "1")]));
            assert_eq!(btc.view().rows.len(), 1);

            let mut unit = btc
                .ports
                .borrow()
                .units
                .unit(UnitId::SEED_BTC)
                .expect("btc");
            unit.link.symbol = "wrapped-bitcoin".into();
            btc.ports.borrow_mut().units.upsert_unit(unit);
            assert!(
                btc.view().rows.is_empty(),
                "the old link's series no longer shows"
            );
            let sent = btc.sent();
            btc.press(UnitPricesEvent::CycleGrain);
            btc.press(UnitPricesEvent::SetGrain(Grain::Day));
            assert_eq!(btc.sent(), sent + 2);
            assert!(btc.last_url().contains("/coins/wrapped-bitcoin/"));
        }
    }
}
