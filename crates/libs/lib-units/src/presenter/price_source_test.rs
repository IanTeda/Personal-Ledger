//! A Price Source test's one home (#1414, ADR-0062's amendment "a response lands through one call,
//! and fetching never writes"): the only caller of [`TestPriceSource`]. It sends a test, holds the
//! last one's request, drops that request on a retest and when its form closes, and turns a landed
//! result into the form's rows or into the Toast `t` on a row answers with. The form keeps only
//! what depends on its fields, whether a test can run and why not; the Units page's `t` is the
//! one-shot [`PriceSourceTest::in_background`]. A form's result is pulled from fetching's held
//! results on each repaint, never pushed, so `LedgerPrices` keeps no registry of open forms.

use chrono::Local;
use lib_core::{PriceSourceId, UnitId};
use lib_locale::format::format_time_minutes;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::count;
use secrecy::SecretString;

use super::Clock;
use super::fetch_text::{Asked, failure_detail, failure_name};
use super::units_pane::amount;
use crate::fetching::{
    FetchFailure, HistoryTestOutcome, HistoryTestResult, RequestId, SharedFetching, TestOutcome,
    TestResult,
};
use crate::interactor::TestPriceSource;
use crate::protocols::UnitsPorts;
use crate::{Endpoint, Error, UnitRow, msg};

/// How the last test went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestState {
    Testing,
    Found,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultView {
    pub state: TestState,
    /// *Price found · 412 ms*, *Testing…* or a reason's name.
    pub heading: String,
    /// `VAS.AX · 101.84 AUD · tested 14:36`, or the reason's detail.
    pub detail: String,
    /// *with unsaved changes*, when an edited source was tested with fields not yet saved.
    pub unsaved: Option<String>,
    /// A Custom source's response; `None` for a built-in, whose price path is code, and when
    /// nothing came back.
    pub response: Option<ResponseView>,
    /// The history the test asked once the price passed; `None` for a source with no history
    /// request, and when the price failed.
    pub history: Option<TestHistoryView>,
}

/// *History · 10 points · newest Nov 20, 2026 · 1.23 AUD*, or the reason it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestHistoryView {
    /// Found or Failed; an empty series is Failed, No price.
    pub state: TestState,
    pub text: String,
}

/// The collapsed *Response · HTTP n* disclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseView {
    pub label: String,
    /// The body, indented when it is JSON, as text when it isn't, any echo of the key masked.
    pub body: String,
    /// Open by itself on a price reason that helps fix the Price path, or on a failed history.
    pub open: bool,
    pub focused: bool,
    /// The history's response, under the price's, once a history came back.
    pub history: Option<HistoryResponseView>,
}

/// The history's part of the Response disclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryResponseView {
    /// `History response · HTTP 200`.
    pub label: String,
    /// The body, indented when it is JSON, any echo of the key masked.
    pub body: String,
}

/// A test a form has everything for, as the form's fields stand: what it asks of which endpoint.
pub(crate) struct FormTest {
    /// The stored source, `None` for one still being added.
    pub source: Option<PriceSourceId>,
    pub source_name: String,
    pub endpoint: Endpoint,
    pub symbol: String,
    pub quote: UnitId,
    pub quote_code: String,
    /// The typed key, else the stored one.
    pub key: Option<SecretString>,
    /// The form differs from the stored source, so its result is tagged *with unsaved changes*.
    pub unsaved: bool,
}

/// A form's test: the last one it sent, read back from fetching as it lands.
pub(crate) struct PriceSourceTest {
    fetching: SharedFetching,
    clock: Clock,
    /// The last test sent, whose result the form shows.
    last: Option<RequestId>,
    /// The person's choice to open or close the Response; `None` leaves it to the reason.
    response_open: Option<bool>,
}

