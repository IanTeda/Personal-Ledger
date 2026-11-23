//! Fetching a Unit's price from its Price Source (ADR-0062): the plain transport port the Client's
//! adapter (`lib-price-feed`) answers, the request it is handed and the response it hands back,
//! and [`PriceFetching`], the per-Client bookkeeping of what is in flight.
//!
//! Every piece of provider knowledge stays here: filling an [`Endpoint`]'s placeholders into a
//! request and reading the price at its price path. The transport only carries bytes, so these
//! rules are tested with no network.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::{Money, UnitId};

use crate::{Endpoint, PriceLink};

/// Names one request sent, so its response can be matched to what asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(pub u64);

/// The HTTP method a request goes by. Every provider answers a price on a GET.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
}

/// One request for a price, built in full here so the transport knows nothing about providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRequest {
    pub method: Method,
    pub url: String,
    /// Header names and values, in the order sent.
    pub headers: Vec<(String, String)>,
}

/// Sends a request without waiting: its response comes back later as a [`PriceResponse`], which
/// the Client hands to [`crate::presenter::FetchedPricesPresenter`] through its input door.
pub trait PriceTransport {
    fn send(&self, id: RequestId, request: PriceRequest);
}

/// What the transport made of request `request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceResponse {
    pub request: RequestId,
    pub outcome: TransportOutcome,
}

/// The provider answered with `status` and `body`, whatever the status, or no answer came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportOutcome {
    Answered { status: u16, body: String },
    Failed(TransportError),
}

/// Why no answer came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The provider couldn't be reached: no network, a refused connection or a failed lookup.
    Offline,
    /// The provider took longer than the transport's timeout.
    Timeout,
    /// The secure connection couldn't be made.
    Tls,
    /// Anything else: a request the transport couldn't build or a body it couldn't read.
    Other,
}

/// What a request was for: the Unit and its link as they were when it went out, so a response for
/// a Unit relinked since writes nothing, and the price path filled for that link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InFlight {
    pub unit: UnitId,
    pub link: PriceLink,
    pub price_path: String,
}

/// The fetching state one Client keeps per open Ledger, beside its ports, and drops when the
/// Ledger closes: never persisted (ADR-0062). For now, only the requests in flight.
pub struct PriceFetching {
    transport: Rc<dyn PriceTransport>,
    next: u64,
    in_flight: HashMap<RequestId, InFlight>,
}

/// [`PriceFetching`] as the Presenters share it, single-threaded as both Clients' UI threads are.
pub type SharedFetching = Rc<RefCell<PriceFetching>>;

impl PriceFetching {
    pub fn new(transport: Rc<dyn PriceTransport>) -> Self {
        Self {
            transport,
            next: 0,
            in_flight: HashMap::new(),
        }
    }

    pub fn shared(transport: Rc<dyn PriceTransport>) -> SharedFetching {
        Rc::new(RefCell::new(Self::new(transport)))
    }

    /// How many requests are out.
    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// Whether a request for `unit` is out.
    pub fn fetching(&self, unit: UnitId) -> bool {
        self.in_flight.values().any(|sent| sent.unit == unit)
    }

    /// Sends `request` for `sent`, remembering it until its response comes back.
    pub(crate) fn send(&mut self, request: PriceRequest, sent: InFlight) -> RequestId {
        let id = RequestId(self.next);
        self.next += 1;
        self.in_flight.insert(id, sent);
        self.transport.send(id, request);
        id
    }

    /// What request `id` was for, forgetting it: `None` for one this Ledger never sent.
    pub(crate) fn take(&mut self, id: RequestId) -> Option<InFlight> {
        self.in_flight.remove(&id)
    }
}

/// `text` with `{symbol}` and `{quote}` filled from a link.
fn fill(text: &str, symbol: &str, quote: &str) -> String {
    text.replace("{symbol}", symbol).replace("{quote}", quote)
}

/// The request asking `endpoint` for `symbol`'s price in `quote` (the quote Unit's code), and the
/// price path filled the same way.
pub(crate) fn price_request(
    endpoint: &Endpoint,
    symbol: &str,
    quote: &str,
) -> (PriceRequest, String) {
    let request = PriceRequest {
        method: Method::Get,
        url: format!(
            "{}{}",
            endpoint.address.trim_end_matches('/'),
            fill(&endpoint.request, symbol, quote)
        ),
        headers: Vec::new(),
    };
    (request, fill(&endpoint.price_path, symbol, quote))
}

/// The price at JSON pointer `path` in `body`: a number, or a string holding one. `None` for a
/// body that isn't JSON or has no price there.
pub(crate) fn parse_price(body: &str, path: &str) -> Option<Money> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let text = match json.pointer(path)? {
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::String(text) => text.clone(),
        _ => return None,
    };
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FRANKFURTER, built_in};

    #[test]
    fn frankfurters_request_is_v2_rate_for_the_symbol_in_the_quote() {
        let endpoint = &built_in(FRANKFURTER).expect("a built-in").endpoint;

        let (request, path) = price_request(endpoint, "EUR", "AUD");

        assert_eq!(
            request,
            PriceRequest {
                method: Method::Get,
                url: "https://api.frankfurter.dev/v2/rate/EUR/AUD".into(),
                headers: Vec::new(),
            }
        );
        assert_eq!(path, "/rate");
    }

    #[test]
    fn a_price_is_read_at_its_path_as_a_number_or_a_string() {
        let money = |text: &str| text.parse::<Money>().expect("a decimal");
        assert_eq!(
            parse_price(r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#, "/rate"),
            Some(money("1.6512"))
        );
        assert_eq!(
            parse_price(r#"{"a":{"b":"0.25"}}"#, "/a/b"),
            Some(money("0.25"))
        );
    }

    #[test]
    fn no_price_is_read_from_a_missing_path_a_non_number_or_a_body_that_isnt_json() {
        assert_eq!(parse_price(r#"{"rate":1.5}"#, "/rates/AUD"), None);
        assert_eq!(parse_price(r#"{"rate":true}"#, "/rate"), None);
        assert_eq!(parse_price(r#"{"rate":"n/a"}"#, "/rate"), None);
        assert_eq!(parse_price("<html>", "/rate"), None);
    }
}
