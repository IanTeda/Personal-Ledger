//! Price Sources (`GLOSSARY.md`'s Price Source): where a Unit's prices come from.
//!
//! The built-in sources are a `const` table here, referenced by id, so an app update that fixes a
//! provider's address fixes it on every Ledger at once (ADR-0043). A Ledger stores only what a
//! person changed: a built-in's name and Updates override, or a whole Custom source. Manual entry
//! is the fallback every Unit can sit on: it prices every Kind and never fetches. API keys are
//! never stored here, only the name of the parameter or header that carries one.

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use lib_core::{PriceSourceId, UnitKind};

use crate::UnitRow;

/// Manual entry's id. These ids are fixed for good, not stub seeds: a Ledger refers to Manual
/// entry and the built-ins by them.
pub const MANUAL_ENTRY: PriceSourceId = PriceSourceId::seed(0);
pub const COINGECKO: PriceSourceId = PriceSourceId::seed(1);
pub const YAHOO_FINANCE: PriceSourceId = PriceSourceId::seed(2);
pub const FRANKFURTER: PriceSourceId = PriceSourceId::seed(3);

/// How often a source is asked for a new price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Updates {
    Every15Minutes,
    Hourly,
    Daily,
    ManualOnly,
}

/// Where a source's API key goes in the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPlacement {
    /// A query parameter.
    Param,
    /// A request header.
    Header,
}

/// How a source authenticates. Only the name the key travels under is stored: the key itself is
/// each Client's own and never syncs (ADR-0043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceSourceAuth {
    None,
    ApiKey {
        placement: KeyPlacement,
        name: Cow<'static, str>,
    },
}

/// How to ask a source for one price. `request` and `price_path` take `{symbol}` and `{quote}`
/// placeholders, filled from the Unit's link. `Cow` lets the built-in table borrow its strings in
/// a `const` while a Custom source owns what a person typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The base address, e.g. `https://api.coingecko.com/api/v3`.
    pub address: Cow<'static, str>,
    /// The path and query appended to the address.
    pub request: Cow<'static, str>,
    /// A JSON pointer to the price in the response.
    pub price_path: Cow<'static, str>,
    pub auth: PriceSourceAuth,
}

/// A source shipped in code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltInSource {
    pub id: PriceSourceId,
    /// The provider's own name, which is not translated.
    pub name: &'static str,
    pub endpoint: Endpoint,
    /// Used until a person sets their own.
    pub updates: Updates,
    /// The Unit Kinds it can price.
    pub kinds: &'static [UnitKind],
}

/// The built-in sources, always listed.
pub const BUILT_IN_SOURCES: &[BuiltInSource] = &[
    BuiltInSource {
        id: COINGECKO,
        name: "CoinGecko",
        endpoint: Endpoint {
            address: Cow::Borrowed("https://api.coingecko.com/api/v3"),
            request: Cow::Borrowed("/simple/price?ids={symbol}&vs_currencies={quote}"),
            price_path: Cow::Borrowed("/{symbol}/{quote}"),
            auth: PriceSourceAuth::None,
        },
        updates: Updates::Every15Minutes,
        kinds: &[UnitKind::Crypto],
    },
    BuiltInSource {
        id: YAHOO_FINANCE,
        name: "Yahoo Finance",
        endpoint: Endpoint {
            address: Cow::Borrowed("https://query1.finance.yahoo.com"),
            request: Cow::Borrowed("/v8/finance/chart/{symbol}"),
            price_path: Cow::Borrowed("/chart/result/0/meta/regularMarketPrice"),
            auth: PriceSourceAuth::None,
        },
        updates: Updates::Hourly,
        kinds: &[UnitKind::Equity, UnitKind::FixedIncome, UnitKind::Commodity],
    },
    BuiltInSource {
        id: FRANKFURTER,
        name: "Frankfurter",
        endpoint: Endpoint {
            address: Cow::Borrowed("https://api.frankfurter.app"),
            request: Cow::Borrowed("/latest?from={symbol}&to={quote}"),
            price_path: Cow::Borrowed("/rates/{quote}"),
            auth: PriceSourceAuth::None,
        },
        // Frankfurter publishes the ECB's reference rates once a working day.
        updates: Updates::Daily,
        kinds: &[UnitKind::Currency],
    },
];

/// The built-in source `id`, if it is one.
pub fn built_in(id: PriceSourceId) -> Option<&'static BuiltInSource> {
    BUILT_IN_SOURCES.iter().find(|source| source.id == id)
}

/// A person's name and Updates for a built-in source: all the Ledger stores of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltInOverride {
    pub id: PriceSourceId,
    pub name: String,
    pub updates: Updates,
}

