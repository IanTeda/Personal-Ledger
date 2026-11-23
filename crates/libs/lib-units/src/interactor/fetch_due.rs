use chrono::{DateTime, NaiveDate, Utc};
use lib_core::PriceSourceId;

use crate::fetching::{
    FetchFailure, PriceFetching, SourceSettings, Wanted, is_due, key_missing, round_calls,
};
use crate::protocols::UnitsPorts;
use crate::units_by_code;

/// The scheduled check (ADR-0043's "where and when fetching runs"): fetch every due Unit, one
/// round per source. Run on Ledger open and every minute after. Only sends: each round is written
/// in one commit when its last response comes back, through
/// [`crate::interactor::RecordFetchedPrices`], and tells no one.
///
/// Never fetches a Unit on Manual entry or on a Manual only source, a deactivated Unit or the base
/// Unit, and leaves a source with a round already in flight alone, as it does a source or Unit
/// still backing off a failure and a source the Unit prices view is asking for finer prices. A
/// Unit this Client fetched on another link is due at once. A source needing a key this Client doesn't have fails its round Key missing without sending. A
/// source with a batch request asks for its due Units in batches; every call waits its turn
/// behind the calls a person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchDue {
    /// The Clock's day, whose week a due Unit needs a price in.
    pub today: NaiveDate,
    /// The moment a price's age is measured from.
    pub now: DateTime<Utc>,
}

impl FetchDue {
    /// Returns the sources a round was started on, in the order the Units list by code.
    pub fn run(self, ports: &dyn UnitsPorts, fetching: &mut PriceFetching) -> Vec<PriceSourceId> {
        let rounds = self.plan(ports, fetching);
        let mut started = Vec::with_capacity(rounds.len());
        let catalogue = ports.source_catalogue();
        for (source, wanted) in rounds {
            let Some(listed) = catalogue.source(source) else {
                continue;
            };
            let Some(endpoint) = listed.endpoint() else {
                continue;
            };
            let settings = SourceSettings::of(&listed);
            let key = ports.keys().key(source);
            if key_missing(endpoint, key.as_ref()) {
                tracing::warn!(price_source_id = %source, "A price source needs an API key this Client doesn't have");
                fetching.record_source(source, &settings, self.now, Some(FetchFailure::KeyMissing));
                for one in wanted {
                    fetching.record_unit(
                        one.unit,
                        one.link,
                        &settings,
                        self.now,
                        Some(FetchFailure::KeyMissing),
                    );
                }
                continue;
            }
            let requests = round_calls(endpoint, &wanted, key.as_ref(), &settings);
            tracing::info!(
                price_source_id = %source,
                units = wanted.len(),
                calls = requests.len(),
                "Fetching due Units' prices"
            );
            let round = fetching.open_round(&listed, false);
            for (request, sent) in requests {
                fetching.send(round, request, sent, self.now);
            }
            started.push(source);
        }
        if started.is_empty() {
            tracing::debug!("No Unit's price is due");
        }
        started
    }

    /// Every due Unit, grouped by source.
    fn plan(
        self,
        ports: &dyn UnitsPorts,
        fetching: &PriceFetching,
    ) -> Vec<(PriceSourceId, Vec<Wanted>)> {
        let base = ports.base_unit().base_unit();
        let catalogue = ports.source_catalogue();
        let units = ports.units().units();
        let mut rounds: Vec<(PriceSourceId, Vec<Wanted>)> = Vec::new();
        for unit in units_by_code(&units) {
            if !unit.is_active || unit.id == base {
                continue;
            }
            let Some(source) = catalogue.source(unit.link.source_id) else {
                continue;
            };
            if source.endpoint().is_none()
                || fetching.source_in_flight(source.id)
                || fetching.fine_in_flight(source.id)
                || fetching.source_backing_off(&source, self.now)
                || fetching.unit_backing_off(unit, &source, self.now)
                || !(fetching.relinked(unit) || is_due(unit, source.updates, self.today, self.now))
            {
                continue;
            }
            let Some(quote) = units.iter().find(|quote| quote.id == unit.link.quote) else {
                tracing::warn!(unit_id = %unit.id, "Skipped a due Unit whose quote Unit is gone");
                continue;
            };
            let wanted = Wanted {
                unit: unit.id,
                link: unit.link.clone(),
                quote: quote.code.clone(),
            };
            match rounds.iter_mut().find(|(id, _)| *id == source.id) {
                Some((_, due)) => due.push(wanted),
                None => rounds.push((source.id, vec![wanted])),
            }
        }
        rounds
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Duration;
    use lib_core::{UnitId, UnitKind};

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::{
        BuiltInOverride, COINGECKO, FRANKFURTER, MANUAL_ENTRY, Precision, PriceLink,
        PriceSourceRecord, UnitRow, Updates,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 21).expect("a real date")
    }

    fn now() -> DateTime<Utc> {
        today().and_hms_opt(9, 0, 0).expect("a real time").and_utc()
    }

    fn check() -> FetchDue {
        FetchDue {
            today: today(),
            now: now(),
        }
    }

    fn unit(code: &str, kind: UnitKind, source_id: PriceSourceId, symbol: &str) -> UnitRow {
        UnitRow::new(
            code.into(),
            code.into(),
            kind,
            Precision::of_kind(kind),
            PriceLink {
                source_id,
                symbol: symbol.into(),
                quote: UnitId::SEED_AUD,
            },
        )
    }

