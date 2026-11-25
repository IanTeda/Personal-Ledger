//! The tests in flight and landed: what each asked, the results held for the form that asked, and
//! the tests whose form has closed. A test is never part of a round, so it writes nothing when it
//! lands.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{Money, PriceSourceId, UnitId};
use secrecy::{ExposeSecret as _, SecretString};

use super::{
    FetchFailure, HistoryReading, PriceRequest, PriceResponse, Reading, RequestId,
    TransportOutcome, price_request, read, read_history,
};
use crate::{Endpoint, HistoryEndpoint};

/// The facts of a Price Source test (`GLOSSARY.md`) a form or a row hands fetching, which builds
/// the requests and the record of what is in flight from them.
pub(crate) struct TestAsk {
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

impl TestAsk {
    /// The price request and what is in flight once it is sent at `now`: the history test is the
    /// last 14 days to the local day of `now`, only when the endpoint has a history request and
    /// the test is a form's, not a row's Toast.
    pub(super) fn build(self, now: DateTime<Utc>) -> (PriceRequest, TestInFlight) {
        let today = now.with_timezone(&chrono::Local).date_naive();
        let history = self
            .endpoint
            .history
            .as_deref()
            .filter(|_| !self.toast)
            .map(|history| TestHistory {
                endpoint: self.endpoint.clone(),
                history: history.clone(),
                range: (today - Duration::days(13), today),
            });
        let (request, price_path) = price_request(
            &self.endpoint,
            &self.symbol,
            &self.quote_code,
            self.key.clone(),
        );
        let asked = TestInFlight {
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
        };
        (request, asked)
    }
}

/// What a test asked (`GLOSSARY.md`'s Price Source, test): one symbol's price, shown and never
/// written.
#[derive(Clone)]
pub(crate) struct TestInFlight {
    /// The stored source tested, `None` for an unsaved one being added.
    pub source: Option<PriceSourceId>,
    /// The name its Toast and detail give it.
    pub source_name: String,
    pub symbol: String,
    pub quote: UnitId,
    pub quote_code: String,
    pub price_path: String,
    /// The key sent, so its echo in the response can be masked.
    pub key: Option<SecretString>,
    pub sent_at: DateTime<Utc>,
    /// `t` on a row: it answers with a Toast rather than in a form.
    pub toast: bool,
    /// Whether the form's fields differed from the stored source when it was sent.
    pub unsaved: bool,
    /// The history request a form's test asks once its price passes; `None` for a source with
    /// none, and for `t` on a row, which answers with one Toast.
    pub history: Option<TestHistory>,
}

/// What a test asks of a source's history (ADR-0043's amendment "a Custom source's history
/// request"): the last 14 days, through the endpoint the price came from.
#[derive(Clone)]
pub(crate) struct TestHistory {
    pub endpoint: Endpoint,
    pub history: HistoryEndpoint,
    /// The days asked, both included.
    pub range: (NaiveDate, NaiveDate),
}

/// How a test's history went: its points, an empty series being No price, or a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HistoryTestOutcome {
    Points {
        count: usize,
        newest: NaiveDate,
        /// The newest point's close.
        close: Money,
    },
    Failed(FetchFailure),
}

/// A test's landed history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoryTestResult {
    pub outcome: HistoryTestOutcome,
    /// The status and body the provider answered, the key masked; `None` when nothing came back.
    pub response: Option<(u16, String)>,
}

/// How a test went: a price, or one of the seven reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TestOutcome {
    Priced(Money),
    Failed(FetchFailure),
}

/// A landed test, as the form or the Toast shows it.
#[derive(Clone)]
pub(crate) struct TestResult {
    pub asked: TestInFlight,
    pub outcome: TestOutcome,
    pub elapsed: Duration,
    pub at: DateTime<Utc>,
    /// The status and body the provider answered, the key masked; `None` when nothing came back.
    pub response: Option<(u16, String)>,
    /// The history the test asked once its price passed, once it has landed.
    pub history: Option<HistoryTestResult>,
}

/// What stands in for a key echoed back in a response.
const MASK: &str = "\u{2022}\u{2022}\u{2022}";

/// `body` with every echo of `key` masked.
fn masked(body: &str, key: Option<&SecretString>) -> String {
    match key.map(|key| key.expose_secret().trim()) {
        Some(key) if !key.is_empty() => body.replace(key, MASK),
        _ => body.to_string(),
    }
}

/// The status and body `outcome` answered, the key masked; `None` when nothing came back.
fn answered(outcome: &TransportOutcome, key: Option<&SecretString>) -> Option<(u16, String)> {
    match outcome {
        TransportOutcome::Answered { status, body, .. } => Some((*status, masked(body, key))),
        TransportOutcome::Failed(_) => None,
    }
}

