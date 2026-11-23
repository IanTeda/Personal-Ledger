use chrono::{DateTime, Utc};
use lib_core::PriceSourceId;

use super::FetchSource;
use crate::Result;
use crate::fetching::PriceFetching;
use crate::protocols::UnitsPorts;

/// Ask every fetching Price Source for its Units' prices straight away: one [`FetchSource`] round
/// per source, in the order the sources list. A source on Manual entry is not asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchAll;

impl FetchAll {
    /// Returns each source asked and what came of it, as [`FetchSource::run`] returns it. A source
    /// with a round already in flight is refused and the others still go out.
    pub fn run(
        self,
        ports: &dyn UnitsPorts,
        fetching: &mut PriceFetching,
        now: DateTime<Utc>,
    ) -> Vec<(PriceSourceId, Result<usize>)> {
        let ids: Vec<_> = ports
            .source_catalogue()
            .sources()
            .iter()
            .filter(|source| source.endpoint().is_some())
            .map(|source| source.id)
            .collect();
        ids.into_iter()
            .map(|id| (id, FetchSource { id }.run(ports, fetching, now)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Utc;
    use lib_core::UnitId;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::interactor::FetchSource;
    use crate::{COINGECKO, Error, FRANKFURTER, Precision, PriceLink, UnitKind, UnitRow};

    fn unit(code: &str, source_id: PriceSourceId, symbol: &str) -> UnitRow {
        UnitRow::new(
            code.into(),
            code.into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id,
                symbol: symbol.into(),
                quote: UnitId::SEED_AUD,
            },
        )
    }

    #[test]
    fn every_fetching_source_gets_its_own_round_and_one_in_flight_is_refused() {
        let mut ports = InMemoryPorts::sample(Utc::now());
        ports.units.units.extend([
            unit("EUR", FRANKFURTER, "EUR"),
            unit("ETH", COINGECKO, "ethereum"),
        ]);
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        FetchSource { id: FRANKFURTER }
            .run(&ports, &mut fetching, Utc::now())
            .expect("sent");

        let outcomes = FetchAll.run(&ports, &mut fetching, Utc::now());

        let of = |id| {
            outcomes
                .iter()
                .find(|(source, _)| *source == id)
                .map(|(_, outcome)| *outcome)
        };
        assert_eq!(of(FRANKFURTER), Some(Err(Error::AlreadyFetching)));
        assert_eq!(of(COINGECKO), Some(Ok(1)), "btc and eth in one batch");
        assert!(fetching.source_asked_for(COINGECKO));
        assert_eq!(
            transport
                .urls()
                .into_iter()
                .filter(|url| url.contains("coingecko"))
                .collect::<Vec<_>>(),
            [
                "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum&vs_currencies=aud"
            ]
        );
    }

    #[test]
    fn frankfurter_and_yahoo_still_ask_one_call_per_unit() {
        let mut ports = InMemoryPorts::sample(Utc::now());
        ports.units.units.extend([
            unit("EUR", FRANKFURTER, "EUR"),
            unit("USD", FRANKFURTER, "USD"),
            unit("BHP", crate::YAHOO_FINANCE, "BHP.AX"),
            unit("CBA", crate::YAHOO_FINANCE, "CBA.AX"),
        ]);
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));

        let outcomes = FetchAll.run(&ports, &mut fetching, Utc::now());

        let of = |id| {
            outcomes
                .iter()
                .find(|(source, _)| *source == id)
                .map(|(_, outcome)| *outcome)
        };
        assert_eq!(of(FRANKFURTER), Some(Ok(2)));
        assert_eq!(of(crate::YAHOO_FINANCE), Some(Ok(2)));
        assert_eq!(
            transport.urls().len(),
            5,
            "one CoinGecko batch, four single calls"
        );
    }
}