    struct Check {
        ports: InMemoryPorts,
        transport: FakeTransport,
        fetching: PriceFetching,
    }

    /// The sample, its btc priced five minutes before [`now`], with `units` added.
    fn on(units: Vec<UnitRow>) -> Check {
        let mut ports = InMemoryPorts::sample(now());
        ports.units.units.extend(units);
        let transport = FakeTransport::default();
        let fetching = PriceFetching::new(Rc::new(transport.clone()));
        Check {
            ports,
            transport,
            fetching,
        }
    }

    impl Check {
        fn run(&mut self) -> Vec<PriceSourceId> {
            check().run(&self.ports, &mut self.fetching)
        }
    }

    #[test]
    fn every_due_unit_is_fetched_one_round_per_source() {
        let mut check = on(vec![
            unit("USD", UnitKind::Currency, FRANKFURTER, "USD"),
            unit("ETH", UnitKind::Crypto, COINGECKO, "ethereum"),
            unit("EUR", UnitKind::Currency, FRANKFURTER, "EUR"),
        ]);

        let started = check.run();

        assert_eq!(started, [COINGECKO, FRANKFURTER]);
        assert_eq!(
            check.transport.urls(),
            [
                "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=aud",
                "https://api.frankfurter.dev/v2/rate/EUR/AUD",
                "https://api.frankfurter.dev/v2/rate/USD/AUD",
            ],
            "btc, priced five minutes ago on a 15-minute source, isn't due"
        );
        assert_eq!(check.fetching.in_flight(), 3);
    }

    #[test]
    fn coingeckos_due_units_go_out_in_one_batch() {
        let mut check = on(vec![
            unit("SOL", UnitKind::Crypto, COINGECKO, "solana"),
            unit("ETH", UnitKind::Crypto, COINGECKO, "ethereum"),
        ]);

        assert_eq!(check.run(), [COINGECKO]);

        assert_eq!(
            check.transport.urls(),
            ["https://api.coingecko.com/api/v3/simple/price?ids=ethereum,solana&vs_currencies=aud"],
            "btc, priced five minutes ago, isn't due"
        );
        assert_eq!(check.fetching.in_flight(), 1);
        assert!(check.fetching.fetching(check.ports.units.units[3].id));
    }

    #[test]
    fn a_round_queued_behind_the_call_rate_is_in_flight_so_the_next_check_skips_it() {
        let mut check = on(vec![unit("ETH", UnitKind::Crypto, COINGECKO, "ethereum")]);
        let btc = crate::interactor::FetchNow {
            id: UnitId::SEED_BTC,
        };
        // CoinGecko keyless takes 5 calls a minute: five asked-for fetches fill the window.
        for _ in 0..5 {
            btc.run(&check.ports, &mut check.fetching, now())
                .expect("sent");
            let answer = check.transport.answer_last(200, r#"{"bitcoin":{"aud":1}}"#);
            let landed = check.fetching.land(answer, now()).expect("a round of one");
            crate::interactor::RecordFetchedPrices { landed }.run(
                &mut check.ports.units,
                &mut check.fetching,
                today(),
                now(),
            );
        }

        assert_eq!(check.run(), [COINGECKO], "eth's round starts, queued");
        assert_eq!(check.transport.urls().len(), 5, "and nothing more goes out");

        assert!(check.run().is_empty(), "the queued round is in flight");
        assert_eq!(check.transport.urls().len(), 5);
    }

    #[test]
    fn a_source_with_a_round_in_flight_is_skipped_and_the_others_still_fetch() {
        let mut check = on(vec![unit("EUR", UnitKind::Currency, FRANKFURTER, "EUR")]);
        check.run();
        check
            .ports
            .units
            .units
            .push(unit("ETH", UnitKind::Crypto, COINGECKO, "ethereum"));

        let started = check.run();

        assert_eq!(started, [COINGECKO]);
        assert_eq!(check.transport.urls().len(), 2, "eur's round is still out");
    }

    #[test]
    fn a_unit_on_a_fresh_price_is_left_until_its_interval_passes() {
        let mut eur = unit("EUR", UnitKind::Currency, FRANKFURTER, "EUR");
        eur.record_price(
            "1.65".parse().expect("a decimal"),
            today(),
            now() - Duration::hours(23),
        );
        let mut check = on(vec![eur]);

        assert!(check.run().is_empty());
        assert!(check.transport.requests().is_empty());
    }

    #[test]
    fn manual_entry_deactivated_manual_only_and_base_units_are_never_fetched() {
        let mut inactive = unit("EUR", UnitKind::Currency, FRANKFURTER, "EUR");
        inactive.is_active = false;
        let mut check = on(vec![
            inactive,
            unit("GOLD", UnitKind::Commodity, MANUAL_ENTRY, ""),
            unit("ETH", UnitKind::Crypto, COINGECKO, "ethereum"),
        ]);
        check
            .ports
            .price_sources
            .records
            .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: COINGECKO,
                name: "CoinGecko".into(),
                updates: Updates::ManualOnly,
            }));
        // The base, aud, linked to a fetching source it can't price itself on.
        let base = check
            .ports
            .units
            .units
            .iter_mut()
            .find(|unit| unit.id == UnitId::SEED_AUD)
            .expect("aud");
        base.link.source_id = FRANKFURTER;
        base.link.symbol = "AUD".into();

        assert!(check.run().is_empty());
        assert!(check.transport.requests().is_empty());
    }
}
