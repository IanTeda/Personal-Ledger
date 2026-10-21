# The Ledger persists through one store-wide repository fed by a change list

ADR-0038 kept `Ledger` in memory and left its persistence port to research (#597). The domain crates are I/O-free, and no adapter connects them to `lib-database`. Some writes span domains: paying a Bill writes Transactions, and deleting a Category re-points Splits. Decided in #634 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map; the research is `docs/research/persistence-seam.md` on the `research/persistence-seam` branch.

## Decision

- **One port, on `Ledger`.** `lib-ledger` defines a single store-wide `LedgerRepository` with `load()` and `apply(changes)`, not one port per domain crate. Only `Ledger` sees a cross-domain write whole, so only it can hand storage one atomic unit.
- **Commands record changes, they don't call storage.** A `Ledger` command stays synchronous and pure, and on success records what it changed as `LedgerChange`s (e.g. `Upsert(Transaction)`, `Delete(PayeeId)`). The store drains them and passes them to `apply`, which writes one command's changes in one transaction.
- **Two adapters.** A stub adapter serves `Ledger::stub()` and records changes for tests, so UI-first work and the headless UI tests carry on unchanged. A SQLite adapter maps changes to `lib-database` queries. `lib-ledger` never depends on `lib-database`.
- **Recording now, wiring later.** The Ledger work on this map records `LedgerChange`s from the start. The trait, its adapters, whether it is async, and where the SQLite adapter lives are decided and built in [Wire Ledger persistence to SQLite](https://github.com/IanTeda/Personal-Ledger/issues/636).

## Consequences

- Commands are tested without a fake repository: a test asserts on the recorded changes.
- The same change list can feed ADR-0009's Change Sets as the Client's sync outbox.
- Every command must record every change it makes, or the change is lost on save.

## Considered Options

- **A port per domain crate.** Rejected: it brings back the missing home for cross-domain writes that ADR-0038 removed, and puts I/O-shaped traits into crates that must stay I/O-free.
- **Commands call the repository directly.** Rejected: every command test would need a fake repository, and the commands would become async.

## Amendment: decided in ADR-0044

The trait is synchronous, applied on a serial writer thread with optimistic writes, and the SQLite adapter is the `lib-ledger-sqlite` crate (#685, ADR-0044).

## Amendment: per-domain ports (VIPER)

Superseded in part by `docs/development/viper-architecture.md` and its grilling. Each feature crate now owns its repository ports in `protocols.rs`, and `lib-database` implements them, so the "port per domain crate" option above is accepted. Its two objections are answered this way: a port is a trait with no I/O of its own, so the crate stays I/O-free, a read of another domain is a read port in the reading crate, and a write that spans domains (paying a Bill writes Transactions) belongs to a cross-domain module, one crate per user workflow, whose single port `lib-database` implements with one database transaction, so it still reaches storage as one atomic unit. The considered alternative, a shared unit-of-work port in `lib-core`, was rejected because every domain port would have to accept it. The single `LedgerRepository`, the `LedgerChange` list and the ADR-0044 writer thread stay until the transition tickets replace them.