/// A source a person added. It declares no Unit Kinds, so it is offered for every one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomSource {
    pub id: PriceSourceId,
    pub name: String,
    pub endpoint: Endpoint,
    pub updates: Updates,
}

/// One stored Price Source row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceSourceRecord {
    BuiltIn(BuiltInOverride),
    Custom(CustomSource),
}

impl PriceSourceRecord {
    pub fn id(&self) -> PriceSourceId {
        match self {
            Self::BuiltIn(source) => source.id,
            Self::Custom(source) => source.id,
        }
    }
}

/// What a listed source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOrigin<'a> {
    ManualEntry,
    BuiltIn(&'static BuiltInSource),
    Custom(&'a CustomSource),
}

/// A source as a Client lists it: Manual entry, a built-in with any override applied, or a Custom
/// source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceSource<'a> {
    pub id: PriceSourceId,
    pub name: &'a str,
    pub updates: Updates,
    pub origin: SourceOrigin<'a>,
}

impl PriceSource<'_> {
    /// Whether it can price a Unit of `kind`: a built-in only its own Kinds, Manual entry and a
    /// Custom source every Kind.
    pub fn prices(&self, kind: UnitKind) -> bool {
        match self.origin {
            SourceOrigin::BuiltIn(source) => source.kinds.contains(&kind),
            SourceOrigin::ManualEntry | SourceOrigin::Custom(_) => true,
        }
    }

    /// How it is asked for a price; Manual entry never fetches, so has none.
    pub fn endpoint(&self) -> Option<&Endpoint> {
        match self.origin {
            SourceOrigin::ManualEntry => None,
            SourceOrigin::BuiltIn(source) => Some(&source.endpoint),
            SourceOrigin::Custom(source) => Some(&source.endpoint),
        }
    }
}

/// Every source, sorted by its current name ignoring case, with nothing pinned. Manual entry's
/// name is a Message, so the Client passes it in as `manual_name` and it sorts in with the rest.
pub fn price_sources<'a>(
    records: &'a [PriceSourceRecord],
    manual_name: &'a str,
) -> Vec<PriceSource<'a>> {
    let manual = PriceSource {
        id: MANUAL_ENTRY,
        name: manual_name,
        updates: Updates::ManualOnly,
        origin: SourceOrigin::ManualEntry,
    };
    let built_ins = BUILT_IN_SOURCES.iter().map(|source| {
        let stored = records.iter().find_map(|record| match record {
            PriceSourceRecord::BuiltIn(stored) if stored.id == source.id => Some(stored),
            _ => None,
        });
        PriceSource {
            id: source.id,
            name: stored.map_or(source.name, |stored| stored.name.as_str()),
            updates: stored.map_or(source.updates, |stored| stored.updates),
            origin: SourceOrigin::BuiltIn(source),
        }
    });
    let customs = records.iter().filter_map(|record| match record {
        PriceSourceRecord::Custom(source) => Some(PriceSource {
            id: source.id,
            name: source.name.as_str(),
            updates: source.updates,
            origin: SourceOrigin::Custom(source),
        }),
        PriceSourceRecord::BuiltIn(_) => None,
    });
    let mut sources: Vec<_> = std::iter::once(manual)
        .chain(built_ins)
        .chain(customs)
        .collect();
    sources.sort_by_cached_key(|source| (source.name.to_lowercase(), source.id));
    sources
}

/// Whether a Unit can link to source `id`: Manual entry, a built-in or a stored Custom source.
pub fn price_source_exists(records: &[PriceSourceRecord], id: PriceSourceId) -> bool {
    id == MANUAL_ENTRY || built_in(id).is_some() || records.iter().any(|record| record.id() == id)
}

/// The source `id`, if it exists.
pub fn price_source<'a>(
    records: &'a [PriceSourceRecord],
    manual_name: &'a str,
    id: PriceSourceId,
) -> Option<PriceSource<'a>> {
    price_sources(records, manual_name)
        .into_iter()
        .find(|source| source.id == id)
}

/// The sources able to price a Unit of `kind`, in [`price_sources`]' order: the built-ins for that
/// Kind, Manual entry and every Custom source.
pub fn sources_for_kind<'a>(
    records: &'a [PriceSourceRecord],
    manual_name: &'a str,
    kind: UnitKind,
) -> Vec<PriceSource<'a>> {
    price_sources(records, manual_name)
        .into_iter()
        .filter(|source| source.prices(kind))
        .collect()
}

