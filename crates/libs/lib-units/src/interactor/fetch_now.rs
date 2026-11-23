use lib_core::UnitId;

use crate::fetching::{InFlight, PriceFetching, RequestId, price_request};
use crate::protocols::UnitsPorts;
use crate::{Error, Result};

/// Ask a Unit's Price Source for its price straight away, whatever the schedule says (`GLOSSARY.md`'s
/// Price Source, fetch now). Only sends: the price is written when the response comes back,
/// through [`crate::interactor::RecordFetchedPrices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchNow {
    pub id: UnitId,
}

impl FetchNow {
    /// Returns the request sent. Refused for a Unit on Manual entry or deactivated, which is never
    /// fetched, and for a Unit whose source or quote Unit is gone.
    pub fn run(self, ports: &dyn UnitsPorts, fetching: &mut PriceFetching) -> Result<RequestId> {
        let outcome = self.apply(ports, fetching);
        match &outcome {
            Ok(request) => {
                tracing::info!(unit_id = %self.id, request = request.0, "Fetching a Unit's price");
            }
            Err(refusal) => {
                tracing::info!(unit_id = %self.id, %refusal, "Refused to fetch a Unit's price");
            }
        }
        outcome
    }

    fn apply(self, ports: &dyn UnitsPorts, fetching: &mut PriceFetching) -> Result<RequestId> {
        let unit = ports.units().unit(self.id).ok_or(Error::UnitGone)?;
        if !unit.is_active {
            return Err(Error::NotFetched);
        }
        let catalogue = ports.source_catalogue();
        let source = catalogue
            .source(unit.link.source_id)
            .ok_or(Error::PriceSourceGone)?;
        let endpoint = source.endpoint().ok_or(Error::NotFetched)?;
        let quote = ports
            .units()
            .unit(unit.link.quote)
            .ok_or(Error::QuoteGone)?;
        let (request, price_path) = price_request(endpoint, &unit.link.symbol, &quote.code);
        Ok(fetching.send(
            request,
            InFlight {
                unit: unit.id,
                link: unit.link,
                price_path,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Utc;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::{FRANKFURTER, Precision, PriceLink, UnitKind, UnitRow};

    fn eur(ports: &mut InMemoryPorts) -> UnitId {
        let unit = UnitRow::new(
            "EUR".into(),
            "Euro".into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id: FRANKFURTER,
                symbol: "EUR".into(),
                quote: UnitId::SEED_AUD,
            },
        );
        let id = unit.id;
        ports.units.units.push(unit);
        id
    }

    #[test]
    fn a_unit_on_frankfurter_sends_frankfurters_v2_request_and_is_in_flight() {
        let mut ports = InMemoryPorts::sample(Utc::now());
        let id = eur(&mut ports);
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));

        let request = FetchNow { id }.run(&ports, &mut fetching).expect("sent");

        assert_eq!(
            transport.urls(),
            ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
        );
        assert_eq!(transport.requests()[0].0, request);
        assert!(fetching.fetching(id));
    }

    #[test]
    fn a_manual_entry_or_deactivated_unit_is_refused_and_sends_nothing() {
        let mut ports = InMemoryPorts::sample(Utc::now());
        let id = eur(&mut ports);
        ports.units.units.last_mut().expect("eur").is_active = false;
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));

        assert_eq!(
            FetchNow { id }.run(&ports, &mut fetching),
            Err(Error::NotFetched)
        );
        assert_eq!(
            FetchNow {
                id: UnitId::SEED_VAS
            }
            .run(&ports, &mut fetching),
            Err(Error::NotFetched)
        );
        assert_eq!(
            FetchNow { id: UnitId::new() }.run(&ports, &mut fetching),
            Err(Error::UnitGone)
        );
        assert!(transport.requests().is_empty());
        assert_eq!(fetching.in_flight(), 0);
    }
}
