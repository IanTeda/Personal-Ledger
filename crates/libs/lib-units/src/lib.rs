//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with their price sources. Pure and I/O-free: the desktop's Settings › Units page reads
//! them through `LedgerStore`, and the Type selector's labels come from `lib_locale::Label`, which owns the
//! Message catalogue.

pub use lib_core::{UnitId, UnitKind};

/// One row of the **Units** section's table (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting
/// state" markup: CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS columns as of issue #189, up
/// from CODE / NAME / TYPE). Owned `String` fields, not `&'static str` --
/// issue #184's own Add unit dialog is this crate's first real typed-text input, so a row can
/// now hold text a person actually typed, not just compile-time dummy data. `kind` is the
/// [`UnitKind`] enum, never a display label: the label is read through `lib_locale::Label` at render
/// time, so a Locale switch can't change what an Edit then saves back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    /// Addresses the row across edits; `code` can change, so nothing else holds it as a key.
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    /// The SOURCE column shorthand (issue #189) -- e.g. `"USD"` (aud's rate is sourced against
    /// USD as the reference currency), `"CoinGecko"`, `"Manual entry"`. A unit added via the Add
    /// unit dialog (issue #184) has no real price-source integration, so it defaults to `"Manual
    /// entry"`, the same fallback `vas` already uses.
    pub source: String,
    /// The FLAGS column's `base` pill (`.tag.tag-accent`) -- the ledger's own base unit. Only
    /// `aud` carries this in the seeded data; nothing in this map lets a user change which unit
    /// is base, so a dialog-created row always defaults to `false`.
    pub is_base: bool,
    /// The FLAGS column's `default` pill (`.tag.tag-outline`) -- the default unit for new
    /// entries (formerly the removed "Ledger & units" section's own standalone control, now
    /// folded into this one flag). Same "only `aud`, dialogs default to `false`" reasoning as
    /// [`Self::is_base`].
    pub is_default: bool,
}

/// The mockup's own three seeded rows, in its own order -- a function rather than a `const`
/// slice now that [`UnitRow`] owns its strings (`String` has no `const` constructor).
pub fn default_units() -> Vec<UnitRow> {
    vec![
        UnitRow {
            id: UnitId::SEED_AUD,
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: UnitKind::Fiat,
            source: "USD".to_string(),
            is_base: true,
            is_default: true,
        },
        UnitRow {
            id: UnitId::SEED_BTC,
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: UnitKind::Crypto,
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        },
        UnitRow {
            id: UnitId::SEED_VAS,
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: UnitKind::Stock,
            source: "Manual entry".to_string(),
            is_base: false,
            is_default: false,
        },
    ]
}

/// One row of the **Units** section's own "Price Sources" subsection (issue #189's own README
/// revision) -- a table about the *relationship* to an external price source, not the unit
/// record itself, so it's keyed by the unit's `name`, not its `code` (the README's own "the one
/// place the unit is referenced by name rather than code" callout). Only `btc`/`vas` appear here
/// -- `aud` has no price-source relationship of this kind (its own SOURCE column value, "USD",
/// is a reference-currency note on the Units table itself, not an external feed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceRow {
    pub unit_name: String,
    pub source: String,
    pub last_updated: String,
}

/// The mockup's own two seeded rows, in its own order.
pub fn default_price_sources() -> Vec<PriceSourceRow> {
    vec![
        PriceSourceRow {
            unit_name: "Bitcoin".to_string(),
            source: "CoinGecko \u{2014} auto, every 15 min".to_string(),
            last_updated: "5 minutes ago".to_string(),
        },
        PriceSourceRow {
            unit_name: "Vanguard Aus Shares".to_string(),
            source: "Manual entry".to_string(),
            last_updated: "\u{2014}".to_string(),
        },
    ]
}

/// Appends a unit from the Add unit dialog. It has no price-source integration yet, so its source
/// is "Manual entry", and it can't be the base or default unit: nothing lets a user change which
/// one that is. Returns the row it appended.
pub fn add_unit(units: &mut Vec<UnitRow>, code: String, name: String, kind: UnitKind) -> UnitRow {
    let row = UnitRow {
        id: UnitId::new(),
        code,
        name,
        kind,
        source: MANUAL_SOURCE.to_string(),
        is_base: false,
        is_default: false,
    };
    units.push(row.clone());
    row
}

