use lib_core::PriceSourceId;

use crate::protocols::PriceSourceRepository;
use crate::{CustomSource, Endpoint, PriceSourceRecord, Updates};

/// Add a Custom Price Source. The form has already checked it, so an add is never refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPriceSource {
    pub name: String,
    pub endpoint: Endpoint,
    pub updates: Updates,
}

impl AddPriceSource {
    /// Returns the source added.
    pub fn run(self, price_sources: &mut dyn PriceSourceRepository) -> CustomSource {
        let source = CustomSource {
            id: PriceSourceId::new(),
            name: self.name,
            endpoint: self.endpoint,
            updates: self.updates,
        };
        price_sources.upsert_price_source(PriceSourceRecord::Custom(source.clone()));
        source
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::{KeyPlacement, PriceSourceAuth, UnitKind, price_source};

    #[test]
    fn adding_a_custom_source_stores_it_for_every_unit_kind() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let endpoint = Endpoint {
            address: "https://broker.example".to_string().into(),
            request: "/price?id={symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "key".to_string().into(),
            },
        };

        let added = AddPriceSource {
            name: "Broker".into(),
            endpoint: endpoint.clone(),
            updates: Updates::Daily,
        }
        .run(&mut ports.price_sources);

        let records = ports.price_sources.price_source_records();
        assert_eq!(records, vec![PriceSourceRecord::Custom(added.clone())]);
        let listed = price_source(&records, "Manual entry", added.id).unwrap();
        assert_eq!((listed.name, listed.updates), ("Broker", Updates::Daily));
        assert_eq!(listed.endpoint(), Some(&endpoint));
        assert!(UnitKind::all().iter().all(|kind| listed.prices(*kind)));
    }
}
