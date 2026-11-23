//! The price feed: `lib-units`' price transport port over async `reqwest` (rustls on ring), the
//! second adapter crate after `lib-database` (ADR-0062). It sends each request on the Tokio
//! runtime the bin hands it, with a timeout, and returns the status and body, or why none came,
//! over a channel the bin polls and hands to `lib-units` through its input door.
//!
//! It knows nothing about providers: the URL, headers and how to read the answer are `lib-units`'.
//! An API key comes as a `SecretString` and is exposed only here, as it goes into the request, so
//! no URL or header it is handed and nothing it traces ever holds one (ADR-0043).

mod error;

use std::error::Error as _;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use lib_units::{
    KeyPlacement, Method, PriceRequest, PriceResponse, PriceTransport, RequestId, RequestKey,
    TransportError, TransportOutcome,
};
use secrecy::ExposeSecret as _;
use tokio::runtime::Handle;

pub use error::{Error, Result};

/// How long a request may take, connecting included, before it is given up as timed out.
pub const TIMEOUT: Duration = Duration::from_secs(20);

pub struct PriceFeed {
    runtime: Handle,
    client: reqwest::Client,
    responses: Sender<PriceResponse>,
}

impl PriceFeed {
    /// A feed sending on `runtime`, giving each request up after `timeout`, and the channel its
    /// responses arrive on. A response sent after the receiver is dropped (the Ledger closed) is
    /// discarded.
    ///
    /// # Errors
    /// When the HTTP client's TLS backend won't start.
    pub fn new(runtime: Handle, timeout: Duration) -> Result<(Self, Receiver<PriceResponse>)> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(timeout)
            .build()?;
        let (responses, receiver) = std::sync::mpsc::channel();
        Ok((
            Self {
                runtime,
                client,
                responses,
            },
            receiver,
        ))
    }
}

impl PriceTransport for PriceFeed {
    fn send(&self, id: RequestId, request: PriceRequest) {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
        };
        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(RequestKey {
            placement,
            name,
            value,
        }) = &request.key
        {
            builder = match placement {
                KeyPlacement::Header => builder.header(name, value.expose_secret()),
                KeyPlacement::Param => builder.query(&[(name, value.expose_secret())]),
            };
        }
        let responses = self.responses.clone();
        self.runtime.spawn(async move {
            let outcome = match builder.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    match response.text().await {
                        Ok(body) => TransportOutcome::Answered { status, body },
                        Err(error) => TransportOutcome::Failed(failure(&error)),
                    }
                }
                Err(error) => TransportOutcome::Failed(failure(&error)),
            };
            if let TransportOutcome::Failed(reason) = outcome {
                tracing::debug!(request = id.0, ?reason, "A price request failed");
            }
            // The receiver is gone once its Ledger closes, and the response with it.
            let _ = responses.send(PriceResponse {
                request: id,
                outcome,
            });
        });
    }
}

/// What `error` means to `lib-units`: a timeout, a failed TLS handshake, an unreachable host, or
/// anything else.
fn failure(error: &reqwest::Error) -> TransportError {
    if error.is_timeout() {
        TransportError::Timeout
    } else if is_tls(error) {
        TransportError::Tls
    } else if error.is_connect() {
        TransportError::Offline
    } else {
        TransportError::Other
    }
}