/// Closing the form drops a test still out, so its response lands as no one's.
impl Drop for PriceSourceTest {
    fn drop(&mut self) {
        if let Some(id) = self.last
            && let Ok(mut fetching) = self.fetching.try_borrow_mut()
        {
            fetching.drop_test(id);
        }
    }
}

impl PriceSourceTest {
    /// A form's test through `fetching`, the Client's for its open Ledger, timed by `clock`.
    pub(crate) fn new(fetching: SharedFetching, clock: Clock) -> Self {
        Self {
            fetching,
            clock,
            last: None,
            response_open: None,
        }
    }

    /// Whether the last test, or the history it went on to ask, is still out.
    pub(crate) fn testing(&self) -> bool {
        self.last
            .is_some_and(|id| self.fetching.borrow().testing(id))
    }

    /// Sends `test`, dropping the last one and closing a Response the person opened. Writes
    /// nothing; its result is held for the form, never raised as a Toast.
    pub(crate) fn run(&mut self, test: FormTest) {
        let mut fetching = self.fetching.borrow_mut();
        if let Some(last) = self.last.take() {
            fetching.drop_test(last);
        }
        self.response_open = None;
        let test = TestPriceSource {
            source: test.source,
            source_name: test.source_name,
            endpoint: test.endpoint,
            symbol: test.symbol,
            quote: test.quote,
            quote_code: test.quote_code,
            key: test.key,
            toast: false,
            unsaved: test.unsaved,
        };
        // A form sends only a test its readiness found a key for, so no refusal is left.
        self.last = test.run(&mut fetching, (self.clock)()).ok();
    }

    /// Opens or closes the Response, once one came back.
    pub(crate) fn toggle_response(&mut self) {
        if let Some(open) = self.response(false).map(|response| response.open) {
            self.response_open = Some(!open);
        }
    }

    /// The last test's result, once it has landed.
    fn result(&self) -> Option<TestResult> {
        self.fetching.borrow().test_result(self.last?).cloned()
    }

    /// The last response, `focused` as the form's focus says, open by itself on No price, Not
    /// found and Key rejected, and on a failed history, whose response is its second part.
    pub(crate) fn response(&self, focused: bool) -> Option<ResponseView> {
        let result = self.result()?;
        let (status, body) = result.response?;
        let history_failed = result
            .history
            .as_ref()
            .is_some_and(|history| matches!(history.outcome, HistoryTestOutcome::Failed(_)));
        let auto = history_failed
            || matches!(
                result.outcome,
                TestOutcome::Failed(
                    FetchFailure::NoPrice | FetchFailure::NotFound | FetchFailure::KeyRejected
                )
            );
        Some(ResponseView {
            label: msg::units_price_source_test_response(&status.to_string()),
            body: pretty(body),
            open: self.response_open.unwrap_or(auto),
            focused,
            history: result
                .history
                .and_then(|history| history.response)
                .map(|(status, body)| HistoryResponseView {
                    label: msg::units_price_source_test_history_response(&status.to_string()),
                    body: pretty(body),
                }),
        })
    }

    /// *Testing…* while the last test is out, else how it went once it landed, with `response`
    /// under it: the form shows a Custom source's, and a built-in's none.
    pub(crate) fn view(
        &self,
        ports: &dyn UnitsPorts,
        response: Option<ResponseView>,
    ) -> Option<TestResultView> {
        if self.testing() {
            return Some(TestResultView {
                state: TestState::Testing,
                heading: msg::units_price_source_testing(),
                detail: String::new(),
                unsaved: None,
                response: None,
                history: None,
            });
        }
        let result = self.result()?;
        let quote = ports.units().unit(result.asked.quote);
        let (heading, detail) = test_lines(&result, quote.as_ref());
        Some(TestResultView {
            state: match result.outcome {
                TestOutcome::Priced(_) => TestState::Found,
                TestOutcome::Failed(_) => TestState::Failed,
            },
            heading,
            detail,
            unsaved: result
                .asked
                .unsaved
                .then(msg::units_price_source_test_unsaved),
            response,
            history: result
                .history
                .as_ref()
                .map(|history| history_view(&result, history, quote.as_ref())),
        })
    }

