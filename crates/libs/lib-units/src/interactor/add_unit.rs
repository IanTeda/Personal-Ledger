use crate::interactor::PriceSourceGone;
use crate::protocols::{PriceSourceRepository, UnitRepository};
use crate::{PriceLink, UnitKind, UnitRow, price_source_exists};

/// Add a Unit. The form has already checked the fields, so only a gone Price Source refuses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUnit {
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub link: PriceLink,
}

impl AddUnit {
    /// Returns the Unit added.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        price_sources: &dyn PriceSourceRepository,
    ) -> Result<UnitRow, PriceSourceGone> {
        if !price_source_exists(&price_sources.price_source_records(), self.link.source_id) {
            return Err(PriceSourceGone);
        }
        let unit = UnitRow::new(self.code, self.name, self.kind, self.link);
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use lib_core::{PriceSourceId, UnitId};

    use super::*;
    use crate::COINGECKO;
    use crate::fake::InMemoryPorts;

    fn add(link: PriceLink) -> AddUnit {
        AddUnit {
            code: "eth".into(),
            name: "Ethereum".into(),
            kind: UnitKind::Crypto,
            link,
        }
    }

    #[test]
    fn adding_a_unit_stores_it_active_with_its_link_and_no_price() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let link = PriceLink {
            source_id: COINGECKO,
            symbol: "ethereum".into(),
            quote: UnitId::SEED_AUD,
        };

        let added = add(link.clone())
            .run(&mut ports.units, &ports.price_sources)
            .unwrap();

        assert_eq!(
            (added.code.as_str(), added.name.as_str()),
            ("eth", "Ethereum")
        );
        assert_eq!(added.link, link);
        assert_eq!(added.last_price, None);
        assert!(added.is_active);
        assert_eq!(ports.units.unit(added.id), Some(added));
        assert_eq!(ports.units.units.len(), 4);
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
            add(gone).run(&mut ports.units, &ports.price_sources),
            Err(PriceSourceGone)
        );
        assert_eq!(ports, before);
    }
}