/// When source `id` last fetched a price for any of its Units, or `None` when it has not fetched
/// yet.
pub fn last_fetched(units: &[UnitRow], id: PriceSourceId) -> Option<DateTime<Utc>> {
    units
        .iter()
        .filter(|unit| unit.link.source_id == id)
        .filter_map(UnitRow::last_price)
        .map(|price| price.written_at)
        .max()
}

/// Appends a Custom source and returns it.
pub fn add_custom_source(
    records: &mut Vec<PriceSourceRecord>,
    name: String,
    endpoint: Endpoint,
    updates: Updates,
) -> CustomSource {
    let source = CustomSource {
        id: PriceSourceId::new(),
        name,
        endpoint,
        updates,
    };
    records.push(PriceSourceRecord::Custom(source.clone()));
    source
}

/// Stores a person's name and Updates for built-in `id`, replacing any earlier override, and
/// returns the record written.
pub fn override_built_in(
    records: &mut Vec<PriceSourceRecord>,
    id: PriceSourceId,
    name: String,
    updates: Updates,
) -> PriceSourceRecord {
    let record = PriceSourceRecord::BuiltIn(BuiltInOverride { id, name, updates });
    match records.iter_mut().find(|stored| stored.id() == id) {
        Some(stored) => *stored = record.clone(),
        None => records.push(record.clone()),
    }
    record
}

/// Rewrites Custom source `id`, keeping its endpoint when `endpoint` is `None`. Returns the record
/// written, or `None` when no Custom source has `id`.
pub fn edit_custom_source(
    records: &mut [PriceSourceRecord],
    id: PriceSourceId,
    name: String,
    updates: Updates,
    endpoint: Option<Endpoint>,
) -> Option<PriceSourceRecord> {
    let source = records.iter_mut().find_map(|record| match record {
        PriceSourceRecord::Custom(source) if source.id == id => Some(source),
        _ => None,
    })?;
    source.name = name;
    source.updates = updates;
    if let Some(endpoint) = endpoint {
        source.endpoint = endpoint;
    }
    Some(PriceSourceRecord::Custom(source.clone()))
}

/// Removes Custom source `id` and returns it, or `None` when no Custom source has `id`.
pub fn remove_custom_source(
    records: &mut Vec<PriceSourceRecord>,
    id: PriceSourceId,
) -> Option<CustomSource> {
    let index = records.iter().position(
        |record| matches!(record, PriceSourceRecord::Custom(source) if source.id == id),
    )?;
    match records.remove(index) {
        PriceSourceRecord::Custom(source) => Some(source),
        PriceSourceRecord::BuiltIn(_) => unreachable!("the position matched a Custom source"),
    }
}

