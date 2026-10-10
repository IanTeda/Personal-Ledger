//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`CONTEXT.md`'s
//! Unit), with their price sources. `gpui`-free; Settings' Units page is only where they're edited.

pub(crate) mod form;

pub use lib_units::{PriceSourceRow, UnitRow, default_price_sources, default_units};

/// The Add/Edit unit dialogs' own "Type" selector (`docs/ux/desktop-mockups/16-settings/README.md`'s "2b —
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