/// Rewrites the row `id` from the Edit unit dialog. The dialog has no field for the source
/// or the base and default flags, so those are kept from the row being edited. Returns the row it
/// wrote, or `None` when no row has `id`.
pub fn edit_unit(
    units: &mut [UnitRow],
    id: UnitId,
    code: String,
    name: String,
    kind: UnitKind,
) -> Option<&UnitRow> {
    let existing = units.iter_mut().find(|row| row.id == id)?;
    *existing = UnitRow {
        id,
        code,
        name,
        kind,
        source: existing.source.clone(),
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

/// The SOURCE a dialog-created unit gets, the same fallback the seeded `vas` row uses.
const MANUAL_SOURCE: &str = "Manual entry";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_units_matches_the_mockups_own_three_seeded_rows() {
        let codes: Vec<_> = default_units().into_iter().map(|unit| unit.code).collect();
        assert_eq!(codes, vec!["aud", "btc", "vas"]);
    }

    #[test]
    fn only_aud_carries_the_base_and_default_flags() {
        let units = default_units();
        let flagged: Vec<_> = units
            .iter()
            .filter(|unit| unit.is_base || unit.is_default)
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(flagged, vec!["aud"]);
    }

    #[test]
    fn default_price_sources_matches_the_mockups_own_two_seeded_rows() {
        let names: Vec<_> = default_price_sources()
            .into_iter()
            .map(|row| row.unit_name)
            .collect();
        assert_eq!(names, vec!["Bitcoin", "Vanguard Aus Shares"]);
    }

    #[test]
    fn a_dialog_added_unit_is_manual_and_neither_base_nor_default() {
        let mut units = default_units();
        let added = add_unit(
            &mut units,
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
        );
        assert_eq!(added.code, "eth");
        assert_eq!(added.source, "Manual entry");
        assert!(!added.is_base && !added.is_default);
        assert_eq!(units.len(), 4);
    }

    #[test]
    fn editing_keeps_the_source_and_the_base_and_default_flags() {
        let mut units = default_units();
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_AUD,
            "aud2".into(),
            "Aussie".into(),
            UnitKind::Fiat,
        )
        .expect("aud is seeded");
        assert_eq!(edited.code, "aud2");
        assert_eq!(edited.source, "USD");
        assert!(edited.is_base && edited.is_default);
    }

    #[test]
    fn a_unit_kind_survives_an_edit_round_trip() {
        let mut units = default_units();
        for kind in UnitKind::all() {
            let edited = edit_unit(
                &mut units,
                UnitId::SEED_BTC,
                "btc".into(),
                "Bitcoin".into(),
                *kind,
            )
            .expect("btc is seeded");
            assert_eq!(edited.kind, *kind);
        }
    }

    #[test]
    fn editing_an_unknown_id_changes_nothing() {
        let mut units = default_units();
        let before = units.clone();
        assert!(
            edit_unit(
                &mut units,
                UnitId::new(),
                "x".into(),
                "x".into(),
                UnitKind::Fiat
            )
            .is_none()
        );
        assert_eq!(units, before);
    }

    #[test]
    fn removing_takes_the_row_with_that_id() {
        let mut units = default_units();
        assert!(remove_unit(&mut units, UnitId::new()).is_none());
        assert_eq!(units.len(), 3);
        let removed = remove_unit(&mut units, UnitId::SEED_BTC).expect("btc is seeded");
        assert_eq!(removed.code, "btc");
        assert_eq!(units.len(), 2);
    }

    #[test]
    fn editing_the_code_keeps_the_id() {
        let mut units = default_units();
        let edited = edit_unit(
            &mut units,
            UnitId::SEED_BTC,
            "xbt".into(),
            "Bitcoin".into(),
            UnitKind::Crypto,
        )
        .expect("btc is seeded");
        assert_eq!(edited.id, UnitId::SEED_BTC);
    }
}
