# Per-domain adapters hold rows in memory behind one shared writer

`docs/development/viper-architecture.md` gives each feature crate its own repository ports, and Interactors and Presenters are synchronous: `handle(event)` returns and the View redraws from `view_model()` straight away. `sqlx` is async, so synchronous code has to meet it somewhere. ADR-0044 put that meeting point in `LedgerStore`, which the per-domain ports replace. Decided in the [VIPER transition grilling](https://github.com/IanTeda/Personal-Ledger/issues/745), with Units and Price Sources as the first slice.

## Decision

- **Ports stay synchronous.** A port in a feature crate's `protocols.rs` is a plain trait, so Entities, Interactors and Presenters never become async.
- **Each `lib-database` adapter holds its domain's rows in memory.** It loads them, blocking, when the Client starts. Reads are served from memory and never wait on SQLite.
- **One writer thread for all domains.** A write changes the adapter's memory at once and is sent to a single `lib-database` writer thread that owns a current-thread `tokio` runtime and runs the `sqlx` calls in order. One thread, not one per domain, keeps writes ordered across domains and leaves one connection writing.
- **One transaction per Interactor call,** with `PRAGMA foreign_keys = ON`, as ADR-0044 had per command.
- **Optimistic, with rollback by reload.** When a write fails, the writer drops the writes queued behind it, the adapter reloads its rows from SQLite, and the failure is reported on a channel the bin listens on. The bin rebuilds the affected view models, refreshes any read copy in `Ledger` and shows an error Toast. Ports know nothing about the channel. Since [ADR-0055](0055-the-commit-names-what-an-input-changed-and-lib-database-answers-each-domains-ports.md) the bin reloads a domain by its `Domain` in one call and announces what the commit names, rather than each module counting its writes.
- **The Sync Server calls Interactors through `spawn_blocking`,** since it is async and the ports are not.

## Considered Options

- **Block on SQLite on the calling thread.** Rejected: every read in `view_model()` and every write would hold gpui's thread, so the Desktop would stop drawing while it waited.
- **Async ports.** Rejected: async would spread into every Interactor, Presenter and View, and each Presenter would need a pending state per operation.
- **A writer thread per domain.** Rejected: writes from two domains could land out of order.

## Consequences

- The UI never waits on storage, but a failed write shows only as a Toast and a reload that drops the queued writes, as under ADR-0044.
- Every adapter holds its whole domain in memory. That is fine at a personal ledger's size, and Transactions are the first place to check it.
- A cross-domain module's atomic port goes through the same writer, so its one transaction is still ordered with every other write.
- The usage counts other modules read (the Splits naming a Payee, Category or Tag, the Transactions booked to an Account, the Bill Plans naming a Payee) once queried the file through the writer; [#1123](https://github.com/IanTeda/Personal-Ledger/issues/1123) brought them back in line with this decision, as free functions over the rows the stores hold.

## Amendment (2026-11-02): a cascade reloads the rows it changed

Persisting Documents ([#812](https://github.com/IanTeda/Personal-Ledger/issues/812)) puts foreign keys with `ON DELETE CASCADE` on `document_links`: deleting a linked Transaction, Account, Payee, Bill Plan, Inventory Item or Location drops its Document Links and keeps the Document (GLOSSARY **Document Link**). SQLite then changes rows the Documents adapter holds in memory, without the adapter knowing, and the next upsert of that Document would write the dead Link back and fail on the foreign key. Decided in the slice 9 grilling.

- **The schema is the source of truth for a cascade.** A delete in another domain never writes Documents itself, so Payee delete, Transaction delete and Account delete stay single-domain or keep the stores they have.
- **A commit that deleted a linkable row reloads the Documents adapter's Links.** The pending batch knows which deletes it holds, and the Desktop's `input.rs` `send` asks for the reload after it commits. The reload goes through the writer like every load, queued behind the commit, so it sees the cascade.
- **A later cascade follows the same rule.** A new table whose rows a cascade can change is reloaded the same way, by the commit that deleted the parent. The commit returns the cascaded domains as `Committed::cascaded`, beside the domains it wrote ([ADR-0055](0055-the-commit-names-what-an-input-changed-and-lib-database-answers-each-domains-ports.md)).

Considered and rejected: each deleting Interactor also dropping the Links through a Documents port, which would make every delete of a linkable record a cross-domain write and give `AccountDeletionAdapter` a fourth store; and the Documents adapter filtering dead Links on upsert, which needs reads into every linked domain's store.