    /// `t` on a row: tests `unit`'s link on its source in the background, writing nothing; its
    /// response raises one Toast as it lands. Answers at once instead, sending nothing, while that
    /// source's test is out or with no key where one is needed. A Unit whose source no longer
    /// fetches, or whose quote is gone, is left alone.
    pub(crate) fn in_background(
        fetching: &SharedFetching,
        clock: &Clock,
        ports: &dyn UnitsPorts,
        unit: &UnitRow,
    ) -> Option<Notice> {
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(unit.link.source_id)?;
        let endpoint = source.endpoint()?;
        let quote = ports.units().unit(unit.link.quote)?;
        if fetching.borrow().source_testing(source.id) {
            return Some(Notice::Toast(
                ToastKind::Info,
                msg::units_toast_already_testing(source.name),
            ));
        }
        let test = TestPriceSource {
            source: Some(source.id),
            source_name: source.name.to_string(),
            endpoint: endpoint.clone(),
            symbol: unit.link.symbol.clone(),
            quote: quote.id,
            quote_code: quote.code.clone(),
            key: ports.keys().key(source.id),
            toast: true,
            unsaved: false,
        };
        let refused = test.run(&mut fetching.borrow_mut(), clock());
        matches!(refused, Err(Error::KeyMissing)).then(|| {
            test_failed_toast(
                FetchFailure::KeyMissing,
                source.name,
                Asked {
                    symbol: &unit.link.symbol,
                    quote: &quote.code,
                },
            )
        })
    }

    /// The one Toast a test sent by [`Self::in_background`] answers with as it lands: Info on a
    /// price, Warning naming the reason.
    pub(crate) fn landed_toast(result: &TestResult, ports: &dyn UnitsPorts) -> Notice {
        let quote = ports.units().unit(result.asked.quote);
        let source = &result.asked.source_name;
        match &result.outcome {
            TestOutcome::Priced(price) => Notice::Toast(
                ToastKind::Info,
                msg::units_toast_test_found(
                    source,
                    &result.asked.symbol,
                    &amount(price, quote.as_ref()),
                    &millis(result),
                ),
            ),
            TestOutcome::Failed(reason) => test_failed_toast(*reason, source, test_asked(result)),
        }
    }
}

/// How long a test's answer took, in whole milliseconds.
fn millis(result: &TestResult) -> String {
    result.elapsed.num_milliseconds().max(0).to_string()
}

/// What the test asked, for a failure's detail.
fn test_asked(result: &TestResult) -> Asked<'_> {
    Asked {
        symbol: &result.asked.symbol,
        quote: &result.asked.quote_code,
    }
}

/// A landed test as its form shows it: `Price found · 412 ms` over `VAS.AX · 101.84 AUD · tested
/// 14:36`, or a reason's name over its detail. `quote` is the quote Unit, for the price's places.
fn test_lines(result: &TestResult, quote: Option<&UnitRow>) -> (String, String) {
    match &result.outcome {
        TestOutcome::Priced(price) => (
            msg::units_price_source_test_found(&millis(result)),
            msg::units_price_source_test_found_detail(
                &result.asked.symbol,
                &amount(price, quote),
                &format_time_minutes(result.at.with_timezone(&Local).time()),
            ),
        ),
        TestOutcome::Failed(reason) => failure_lines(*reason, result),
    }
}

/// A test's failure `reason` as its name and detail, naming what the test asked.
fn failure_lines(reason: FetchFailure, result: &TestResult) -> (String, String) {
    (
        failure_name(reason),
        failure_detail(reason, &result.asked.source_name, Some(test_asked(result))),
    )
}

