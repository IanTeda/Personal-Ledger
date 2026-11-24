//! The plain transport port the Client's adapter answers, the request it is handed and the
//! response it hands back.

use secrecy::{ExposeSecret as _, SecretString};

use crate::KeyPlacement;

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
    /// Never holds the key, so the request can be printed and traced.
    pub url: String,
    /// Header names and values, in the order sent.
    pub headers: Vec<(String, String)>,
    /// The source's API key, which the transport puts where `placement` says as the request goes
    /// out: the only place it is exposed (ADR-0043).
    pub key: Option<RequestKey>,
}

/// An API key a request carries: a query parameter or a header named `name`.
#[derive(Clone)]
pub struct RequestKey {
    pub placement: KeyPlacement,
    pub name: String,
    pub value: SecretString,
}

/// The key's name and placement, never the key.
impl std::fmt::Debug for RequestKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestKey")
            .field("placement", &self.placement)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Equal when the same key goes in the same place: what a test asks of a request it recorded.
impl PartialEq for RequestKey {
    fn eq(&self, other: &Self) -> bool {
        self.placement == other.placement
            && self.name == other.name
            && self.value.expose_secret() == other.value.expose_secret()
    }
}

impl Eq for RequestKey {}

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
/// `retry_after` is the raw `Retry-After` header, if it sent one, which only a 429 is read for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportOutcome {
    Answered {
        status: u16,
        body: String,
        retry_after: Option<String>,
    },
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