/// Moves every Unit on source `id` to Manual entry, keeping its last price, and returns the Units
/// it moved.
pub fn move_units_to_manual(units: &mut [UnitRow], id: PriceSourceId) -> Vec<UnitRow> {
    units
        .iter_mut()
        .filter(|unit| unit.link.source_id == id)
        .map(|unit| {
            unit.move_to_manual();
            unit.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_units;

    const MANUAL: &str = "Manual entry";

    fn endpoint() -> Endpoint {
        Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/{symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Header,
                name: "x-api-key".to_string().into(),
            },
        }
    }

    fn names(sources: &[PriceSource<'_>]) -> Vec<String> {
        sources
            .iter()
            .map(|source| source.name.to_string())
            .collect()
    }

    #[test]
    fn the_built_in_table_holds_coingecko_yahoo_and_frankfurter_with_their_kinds() {
        let table: Vec<_> = BUILT_IN_SOURCES
            .iter()
            .map(|source| (source.name, source.kinds))
            .collect();
        assert_eq!(
            table,
            vec![
                ("CoinGecko", &[UnitKind::Crypto][..]),
                (
                    "Yahoo Finance",
                    &[UnitKind::Equity, UnitKind::FixedIncome, UnitKind::Commodity][..]
                ),
                ("Frankfurter", &[UnitKind::Currency][..]),
            ]
        );
    }

    #[test]
    fn built_in_ids_are_distinct_and_none_is_manual_entry() {
        let mut ids: Vec<_> = BUILT_IN_SOURCES.iter().map(|source| source.id).collect();
        ids.push(MANUAL_ENTRY);
        let count = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }

    #[test]
    fn every_built_in_request_names_its_symbol() {
        for source in BUILT_IN_SOURCES {
            assert!(
                source.endpoint.request.contains("{symbol}"),
                "{}",
                source.name
            );
            assert!(
                source.endpoint.price_path.starts_with('/'),
                "{}",
                source.name
            );
        }
    }

    #[test]
    fn with_nothing_stored_the_built_ins_and_manual_entry_are_listed_by_name() {
        assert_eq!(
            names(&price_sources(&[], MANUAL)),
            vec!["CoinGecko", "Frankfurter", "Manual entry", "Yahoo Finance"]
        );
    }

    #[test]
    fn sources_sort_by_current_name_ignoring_case_with_nothing_pinned() {
        let mut records = Vec::new();
        add_custom_source(&mut records, "alpha".into(), endpoint(), Updates::Daily);
        add_custom_source(&mut records, "Zeta".into(), endpoint(), Updates::Daily);
        override_built_in(&mut records, COINGECKO, "my coins".into(), Updates::Hourly);
        assert_eq!(
            names(&price_sources(&records, MANUAL)),
            vec![
                "alpha",
                "Frankfurter",
                "Manual entry",
                "my coins",
                "Yahoo Finance",
                "Zeta"
            ]
        );
    }

    #[test]
    fn an_override_renames_and_retimes_a_built_in_but_keeps_its_endpoint() {
        let mut records = Vec::new();
        override_built_in(&mut records, COINGECKO, "Coins".into(), Updates::Daily);
        override_built_in(&mut records, COINGECKO, "Gecko".into(), Updates::Hourly);
        assert_eq!(records.len(), 1, "a second override replaces the first");
        let source = price_source(&records, MANUAL, COINGECKO).unwrap();
        assert_eq!((source.name, source.updates), ("Gecko", Updates::Hourly));
        assert_eq!(
            source.endpoint(),
            Some(&built_in(COINGECKO).unwrap().endpoint)
        );
    }

    #[test]
    fn a_kind_is_offered_its_built_ins_manual_entry_and_every_custom_source() {
        let mut records = Vec::new();
        add_custom_source(&mut records, "Mine".into(), endpoint(), Updates::Daily);
        assert_eq!(
            names(&sources_for_kind(&records, MANUAL, UnitKind::Crypto)),
            vec!["CoinGecko", "Manual entry", "Mine"]
        );
        assert_eq!(
            names(&sources_for_kind(&records, MANUAL, UnitKind::Commodity)),
            vec!["Manual entry", "Mine", "Yahoo Finance"]
        );
        assert_eq!(
            names(&sources_for_kind(&records, MANUAL, UnitKind::Custom)),
            vec!["Manual entry", "Mine"]
        );
    }

    #[test]
    fn manual_entry_never_fetches() {
        let manual = price_source(&[], MANUAL, MANUAL_ENTRY).unwrap();
        assert_eq!(manual.endpoint(), None);
        assert_eq!(manual.updates, Updates::ManualOnly);
    }

    #[test]
    fn last_fetched_is_the_newest_price_among_a_sources_units() {
        let now = Utc::now();
        let units = default_units(now);
        assert_eq!(
            last_fetched(&units, COINGECKO),
            Some(now - chrono::Duration::minutes(5))
        );
        assert_eq!(last_fetched(&units, FRANKFURTER), None);
        assert_eq!(last_fetched(&units, MANUAL_ENTRY), None);
    }

    #[test]
    fn editing_a_custom_source_keeps_its_endpoint_unless_given_one() {
        let mut records = Vec::new();
        let added = add_custom_source(&mut records, "Mine".into(), endpoint(), Updates::Daily);
        let edited =
            edit_custom_source(&mut records, added.id, "Ours".into(), Updates::Hourly, None)
                .unwrap();
        let PriceSourceRecord::Custom(edited) = edited else {
            panic!("a Custom source stays Custom");
        };
        assert_eq!(
            (edited.name.as_str(), edited.updates),
            ("Ours", Updates::Hourly)
        );
        assert_eq!(edited.endpoint, endpoint());
        assert!(
            edit_custom_source(&mut records, COINGECKO, "x".into(), Updates::Daily, None).is_none()
        );
    }

    #[test]
    fn removing_takes_only_a_custom_source() {
        let mut records = Vec::new();
        override_built_in(&mut records, COINGECKO, "Coins".into(), Updates::Daily);
        let added = add_custom_source(&mut records, "Mine".into(), endpoint(), Updates::Daily);
        assert!(remove_custom_source(&mut records, COINGECKO).is_none());
        assert_eq!(remove_custom_source(&mut records, added.id), Some(added));
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn moving_units_to_manual_keeps_their_weekly_prices() {
        let mut units = default_units(Utc::now());
        let before = units[1].prices.clone();
        let moved = move_units_to_manual(&mut units, COINGECKO);
        assert_eq!(moved.len(), 1);
        assert_eq!(units[1].link.source_id, MANUAL_ENTRY);
        assert_eq!(units[1].prices, before);
        assert!(!before.is_empty());
    }
}
