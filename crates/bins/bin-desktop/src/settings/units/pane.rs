//! What the X-Large detail pane's PRICE SOURCE and VALUES groups say (DESKTOP-125, DESKTOP-140),
//! resolved for the Locale from the Ledger. `render.rs` draws it and the test snapshot reads it
//! back, so both see the same text.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use lib_locale::Label;
use lib_units::{LastPrice, PriceSourceAuth, SourceOrigin, UnitRow};

use super::{LastUpdated, NOT_FETCHED_GLYPH, SourceKind, SourceRow};
use crate::settings::units::dialogs::sources::updates_label;

/// One fact row: a muted label over a right-aligned value, monospace for a URL part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Fact {
    pub label: String,
    pub value: String,
    pub mono: bool,
}

impl Fact {
    fn plain(label: String, value: String) -> Self {
        Self {
            label,
            value,
            mono: false,
        }
    }

    fn mono(label: String, value: String) -> Self {
        Self {
            label,
            value,
            mono: true,
        }
    }
}

/// A selected Unit's PRICE SOURCE group (DESKTOP-125): the source its link names and the Unit's
/// own last price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedSource {
    pub row: SourceRow,
    pub facts: Vec<Fact>,
}

/// A selected Price Source's PRICE SOURCE and VALUES groups (DESKTOP-140). Manual entry has no
/// meta line, no fact rows and no buttons, only its note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceDetail {
    pub row: SourceRow,
    /// `built-in · prices Crypto · key not needed`; `None` for Manual entry.
    pub meta: Option<String>,
    pub facts: Vec<Fact>,
    /// The note under the fact rows: a built-in's "isn't edited here", Manual entry's "Prices you
    /// enter by hand.", none for a Custom source.
    pub note: Option<String>,
    pub values: Vec<ValueRow>,
    /// VALUES' closing note; `None` while no Unit uses the source and for Manual entry.
    pub values_note: Option<String>,
}

/// One Unit in VALUES: `code · name` over its symbol and quote on the left, its last price over
/// its age on the right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValueRow {
    pub code: String,
    pub name: String,
    /// What the source calls the Unit; `None` when it has no symbol (a Unit priced by hand).
    pub symbol: Option<String>,
    /// `quoted in aud`.
    pub quoted_in: String,
    /// The last price in its quote Unit, or — before the first.
    pub price: String,
    /// How long ago that price was set, or the not-fetched label for a source that fetches.
    pub age: Option<String>,
}

impl ValueRow {
    /// The sub-line as text: `symbol bitcoin · quoted in aud`.
    pub fn detail(&self) -> String {
        match &self.symbol {
            Some(symbol) => format!(
                "{} {symbol} \u{b7} {}",
                crate::msg::desktop_settings_units_pane_symbol(),
                self.quoted_in
            ),
            None => self.quoted_in.clone(),
        }
    }
}

/// The code of the Unit `unit`'s price is quoted in, empty when that Unit is gone.
fn quote_code(unit: &UnitRow, units: &[UnitRow]) -> String {
    units
        .iter()
        .find(|quote| quote.id == unit.link.quote)
        .map(|quote| quote.code.clone())
        .unwrap_or_default()
}

fn ago(at: DateTime<Utc>) -> String {
    crate::navigation::explorer::format_modified(at.into(), SystemTime::now())
}

/// A price in its quote Unit: `148,212.40 aud`.
fn price_text(price: &LastPrice, quote: &str) -> String {
    format!("{} {quote}", crate::view::format::amount(&price.price).1)
}

fn not_fetched() -> String {
    format!(
        "{NOT_FETCHED_GLYPH} {}",
        crate::msg::desktop_settings_units_not_fetched_yet()
    )
}

/// `row`'s LAST UPDATED with the ✗ drawn in front of a source that hasn't fetched yet.
fn last_updated(row: &SourceRow) -> String {
    match row.last_updated {
        LastUpdated::NotFetched => not_fetched(),
        LastUpdated::Never | LastUpdated::Ago(_) => row.last_updated_text(),
    }
}

/// The PRICE SOURCE group for `unit`, linked to `source` (never Manual entry: a Unit on it offers
/// Add price source instead).
pub(super) fn linked_source(
    source: &lib_units::PriceSource<'_>,
    row: SourceRow,
    unit: &UnitRow,
    units: &[UnitRow],
) -> LinkedSource {
    let none = crate::msg::desktop_settings_units_pane_none;
    let quote = quote_code(unit, units);
    let facts = vec![
        Fact::plain(
            crate::msg::desktop_settings_units_column_source(),
            row.name.clone(),
        ),
        Fact::mono(
            crate::msg::desktop_settings_units_pane_url(),
            row.address.clone().unwrap_or_else(none),
        ),
        Fact::plain(
            crate::msg::desktop_settings_price_source_field_updates(),
            updates_label(source.updates),
        ),
        Fact::plain(
            crate::msg::desktop_settings_units_pane_last_price(),
            unit.last_price
                .as_ref()
                .map_or_else(none, |price| price_text(price, &quote)),
        ),
        Fact::plain(
            crate::msg::desktop_settings_units_column_last_updated(),
            unit.last_price
                .as_ref()
                .map_or_else(not_fetched, |price| ago(price.fetched_at)),
        ),
    ];
    LinkedSource { row, facts }
}

