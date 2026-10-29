//! A Unit's Precision (`GLOSSARY.md`): the decimal places its amounts are shown and entered to.

use lib_core::UnitKind;

/// How many decimal places a Unit's amounts are shown and entered to, from 0 to [`Self::MAX`]. A
/// price shows at its quote Unit's Precision, since it is an amount in that Unit. It changes only
/// how amounts read: a stored amount keeps its own places, and display rounds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Precision(u8);

impl Precision {
    /// The most places a Unit takes. `Money` holds any scale, so this is the finest sub-unit in
    /// common use, Ether's wei, not a storage limit.
    pub const MAX: u8 = 18;

    /// `places` as a Precision, or `None` above [`Self::MAX`].
    pub const fn new(places: u8) -> Option<Self> {
        if places <= Self::MAX {
            Some(Self(places))
        } else {
            None
        }
    }

    /// What a new Unit of `kind` starts on: a satoshi's eight places for Crypto, cents for the rest.
    pub const fn of_kind(kind: UnitKind) -> Self {
        match kind {
            UnitKind::Crypto => Self(8),
            UnitKind::Currency
            | UnitKind::Equity
            | UnitKind::FixedIncome
            | UnitKind::Commodity
            | UnitKind::Custom => Self(2),
        }
    }

    /// Typed text as a Precision: a whole number from 0 to [`Self::MAX`], or `None`.
    pub fn parse(text: &str) -> Option<Self> {
        text.trim().parse().ok().and_then(Self::new)
    }

    /// The places, as stored.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// The places, as an amount style takes them.
    pub fn places(self) -> u32 {
        u32::from(self.0)
    }
}

impl std::fmt::Display for Precision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_unit_takes_eight_places_for_crypto_and_two_for_every_other_kind() {
        let places: Vec<_> = UnitKind::all()
            .iter()
            .map(|kind| (*kind, Precision::of_kind(*kind).places()))
            .collect();

        assert_eq!(
            places,
            [
                (UnitKind::Currency, 2),
                (UnitKind::Crypto, 8),
                (UnitKind::Equity, 2),
                (UnitKind::FixedIncome, 2),
                (UnitKind::Commodity, 2),
                (UnitKind::Custom, 2),
            ]
        );
    }

    #[test]
    fn typed_text_is_a_precision_only_as_a_whole_number_from_0_to_18() {
        assert_eq!(Precision::parse(" 8 ").map(Precision::places), Some(8));
        assert_eq!(Precision::parse("0").map(Precision::places), Some(0));
        assert_eq!(Precision::parse("18").map(Precision::places), Some(18));
        for refused in ["", "19", "-1", "2.5", "two", "300"] {
            assert_eq!(Precision::parse(refused), None, "{refused:?}");
        }
        assert_eq!(Precision::new(19), None);
    }
}
