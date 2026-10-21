use lib_core::PriceSourceId;

use crate::protocols::PriceSourceRepository;
use crate::{BuiltInOverride, Endpoint, MANUAL_ENTRY, PriceSourceRecord, Updates, built_in};

/// Rename and retime a Price Source. A Custom source also takes a new endpoint when `endpoint` is
/// given; a built-in stores only the name and Updates, since its endpoint is code (ADR-0043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPriceSource {
    pub id: PriceSourceId,
    pub name: String,
    pub updates: Updates,
    /// A Custom source's new endpoint, or `None` to keep it. Always `None` for a built-in.
    pub endpoint: Option<Endpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditPriceSourceRefusal {
    /// No source has the id, which only happens when it was deleted while its Dialog was open.
    #[error("price source not found")]
    Gone,
    /// Manual entry has nothing to edit.
    #[error("manual entry can't be edited")]
    ManualEntry,
    /// A built-in's address, request, price path and auth are fixed in code.
    #[error("a built-in price source's endpoint can't be edited")]
    BuiltInEndpoint,
}

impl EditPriceSource {
    /// Returns the record written.
    pub fn run(
        self,
        price_sources: &mut dyn PriceSourceRepository,
    ) -> Result<PriceSourceRecord, EditPriceSourceRefusal> {
        if self.id == MANUAL_ENTRY {
            return Err(EditPriceSourceRefusal::ManualEntry);
        }
        let record = if built_in(self.id).is_some() {
            if self.endpoint.is_some() {
                return Err(EditPriceSourceRefusal::BuiltInEndpoint);
            }
            PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: self.id,
                name: self.name,
                updates: self.updates,
            })
        } else {
            let mut source = price_sources
                .price_source_records()
                .into_iter()
                .find_map(|record| match record {
                    PriceSourceRecord::Custom(source) if source.id == self.id => Some(source),
                    _ => None,
                })
                .ok_or(EditPriceSourceRefusal::Gone)?;
            source.name = self.name;
            source.updates = self.updates;
            if let Some(endpoint) = self.endpoint {
                source.endpoint = endpoint;
            }
            PriceSourceRecord::Custom(source)
        };
        price_sources.upsert_price_source(record.clone());
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::AddPriceSource;
    use crate::{COINGECKO, PriceSourceAuth, price_source};

    const MANUAL: &str = "Manual entry";

    fn endpoint(address: &str) -> Endpoint {
        Endpoint {
            address: address.to_string().into(),
            request: "/price?id={symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::None,
        }
    }

    fn edit(id: PriceSourceId, endpoint: Option<Endpoint>) -> EditPriceSource {
        EditPriceSource {
            id,
            name: "My broker".into(),
            updates: Updates::Hourly,
            endpoint,
        }
    }

    #[test]
    fn editing_a_custom_source_rewrites_its_name_updates_and_endpoint() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let added = AddPriceSource {
            name: "Broker".into(),
            endpoint: endpoint("https://broker.example"),
            updates: Updates::Daily,
        }
        .run(&mut ports.price_sources);

        edit(added.id, Some(endpoint("https://new.example")))
            .run(&mut ports.price_sources)
            .unwrap();

        let records = ports.price_sources.price_source_records();
        let source = price_source(&records, MANUAL, added.id).unwrap();
        assert_eq!(
            (source.name, source.updates),
            ("My broker", Updates::Hourly)
        );
        assert_eq!(source.endpoint(), Some(&endpoint("https://new.example")));
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn editing_a_custom_source_without_an_endpoint_keeps_its_endpoint() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let added = AddPriceSource {
            name: "Broker".into(),
            endpoint: endpoint("https://broker.example"),
            updates: Updates::Daily,
        }
        .run(&mut ports.price_sources);

        edit(added.id, None).run(&mut ports.price_sources).unwrap();

        let records = ports.price_sources.price_source_records();
        let source = price_source(&records, MANUAL, added.id).unwrap();
        assert_eq!(source.endpoint(), Some(&endpoint("https://broker.example")));
    }

    #[test]
    fn editing_a_built_in_stores_only_its_name_and_updates() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let written = edit(COINGECKO, None).run(&mut ports.price_sources).unwrap();

        let stored = PriceSourceRecord::BuiltIn(BuiltInOverride {
            id: COINGECKO,
            name: "My broker".into(),
            updates: Updates::Hourly,
        });
        assert_eq!(written, stored);
        assert_eq!(ports.price_sources.price_source_records(), vec![stored]);
        let records = ports.price_sources.price_source_records();
        let source = price_source(&records, MANUAL, COINGECKO).unwrap();
        assert_eq!(
            source.endpoint(),
            Some(&built_in(COINGECKO).unwrap().endpoint)
        );
    }

    #[test]
    fn a_built_ins_endpoint_manual_entry_and_a_gone_source_are_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        for (request, refusal) in [
            (
                edit(COINGECKO, Some(endpoint("https://x.example"))),
                EditPriceSourceRefusal::BuiltInEndpoint,
            ),
            (
                edit(MANUAL_ENTRY, None),
                EditPriceSourceRefusal::ManualEntry,
            ),
            (
                edit(PriceSourceId::new(), None),
                EditPriceSourceRefusal::Gone,
            ),
        ] {
            assert_eq!(request.run(&mut ports.price_sources), Err(refusal));
        }
        assert_eq!(ports, before);
    }
}
