use lib_core::UnitId;

use crate::protocols::{PriceSourceRepository, UnitRepository};
use crate::{PriceLink, UnitKind, UnitRow, price_source_exists};

/// Add a Unit. The form has already checked the fields, so only a link naming something gone
/// refuses it. The caller picks the id, so a Unit can be quoted in itself, as the first one is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUnit {
    pub id: UnitId,
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub link: PriceLink,
}

/// Both only happen when what the link names was removed while the Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AddUnitRefusal {
    #[error("price source not found")]
    PriceSourceGone,
    /// The quote Unit isn't on the Ledger; storage keeps every quote on a Unit.
    #[error("quote unit not found")]
    QuoteGone,
}

impl AddUnit {
    /// Returns the Unit added.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        price_sources: &dyn PriceSourceRepository,
    ) -> Result<UnitRow, AddUnitRefusal> {
        if !price_source_exists(&price_sources.price_source_records(), self.link.source_id) {
            return Err(AddUnitRefusal::PriceSourceGone);
        }
        if self.link.quote != self.id && units.unit(self.link.quote).is_none() {
            return Err(AddUnitRefusal::QuoteGone);
        }
        let unit = UnitRow {
            id: self.id,
            ..UnitRow::new(self.code, self.name, self.kind, self.link)
        };
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use lib_core::PriceSourceId;

    use super::*;
    use crate::COINGECKO;
    use crate::fake::InMemoryPorts;

    fn add(link: PriceLink) -> AddUnit {
        AddUnit {
            id: UnitId::new(),
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
            Err(AddUnitRefusal::PriceSourceGone)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_quote_unit_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        assert_eq!(
            add(PriceLink::manual(UnitId::new())).run(&mut ports.units, &ports.price_sources),
            Err(AddUnitRefusal::QuoteGone)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_unit_quoted_in_itself_is_added() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units.units.clear();
        let id = UnitId::new();

        let added = AddUnit {
            id,
            ..add(PriceLink::manual(id))
        }
        .run(&mut ports.units, &ports.price_sources)
        .expect("a Unit may quote itself");

        assert_eq!((added.id, added.link.quote), (id, id));
        assert_eq!(ports.units.units, [added]);
    }
}
