//! The Units Entities: a Unit, its link to a Price Source and its newest price, with the rules
//! about themselves only. The Price Sources are in `price_source`.

mod price_source;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use lib_core::{Money, PriceSourceId, UnitId, UnitKind};
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
    /// Where its price comes from.
    pub link: PriceLink,
    /// The newest price, kept with no history.
    pub last_price: Option<LastPrice>,
    /// A deactivated Unit stays on the Ledger and on whatever uses it, but is no longer offered to
    /// pick (the lifecycle Category, Payee and Account follow).
    pub is_active: bool,
}

impl UnitRow {
    /// A new Unit from the Add unit dialog: active, with no price yet. Which Unit is the base is
    /// the Ledger's, so a new Unit is never it.
    pub fn new(code: String, name: String, kind: UnitKind, link: PriceLink) -> Self {
        Self {
            id: UnitId::new(),
            code,
            name,
            kind,
            link,
            last_price: None,
            is_active: true,
        }
    }

    /// Rewrites the Unit from the Edit unit dialog, keeping its id and whether it is active. The
    /// last price is kept unless the quote Unit changes, since a price in the old quote would read
    /// as one in the new.
    pub fn edit(&mut self, code: String, name: String, kind: UnitKind, link: PriceLink) {
        if self.link.quote != link.quote {
            self.last_price = None;
        }
        self.code = code;
        self.name = name;
        self.kind = kind;
        self.link = link;
    }

    /// Moves the Unit to Manual entry, keeping its symbol and last price, for when its Price
    /// Source is deleted.
    pub fn move_to_manual(&mut self) {
        self.link.source_id = MANUAL_ENTRY;
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

/// A Unit's newest price, in its link's quote Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastPrice {
    pub price: Money,
    pub fetched_at: DateTime<Utc>,
}

impl LastPrice {
    /// A balance of the Unit at this price, rounded to two places. It is in the link's quote
    /// Unit, not the base Unit: a Unit quoted in usd values in usd even on an aud Ledger.
    pub fn value(&self, balance: &Money) -> Money {
        Money((&balance.0 * &self.price.0).round(2))
    }
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
            is_active: true,
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
            is_active: true,
        },
        UnitRow {
            id: UnitId::SEED_VAS,
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: UnitKind::Equity,
            link: PriceLink::manual(UnitId::SEED_AUD),
            last_price: None,
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
    link: PriceLink,
) -> UnitRow {
    let row = UnitRow::new(code, name, kind, link);
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
    link: PriceLink,
) -> Option<&UnitRow> {
    let existing = units.iter_mut().find(|row| row.id == id)?;
    existing.edit(code, name, kind, link);
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
    fn a_balance_is_valued_at_the_last_price_in_the_quote_unit_to_two_places() {
        let btc = &seed()[1];
        let price = btc.last_price.as_ref().expect("btc is seeded with a price");

        let value = price.value(&"0.4120".parse().unwrap());

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
            link.clone(),
        );
        assert_eq!(added.code, "eth");
        assert_eq!(added.link, link);
        assert_eq!(added.last_price, None);
        assert_eq!(units.len(), 4);
    }

    #[test]
    fn editing_writes_the_link_and_keeps_the_price_in_the_same_quote() {
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

    #[test]
    fn every_new_unit_starts_active_and_set_unit_active_flips_only_that_row() {
        let mut units = seed();
        assert!(units.iter().all(|unit| unit.is_active));
        let added = add_unit(
            &mut units,
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
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
            link,
        );

        assert!(!units[2].is_active);
    }
}
