use lib_core::UnitId;

use crate::fetching::{PriceFetching, RequestId, Wanted};
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
    /// Returns the request queued, which goes out at once unless the source's call rate or a
    /// `Retry-After` holds it. Refused for a Unit on Manual entry or deactivated, which is never
    /// fetched, and for a Unit whose source or quote Unit is gone. A source needing a key this
    /// Client doesn't have sends nothing: its round and the Unit fail Key missing at `now`, and
    /// the refusal says so.
    pub fn run(
        self,
        ports: &dyn UnitsPorts,
        fetching: &mut PriceFetching,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<RequestId> {
        let outcome = self.apply(ports, fetching, now);
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

    fn apply(
        self,
        ports: &dyn UnitsPorts,
        fetching: &mut PriceFetching,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<RequestId> {
        let unit = ports.units().unit(self.id).ok_or(Error::UnitGone)?;
        if !unit.is_active {
            return Err(Error::NotFetched);
        }
        let catalogue = ports.source_catalogue();
        let source = catalogue
            .source(unit.link.source_id)
            .ok_or(Error::PriceSourceGone)?;
        if source.endpoint().is_none() {
            return Err(Error::NotFetched);
        }
        let quote = ports
            .units()
            .unit(unit.link.quote)
            .ok_or(Error::QuoteGone)?;
        let wanted = [Wanted {
            unit: unit.id,
            link: unit.link,
            quote: quote.code,
        }];
        // A round of one, asked for, so its outcome is told; a source with a batch request asks
        // through it, as a batch of one.
        let key = ports.keys().key(source.id);
        fetching
            .start_round(&source, &wanted, key.as_ref(), true, now)?
            .into_iter()
            .next()
            .ok_or(Error::NotFetched)
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

        let request = FetchNow { id }
            .run(&ports, &mut fetching, Utc::now())
            .expect("sent");

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
            FetchNow { id }.run(&ports, &mut fetching, Utc::now()),
            Err(Error::NotFetched)
        );
        assert_eq!(
            FetchNow {
                id: UnitId::SEED_VAS
            }
            .run(&ports, &mut fetching, Utc::now()),
            Err(Error::NotFetched)
        );
        assert_eq!(
            FetchNow { id: UnitId::new() }.run(&ports, &mut fetching, Utc::now()),
            Err(Error::UnitGone)
        );
        assert!(transport.requests().is_empty());
        assert_eq!(fetching.in_flight(), 0);
    }
}
