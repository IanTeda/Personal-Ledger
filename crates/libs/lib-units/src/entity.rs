//! The Units Entities: a Unit, its link to a Price Source and its Weekly Prices, with the rules
//! about themselves only. The Price Sources are in `price_source`.

mod precision;
mod price_source;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
pub use precision::Precision;
pub use price_source::{
    BUILT_IN_SOURCES, BuiltInOverride, BuiltInSource, COINGECKO, CustomSource, Endpoint,
    FRANKFURTER, KeyPlacement, MANUAL_ENTRY, PriceSource, PriceSourceAuth, PriceSourceRecord,
    SourceOrigin, Updates, YAHOO_FINANCE, add_custom_source, built_in, edit_custom_source,
    last_fetched, move_units_to_manual, override_built_in, price_source, price_source_exists,
    price_sources, remove_custom_source, sources_for_kind,
};

/// One Unit. Owned `String` fields, since the Add unit dialog stores what a person typed. `kind` is
/// the [`UnitKind`] enum, never a display label: the label is read through `lib_locale::Label` at
/// render time, so a Locale switch can't change what an Edit then saves back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    /// Addresses the row across edits; `code` can change, so nothing else holds it as a key.
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    /// The places its amounts, and the prices quoted in it, show at.
    pub precision: Precision,
    /// Where its price comes from.
    pub link: PriceLink,
    /// Its price history, oldest week first, at most one per week and kept forever (ADR-0052).
    /// On the row rather than a store of its own, since the Units page reads every Unit's last
    /// price and the Unit prices view one Unit's history, both from the Units it already holds.
    pub prices: Vec<WeeklyPrice>,
    /// A deactivated Unit stays on the Ledger and on whatever uses it, but is no longer offered to
    /// pick (the lifecycle Category, Payee and Account follow).
    pub is_active: bool,
}

impl UnitRow {
    /// A new Unit from the Add unit dialog: active, with no price yet. Which Unit is the base is
    /// the Ledger's, so a new Unit is never it.
    pub fn new(
        code: String,
        name: String,
        kind: UnitKind,
        precision: Precision,
        link: PriceLink,
    ) -> Self {
        Self {
            id: UnitId::new(),
            code,
            name,
            kind,
            precision,
            link,
            prices: Vec::new(),
            is_active: true,
        }
    }

    /// Rewrites the Unit from the Edit unit dialog, keeping its id and whether it is active. The
    /// Weekly Prices are kept unless the quote Unit changes, since a price in the old quote would
    /// read as one in the new. A Precision change keeps them: it only reformats.
    pub fn edit(
        &mut self,
        code: String,
        name: String,
        kind: UnitKind,
        precision: Precision,
        link: PriceLink,
    ) {
        if self.link.quote != link.quote {
            self.prices.clear();
        }
        self.code = code;
        self.name = name;
        self.kind = kind;
        self.precision = precision;
        self.link = link;
    }

    /// Moves the Unit to Manual entry, keeping its symbol and Weekly Prices, for when its Price
    /// Source is deleted.
    pub fn move_to_manual(&mut self) {
        self.link.source_id = MANUAL_ENTRY;
    }

    /// The newest Weekly Price, whose close is the Unit's last price, or `None` before its first.
    pub fn last_price(&self) -> Option<&WeeklyPrice> {
        self.prices.iter().max_by_key(|price| price.week)
    }

    /// Writes `close`, a price as of `at`, into the week holding `today`. A week with no entry yet
    /// starts with the close only; the current week's entry is provisional, so a new price
    /// overwrites its close and widens its high and low. `today` is the Clock's day rather than
    /// `at`'s, so the week is the one the person sees.
    pub fn record_price(&mut self, close: Money, today: NaiveDate, at: DateTime<Utc>) {
        let week = WeeklyPrice::week_of(today);
        if let Some(current) = self.prices.iter_mut().find(|price| price.week == week) {
            current.widen(close, at);
            return;
        }
        self.prices.push(WeeklyPrice {
            week,
            close,
            open: None,
            high: None,
            low: None,
            written_at: at,
        });
        self.prices.sort_by_key(|price| price.week);
    }
}

/// A Unit's link to its Price Source. The symbol and quote live here, not on the source, since one
/// source prices many Units (ADR-0043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceLink {
    pub source_id: PriceSourceId,
    /// What the source calls the Unit, filling a request's `{symbol}` (e.g. `bitcoin`).
    pub symbol: String,
    /// The Unit the price is quoted in, filling `{quote}`; the base Unit unless a person picks
    /// another.
    pub quote: UnitId,
}

