# Price fetching lives in lib-units behind a transport port, and adapter crates hold the I/O

Units that price themselves ([CORE: Units price themselves from their Price Sources](https://github.com/IanTeda/Personal-Ledger/issues/1367)) need HTTP, and every feature crate is I/O-free ([viper architecture](../development/viper-architecture.md)). [CORE: which crates own the price fetch port, its HTTP adapter and the API key port](https://github.com/IanTeda/Personal-Ledger/issues/1378) decided that `lib-units` owns every fetching rule and that the HTTP itself runs in a new crate, `lib-price-feed`, the second **adapter crate** after `lib-database`.

- **`lib-units` owns fetching.** Fetching writes only to the Units domain, so it needs no feature crate of its own. The Interactors (scheduled rounds, fetch now, backfill, test, Daily and Hourly requests), a read port for the Accounts' Opened dates (answered by a `Stored…Reads` in `lib-database`) and the transport port all live there.
- **The port is plain transport.** `lib-units` builds a `PriceRequest` (method, URL, headers) from a Price Source, its symbols and its key, and sends it without waiting (`send(id, request)`). Every piece of provider knowledge (placeholders, history and batch requests, `Endpoint` headers, parsing, the seven failure reasons, a test's response) stays in `lib-units`, tested on fake responses with no HTTP.
- **Responses come back as an input.** The bin sends `lib_units::PriceResponse { request, outcome }` through the same door as a key press; `lib-units` matches it to its request, parses it, and writes a round through `RecordFetchedPrices` as one commit.
- **Fetching state is one struct, `PriceFetching`**, in `lib-units`: in-flight rounds, back-off, held failures, the call budget and the Daily and Hourly cache. Each Client keeps one per open Ledger, next to `LedgerPorts`, never persisted; the adapter holds none of it.
- **API keys:** `lib-units` declares `PriceSourceKeys::key(source) -> Option<SecretString>`, `None` meaning missing or undecryptable. `lib-database` answers it, decrypting with the API key secret the bin passes into `LedgerPorts::new`.
- **`lib-price-feed`** implements the transport port over async `reqwest` (rustls on ring) on the Tokio handle the bin gives it, with a request timeout, returning status and body, or a transport error, over a channel the bin polls. It knows nothing about providers. Only the bins build it, and they hold it beside `LedgerPorts`, not inside it, since it holds no stores.

## The dependency rule

`viper_dependencies.rs` gains an `ADAPTER_CRATES` list (`lib_database`, `lib_price_feed`): no crate in `FEATURE_CRATES` or `SHARED_CRATES` may depend on one, and `lib_price_feed` may depend on no feature crate but `lib_units`. `deny.toml` lets only `lib_price_feed` and `bin_sync_server` (its test-only use) wrap `reqwest`, and only `bin_desktop` and `bin_tui` wrap `lib_price_feed`. A future crate doing I/O for a feature crate joins `ADAPTER_CRATES`.

## Considered Options

- **Fold the adapter into `lib-units`, behind a Cargo feature.** Rejected: `reqwest`, rustls and `tokio` would compile into everything that depends on `lib-units`, and feature unification turns the feature on anyway.
- **Put the HTTP adapter in `lib-database`.** Rejected: it holds no stores, and `lib-database` would grow a network stack unrelated to persistence.
- **A smart port that takes a Price Source and returns prices.** Rejected: provider knowledge would move into the adapter, out of reach of `lib-units`' tests, and Test would need a second path to show the raw response.
- **A feature crate of its own for fetching.** Rejected: fetching writes only Units, so it is not a cross-domain workflow.

## Consequences

- `lib-units` stays I/O-free and its fetching rules are tested with no network.
- Both Clients get fetching from the same Interactors; each only runs the minute timer, polls the response channel and hands responses to the door.
- `keyring` is not a dependency.
