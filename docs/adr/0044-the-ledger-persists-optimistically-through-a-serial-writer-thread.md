# The Ledger persists optimistically through a serial writer thread

ADR-0039 gave the Ledger one `LedgerRepository` port fed by recorded `LedgerChange`s, and left whether it is async and where the SQLite adapter lives to #636. Units are the first domain to persist; decided in the #685 grilling on the [Units functionality](https://github.com/IanTeda/Personal-Ledger/issues/667) map.

## Decision

- **A synchronous port.** `LedgerRepository` in `lib-ledger` has `load()` and `apply(changes)`, both synchronous, so the domain crates stay free of async. `StubRepository` serves `Ledger::stub()`, records what it is given and can be set to fail.
- **A dedicated writer thread.** `LedgerStore` applies a command in memory, emits `LedgerChanged`, and sends the drained changes over a channel to one writer thread that applies them serially. The SQLite adapter owns a current-thread `tokio` runtime and `block_on`s each `sqlx` call there, so a slow write never holds gpui's executor.
- **Optimistic, with rollback by reload.** When a write fails, the writer drops every batch queued behind it, the store replaces its `Ledger` with a fresh `load()`, emits `LedgerChanged` for every domain and shows an error Toast. If the reload fails too, the Toast persists and the in-memory Ledger stands.
- **One transaction per command.** `apply` writes a command's changes in one transaction, in recorded order, with `PRAGMA foreign_keys = ON`, so `SetBaseUnit` lands before the `DeleteUnit` that needed it. Deletes are hard deletes: commands already refuse a row in use.
- **The adapter is its own crate.** `lib-ledger-sqlite` depends on `lib-ledger` and `lib-database`; `lib-database` stays per-entity queries (ADR-0041) and `lib-ledger` never sees it.
- **The base Unit is a one-row `ledger` table.** `base_unit_id` references `units`, so there is exactly one base and SQLite enforces it. `preferences` no longer seeds a default Unit (ADR-0014's amendment).
- **Blocking load at start.** The Desktop runs migrations and `load()` before building the Shell, and exits with a logged error if it can't. The file is the `[Personal-Ledger]` `database` key, defaulting to the XDG data directory.
- **Migration halfway state.** Until a domain is persisted, `load()` takes it from `Ledger::stub()` and `apply` ignores its changes; an empty database is seeded with the stub's Units so stub rows keep resolving.
- **One contract suite.** `lib-ledger` exposes contract tests behind a `contract-tests` feature, generic over a repository factory: load, apply a command's changes, load again, and get the same `Ledger`. Both adapters run it. The headless UI tests (ADR-0030) run on `StubRepository`.

## Considered Options

- **An async trait.** Rejected: it spreads async into `lib-ledger` for one adapter's sake.
- **`cx.background_spawn` per write.** Rejected: gpui's executor gives no ordering between writes, and a blocking `sqlx` call would hold one of its threads.
- **`is_base` on `units`, or the base in `preferences`.** Rejected: neither makes "exactly one base" a constraint, and the base Unit is Ledger data, not a Preference.
- **The adapter inside `lib-database`.** Rejected: it would make the query crate depend on `lib-ledger`.

## Consequences

- The UI never waits on storage, but a failed write is visible only as a Toast and a reload that drops the queued writes.
- Change Sets (ADR-0009) are still not emitted; the change list remains the future sync outbox.