impl PriceLink {
    /// A link to Manual entry, quoted in `quote`.
    pub fn manual(quote: UnitId) -> Self {
        Self {
            source_id: MANUAL_ENTRY,
            symbol: String::new(),
            quote,
        }
    }
}

/// A Unit's price for one ISO week, in its link's quote Unit (`GLOSSARY.md`'s Weekly Price). A
/// week with all four prices draws as a candle, any other as a close only, which is what Manual
/// entry usually gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeeklyPrice {
    /// The Monday of its ISO week, Monday to Sunday in every Locale, so one Ledger buckets the same
    /// prices the same way on every machine.
    pub week: NaiveDate,
    pub close: Money,
    pub open: Option<Money>,
    pub high: Option<Money>,
    pub low: Option<Money>,
    /// When it was last written: the last price's age.
    pub written_at: DateTime<Utc>,
}

impl WeeklyPrice {
    /// The Monday starting `day`'s ISO week.
    pub fn week_of(day: NaiveDate) -> NaiveDate {
        day - Duration::days(i64::from(day.weekday().num_days_from_monday()))
    }

    /// A balance of the Unit at this close, rounded to `precision`, the quote Unit's. It is in the
    /// link's quote Unit, not the base Unit: a Unit quoted in usd values in usd even on an aud
    /// Ledger.
    pub fn value(&self, balance: &Money, precision: Precision) -> Money {
        Money((&balance.0 * &self.close.0).round(i64::from(precision.get())))
    }

    /// Overwrites the close with `close` and widens the high and low to take it in. An absent high
    /// or low starts from the close it replaces, a price the week has already had.
    fn widen(&mut self, close: Money, at: DateTime<Utc>) {
        let high = self.high.take().unwrap_or_else(|| self.close.clone());
        let low = self.low.take().unwrap_or_else(|| self.close.clone());
        self.high = Some(high.max(close.clone()));
        self.low = Some(low.min(close.clone()));
        self.close = close;
        self.written_at = at;
    }
}

/// How many weeks of Weekly Prices [`seed_weekly_prices`] gives: a year, for the 52-week range.
const SEED_WEEKS: i64 = 52;

/// A year of deterministic Weekly Prices for a seeded Unit, ending with a close of `close_cents`
/// written at `written_at` in that day's week. The past weeks climb from about 70% of it with a
/// wobble `salt` varies, each with an open, high and low, so the Unit prices view has candles to
/// draw. The same arguments always give the same prices.
pub fn seed_weekly_prices(
    close_cents: i64,
    written_at: DateTime<Utc>,
    salt: u32,
) -> Vec<WeeklyPrice> {
    let money = |cents: i64| Money(BigDecimal::new(cents.into(), 2));
    let current = WeeklyPrice::week_of(written_at.date_naive());
    let mut open = close_cents * 68 / 100;
    (0..SEED_WEEKS)
        .map(|index| {
            let week = current - Duration::weeks(SEED_WEEKS - 1 - index);
            let close = if index == SEED_WEEKS - 1 {
                close_cents
            } else {
                // A ±5% wobble on a climb from 70% to 100% of the close.
                let wobble = (index * 7 + i64::from(salt) * 13) % 11 - 5;
                close_cents * (70 + 30 * index / SEED_WEEKS + wobble) / 100
            };
            let price = WeeklyPrice {
                week,
                close: money(close),
                open: Some(money(open)),
                high: Some(money(open.max(close) * 103 / 100)),
                low: Some(money(open.min(close) * 97 / 100)),
                written_at: if index == SEED_WEEKS - 1 {
                    written_at
                } else {
                    (week + Duration::days(6))
                        .and_time(chrono::NaiveTime::MIN)
                        .and_utc()
                },
            };
            open = close;
            price
        })
        .collect()
}

/// The stub seed's three Units: aud and vas on Manual entry, btc on CoinGecko with a year of
/// Weekly Prices, the newest written five minutes before `now`.
pub fn default_units(now: DateTime<Utc>) -> Vec<UnitRow> {
    vec![
        UnitRow {
            id: UnitId::SEED_AUD,
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: UnitKind::Currency,
            precision: Precision::of_kind(UnitKind::Currency),
            link: PriceLink::manual(UnitId::SEED_AUD),
            prices: Vec::new(),
            is_active: true,
        },
        UnitRow {
            id: UnitId::SEED_BTC,
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: UnitKind::Crypto,
            precision: Precision::of_kind(UnitKind::Crypto),
            link: PriceLink {
                source_id: COINGECKO,
                symbol: "bitcoin".to_string(),
                quote: UnitId::SEED_AUD,
            },
            prices: seed_weekly_prices(14_821_240, now - Duration::minutes(5), 2),
            is_active: true,
        },
        UnitRow {
            id: UnitId::SEED_VAS,
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: UnitKind::Equity,
            precision: Precision::of_kind(UnitKind::Equity),
            link: PriceLink::manual(UnitId::SEED_AUD),
            prices: Vec::new(),
            is_active: true,
        },
    ]
}