/// Whether a rustls error is anywhere in `error`'s chain. An `io::Error`'s `source` skips the
/// error it wraps, so its own inner error is looked at too.
fn is_tls(error: &reqwest::Error) -> bool {
    let mut next: Option<&(dyn std::error::Error + 'static)> = error.source();
    while let Some(cause) = next {
        if cause.is::<rustls::Error>() {
            return true;
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>()
            && io
                .get_ref()
                .is_some_and(|inner| inner.is::<rustls::Error>())
        {
            return true;
        }
        next = cause.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::RecvTimeoutError;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    fn get(url: String) -> PriceRequest {
        PriceRequest {
            method: Method::Get,
            url,
            headers: vec![("user-agent".into(), "Personal-Ledger/test".into())],
            key: None,
        }
    }

    /// The first request a listener receives, lower-cased, answered with an empty 200.
    async fn received(listener: TcpListener) -> String {
        let (mut socket, _) = listener.accept().await.expect("a connection");
        let mut request = [0_u8; 1024];
        let read = socket.read(&mut request).await.expect("the request");
        let reply = "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}";
        socket.write_all(reply.as_bytes()).await.expect("the reply");
        String::from_utf8_lossy(&request[..read]).to_lowercase()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_key_goes_in_its_header_or_its_query_parameter() {
        for (placement, expected) in [
            (KeyPlacement::Header, "x-cg-demo-api-key: k3y"),
            (KeyPlacement::Param, "get /v2/rate/eur/aud?apikey=k3y "),
        ] {
            let (listener, address) = listener().await;
            let server = tokio::spawn(received(listener));
            let mut request = get(format!("{address}/v2/rate/EUR/AUD"));
            request.key = Some(RequestKey {
                placement,
                name: match placement {
                    KeyPlacement::Header => "x-cg-demo-api-key".into(),
                    KeyPlacement::Param => "apikey".into(),
                },
                value: "k3y".into(),
            });

            fetch(request, TIMEOUT).await;

            let text = server.await.expect("the server ran");
            assert!(text.contains(expected), "{placement:?}: {text}");
        }
    }

    /// Sends `request` through a feed with `timeout` and waits for its response off the runtime.
    async fn fetch(request: PriceRequest, timeout: Duration) -> PriceResponse {
        let (feed, responses) = PriceFeed::new(Handle::current(), timeout).expect("a client");
        feed.send(RequestId(7), request);
        tokio::task::spawn_blocking(
            move || match responses.recv_timeout(Duration::from_secs(10)) {
                Ok(response) => response,
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    panic!("no response came")
                }
            },
        )
        .await
        .expect("the wait ran")
    }

    /// A listener on a free local port, and its address.
    async fn listener() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = format!("http://{}", listener.local_addr().expect("an address"));
        (listener, address)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_comes_back_as_its_status_and_body_whatever_the_status() {
        let (listener, address) = listener().await;
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("a connection");
            let mut request = [0_u8; 1024];
            let read = socket.read(&mut request).await.expect("the request");
            let text = String::from_utf8_lossy(&request[..read]).to_lowercase();
            assert!(text.starts_with("get /v2/rate/eur/aud "), "{text}");
            assert!(text.contains("user-agent: personal-ledger/test"), "{text}");
            let body = r#"{"message":"not found"}"#;
            let reply = format!(
                "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(reply.as_bytes()).await.expect("the reply");
        });

        let response = fetch(get(format!("{address}/v2/rate/EUR/AUD")), TIMEOUT).await;

        assert_eq!(
            response,
            PriceResponse {
                request: RequestId(7),
                outcome: TransportOutcome::Answered {
                    status: 404,
                    body: r#"{"message":"not found"}"#.into(),
                },
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_provider_that_never_answers_times_out() {
        let (listener, address) = listener().await;
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("a connection");
            // Held open, unanswered, until the test ends.
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(socket);
        });

        let response = fetch(get(address), Duration::from_millis(200)).await;

        assert_eq!(
            response.outcome,
            TransportOutcome::Failed(TransportError::Timeout)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_refused_connection_is_offline() {
        let (listener, address) = listener().await;
        drop(listener);

        let response = fetch(get(address), TIMEOUT).await;

        assert_eq!(
            response.outcome,
            TransportOutcome::Failed(TransportError::Offline)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_url_that_cant_be_requested_is_another_failure() {
        let response = fetch(get("not a url".into()), TIMEOUT).await;

        assert_eq!(
            response.outcome,
            TransportOutcome::Failed(TransportError::Other)
        );
    }
}
