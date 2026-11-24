use chrono::{DateTime, Utc};
use lib_core::{PriceSourceId, UnitId};
use secrecy::SecretString;

use crate::fetching::{
    PriceFetching, RequestId, TestHistory, TestInFlight, key_missing, price_request,
};
use crate::{Endpoint, Error, Result};

/// Test a Price Source (`GLOSSARY.md`): ask `endpoint` for one symbol's price and show it, writing
/// nothing. No price, no fetch failure or back-off and no key: the response lands through
/// [`crate::presenter::FetchedPricesPresenter`], which hands it to the form that asked or raises
/// its Toast. The endpoint is whatever the form holds, saved or not. A form's test of a source with
/// a history request asks it for the last 14 days once the price passes (ADR-0043's amendment "a
/// Custom source's history request"); `t` on a row asks only the price.
#[derive(Clone)]
pub struct TestPriceSource {
    /// The stored source, `None` for one still being added.
    pub source: Option<PriceSourceId>,
    pub source_name: String,
    pub endpoint: Endpoint,
    pub symbol: String,
    pub quote: UnitId,
    pub quote_code: String,
    /// The typed key, else the stored one.
    pub key: Option<SecretString>,
    /// `t` on a row: it answers with one Toast.
    pub toast: bool,
    /// The form differs from the stored source, so its result is tagged *with unsaved changes*.
    pub unsaved: bool,
}

impl TestPriceSource {
    /// Returns the test's request, queued as a call a person asked for: it waits out the source's
    /// call rate and any unexpired `Retry-After`, never its back-off. Refused with [`Error::KeyMissing`], sending nothing, when the
    /// endpoint needs a key and there is none.
    pub fn run(self, fetching: &mut PriceFetching, now: DateTime<Utc>) -> Result<RequestId> {
        let source_id = self.source.map(|id| id.to_string()).unwrap_or_default();
        if key_missing(&self.endpoint, self.key.as_ref()) {
            tracing::info!(
                price_source_id = %source_id,
                refusal = %Error::KeyMissing,
                "Refused to test a Price Source"
            );
            return Err(Error::KeyMissing);
        }
        let today = now.with_timezone(&chrono::Local).date_naive();
        let history = self
            .endpoint
            .history
            .as_deref()
            .filter(|_| !self.toast)
            .map(|history| TestHistory {
                endpoint: self.endpoint.clone(),
                history: history.clone(),
                range: (today - chrono::Duration::days(13), today),
            });
        let (request, price_path) = price_request(
            &self.endpoint,
            &self.symbol,
            &self.quote_code,
            self.key.clone(),
        );
        let id = fetching.send_test(
            request,
            TestInFlight {
                source: self.source,
                source_name: self.source_name,
                symbol: self.symbol,
                quote: self.quote,
                quote_code: self.quote_code,
                price_path,
                key: self.key,
                sent_at: now,
                toast: self.toast,
                unsaved: self.unsaved,
                history,
            },
            now,
        );
        tracing::debug!(price_source_id = %source_id, request = id.0, "Testing a Price Source");
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryUnits};
    use crate::fetching::{HistoryTestOutcome, Landing, PriceResponse};
    use crate::{KeyPlacement, PriceSourceAuth, TransportOutcome};