/// The Units sorted by code, ignoring case.
pub fn units_by_code(units: &[UnitRow]) -> Vec<&UnitRow> {
    let mut sorted: Vec<_> = units.iter().collect();
    sorted.sort_by_cached_key(|unit| (unit.code.to_lowercase(), unit.id));
    sorted
}

/// Appends a Unit from the Add unit dialog (see [`UnitRow::new`]) and returns the row it appended.
pub fn add_unit(
    units: &mut Vec<UnitRow>,
    code: String,
    name: String,
    kind: UnitKind,
    precision: Precision,
    link: PriceLink,
) -> UnitRow {
    let row = UnitRow::new(code, name, kind, precision, link);
    units.push(row.clone());
    row
}

/// Rewrites the row `id` from the Edit unit dialog (see [`UnitRow::edit`]). Returns the row it
/// wrote, or `None` when no row has `id`.
pub fn edit_unit(
    units: &mut [UnitRow],
    id: UnitId,
    code: String,
    name: String,
    kind: UnitKind,
    precision: Precision,
    link: PriceLink,
) -> Option<&UnitRow> {
    let existing = units.iter_mut().find(|row| row.id == id)?;
    existing.edit(code, name, kind, precision, link);
    Some(existing)
}

/// Marks the row `id` active or deactivated and returns it, or `None` when no row has `id`.
pub fn set_unit_active(units: &mut [UnitRow], id: UnitId, is_active: bool) -> Option<&UnitRow> {
    let existing = units.iter_mut().find(|row| row.id == id)?;
    existing.is_active = is_active;
    Some(existing)
}

