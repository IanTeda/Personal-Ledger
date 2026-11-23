//! What can stop the price feed starting. A request that fails once it is running isn't here: it
//! comes back as a `lib_units::TransportOutcome`, as `lib-units` reads it.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The HTTP client couldn't be built: its TLS backend wouldn't start.
    #[error("the price feed's HTTP client couldn't be built: {0}")]
    Client(#[from] reqwest::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
