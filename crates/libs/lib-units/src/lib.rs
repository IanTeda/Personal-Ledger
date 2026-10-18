//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with the Price Sources that price them. Pure and I/O-free: the desktop's Settings › Units
//! page reads them through `LedgerStore`, and the Type selector's labels come from
//! `lib_locale::Label`, which owns the Message catalogue.

mod price_source;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
pub use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
pub use price_source::{
    BUILT_IN_SOURCES, BuiltInOverride, BuiltInSource, COINGECKO, CustomSource, Endpoint,
    FRANKFURTER, KeyPlacement, MANUAL_ENTRY, PriceSource, PriceSourceAuth, PriceSourceRecord,
    SourceOrigin, Updates, YAHOO_FINANCE, add_custom_source, built_in, edit_custom_source,
    last_fetched, move_units_to_manual, override_built_in, price_source, price_sources,
    remove_custom_source, sources_for_kind,
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
    /// Where its price comes from.
    pub link: PriceLink,
    /// The newest price, kept with no history.
    pub last_price: Option<LastPrice>,
    /// The Ledger's own base Unit. Nothing lets a person change which Unit is base yet, so a
    /// dialog-created Unit is never base.
    pub is_base: bool,
    /// The default Unit for new entries; the same "dialogs never set it" as [`Self::is_base`].
    pub is_default: bool,
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

/// A Unit's newest price, in its link's quote Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastPrice {
    pub price: Money,
    pub fetched_at: DateTime<Utc>,
}

/// The stub seed's three Units: aud and vas on Manual entry, btc on CoinGecko with a price
/// fetched five minutes before `now`.
pub fn default_units(now: DateTime<Utc>) -> Vec<UnitRow> {
    vec![
        UnitRow {
            id: UnitId::SEED_AUD,
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: UnitKind::Currency,
            link: PriceLink::manual(UnitId::SEED_AUD),
            last_price: None,
            is_base: true,
            is_default: true,
        },
        UnitRow {
            id: UnitId::SEED_BTC,
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: UnitKind::Crypto,
            link: PriceLink {
                source_id: COINGECKO,
                symbol: "bitcoin".to_string(),
                quote: UnitId::SEED_AUD,
            },
            last_price: Some(LastPrice {
                price: Money(BigDecimal::new(14_821_240.into(), 2)),
                fetched_at: now - chrono::Duration::minutes(5),
            }),
            is_base: false,
            is_default: false,
        },
        UnitRow {
            id: UnitId::SEED_VAS,
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: UnitKind::Equity,
            link: PriceLink::manual(UnitId::SEED_AUD),
            last_price: None,
            is_base: false,
            is_default: false,
        },
    ]
}

/// The Ledger's base Unit, if one is flagged.
pub fn base_unit(units: &[UnitRow]) -> Option<&UnitRow> {
    units.iter().find(|unit| unit.is_base)
}

/// The Units sorted by code, ignoring case.
pub fn units_by_code(units: &[UnitRow]) -> Vec<&UnitRow> {
    let mut sorted: Vec<_> = units.iter().collect();
    sorted.sort_by_cached_key(|unit| (unit.code.to_lowercase(), unit.id));
    sorted
}

/// Appends a Unit from the Add unit dialog, with no price yet. It can't be the base or default
/// Unit: nothing lets a person change which one that is. Returns the row it appended.
pub fn add_unit(
    units: &mut Vec<UnitRow>,
    code: String,
    name: String,
    kind: UnitKind,
    link: PriceLink,
) -> UnitRow {
    let row = UnitRow {
        id: UnitId::new(),
        code,
        name,
        kind,
        link,
        last_price: None,
        is_base: false,
        is_default: false,
    };
    units.push(row.clone());
    row
}

/// Rewrites the row `id` from the Edit unit dialog, keeping the base and default flags, which the
/// dialog has no field for. The last price is kept unless the quote Unit changes, since a price in
/// the old quote would read as one in the new. Returns the row it wrote, or `None` when no row has
/// `id`.
pub fn edit_unit(
    units: &mut [UnitRow],
    id: UnitId,
    code: String,
    name: String,
    kind: UnitKind,
    link: PriceLink,
) -> Option<&UnitRow> {
    let existing = units.iter_mut().find(|row| row.id == id)?;
    let last_price = if existing.link.quote == link.quote {
        existing.last_price.take()
    } else {
        None
    };
    *existing = UnitRow {
        id,
        code,
        name,
        kind,
        link,
        last_price,
        is_base: existing.is_base,
        is_default: existing.is_default,
    };
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
        assert_eq!(
            btc.last_price,
            Some(LastPrice {
                price: "148212.40".parse().unwrap(),
                fetched_at: now - chrono::Duration::minutes(5),
            })
        );
        for unit in [&units[0], &units[2]] {
            assert_eq!(
                unit.link,
                PriceLink::manual(UnitId::SEED_AUD),
                "{}",
                unit.code
            );
            assert_eq!(unit.last_price, None);
        }
    }

    #[test]
    fn only_aud_carries_the_base_and_default_flags() {
        let units = seed();
        let flagged: Vec<_> = units
            .iter()
            .filter(|unit| unit.is_base || unit.is_default)
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(flagged, vec!["aud"]);
        assert_eq!(
            base_unit(&units).map(|unit| unit.id),
            Some(UnitId::SEED_AUD)
        );
    }

    #[test]
    fn units_sort_by_code_ignoring_case() {
        let mut units = seed();
        add_unit(
            &mut units,
            "ETH".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            PriceLink::manual(UnitId::SEED_AUD),
        );
        add_unit(
            &mut units,
            "Ada".into(),
            "Cardano".into(),
            UnitKind::Crypto,
            PriceLink::manual(UnitId::SEED_AUD),
        );
        let codes: Vec<_> = units_by_code(&units)
            .into_iter()
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(codes, vec!["Ada", "aud", "btc", "ETH", "vas"]);
    }

    #[test]
    fn a_dialog_added_unit_has_its_link_no_price_and_neither_flag() {
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
            link.clone(),
        );
        assert_eq!(added.code, "eth");
        assert_eq!(added.link, link);
        assert_eq!(added.last_price, None);
        assert!(!added.is_base && !added.is_default);
        assert_eq!(units.len(), 4);
    }

    #[test]
    fn editing_writes_the_link_and_keeps_the_flags_and_the_price_in_the_same_quote() {
        let mut units = seed();
        let price = units[1].last_price.clone();
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
            link.clone(),
        )
        .expect("btc is seeded");
        assert_eq!(edited.code, "xbt");
        assert_eq!(edited.link, link);
        assert_eq!(edited.last_price, price);
        assert!(!edited.is_base && !edited.is_default);

        let aud = edit_unit(
            &mut units,
            UnitId::SEED_AUD,
            "aud".into(),
            "Aussie".into(),
            UnitKind::Currency,
            PriceLink::manual(UnitId::SEED_AUD),
        )
        .expect("aud is seeded");
        assert!(aud.is_base && aud.is_default);
    }

    #[test]
    fn changing_the_quote_drops_the_last_price() {
        let mut units = seed();
        let mut link = units[1].link.clone();
        link.quote = UnitId::SEED_VAS;
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "btc".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
            link,
        )
        .expect("btc is seeded");
        assert_eq!(edited.last_price, None);
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
            link,
        )
        .expect("btc is seeded");
        assert_eq!(edited.id, UnitId::SEED_BTC);
    }
}
