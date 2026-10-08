//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`CONTEXT.md`'s
//! Unit), with their price sources. `gpui`-free; Settings' Units page is only where they're edited.

pub(crate) mod form;

/// One row of the **Units** section's table (`docs/ux/desktop/16-settings/README.md`'s "2a resting
/// state" markup: CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS columns as of issue #189, up
/// from CODE / NAME / TYPE). Owned `String` fields, not `&'static str` --
/// issue #184's own Add unit dialog is this crate's first real typed-text input, so a row can
/// now hold text a person actually typed, not just compile-time dummy data. `kind` stays a
/// free-form string rather than [`UnitKind`]: the table just displays it, and legacy seeded rows
/// ("crypto", "etf") don't match any `UnitKind` label anyway. Issue #185's Edit dialog reads it
/// back through [`UnitKind::from_label`] to pre-fill the Type selector, but the row itself is
/// never typed narrower than a string -- `UnitKind` only governs the dialogs' own selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    pub code: String,
    pub name: String,
    pub kind: String,
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
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: "currency".to_string(),
            source: "USD".to_string(),
            is_base: true,
            is_default: true,
        },
        UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        },
        UnitRow {
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: "etf".to_string(),
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

/// The Add/Edit unit dialogs' own "Type" selector (`docs/ux/desktop/16-settings/README.md`'s "2b —
/// Add unit": "Type (select: currency / cryptocurrency / custom)") -- rendered as a segmented
/// control (like [`TracingLevel`]), not a real `<select>` dropdown, same reasoning as every
/// other "pick one of a few options" control this map has built: dropdown-open behaviour has no
/// precedent in this crate yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitKind {
    #[default]
    Currency,
    Cryptocurrency,
    Custom,
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [Self::Currency, Self::Cryptocurrency, Self::Custom];

    pub fn label(self) -> String {
        match self {
            Self::Currency => crate::msg::desktop_settings_unit_kind_currency(),
            Self::Cryptocurrency => crate::msg::desktop_settings_unit_kind_cryptocurrency(),
            Self::Custom => crate::msg::desktop_settings_unit_kind_custom(),
        }
    }

    /// Maps a [`UnitRow::kind`] free-form string back to the closest `UnitKind` (issue #185's
    /// own Edit unit dialog: pre-filling the Type selector from an existing row). Falls back to
    /// `Custom` on no exact match -- `Custom` is the catch-all category by definition, and a
    /// legacy seeded row's own free-form label ("crypto", "etf") was never guaranteed to match
    /// one of these three canonical options in the first place (see [`UnitRow`]'s own doc).
    pub fn from_label(label: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|kind| kind.label() == label)
            .unwrap_or(Self::Custom)
    }
}

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
    fn unit_kind_defaults_to_currency() {
        assert_eq!(UnitKind::default(), UnitKind::Currency);
    }

    #[test]
    fn unit_kind_from_label_matches_exactly_or_falls_back_to_custom() {
        crate::locale::init_for_tests();
        assert_eq!(UnitKind::from_label("currency"), UnitKind::Currency);
        assert_eq!(
            UnitKind::from_label("cryptocurrency"),
            UnitKind::Cryptocurrency
        );
        assert_eq!(UnitKind::from_label("crypto"), UnitKind::Custom);
        assert_eq!(UnitKind::from_label("etf"), UnitKind::Custom);
    }
}
