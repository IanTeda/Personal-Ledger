use lib_core::PriceSourceId;

use crate::protocols::{PriceSourceRepository, UnitRepository};
use crate::{CustomSource, Error, MANUAL_ENTRY, PriceSourceRecord, Result, built_in};

/// Delete a Custom Price Source, moving every Unit on it to Manual entry so none links to a source
/// that doesn't exist. Each moved Unit keeps its symbol and last price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePriceSource(pub PriceSourceId);

/// What deleting a Custom source did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceDeleted {
    pub source: CustomSource,
    /// How many Units moved to Manual entry.
    pub moved: usize,
}

impl DeletePriceSource {
    pub fn run(
        self,
        price_sources: &mut dyn PriceSourceRepository,
        units: &mut dyn UnitRepository,
    ) -> Result<PriceSourceDeleted> {
        let outcome = self.apply(price_sources, units);
        match &outcome {
            Ok(deleted) => tracing::info!(
                price_source_id = %self.0,
                units_moved_to_manual = deleted.moved,
                "Deleted a Price Source"
            ),
            Err(refusal) => {
                tracing::info!(price_source_id = %self.0, %refusal, "Refused to delete a Price Source");
            }
        }
        outcome
    }

    fn apply(
        self,
        price_sources: &mut dyn PriceSourceRepository,
        units: &mut dyn UnitRepository,
    ) -> Result<PriceSourceDeleted> {
        let id = self.0;
        if id == MANUAL_ENTRY {
            return Err(Error::ChangeManualEntry);
        }
        if built_in(id).is_some() {
            return Err(Error::DeleteBuiltIn);
        }
        let source = price_sources
            .price_source_records()
            .into_iter()
            .find_map(|record| match record {
                PriceSourceRecord::Custom(source) if source.id == id => Some(source),
                _ => None,
            })
            .ok_or(Error::PriceSourceGone)?;
        let mut moved = 0;
        for mut unit in units.units() {
            if unit.link.source_id == id {
                unit.move_to_manual();
                units.upsert_unit(unit);
                moved += 1;
            }
        }
        price_sources.delete_price_source(id);
        Ok(PriceSourceDeleted { source, moved })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use lib_core::UnitId;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::{AddPriceSource, EditPriceSource};
    use crate::{COINGECKO, Endpoint, FRANKFURTER, PriceSourceAuth, Updates};

    fn add_custom(ports: &mut InMemoryPorts) -> CustomSource {
        AddPriceSource {
            name: "Broker".into(),
            endpoint: Endpoint {
                address: "https://broker.example".to_string().into(),
                request: "/price?id={symbol}".to_string().into(),
                price_path: "/price".to_string().into(),
                auth: PriceSourceAuth::None,
            },
            updates: Updates::Daily,
        }
        .run(&mut ports.price_sources)
    }

    #[test]
    fn deleting_a_custom_source_moves_its_units_to_manual_entry_with_their_weekly_prices() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let source = add_custom(&mut ports);
        ports.units.units[1].link.source_id = source.id;
        let btc = ports.units.units[1].clone();

        let deleted = DeletePriceSource(source.id)
            .run(&mut ports.price_sources, &mut ports.units)
            .unwrap();

        assert_eq!(deleted, PriceSourceDeleted { source, moved: 1 });
        let moved = ports.units.unit(UnitId::SEED_BTC).unwrap();
        assert_eq!(moved.link.source_id, MANUAL_ENTRY);
        assert_eq!(moved.link.symbol, btc.link.symbol);
        assert_eq!(moved.prices, btc.prices);
        assert!(!moved.prices.is_empty());
        assert!(ports.price_sources.price_source_records().is_empty());
    }

    #[test]
    fn deleting_a_custom_source_no_unit_uses_moves_nothing() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let units = ports.units.clone();
        let source = add_custom(&mut ports);

        let deleted = DeletePriceSource(source.id)
            .run(&mut ports.price_sources, &mut ports.units)
            .unwrap();

        assert_eq!(deleted.moved, 0);
        assert_eq!(ports.units, units);
    }

    #[test]
    fn deleting_a_built_in_manual_entry_or_a_gone_source_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        EditPriceSource {
            id: COINGECKO,
            name: "Coins".into(),
            updates: Updates::Daily,
            endpoint: None,
        }
        .run(&mut ports.price_sources)
        .unwrap();
        let before = ports.clone();

        for (id, refusal) in [
            (COINGECKO, Error::DeleteBuiltIn),
            (FRANKFURTER, Error::DeleteBuiltIn),
            (MANUAL_ENTRY, Error::ChangeManualEntry),
            (PriceSourceId::new(), Error::PriceSourceGone),
        ] {
            assert_eq!(
                DeletePriceSource(id).run(&mut ports.price_sources, &mut ports.units),
                Err(refusal)
            );
        }
        assert_eq!(ports, before);
    }
}
