use lib_core::UnitId;

use crate::protocols::{PriceSourceRepository, UnitRepository};
use crate::{Error, PriceLink, Result, UnitKind, UnitRow, price_source_exists};

/// Edit a Unit's code, name, Kind and Price Source link. The id stays, so editing the code leaves
/// whatever names the Unit on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditUnit {
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub link: PriceLink,
}

impl EditUnit {
    /// Returns the Unit as written.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        price_sources: &dyn PriceSourceRepository,
    ) -> Result<UnitRow> {
        let (id, kind) = (self.id, self.kind);
        let outcome = self.apply(units, price_sources);
        match &outcome {
            Ok(_) => tracing::info!(unit_id = %id, ?kind, "Edited a Unit"),
            Err(refusal) => {
                tracing::info!(unit_id = %id, ?kind, %refusal, "Refused to edit a Unit");
            }
        }
        outcome
    }

    fn apply(
        self,
        units: &mut dyn UnitRepository,
        price_sources: &dyn PriceSourceRepository,
    ) -> Result<UnitRow> {
        let mut unit = units.unit(self.id).ok_or(Error::UnitGone)?;
        if !price_source_exists(&price_sources.price_source_records(), self.link.source_id) {
            return Err(Error::PriceSourceGone);
        }
        if units.unit(self.link.quote).is_none() {
            return Err(Error::QuoteGone);
        }
        unit.edit(self.code, self.name, self.kind, self.link);
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use lib_core::PriceSourceId;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::DeactivateUnit;

    fn edit(id: UnitId, link: PriceLink) -> EditUnit {
        EditUnit {
            id,
            code: "xbt".into(),
            name: "Bitcoin core".into(),
            kind: UnitKind::Crypto,
            link,
        }
    }

    #[test]
    fn editing_a_unit_writes_its_fields_and_keeps_its_id_and_price_in_the_same_quote() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let btc = ports.units.unit(UnitId::SEED_BTC).unwrap();

        let edited = edit(UnitId::SEED_BTC, btc.link.clone())
            .run(&mut ports.units, &ports.price_sources)
            .unwrap();

        assert_eq!(edited.id, UnitId::SEED_BTC);
        assert_eq!(
            (edited.code.as_str(), edited.name.as_str()),
            ("xbt", "Bitcoin core")
        );
        assert_eq!(edited.prices, btc.prices);
        assert!(!edited.prices.is_empty());
        assert_eq!(ports.units.unit(UnitId::SEED_BTC), Some(edited));
    }

    #[test]
    fn changing_the_quote_unit_clears_the_weekly_prices() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let mut link = ports.units.unit(UnitId::SEED_BTC).unwrap().link;
        link.quote = UnitId::SEED_VAS;

        let edited = edit(UnitId::SEED_BTC, link)
            .run(&mut ports.units, &ports.price_sources)
            .unwrap();

        assert!(edited.prices.is_empty());
    }

    #[test]
    fn editing_a_deactivated_unit_keeps_it_deactivated() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        DeactivateUnit(UnitId::SEED_BTC)
            .run(&mut ports.units, &ports.base_unit)
            .unwrap();
        let link = ports.units.unit(UnitId::SEED_BTC).unwrap().link;

        let edited = edit(UnitId::SEED_BTC, link)
            .run(&mut ports.units, &ports.price_sources)
            .unwrap();

        assert!(!edited.is_active);
    }

    #[test]
    fn editing_a_unit_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        assert_eq!(
            edit(UnitId::new(), PriceLink::manual(UnitId::SEED_AUD))
                .run(&mut ports.units, &ports.price_sources),
            Err(Error::UnitGone)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_link_to_a_price_source_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();
        let gone = PriceLink {
            source_id: PriceSourceId::new(),
            symbol: "x".into(),
            quote: UnitId::SEED_AUD,
        };

        assert_eq!(
            edit(UnitId::SEED_BTC, gone).run(&mut ports.units, &ports.price_sources),
            Err(Error::PriceSourceGone)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_quote_unit_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();
        let mut link = ports.units.unit(UnitId::SEED_BTC).unwrap().link;
        link.quote = UnitId::new();

        assert_eq!(
            edit(UnitId::SEED_BTC, link).run(&mut ports.units, &ports.price_sources),
            Err(Error::QuoteGone)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_unit_can_be_quoted_in_itself() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let edited = edit(UnitId::SEED_BTC, PriceLink::manual(UnitId::SEED_BTC))
            .run(&mut ports.units, &ports.price_sources)
            .expect("a Unit may quote itself");

        assert_eq!(edited.link.quote, UnitId::SEED_BTC);
    }
}