    fn now() -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 21)
            .and_then(|day| day.and_hms_opt(9, 0, 0))
            .expect("a real time")
            .and_utc()
    }

    fn keyed() -> Endpoint {
        Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/q?s={symbol}&in={quote}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "apikey".to_string().into(),
                optional: false,
            },
            headers: Vec::new().into(),
            batch: None,
            history: None,
        }
    }

    /// Lands `response` at `at` as a Client would, on no Units.
    fn land(fetching: &mut PriceFetching, response: PriceResponse, at: DateTime<Utc>) -> Landing {
        fetching.land(
            response,
            at,
            at.date_naive(),
            &Utc,
            &InMemoryUnits::default(),
        )
    }

    fn test(key: Option<&str>) -> TestPriceSource {
        TestPriceSource {
            source: None,
            source_name: "Prices".into(),
            endpoint: keyed(),
            symbol: "ABC".into(),
            quote: UnitId::SEED_AUD,
            quote_code: "AUD".into(),
            key: key.map(SecretString::from),
            toast: false,
            unsaved: false,
        }
    }

    #[test]
    fn a_test_sends_the_forms_request_and_its_result_masks_the_key_echoed_back() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));

        let id = test(Some("s3cr3t"))
            .run(&mut fetching, now())
            .expect("sent");

        assert_eq!(transport.urls(), ["https://prices.example/q?s=ABC&in=AUD"]);
        assert!(fetching.testing(id));
        assert_eq!(fetching.in_flight(), 0, "a test is no fetch");
        let response = transport.answer(
            id,
            TransportOutcome::Answered {
                status: 200,
                body: r#"{"price":"1.25","echo":"s3cr3t"}"#.into(),
                retry_after: None,
            },
        );
        let later = now() + chrono::Duration::milliseconds(412);
        assert!(
            matches!(land(&mut fetching, response, later), Landing::Nothing),
            "a form's test tells nothing"
        );
        let result = fetching.test_result(id).expect("held for the form").clone();
        assert_eq!(
            result.outcome,
            crate::fetching::TestOutcome::Priced("1.25".parse().expect("decimal"))
        );
        assert_eq!(result.elapsed.num_milliseconds(), 412);
        let (status, body) = result.response.expect("answered");
        assert_eq!(status, 200);
        assert!(!body.contains("s3cr3t"), "the key's echo is masked: {body}");
        assert!(fetching.test_result(id).is_some(), "held for the form");
    }

    fn with_history() -> TestPriceSource {
        let mut test = test(Some("k"));
        test.endpoint.history = Some(crate::HistoryRef::from(crate::HistoryEndpoint {
            request: "/h/{symbol}?from={from}&to={to}".to_string().into(),
            series_path: "/data".to_string().into(),
            date_path: "/d".to_string().into(),
            price_path: "/p".to_string().into(),
            shape: crate::SeriesShape::Points,
        }));
        test
    }

    #[test]
    fn once_the_price_passes_a_test_asks_the_last_14_days_of_history_and_counts_its_points() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        let id = with_history().run(&mut fetching, now()).expect("sent");

        let price = transport.answer_last(200, r#"{"price":1.25}"#);
        assert!(matches!(
            land(&mut fetching, price, now()),
            Landing::Nothing
        ));

        assert_eq!(
            transport.urls()[1],
            "https://prices.example/h/ABC?from=2026-10-08&to=2026-10-21"
        );
        assert!(
            fetching.testing(id),
            "the test is out until its history lands"
        );
        let history = transport.answer_last(
            200,
            r#"{"data":[{"d":"2026-10-19","p":1.2},{"d":"2026-10-20","p":"1.2345"},{"d":"x","p":1}]}"#,
        );
        assert!(
            matches!(land(&mut fetching, history, now()), Landing::Nothing),
            "a form's history raises nothing"
        );

        assert!(!fetching.testing(id));
        let result = fetching.test_result(id).expect("held for the form");
        let landed = result.history.as_ref().expect("the history ran");
        assert_eq!(
            landed.outcome,
            HistoryTestOutcome::Points {
                count: 2,
                newest: chrono::NaiveDate::from_ymd_opt(2026, 10, 20).expect("a real date"),
                close: "1.2345".parse().expect("decimal"),
            }
        );
        assert_eq!(
            landed.response.as_ref().map(|(status, _)| *status),
            Some(200)
        );
    }

    #[test]
    fn an_empty_history_reads_no_price_and_a_failed_price_asks_no_history() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        let id = with_history().run(&mut fetching, now()).expect("sent");
        land(
            &mut fetching,
            transport.answer_last(200, r#"{"price":1}"#),
            now(),
        );
        land(
            &mut fetching,
            transport.answer_last(200, r#"{"data":[]}"#),
            now(),
        );
        let history = fetching
            .test_result(id)
            .and_then(|result| result.history.clone());
        assert_eq!(
            history.map(|history| history.outcome),
            Some(HistoryTestOutcome::Failed(
                crate::fetching::FetchFailure::NoPrice
            ))
        );

        let failed = with_history().run(&mut fetching, now()).expect("sent");
        land(
            &mut fetching,
            transport.answer_last(200, r#"{"nothing":1}"#),
            now(),
        );
        assert_eq!(
            transport.requests().len(),
            3,
            "no history after a failed price"
        );
        assert!(!fetching.testing(failed));
        assert!(
            fetching
                .test_result(failed)
                .is_some_and(|result| result.history.is_none())
        );
    }

    #[test]
    fn a_dropped_tests_history_lands_as_no_ones() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        let id = with_history().run(&mut fetching, now()).expect("sent");
        land(
            &mut fetching,
            transport.answer_last(200, r#"{"price":1}"#),
            now(),
        );

        fetching.drop_test(id);
        let history = transport.answer_last(200, r#"{"data":[]}"#);

        assert!(matches!(
            land(&mut fetching, history, now()),
            Landing::Nothing
        ));
        assert_eq!(fetching.in_flight(), 0, "never read as a round's");
        assert!(fetching.test_result(id).is_none());
    }

    #[test]
    fn a_needed_key_that_is_missing_sends_nothing() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));

        assert_eq!(test(None).run(&mut fetching, now()), Err(Error::KeyMissing));
        assert!(transport.requests().is_empty());
    }

    #[test]
    fn a_dropped_test_lands_as_no_ones() {
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        let id = test(Some("k")).run(&mut fetching, now()).expect("sent");

        fetching.drop_test(id);
        let response = transport.answer_last(200, r#"{"price":1}"#);

        assert!(matches!(
            land(&mut fetching, response, now()),
            Landing::Nothing
        ));
        assert_eq!(
            fetching.in_flight(),
            0,
            "still known as a test's, never a round's"
        );
        assert!(fetching.test_result(id).is_none());
    }
}
