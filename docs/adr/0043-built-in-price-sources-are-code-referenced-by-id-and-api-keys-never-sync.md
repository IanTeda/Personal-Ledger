# Built-in Price Sources are code referenced by id, and API keys never sync

A Price Source is structured rather than one URL: an address, a request template with `{symbol}` and `{quote}` placeholders, a price path (a JSON pointer into the response), its authentication (the key parameter's name only), an update interval and the Unit Kinds it can price. We decided that the built-in sources (CoinGecko, Yahoo Finance and Frankfurter to start) are a `const` table in `lib-units`, always listed on the Units Settings page, and that the Ledger stores only a built-in's id with the user's own name and update interval for it. A built-in can be renamed and retimed but never deleted; only a Custom source, which stores its own address, request, price path and authentication, is added or deleted, and deleting one moves its Units to Manual entry. A Unit's symbol and quote Unit live on the Unit's link to its source, not on the source. API keys are never part of the synced Ledger: each Client keeps its own, as it does for AI Connections ([ADR-0034](0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md)).

Referencing built-ins by id means an app update that fixes a provider's address or response shape fixes it on every Ledger at once, with no stored copy to migrate or drift. Code rather than data files keeps the table compile-checked and the crate I/O-free.

## Considered Options

- **Copy a built-in's fields into the Ledger when it is added.** Rejected: a provider's API change would leave every existing Ledger holding a broken copy until the user re-entered it.
- **Ship built-ins as data files compiled by `build.rs`, as Colour Themes are.** Deferred, not rejected: worth it only if sources should ship without a Rust change.
- **Sync API keys inside the Ledger.** Rejected: it would spread secrets through the Sync Server's Change Set store.

## Consequences

- The Price Sources table's **+** adds a Custom source only, and a built-in row has no Delete.
- Where a Client stores an API key (the OS keychain, or an encrypted store) is still open; until it is decided, the Custom source Dialog's key field stores nothing.
- Where fetching runs, and how a fetched last price syncs, are still open; until then Test is not built.