/// `CoinGecko test: Not found · CoinGecko has no bitcon in AUD`.
fn test_failed_toast(reason: FetchFailure, source: &str, asked: Asked<'_>) -> Notice {
    Notice::Toast(
        ToastKind::Warning,
        msg::units_toast_test_failed(
            source,
            &failure_name(reason),
            &failure_detail(reason, source, Some(asked)),
        ),
    )
}

/// `body` indented when it is JSON, else as it came.
fn pretty(body: String) -> String {
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|json| serde_json::to_string_pretty(&json).ok())
        .unwrap_or(body)
}

/// A test's history line: *History · 10 points · newest Nov 20, 2026 · 1.23 AUD*, or the reason.
fn history_view(
    result: &TestResult,
    history: &HistoryTestResult,
    quote: Option<&UnitRow>,
) -> TestHistoryView {
    match &history.outcome {
        HistoryTestOutcome::Points {
            count: points,
            newest,
            close,
        } => TestHistoryView {
            state: TestState::Found,
            text: msg::units_price_source_test_history_found(
                count(*points),
                &lib_locale::format::format_date(*newest, None),
                &amount(close, quote),
            ),
        },
        HistoryTestOutcome::Failed(reason) => {
            let (name, detail) = failure_lines(*reason, result);
            TestHistoryView {
                state: TestState::Failed,
                text: msg::units_price_source_test_history_failed(&name, &detail),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{InMemoryPorts, LedgerPricesFixture};
    use crate::presenter::test_support::{now, prices};
    use crate::{
        COINGECKO, CustomSource, FRANKFURTER, HistoryEndpoint, HistoryRef, KeyPlacement, Precision,
        PriceLink, PriceSourceAuth, PriceSourceRecord, SeriesShape, UnitKind, Updates,
    };

    /// Alpha Vantage as a Custom source, keyed in a parameter when `keyed`, with a list-of-points
    /// history request when `history`.
    fn alpha(keyed: bool, history: bool) -> Endpoint {
        Endpoint {
            address: "https://www.alphavantage.co".to_string().into(),
            request: "/query?symbol={symbol}".to_string().into(),
            price_path: "/Global Quote/05. price".to_string().into(),
            auth: if keyed {
                PriceSourceAuth::ApiKey {
                    placement: KeyPlacement::Param,
                    name: "apikey".to_string().into(),
                    optional: false,
                }
            } else {
                PriceSourceAuth::None
            },
            headers: Vec::new().into(),
            batch: None,
            history: history.then(|| {
                HistoryRef::from(HistoryEndpoint {
                    request: "/history?symbol={symbol}&from={from}&to={to}"
                        .to_string()
                        .into(),
                    series_path: "/data".to_string().into(),
                    date_path: "/date".to_string().into(),
                    price_path: "/close".to_string().into(),
                    shape: SeriesShape::Points,
                })
            }),
        }
    }

    /// An unsaved Custom source's test of VAS.AX in AUD on `endpoint`.
    fn vas(endpoint: Endpoint, key: Option<&str>) -> FormTest {
        FormTest {
            source: None,
            source_name: "Alpha Vantage".into(),
            endpoint,
            symbol: "VAS.AX".into(),
            quote: UnitId::SEED_AUD,
            quote_code: "AUD".into(),
            key: key.map(SecretString::from),
            unsaved: false,
        }
    }

    fn form(on: &LedgerPricesFixture) -> PriceSourceTest {
        PriceSourceTest::new(on.fetching().clone(), on.clock())
    }

    /// The result as a Custom source's form shows it, its response included.
    fn shown(on: &LedgerPricesFixture, test: &PriceSourceTest) -> TestResultView {
        test.view(&*on.ports().borrow(), test.response(false))
            .expect("a test was sent")
    }

    const PRICED: &str = r#"{"Global Quote":{"05. price":"101.84"}}"#;

    #[test]
    fn a_forms_test_shows_testing_then_the_price_it_found_raising_no_toast() {
        let mut on = prices();
        let mut test = form(&on);
        assert_eq!(test.view(&*on.ports().borrow(), None), None);

        test.run(vas(alpha(false, false), None));
        assert!(test.testing());
        assert_eq!(
            test.view(&*on.ports().borrow(), None)
                .map(|result| (result.state, result.heading)),
            Some((TestState::Testing, "Testing…".to_string()))
        );

        let notices = on.land(on.transport().answer_last(200, PRICED));
        assert!(notices.is_empty(), "a form's test raises no Toast");
        assert!(!test.testing());
        let result = shown(&on, &test);
        assert_eq!(result.state, TestState::Found);
        assert_eq!(result.heading, "Price found · 0 ms");
        assert!(
            result.detail.starts_with("VAS.AX · 101.84 AUD · tested "),
            "{}",
            result.detail
        );
        assert_eq!(result.unsaved, None);
        assert!(!result.response.expect("answered").open);
    }

    #[test]
    fn no_price_opens_the_response_by_itself_with_the_keys_echo_masked() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(true, false), Some("s3cr3t")));
        on.land(on.transport().answer_last(
            200,
            r#"{"Global Quote":{"5. price":"101.84"},"apikey":"s3cr3t"}"#,
        ));

        let result = shown(&on, &test);
        assert_eq!(
            (result.state, result.heading.as_str()),
            (TestState::Failed, "No price")
        );
        assert_eq!(
            result.detail,
            "Alpha Vantage gave no price for VAS.AX in AUD"
        );
        let response = result.response.expect("shown");
        assert_eq!(response.label, "Response · HTTP 200");
        assert!(response.open, "No price opens it by itself");
        assert!(response.body.contains("\"5. price\""));
        assert!(!response.body.contains("s3cr3t"), "{}", response.body);

        test.toggle_response();
        assert!(!shown(&on, &test).response.expect("shown").open);
        test.run(vas(alpha(true, false), Some("s3cr3t")));
        on.land(on.transport().answer_last(200, r#"{}"#));
        assert!(
            shown(&on, &test).response.expect("shown").open,
            "a new run leaves it to the reason again"
        );
    }

    #[test]
    fn an_unparseable_body_is_shown_as_text() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, false), None));
        on.land(on.transport().answer_last(502, "<html>Bad Gateway</html>"));

        let result = shown(&on, &test);
        assert_eq!(result.heading, "Source error");
        let response = result.response.expect("shown");
        assert_eq!(response.label, "Response · HTTP 502");
        assert_eq!(response.body, "<html>Bad Gateway</html>");
        assert!(!response.open);
    }

    #[test]
    fn an_unsaved_test_is_tagged_with_unsaved_changes() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(FormTest {
            unsaved: true,
            ..vas(alpha(false, false), None)
        });
        on.land(on.transport().answer_last(200, PRICED));

        assert_eq!(
            shown(&on, &test).unsaved.as_deref(),
            Some("with unsaved changes")
        );
    }

    #[test]
    fn once_the_price_passes_the_test_asks_14_days_of_history_and_shows_its_line() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, true), None));
        on.land(on.transport().answer_last(200, PRICED));

        assert_eq!(
            on.transport().urls()[1],
            "https://www.alphavantage.co/history?symbol=VAS.AX&from=2026-10-08&to=2026-10-21"
        );
        assert!(test.testing(), "out until its history lands");

        on.land(on.transport().answer_last(
            200,
            r#"{"data":[{"date":"2026-10-19","close":1.2},{"date":"2026-10-20","close":"1.2345"}]}"#,
        ));
        let result = shown(&on, &test);
        assert_eq!(result.heading, "Price found · 0 ms");
        let history = result.history.expect("the history ran");
        assert_eq!(history.state, TestState::Found);
        assert_eq!(
            history.text,
            "History · 2 points · newest Oct 20, 2026 · 1.23 AUD"
        );
        let response = result.response.expect("answered");
        assert!(!response.open, "nothing failed");
        let part = response.history.expect("the history's part");
        assert_eq!(part.label, "History response · HTTP 200");
        assert!(part.body.contains("\"1.2345\""), "{}", part.body);
    }

    #[test]
    fn an_empty_history_reads_no_price_and_opens_the_response_by_itself() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, true), None));
        on.land(on.transport().answer_last(200, PRICED));
        on.land(on.transport().answer_last(200, r#"{"data":[]}"#));

        let result = shown(&on, &test);
        assert_eq!(result.state, TestState::Found, "the price passed");
        let history = result.history.expect("the history ran");
        assert_eq!(
            (history.state, history.text.as_str()),
            (
                TestState::Failed,
                "History · No price · Alpha Vantage gave no price for VAS.AX in AUD"
            )
        );
        assert!(
            result.response.expect("shown").open,
            "a failed history opens it"
        );
    }

    #[test]
    fn a_failed_price_asks_no_history_and_a_source_without_one_shows_no_line() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, true), None));
        on.land(on.transport().answer_last(404, "{}"));
        assert_eq!(on.transport().requests().len(), 1);
        assert_eq!(shown(&on, &test).history, None);

        let mut plain = form(&on);
        plain.run(vas(alpha(false, false), None));
        on.land(on.transport().answer_last(200, PRICED));
        assert_eq!(
            on.transport().requests().len(),
            2,
            "no history request to ask"
        );
        let result = shown(&on, &plain);
        assert_eq!(result.history, None);
        assert_eq!(result.response.expect("shown").history, None);
    }

    #[test]
    fn a_built_ins_test_asks_its_history_too_and_shows_its_line() {
        let mut on = prices();
        let mut test = form(&on);
        let endpoint = crate::built_in(COINGECKO)
            .expect("a built-in")
            .endpoint
            .clone();
        test.run(FormTest {
            source: Some(COINGECKO),
            source_name: "CoinGecko".into(),
            endpoint,
            symbol: "bitcoin".into(),
            quote: UnitId::SEED_AUD,
            quote_code: "AUD".into(),
            key: None,
            unsaved: false,
        });
        on.land(on.transport().answer_last(200, r#"{"bitcoin":{"aud":1}}"#));
        assert!(
            on.transport().urls()[1].contains("/coins/bitcoin/market_chart/range?"),
            "{:?}",
            on.transport().urls()
        );
        // 20 October 2026, in milliseconds.
        on.land(
            on.transport()
                .answer_last(200, r#"{"prices":[[1792454400000,2]]}"#),
        );

        assert_eq!(
            test.view(&*on.ports().borrow(), None)
                .and_then(|result| result.history)
                .map(|history| history.text)
                .as_deref(),
            Some("History · 1 point · newest Oct 20, 2026 · 2.00 AUD")
        );
    }

    #[test]
    fn a_retest_drops_the_last_test_and_dropping_the_test_drops_one_still_out() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, false), None));
        let first = on.transport().answer_last(200, PRICED);
        test.run(vas(alpha(false, false), None));
        on.land(first);
        assert!(test.testing(), "the first answer is no one's");
        assert_eq!(
            test.view(&*on.ports().borrow(), None)
                .map(|result| result.state),
            Some(TestState::Testing)
        );

        drop(test);
        assert!(on.land(on.transport().answer_last(200, PRICED)).is_empty());
        assert_eq!(on.fetching().borrow().in_flight(), 0, "never a round's");
    }

    /// `t` on a row, in the background.
    mod in_background {
        use super::*;

        const BODY: &str = r#"{"date":"2026-10-21","base":"EUR","quote":"AUD","rate":1.6512}"#;

        /// The sample with EUR on Frankfurter, quoted in aud.
        fn eur() -> (LedgerPricesFixture, UnitRow) {
            let on = prices();
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
            on.ports().borrow_mut().units.units.push(unit.clone());
            (on, unit)
        }

        fn t(on: &LedgerPricesFixture, unit: &UnitRow) -> Option<Notice> {
            PriceSourceTest::in_background(on.fetching(), &on.clock(), &*on.ports().borrow(), unit)
        }

        /// No price on EUR, and no fetch held for it or Frankfurter.
        fn wrote_nothing(on: &LedgerPricesFixture, unit: &UnitRow) -> bool {
            let ports = on.ports().borrow();
            let records = ports.price_sources.records.clone();
            let source =
                crate::price_source(&records, "Manual entry", FRANKFURTER).expect("a built-in");
            let fetching = on.fetching().borrow();
            let eur = ports.units.units.iter().find(|row| row.id == unit.id);
            eur.is_some_and(|eur| eur.prices.is_empty())
                && fetching.source_fetch(&source).is_none()
                && fetching.unit_fetch(unit, &source).is_none()
                && ports.keys.keys.is_empty()
        }

        #[test]
        fn a_price_raises_one_info_toast_and_writes_nothing() {
            let (mut on, unit) = eur();

            assert_eq!(t(&on, &unit), None);
            assert_eq!(
                on.transport().urls(),
                ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
            );
            assert_eq!(on.fetching().borrow().in_flight(), 0, "a test is no fetch");

            assert_eq!(
                on.land(on.transport().answer_last(200, BODY)),
                [Notice::Toast(
                    ToastKind::Info,
                    "Frankfurter works: EUR · 1.65 AUD · 0 ms".into()
                )]
            );
            assert_eq!(on.transport().requests().len(), 1, "it asks no history");
            assert!(wrote_nothing(&on, &unit));
        }

        #[test]
        fn a_failure_warns_with_its_reason_holding_no_back_off() {
            let (mut on, unit) = eur();
            t(&on, &unit);

            assert_eq!(
                on.land(on.transport().answer_last(429, "")),
                [Notice::Toast(
                    ToastKind::Warning,
                    "Frankfurter test: Rate limited · Frankfurter asked to be called less often"
                        .into()
                )]
            );
            assert!(wrote_nothing(&on, &unit), "no failure or back-off is held");
        }

        #[test]
        fn a_second_test_while_the_first_is_out_says_so_and_sends_nothing() {
            let (on, unit) = eur();
            t(&on, &unit);

            assert_eq!(
                t(&on, &unit),
                Some(Notice::Toast(
                    ToastKind::Info,
                    "Already testing Frankfurter".into()
                ))
            );
            assert_eq!(on.transport().requests().len(), 1);
        }

        #[test]
        fn a_source_needing_a_key_this_device_lacks_warns_at_once_and_sends_nothing() {
            let (on, mut unit) = eur();
            let id = PriceSourceId::new();
            on.ports()
                .borrow_mut()
                .price_sources
                .records
                .push(PriceSourceRecord::Custom(CustomSource {
                    id,
                    name: "Rates".into(),
                    endpoint: Endpoint {
                        address: "https://rates.example".into(),
                        request: "/{symbol}".into(),
                        price_path: "/rate".into(),
                        auth: PriceSourceAuth::ApiKey {
                            placement: KeyPlacement::Header,
                            name: "x-key".into(),
                            optional: false,
                        },
                        headers: Vec::new().into(),
                        batch: None,
                        history: None,
                    },
                    updates: Updates::Hourly,
                }));
            unit.link.source_id = id;

            assert_eq!(
                t(&on, &unit),
                Some(Notice::Toast(
                    ToastKind::Warning,
                    "Rates test: Key missing · Rates needs an API key this device doesn't have"
                        .into()
                ))
            );
            assert!(on.transport().requests().is_empty());
        }
    }

    #[test]
    fn the_sample_is_unchanged_by_a_forms_test() {
        let mut on = prices();
        let mut test = form(&on);
        test.run(vas(alpha(false, false), None));
        on.land(on.transport().answer_last(200, PRICED));

        assert_eq!(
            on.ports().borrow().units.units,
            InMemoryPorts::sample(now()).units.units
        );
    }
}