/// Every test out or landed, and the test histories out.
#[derive(Default)]
pub(super) struct PriceTests {
    /// Tests out, by request: never part of a round, so they write nothing when they land.
    tests: HashMap<RequestId, TestInFlight>,
    /// Tests landed for a form that hasn't read them yet, dropped with the form.
    test_results: HashMap<RequestId, TestResult>,
    /// Tests out whose form has closed, whose responses are dropped as they land.
    dropped: HashSet<RequestId>,
    /// A form's test's history requests out, by request, each with the test it belongs to: the
    /// test is out until its history lands too.
    test_histories: HashMap<RequestId, (RequestId, TestHistory)>,
}

impl PriceTests {
    /// Remembers test `id`'s `asked` until it lands.
    pub(super) fn sent(&mut self, id: RequestId, asked: TestInFlight) {
        self.tests.insert(id, asked);
    }

    /// Remembers test history `id` as `test`'s, until it lands.
    pub(super) fn history_sent(&mut self, id: RequestId, test: RequestId, history: TestHistory) {
        self.test_histories.insert(id, (test, history));
    }

    /// Stamps the tests among `ids` as sent at `now`: a test times its answer from when it went
    /// out, not from when it queued.
    pub(super) fn went_out(&mut self, ids: &[RequestId], now: DateTime<Utc>) {
        for id in ids {
            if let Some(test) = self.tests.get_mut(id) {
                test.sent_at = now;
            }
        }
    }

    /// Whether request `id` is a test's price request.
    pub(super) fn asks(&self, id: RequestId) -> bool {
        self.tests.contains_key(&id)
    }

    /// Whether `response` answers a test, dropped or not.
    pub(super) fn is_test(&self, response: &PriceResponse) -> bool {
        self.tests.contains_key(&response.request)
            || self.test_histories.contains_key(&response.request)
    }

    /// Reads a test's `response` at `now`, holding it for its form unless it answers with a
    /// Toast. `None` for a response that is no test's, a dropped one's or a history's, which it
    /// reads onto its test. The history, when one is due, is the test's to ask next.
    pub(super) fn land(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
    ) -> Option<(TestResult, Option<TestHistory>)> {
        if self.test_histories.contains_key(&response.request) {
            self.land_history(response);
            return None;
        }
        let asked = self.tests.remove(&response.request)?;
        if self.dropped.remove(&response.request) {
            return None;
        }
        let outcome = match read(&response.outcome, &asked.price_path) {
            Reading::Priced(price) => TestOutcome::Priced(price),
            Reading::Failed(reason) => TestOutcome::Failed(reason),
        };
        let result = TestResult {
            elapsed: now - asked.sent_at,
            at: now,
            outcome,
            response: answered(&response.outcome, asked.key.as_ref()),
            history: None,
            asked,
        };
        let mut due = None;
        if !result.asked.toast {
            self.test_results.insert(response.request, result.clone());
            if matches!(result.outcome, TestOutcome::Priced(_)) {
                due = result.asked.history.clone();
            }
        }
        Some((result, due))
    }

    /// Reads a test's history `response` onto its test's result, unless its form has dropped it.
    fn land_history(&mut self, response: &PriceResponse) {
        let Some((test, history)) = self.test_histories.remove(&response.request) else {
            return;
        };
        if self.dropped.remove(&response.request) {
            return;
        }
        let Some(result) = self.test_results.get_mut(&test) else {
            return;
        };
        let asked = &result.asked;
        let outcome = match read_history(
            &response.outcome,
            &history.history,
            &asked.symbol,
            &asked.quote_code,
        ) {
            HistoryReading::Points(mut points) => {
                // Stable, so the last of several points on one day is its close, as a week folds.
                points.sort_by_key(|point| point.day);
                match points.last() {
                    Some(newest) => HistoryTestOutcome::Points {
                        count: points.len(),
                        newest: newest.day,
                        close: newest.close.clone(),
                    },
                    None => HistoryTestOutcome::Failed(FetchFailure::NoPrice),
                }
            }
            HistoryReading::Failed(reason) => HistoryTestOutcome::Failed(reason),
        };
        let response = answered(&response.outcome, asked.key.as_ref());
        result.history = Some(HistoryTestResult { outcome, response });
    }

    /// Whether test `id` is still out.
    pub(super) fn testing(&self, id: RequestId) -> bool {
        self.tests.contains_key(&id) || self.test_histories.values().any(|(test, _)| *test == id)
    }

    /// Whether a test of stored source `source` is out.
    pub(super) fn source_testing(&self, source: PriceSourceId) -> bool {
        self.tests
            .iter()
            .any(|(id, asked)| asked.source == Some(source) && !self.dropped.contains(id))
    }

    /// Test `id`'s result, once it has landed.
    pub(super) fn result(&self, id: RequestId) -> Option<&TestResult> {
        self.test_results.get(&id)
    }

    /// Forgets test `id`: its form closed or tested again. A response still to come lands as no
    /// one's and is dropped.
    pub(super) fn drop_test(&mut self, id: RequestId) {
        self.test_results.remove(&id);
        // Kept among the tests, so its response is still known as a test's, never a round's.
        if self.tests.contains_key(&id) {
            self.dropped.insert(id);
        }
        for (history, (test, _)) in &self.test_histories {
            if *test == id {
                self.dropped.insert(*history);
            }
        }
    }
}