/// The PRICE SOURCE and VALUES groups for a selected `source`; `units` is every Unit, of which
/// VALUES lists those linked to it.
pub(super) fn source_detail(
    source: &lib_units::PriceSource<'_>,
    row: SourceRow,
    units: &[UnitRow],
) -> SourceDetail {
    let values: Vec<ValueRow> = units
        .iter()
        .filter(|unit| unit.link.source_id == source.id)
        .map(|unit| {
            let quote = quote_code(unit, units);
            ValueRow {
                code: unit.code.clone(),
                name: unit.name.clone(),
                symbol: (!unit.link.symbol.is_empty()).then(|| unit.link.symbol.clone()),
                quoted_in: crate::msg::desktop_settings_units_pane_quoted_in(&quote),
                price: unit
                    .last_price
                    .as_ref()
                    .map_or_else(crate::msg::desktop_settings_units_pane_none, |price| {
                        price_text(price, &quote)
                    }),
                age: match &unit.last_price {
                    Some(price) => Some(ago(price.fetched_at)),
                    None if row.kind == SourceKind::ManualEntry => None,
                    None => Some(not_fetched()),
                },
            }
        })
        .collect();
    let Some(endpoint) = source.endpoint() else {
        return SourceDetail {
            row,
            meta: None,
            facts: Vec::new(),
            note: Some(crate::msg::desktop_settings_units_pane_manual_note()),
            values,
            values_note: None,
        };
    };
    let (origin, kinds, source_fact) = match source.origin {
        SourceOrigin::BuiltIn(built_in) => (
            crate::msg::desktop_settings_units_pane_origin_built_in(),
            built_in
                .kinds
                .iter()
                .map(Label::label)
                .collect::<Vec<_>>()
                .join(", "),
            crate::msg::desktop_settings_price_source_built_in(&row.name),
        ),
        // A Custom source declares no Kinds, so it is offered for every one.
        SourceOrigin::Custom(_) | SourceOrigin::ManualEntry => (
            crate::msg::desktop_settings_units_pane_origin_custom(),
            crate::msg::desktop_settings_units_pane_every_kind(),
            crate::msg::desktop_settings_units_pane_source_custom(&row.name),
        ),
    };
    let key = match endpoint.auth {
        PriceSourceAuth::None => crate::msg::desktop_settings_units_pane_key_not_needed(),
        PriceSourceAuth::ApiKey { .. } => crate::msg::desktop_settings_units_pane_key_needed(),
    };
    let mut facts = vec![
        Fact::plain(
            crate::msg::desktop_settings_units_column_source(),
            source_fact,
        ),
        Fact::mono(
            crate::msg::desktop_settings_price_source_field_address(),
            endpoint.address.to_string(),
        ),
        Fact::mono(
            crate::msg::desktop_settings_price_source_field_request(),
            endpoint.request.to_string(),
        ),
        Fact::mono(
            crate::msg::desktop_settings_price_source_field_price_path(),
            endpoint.price_path.to_string(),
        ),
    ];
    let custom = row.kind == SourceKind::Custom;
    if custom {
        // The key parameter's name, never the key: the key is never stored (ADR-0043).
        let (auth, key_param) = match &endpoint.auth {
            PriceSourceAuth::None => (
                crate::msg::desktop_settings_price_source_auth_none(),
                crate::msg::desktop_settings_units_pane_none(),
            ),
            PriceSourceAuth::ApiKey { name, .. } => (
                crate::msg::desktop_settings_price_source_auth_api_key(),
                name.to_string(),
            ),
        };
        facts.push(Fact::plain(
            crate::msg::desktop_settings_price_source_field_auth(),
            auth,
        ));
        facts.push(Fact::mono(
            crate::msg::desktop_settings_price_source_field_key_param(),
            key_param,
        ));
    }
    facts.push(Fact::plain(
        crate::msg::desktop_settings_price_source_field_updates(),
        updates_label(source.updates),
    ));
    facts.push(Fact::plain(
        crate::msg::desktop_settings_units_column_last_updated(),
        last_updated(&row),
    ));
    let values_note = (!values.is_empty()).then(|| {
        if custom {
            crate::msg::desktop_settings_units_pane_values_note_custom()
        } else {
            crate::msg::desktop_settings_units_pane_values_note()
        }
    });
    SourceDetail {
        meta: Some(crate::msg::desktop_settings_units_pane_source_meta(
            &origin, &kinds, &key,
        )),
        facts,
        note: (!custom).then(crate::msg::desktop_settings_price_source_built_in_help),
        row,
        values,
        values_note,
    }
}

/// An account's balance valued at `unit`'s last price ([`LastPrice::value`]), labelled with the
/// quote Unit's code, which is what the value is in. `None` without a price to convert by.
pub(super) fn quote_value(
    balance: &lib_core::Money,
    unit: &UnitRow,
    units: &[UnitRow],
) -> Option<String> {
    let value = unit.last_price.as_ref()?.value(balance);
    Some(format!(
        "{} {}",
        crate::view::format::amount(&value).1,
        quote_code(unit, units)
    ))
}
