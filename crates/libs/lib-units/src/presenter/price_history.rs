//! PRICE HISTORY (#1377, #1386): the history Unit's newest Weekly Prices, which the Units page
//! shows under PRICE SOURCES at Medium and Large, the docked pane's group at X-Large and the TUI's
//! weekly prices region. Read-only: it follows the UNITS selection and keeps the last Unit while a
//! Price Source row is selected, and the full prices view stays one `Enter` away.

use bigdecimal::{Signed as _, Zero as _};
use chrono::NaiveDate;
use lib_core::{Money, UnitId};
use lib_locale::format::{format_amount, format_date};
use lib_viper::view::count;

use super::unit_prices::{PriceColumns, change, manual, number, percent, quoted};
use crate::{PriceSource, UnitRow, WeeklyPrice, msg};

/// How many of the newest weeks the history lists; the prices view holds the rest.
pub const HISTORY_WEEKS: usize = 12;

/// The history as drawn. Not `Eq`: the sparkline is drawn from plain numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceHistory {
    pub unit: UnitId,
    /// `Price history`, which each Client sets in caps.
    pub heading: String,
    /// `BTC · CoinGecko`; the code alone for the base Unit, which has no source.
    pub subtitle: String,
    /// `this week · provisional` while the newest week is the current one.
    pub flag: Option<String>,
    /// `⚠ Out of date · fetch failed`, whichever apply, drawn in the warning colour by the
    /// heading; `None` for neither.
    pub warning: Option<String>,
    /// `enter all prices`, at the heading's right.
    pub open_hint: String,
    pub columns: PriceColumns,
    /// The newest [`HISTORY_WEEKS`] Weekly Prices, newest first.
    pub rows: Vec<HistoryRow>,
    /// `Newest week first · 12 of 52 weeks`, under the table; `None` with no prices.
    pub count: Option<String>,
    /// The pane's `enter opens all 52 weeks`; `None` with no prices.
    pub pane_hint: Option<String>,
    /// Every week's close, oldest first, for the pane's sparkline.
    pub sparkline: Vec<f64>,
    /// What shows in place of the table: the base Unit's, or a Unit's with no prices yet.
    pub empty: Option<String>,
}

/// Which way a week's close moved from the week before's, which the Clients colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Rising,
    Falling,
    /// Unchanged, or no week before to compare.
    Flat,
}

/// One week, each price at the quote Unit's Precision, — where it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub week: NaiveDate,
    /// Its Monday, in the Locale's medium form.
    pub label: String,
    /// — for a Unit on Manual entry, which is priced by its close alone.
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    /// `+1.25%` against the week before, or —.
    pub change: String,
    pub trend: Trend,
    /// `provisional` on the current week's row.
    pub provisional: Option<String>,
}

/// What the history reads, gathered once per view model.
pub(super) struct HistoryInput<'a> {
    pub units: &'a [UnitRow],
    pub sources: &'a [PriceSource<'a>],
    pub base: UnitId,
    /// The chord that opens the prices view, `enter`.
    pub open_chord: &'a str,
    pub today: NaiveDate,
    /// Whether the Unit's last price is Out of date.
    pub out_of_date: bool,
    /// Whether this Client's last fetch of the Unit failed.
    pub fetch_failed: bool,
}

pub(super) fn price_history(input: &HistoryInput<'_>, unit: &UnitRow) -> PriceHistory {
    let quote = input.units.iter().find(|quote| quote.id == unit.link.quote);
    let is_base = unit.id == input.base;
    let by_hand = manual(unit);
    let current = WeeklyPrice::week_of(input.today);
    let style = quoted(quote);
    let places = |money: &Money| format_amount(money, style).text;
    let optional = |money: Option<&Money>| match money {
        Some(money) if !by_hand => places(money),
        _ => msg::units_none(),
    };
    let rows: Vec<HistoryRow> = unit
        .prices
        .iter()
        .rev()
        .take(HISTORY_WEEKS)
        .map(|price| {
            let moved = change(unit, price);
            HistoryRow {
                week: price.week,
                label: format_date(price.week, None),
                open: optional(price.open.as_ref()),
                high: optional(price.high.as_ref()),
                low: optional(price.low.as_ref()),
                close: places(&price.close),
                trend: match &moved {
                    Some(moved) if moved.0.is_negative() => Trend::Falling,
                    Some(moved) if !moved.0.is_zero() => Trend::Rising,
                    _ => Trend::Flat,
                },
                change: moved.map_or_else(msg::units_none, |moved| {
                    msg::units_prices_change(&percent(&moved))
                }),
                provisional: (price.week == current).then(msg::units_history_provisional),
            }
        })
        .collect();
    let weeks = count(unit.prices.len());
    let priced = !rows.is_empty();
    let source = input
        .sources
        .iter()
        .find(|source| source.id == unit.link.source_id)
        .map(|source| source.name.to_string());
    let subtitle = match source {
        Some(source) if !is_base => msg::units_history_subtitle(&unit.code, &source),
        _ => unit.code.clone(),
    };
    PriceHistory {
        unit: unit.id,
        heading: msg::units_history_heading(),
        subtitle,
        flag: rows
            .first()
            .is_some_and(|row| row.week == current)
            .then(msg::units_history_flag_provisional),
        warning: Some(super::joined(
            [
                input.out_of_date.then(msg::units_history_out_of_date),
                input.fetch_failed.then(msg::units_flag_fetch_failed),
            ]
            .into_iter()
            .flatten(),
        ))
        .filter(|text| !text.is_empty())
        .map(|text| msg::units_history_warning(&text)),
        open_hint: msg::units_history_hint_open(input.open_chord),
        columns: PriceColumns {
            week: msg::units_prices_column_week(),
            open: msg::units_prices_column_open(),
            high: msg::units_prices_column_high(),
            low: msg::units_prices_column_low(),
            close: msg::units_prices_column_close(),
            change: msg::units_prices_column_change(),
        },
        count: priced.then(|| msg::units_history_count(&rows.len().to_string(), weeks)),
        pane_hint: priced.then(|| msg::units_history_pane_hint(input.open_chord, weeks)),
        sparkline: unit
            .prices
            .iter()
            .map(|price| number(&price.close))
            .collect(),
        empty: (!priced).then(|| {
            if is_base {
                msg::units_history_base(&unit.code)
            } else {
                msg::units_history_none(&unit.code)
            }
        }),
        rows,
    }
}
