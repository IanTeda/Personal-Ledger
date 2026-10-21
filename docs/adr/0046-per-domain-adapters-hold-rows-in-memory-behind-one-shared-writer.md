# Per-domain adapters hold rows in memory behind one shared writer

`docs/development/viper-architecture.md` gives each feature crate its own repository ports, and Interactors and Presenters are synchronous: `handle(event)` returns and the View redraws from `view_model()` straight away. `sqlx` is async, so synchronous code has to meet it somewhere. ADR-0044 put that meeting point in `LedgerStore`, which the per-domain ports replace. Decided in the [VIPER transition grilling](https://github.com/IanTeda/Personal-Ledger/issues/745), with Units and Price Sources as the first slice.

## Decision

- **Ports stay synchronous.** A port in a feature crate's `protocols.rs` is a plain trait, so Entities, Interactors and Presenters never become async.
- **Each `lib-database` adapter holds its domain's rows in memory.** It loads them, blocking, when the Client starts. Reads are served from memory and never wait on SQLite.
- **One writer thread for all domains.** A write changes the adapter's memory at once and is sent to a single `lib-database` writer thread that owns a current-thread `tokio` runtime and runs the `sqlx` calls in order. One thread, not one per domain, keeps writes ordered across domains and leaves one connection writing.
- **One transaction per Interactor call,** with `PRAGMA foreign_keys = ON`, as ADR-0044 had per command.
- **Optimistic, with rollback by reload.** When a write fails, the writer drops the writes queued behind it, the adapter reloads its rows from SQLite, and the failure is reported on a channel the bin listens on. The bin rebuilds the affected view models, refreshes any read copy in `Ledger` and shows an error Toast. Ports know nothing about the channel.
- **The Sync Server calls Interactors through `spawn_blocking`,** since it is async and the ports are not.

## Considered Options

- **Block on SQLite on the calling thread.** Rejected: every read in `view_model()` and every write would hold gpui's thread, so the Desktop would stop drawing while it waited.
- **Async ports.** Rejected: async would spread into every Interactor, Presenter and View, and each Presenter would need a pending state per operation.
- **A writer thread per domain.** Rejected: writes from two domains could land out of order.

## Consequences

- The UI never waits on storage, but a failed write shows only as a Toast and a reload that drops the queued writes, as under ADR-0044.
- Every adapter holds its whole domain in memory. That is fine at a personal ledger's size, and Transactions are the first place to check it.
- A cross-domain module's atomic port goes through the same writer, so its one transaction is still ordered with every other write.