/// Removes the row `id` and returns it, or `None` when no row has `id`.
pub fn remove_unit(units: &mut Vec<UnitRow>, id: UnitId) -> Option<UnitRow> {
    let index = units.iter().position(|row| row.id == id)?;
    Some(units.remove(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<UnitRow> {
        default_units(Utc::now())
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        day(y, m, d)
            .and_hms_opt(9, 0, 0)
            .expect("a real time")
            .and_utc()
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a decimal")
    }

    #[test]
    fn a_price_in_a_new_week_starts_that_weeks_entry_with_a_close_only() {
        let mut unit = UnitRow::new(
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );

        // Thursday 29 October 2026 is in the ISO week starting Monday 26 October.
        unit.record_price(money("4000.00"), day(2026, 10, 29), at(2026, 10, 29));

        assert_eq!(
            unit.prices,
            vec![WeeklyPrice {
                week: day(2026, 10, 26),
                close: money("4000.00"),
                open: None,
                high: None,
                low: None,
                written_at: at(2026, 10, 29),
            }]
        );
    }

    #[test]
    fn a_price_in_the_current_week_overwrites_its_close_and_widens_its_high_and_low() {
        let mut unit = UnitRow::new(
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        let last_week = WeeklyPrice {
            week: day(2026, 10, 19),
            close: money("3900.00"),
            open: Some(money("3800.00")),
            high: Some(money("3950.00")),
            low: Some(money("3700.00")),
            written_at: at(2026, 10, 25),
        };
        unit.prices.push(last_week.clone());
        unit.record_price(money("4000.00"), day(2026, 10, 26), at(2026, 10, 26));

        // Absent, the high and low are set from the overwritten close and the new price.
        unit.record_price(money("4100.00"), day(2026, 10, 28), at(2026, 10, 28));
        // A price inside the range moves only the close.
        unit.record_price(money("4050.00"), day(2026, 11, 1), at(2026, 11, 1));
        // A price below the range widens the low.
        unit.record_price(money("3950.00"), day(2026, 11, 1), at(2026, 11, 1));

        assert_eq!(unit.prices[0], last_week, "a past week is final");
        assert_eq!(
            unit.prices[1],
            WeeklyPrice {
                week: day(2026, 10, 26),
                close: money("3950.00"),
                open: None,
                high: Some(money("4100.00")),
                low: Some(money("3950.00")),
                written_at: at(2026, 11, 1),
            }
        );
        assert_eq!(unit.prices.len(), 2, "one entry per week");
    }

    #[test]
    fn the_last_price_is_the_newest_weeks_close_and_its_age_when_that_was_written() {
        let mut unit = UnitRow::new(
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        assert_eq!(unit.last_price(), None);
        unit.record_price(money("3900.00"), day(2026, 10, 20), at(2026, 10, 20));
        unit.record_price(money("4000.00"), day(2026, 10, 27), at(2026, 10, 27));

        let last = unit.last_price().expect("two weeks are written");

        assert_eq!(
            (&last.close, last.written_at),
            (&money("4000.00"), at(2026, 10, 27))
        );
    }

    #[test]
    fn the_seed_gives_btc_52_weeks_ending_on_the_current_week_and_the_rest_none() {
        let now = at(2026, 10, 29);
        let units = default_units(now);
        let btc = &units[1];

        let weeks: Vec<_> = btc.prices.iter().map(|price| price.week).collect();

        assert_eq!(weeks.len(), 52);
        assert_eq!(weeks.first(), Some(&day(2025, 11, 3)));
        assert_eq!(weeks.last(), Some(&day(2026, 10, 26)));
        assert!(
            weeks
                .windows(2)
                .all(|pair| pair[1] - pair[0] == Duration::weeks(1))
        );
        for price in &btc.prices[..51] {
            let (open, high, low) = (
                price.open.as_ref().expect("a past week has an open"),
                price.high.as_ref().expect("and a high"),
                price.low.as_ref().expect("and a low"),
            );
            assert!(high >= open && high >= &price.close, "{price:?}");
            assert!(low <= open && low <= &price.close, "{price:?}");
        }
        assert_eq!(default_units(now), units, "the seed is deterministic");
        assert!(units[0].prices.is_empty() && units[2].prices.is_empty());
    }

    #[test]
    fn the_seed_shows_aud_and_vas_to_two_places_and_btc_to_eight() {
        let places: Vec<_> = seed()
            .into_iter()
            .map(|unit| (unit.code, unit.precision.places()))
            .collect();
        assert_eq!(
            places,
            vec![
                ("aud".to_string(), 2),
                ("btc".to_string(), 8),
                ("vas".to_string(), 2),
            ]
        );
    }

    #[test]
    fn editing_writes_the_precision_and_leaves_the_weekly_prices_as_stored() {
        let mut units = seed();
        let prices = units[1].prices.clone();
        let link = units[1].link.clone();
        let six = Precision::new(6).expect("in range");

        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "btc".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
            six,
            link,
        )
        .expect("btc is seeded");

        assert_eq!(edited.precision, six);
        assert_eq!(edited.prices, prices, "a Precision change only reformats");
    }

    #[test]
    fn the_seed_is_aud_currency_btc_crypto_and_vas_equity() {
        let units: Vec<_> = seed()
            .into_iter()
            .map(|unit| (unit.code, unit.kind))
            .collect();
        assert_eq!(
            units,
            vec![
                ("aud".to_string(), UnitKind::Currency),
                ("btc".to_string(), UnitKind::Crypto),
                ("vas".to_string(), UnitKind::Equity),
            ]
        );
    }

    #[test]
    fn the_seed_links_btc_to_coingecko_in_aud_and_the_rest_to_manual_entry() {
        let now = Utc::now();
        let units = default_units(now);
        let btc = &units[1];
        assert_eq!(
            btc.link,
            PriceLink {
                source_id: COINGECKO,
                symbol: "bitcoin".into(),
                quote: UnitId::SEED_AUD
            }
        );
        let last = btc.last_price().expect("btc is seeded with prices");
        assert_eq!(
            (&last.close, last.written_at),
            (&"148212.40".parse().unwrap(), now - Duration::minutes(5))
        );
        for unit in [&units[0], &units[2]] {
            assert_eq!(
                unit.link,
                PriceLink::manual(UnitId::SEED_AUD),
                "{}",
                unit.code
            );
            assert_eq!(unit.last_price(), None);
        }
    }

    #[test]
    fn a_balance_is_valued_at_the_last_price_in_the_quote_unit_to_its_precision() {
        let btc = &seed()[1];
        let price = btc.last_price().expect("btc is seeded with a price");
        let aud = Precision::of_kind(UnitKind::Currency);

        let value = price.value(&"0.4120".parse().unwrap(), aud);

        // 0.4120 × 148,212.40 = 61,063.50888, quoted in btc's quote Unit (aud).
        assert_eq!(value, "61063.51".parse().unwrap());
        assert_eq!(btc.link.quote, UnitId::SEED_AUD);
    }

    #[test]
    fn units_sort_by_code_ignoring_case() {
        let mut units = seed();
        add_unit(
            &mut units,
            "ETH".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        add_unit(
            &mut units,
            "Ada".into(),
            "Cardano".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        let codes: Vec<_> = units_by_code(&units)
            .into_iter()
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(codes, vec!["Ada", "aud", "btc", "ETH", "vas"]);
    }

    #[test]
    fn a_dialog_added_unit_has_its_link_and_no_price() {
        let mut units = seed();
        let link = PriceLink {
            source_id: COINGECKO,
            symbol: "ethereum".into(),
            quote: UnitId::SEED_AUD,
        };
        let added = add_unit(
            &mut units,
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            link.clone(),
        );
        assert_eq!(added.code, "eth");
        assert_eq!(added.link, link);
        assert!(added.prices.is_empty());
        assert_eq!(units.len(), 4);
    }

    #[test]
    fn editing_writes_the_link_and_keeps_the_weekly_prices_in_the_same_quote() {
        let mut units = seed();
        let prices = units[1].prices.clone();
        let link = PriceLink {
            source_id: MANUAL_ENTRY,
            symbol: String::new(),
            quote: UnitId::SEED_AUD,
        };
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "xbt".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            link.clone(),
        )
        .expect("btc is seeded");
        assert_eq!(edited.code, "xbt");
        assert_eq!(edited.link, link);
        assert_eq!(edited.prices, prices);
        assert!(!prices.is_empty());
    }

    #[test]
    fn changing_the_quote_clears_the_weekly_prices() {
        let mut units = seed();
        let mut link = units[1].link.clone();
        link.quote = UnitId::SEED_VAS;
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "btc".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            link,
        )
        .expect("btc is seeded");
        assert!(edited.prices.is_empty());
    }

    #[test]
    fn a_unit_kind_survives_an_edit_round_trip() {
        let mut units = seed();
        for kind in UnitKind::all() {
            let link = units[1].link.clone();
            let edited = edit_unit(
                &mut units,
                UnitId::SEED_BTC,
                "btc".into(),
                "Bitcoin".into(),
                *kind,
                Precision::of_kind(*kind),
                link,
            )
            .expect("btc is seeded");
            assert_eq!(edited.kind, *kind);
        }
    }

    #[test]
    fn editing_an_unknown_id_changes_nothing() {
        let mut units = seed();
        let before = units.clone();
        assert!(
            edit_unit(
                &mut units,
                UnitId::new(),
                "x".into(),
                "x".into(),
                UnitKind::Currency,
                Precision::of_kind(UnitKind::Currency),
                PriceLink::manual(UnitId::SEED_AUD),
            )
            .is_none()
        );
        assert_eq!(units, before);
    }

    #[test]
    fn removing_takes_the_row_with_that_id() {
        let mut units = seed();
        assert!(remove_unit(&mut units, UnitId::new()).is_none());
        assert_eq!(units.len(), 3);
        let removed = remove_unit(&mut units, UnitId::SEED_BTC).expect("btc is seeded");
        assert_eq!(removed.code, "btc");
        assert_eq!(units.len(), 2);
    }

    #[test]
    fn editing_the_code_keeps_the_id() {
        let mut units = seed();
        let link = units[1].link.clone();
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "xbt".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            link,
        )
        .expect("btc is seeded");
        assert_eq!(edited.id, UnitId::SEED_BTC);
    }

    #[test]
    fn every_new_unit_starts_active_and_set_unit_active_flips_only_that_row() {
        let mut units = seed();
        assert!(units.iter().all(|unit| unit.is_active));
        let added = add_unit(
            &mut units,
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        assert!(added.is_active);

        let flipped = set_unit_active(&mut units, UnitId::SEED_BTC, false).map(|u| u.is_active);

        assert_eq!(flipped, Some(false));
        assert_eq!(units.iter().filter(|unit| !unit.is_active).count(), 1);
        assert!(set_unit_active(&mut units, UnitId::new(), false).is_none());
    }

    #[test]
    fn editing_a_deactivated_unit_keeps_it_deactivated() {
        let mut units = seed();
        set_unit_active(&mut units, UnitId::SEED_VAS, false);
        let link = units[2].link.clone();

        edit_unit(
            &mut units,
            UnitId::SEED_VAS,
            "vas".into(),
            "Vanguard".into(),
            UnitKind::Equity,
            Precision::of_kind(UnitKind::Equity),
            link,
        );

        assert!(!units[2].is_active);
    }
}
