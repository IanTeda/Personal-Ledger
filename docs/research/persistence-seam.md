# Research: the persistence seam between the domain libs and lib-database

**Question.** What is the persistence seam between the domain lib crates and `lib-database`, and what must change in `lib-database` before persistence wiring starts? Asked in [#597](https://github.com/IanTeda/Personal-Ledger/issues/597) on the [#588](https://github.com/IanTeda/Personal-Ledger/issues/588) map. Decided context: ADR-0038 puts every read and write behind a synchronous, in-memory `Ledger` in a new `lib-ledger` crate, domain crates depend only on `lib-core`, and the Desktop holds one `LedgerStore`. The comment on #597 from #592 already places the port on `Ledger`, not on each domain crate.

## 1. How comparable Rust apps put a port over SQLx

**Trait over the store, one implementation per backend.** [Atuin](https://github.com/atuinsh/atuin/blob/main/crates/atuin-client/src/database.rs) defines `pub trait Database: Send + Sync + 'static` with async methods (`save`, `load`, `list`, `delete`, …) and implements it once, as `Sqlite`, over an `sqlx::SqlitePool`. The trait is the port; callers hold the trait. It is one store-wide trait, not a trait per entity.

**Sync domain, async I/O at the edge.** Luca Palmieri's *Zero To Production in Rust* ([`zero2prod`](https://github.com/LukeMathWalker/zero-to-production)) keeps domain types (`SubscriberName`, `SubscriberEmail` in `src/domain/`) pure and validating, and does SQLx work in plain async functions in the route handlers that take a `&mut Transaction<'_, Postgres>`. The domain never sees `async` or a pool. This is the "functional core, imperative shell" shape: parse and decide in pure code, then persist.

**Per-entity CRUD traits.** [Lemmy](https://github.com/LemmyNet/lemmy/blob/main/crates/db_schema/src/traits.rs) defines a generic `Crud` trait (`create`, `read`, `update`, `delete`) implemented per table in its `db_schema` crate (Diesel, not SQLx). It suits a server whose handlers each touch one or two tables. Its cost is visible there too: cross-table operations live in the API crates and pass a connection through by hand.

**Unit of work in SQLx itself.** SQLx's unit of work is [`sqlx::Transaction`](https://docs.rs/sqlx/0.8.6/sqlx/struct.Transaction.html): `pool.begin()` then `commit()`, rolled back on drop. Any function taking [`impl sqlx::Executor`](https://docs.rs/sqlx/0.8.6/sqlx/trait.Executor.html) or `&mut SqliteConnection` can run inside one, so per-entity query functions compose into one atomic write without a separate unit-of-work type. A cross-domain command (pay a Bill, delete a Category and re-point Splits) needs exactly this: one transaction spanning several tables.

**Async in traits.** Since Rust 1.75, `async fn` in traits is stable ([Rust blog, Dec 2023](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html)), but such traits are not `dyn`-compatible and the returned future's `Send` bound cannot be named by callers without `trait_variant` or `#[async_trait]`. A port that must be async and object-safe still needs [`async-trait`](https://docs.rs/async-trait) (Atuin uses it).

**Sync over async.** SQLx is async-only ([README](https://github.com/launchbadge/sqlx#readme)); a synchronous caller has to block on a runtime ([`tokio::runtime::Runtime::block_on`](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html#method.block_on)) or hand the work to a background executor. GPUI offers [`cx.background_spawn`](https://docs.rs/gpui/latest/gpui/struct.App.html) for exactly this, returning a `Task` the Entity can await and then `notify`.

**Takeaways.** Real apps either (a) keep one store-wide trait (Atuin) or (b) keep the domain pure and do persistence in the shell after the decision is made (zero2prod). Per-entity repository traits (Lemmy) fit servers with thin handlers, not a Client with cross-domain commands. None of them makes the domain itself async.

## 2. Recommendation: port location and shape

**Location: on `Ledger`, in `lib-ledger`, not in each domain crate.** This matches #592's input and ADR-0038: `Ledger` already owns every collection and is the only place that sees a cross-domain write whole, so it is the only place that can describe it as one atomic unit. A port per domain crate would bring back the problem ADR-0038 removed (no one place for a cross-domain write) and would put I/O-shaped traits into crates that must stay I/O-free.

**Shape: commands stay synchronous and pure; they emit a change list; an adapter persists it.** Concretely:

- `Ledger` commands keep their ADR-0038 shape (`fn pay_bill(&mut self, …) -> Result<Outcome, PayBillRefusal>`) and, on success, record what changed as a `Vec<LedgerChange>` (e.g. `Upsert(Transaction)`, `Delete(PayeeId)`), drained with `ledger.take_changes()`. The rules, refusals and tests stay synchronous and in memory.
- The port is one trait in `lib-ledger`: `trait LedgerRepository { fn load(&self) -> Result<LedgerSnapshot, RepoError>; fn apply(&self, changes: &[LedgerChange]) -> Result<(), RepoError>; }`. It is store-wide (Atuin's shape), not per entity. `apply` runs all changes of one command in one `sqlx::Transaction`.
- Make the trait synchronous and let the SQLite adapter block on its own runtime, or make it `async` and call it from `LedgerStore` through `cx.background_spawn`. Recommended: async (`#[async_trait]` or `trait_variant`), because the Desktop must not block the UI thread, and the stub adapter just returns ready futures. `Ledger` itself never awaits.
- **Adapter 1, stub:** `StubRepository` in `lib-ledger` returning `Ledger::stub()`'s data from `load` and discarding or recording `apply` (tests assert on the recorded changes). This keeps UI-first development and the headless UI tests (ADR-0030) unchanged.
- **Adapter 2, SQLite:** in `lib-database` (or a thin `lib-ledger-sqlite` if `lib-database` should not depend on `lib-ledger`), mapping `LedgerChange` to the existing per-table `insert`/`update`/`delete` functions run against `&mut SqliteConnection` inside one transaction. Dependency direction: adapter crate → `lib-ledger` → domain crates → `lib-core`; `lib-ledger` never depends on `lib-database`.
- **Write path:** optimistic. `LedgerStore` runs the command in memory, emits `LedgerChanged`, then spawns `apply`. On failure it reloads (or reverts) and raises a Toast. For a single-user local SQLite file a failed write is rare and this keeps the UI synchronous.
- **Why a change list rather than a repository called from inside commands:** commands stay testable without a fake repository; the same change list is what the Sync Server's per-field LWW Change Sets (ADR-0009) need, so the Client outbox can be built from it later.

The TUI's `PayeeStore` trait (`crates/bins/bin-tui/src/payee/mod.rs`) is a per-domain, synchronous read port; under ADR-0038 it is replaced by `Ledger` reads, not promoted to the persistence port.

## 3. The id type and ADR-0009

**Recommendation: move to UUIDv7 ids, as typed `lib-core` newtypes over `RowID` (`PayeeId(RowID)`, …), before persistence wiring.**

- ADR-0009 identifies Change Sets and target rows by `RowID` (UUIDv7) and the schema already stores ids as `TEXT` UUIDs (`crates/libs/lib-core/src/row_id.rs`, `lib-database` migrations). Offline Clients that create rows independently need ids that cannot collide across devices without a server round trip; a per-Client `u32` sequence collides on the first sync. UUIDv7 ([RFC 9562 §5.7](https://www.rfc-editor.org/rfc/rfc9562#section-5.7)) is time-ordered, so it also keeps SQLite B-tree inserts mostly appending.
- Keeping `u32` in memory with a mapping table to `RowID` doubles the id space and needs a lookup on every write and sync; no comparable app does this for local-first data.
- ADR-0038 already moves cross-domain ids into `lib-core` newtypes, so the switch is one change per newtype plus the seeds. The stub seeds need deterministic ids: derive them with `Uuid::new_v8`/fixed constants or a seeded generator so UI tests stay stable.
- **Effect on ADR-0009:** none in substance; it is confirmed and extended to Client rows. Its text cites `crates/libs/lib-domain/src/row_id.rs`, which is now `lib-core`; that path is stale. A new ADR should record "Ledger ids are `lib-core` newtypes over `RowID`", superseding the `u32` ids, and reference ADR-0009.
- **One cost to note:** `lib-core` depends on `sqlx` because `RowID` implements `sqlx::Type`/`Encode`/`Decode`. Domain crates therefore pull SQLx transitively, which is not I/O but is weight. Optionally move those impls behind a `sqlx` feature on `lib-core`, enabled only by `lib-database`.

## 4. Keep or collapse the per-entity file split

**Recommendation: collapse to one file per entity (`payees.rs` holding the row struct and its queries), with a separate module only when it grows past a few hundred lines.** `payees/` is nine files and 1,094 lines; `delete.rs` is 73 lines, mostly doc comment, and `builder.rs` is `with_*` setters over `pub` fields. The split adds navigation cost without hiding anything: every file is a thin `impl Payees` block. SQLx's own [examples](https://github.com/launchbadge/sqlx/tree/main/examples/sqlite/todos) keep queries for a table together, and Atuin's whole SQLite store is one `database.rs`.

- Drop the builders: with `LedgerChange` carrying full domain values, the adapter maps a domain struct to a row directly.
- Change query functions to take `impl sqlx::Executor<'_, Database = Sqlite>` (or `&mut SqliteConnection`) instead of `&Pool`, so they compose inside one transaction. Today they take `&sqlx::Pool<sqlx::Sqlite>` (e.g. `payees/find.rs`), which forbids a multi-table atomic write.
- Rework the schema before wiring, as the ticket context lists: named, effective-dated Budgets (ADR-0028/0029), Splits on Transactions, and tables for bills, documents, inventory, tags and institutions. Per CLAUDE.md, edit each `create_<table>_table.sql` rather than adding `ALTER`s.
- If adopted, CLAUDE.md's `lib-database` bullet, ADR-0009's "same per-entity file split" sentence and `/rust-style`'s persistence section change with it.

## 5. What must change in lib-database before wiring

1. Schema brought up to the current domain (Budgets, Splits, missing tables).
2. Query functions take an executor/connection, not a pool.
3. Ids are `RowID`-backed newtypes shared with the domain crates.
4. An adapter implementing `LedgerRepository` (`load` + transactional `apply`).
5. Optionally, the file-split collapse and builder removal.

## Proposed ADRs (for the user to decide)

- **Ledger persistence port:** one store-wide `LedgerRepository` on `lib-ledger`, commands emit `LedgerChange`s, stub and SQLite adapters, optimistic writes via `LedgerStore`.
- **Ledger ids are UUIDv7:** `lib-core` newtypes over `RowID`; references ADR-0009 and fixes its stale path.
- **lib-database layout:** one file per entity, executor-taking queries, no builders; amends CLAUDE.md and ADR-0009's file-split sentence.
