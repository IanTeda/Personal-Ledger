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

## Amendment: API keys default to the OS keychain

Decided in the [VIPER transition grilling](https://github.com/IanTeda/Personal-Ledger/issues/745). A Client keeps each API key in the OS keychain through the `keyring` crate: Keychain on macOS, Credential Manager on Windows, Secret Service (gnome-keyring or KWallet) on Linux. The key never goes into the database, the static config file or the Git backup. A key can't be hashed, since it has to be sent in plain form, and a secret in the config file sits on the same disk as the database it would protect. A password in the UI was rejected: it adds a prompt at every start, and a forgotten password loses every key. On a machine with no keychain, such as a headless Linux machine without Secret Service, the key field says so and stores nothing, and that source's Units stay on Manual entry; an encrypted-file fallback waits until someone needs it. Building this belongs with the Price Source fetching work, since nothing reads a key until fetching exists. The `price_sources` table holds Custom sources only and never a key.

## Amendment: Units keep Weekly Prices

The "last price only" on a Unit's link is superseded by [ADR-0052](0052-units-keep-a-weekly-price-history.md): a Unit keeps a Weekly Price per ISO week, and its last price is the newest one's close.

## Amendment: where and when fetching runs

Decided in [CORE: where and when Unit prices are fetched](https://github.com/IanTeda/Personal-Ledger/issues/1074). Fetching runs in each Client while a Ledger is open: on open, then every minute, whatever View is showing. `lib-units` decides what is due and writes the result; the HTTP request goes through a port, async on the Tokio runtime both bins already start; the Clients only run the timer and hand results back. A Unit is due when it has no Weekly Price, or when its newest Weekly Price was written longer ago than its source's Updates interval (Daily is 24 hours), so the schedule comes from the Ledger itself with no last-fetched record, and a price another Client fetched and synced counts as fresh. Due Units are fetched together per source. Each Client keeps, in memory only, which sources have a fetch in flight and when each was last tried, and a source that failed waits at least its own interval before it is tried again. A source's round comes back as one input through the same commit door as a key press, writing every fetched Unit at once and skipping any Unit gone, deactivated or relinked since the fetch went out. Units on Manual entry, deactivated Units and Units on a source set to Manual only are never fetched on schedule. A fetched price goes into the week of the Clock's day. Daily and Hourly Prices are read through the same port for a View to show, never written, and cached in memory for the Ledger's session.

## Amendment: which built-ins stay, and what an Endpoint carries

Decided in [CORE: which built-in price providers stay, given their terms and limits](https://github.com/IanTeda/Personal-Ledger/issues/1375), from what [CORE: what the built-in price providers offer today](https://github.com/IanTeda/Personal-Ledger/issues/1368) found. All three built-ins stay.

- **Yahoo Finance** stays a normal built-in, although its terms forbid automated collection: it is the only keyless source with long equity history, and it is tolerated in practice. Its chart endpoint refuses a request with no `User-Agent`, so an `Endpoint` gains fixed request headers (a name and a value, never secret), which a Custom source can use too; Yahoo's sends `User-Agent: Personal-Ledger/<version>`, our own name rather than a browser's. The Units page says the API is unofficial and may stop working. Demoting it to opt-in and dropping it were rejected.
- **CoinGecko** stays keyless by default, with an optional free Demo key sent in the `x-cg-demo-api-key` header for 100 calls a minute instead of about 5, so `PriceSourceAuth` gains a key that is optional, and a built-in can take a key. "Powered by CoinGecko" shows in its detail pane on Units Settings and in the pane of every Unit it prices. Its terms ask that cached data refresh every 24 hours; a personal Ledger keeps its Weekly Prices anyway, and we accept that.
- **Frankfurter** moves to v2 at `https://api.frankfurter.dev`: request `/v2/rate/{symbol}/{quote}`, price at `/rate`, blended same-day rates for 219 currencies. v1 is ECB only and deprecated. Its Updates stay Daily.
- **Placeholders** gain `{symbol_lower}` and `{quote_lower}`, since CoinGecko keys its response by the lower-case quote; the built-ins stay plain data rather than a case written into code.
- **History:** each built-in gains an optional history request beside its price request, with `{from}` and `{to}` date placeholders and a path to the series: CoinGecko's `market_chart/range` (365 days on the free tiers), Yahoo's chart with `period1`, `period2` and `interval=1wk`, and Frankfurter v2's time series. How its rows become Weekly Prices is [CORE: backfill rules for a Unit's Weekly Prices](https://github.com/IanTeda/Personal-Ledger/issues/1370); whether a Custom source gets one is still open.

## Amendment: API keys live in the Ledger, encrypted

Decided in [CORE: which built-in price providers stay, given their terms and limits](https://github.com/IanTeda/Personal-Ledger/issues/1375), superseding "API keys default to the OS keychain" above, to keep things simple: one store on every platform, with no keychain to be missing. A Client's static configuration holds an encryption key, and each API key is stored encrypted with it in the Ledger file. The objections the keychain amendment recorded still stand and are accepted: a copy of the configuration file and the Ledger together reveals every key. Whether the encrypted keys sync, and so whether every device on a Ledger shares one configuration key, how that key is made and what a Client does without one, are open in [CORE: entering and removing a Price Source's API key](https://github.com/IanTeda/Personal-Ledger/issues/1374).
