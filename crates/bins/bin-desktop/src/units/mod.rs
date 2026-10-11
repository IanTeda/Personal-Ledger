//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with their price sources. `gpui`-free; Settings' Units page is only where they're edited.

pub(crate) mod form;
pub(crate) mod store;

pub use lib_units::{PriceSourceRow, UnitKind, UnitRow};
#[cfg(test)]
pub use lib_units::{default_price_sources, default_units};
pub use store::UnitsStore;

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
}
