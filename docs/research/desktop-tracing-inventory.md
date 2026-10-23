# Research: what the Desktop logs today

**Question.** Issue [#466](https://github.com/IanTeda/Personal-Ledger/issues/466) asked whether the Desktop Client's logging is too verbose and should be rationalised. This note inventories what the Desktop actually emits, at each level, in the code it runs, as of `concept` at 84a84dc. It is an inventory of this codebase only: a survey of comparable gpui and Rust apps was dropped in grilling, since the answer turns on how this workspace is layered rather than on what other apps do.

**Finding.** The Desktop is close to silent, not verbose. The tracing conventions in the `/tracing` skill were written for the async per-entity database code and have not carried into the VIPER crates the Clients now run.

## Call sites by crate

Counted as `#[tracing::instrument]` attributes (spans) and `trace!`/`debug!`/`info!`/`warn!`/`error!` macros (events).

| Crate | Spans | Events | Reached by a Client |
| --- | --- | --- | --- |
| `bin-desktop` | 0 | 8 | Yes |
| `lib-database` | 93 | 132 | Only through `open_storage` |
| `lib-locale` | 0 | 16 | Yes |
| `lib-tracing` | 0 | 11 | Yes |
| `lib-rpc` | 2 | 3 | Sync Server only |
| `lib-config` | 0 | 1 | Yes |
| Domain and VIPER crates (`lib-units`, `lib-bills`, `lib-bill-payments`, `lib-viper`, `lib-ledger`, the rest) | 0 | 0 | Yes |

## The Desktop bin

Eight events, none per key press or per render:

- `error!` for a Ledger that fails to open, the last writes failing to save on quit, the bundled fonts failing to register, the main window failing to open, and a reload failing after a failed write (`app/write_failures.rs`).
- `warn!` for a Colour Theme pair below its contrast rule (`theme/colours.rs`).
- `info!` when the Bill Schedule is generated for a new month, `debug!` when it is generated on start (`bills/ports.rs`).

## lib-database

Both Clients reach `lib-database` only through `lib_database::open_storage`. The code that path runs, `adapter/*.rs`, `writer.rs` and `storage.rs`, holds about 14 call sites: `debug` spans with `err` on each domain's Load, Reload and Seed, a `debug` span on Open the Ledger, and `error!` when the writer thread stops or a write fails. A write that succeeds emits nothing.

The other ~210 call sites are in the per-entity modules (`categories/`, `payees/`, `accounts/`, `budgets/`, `balance_checks/`, `change_sets/`, …), which neither Client calls. They are legacy that will be folded into `adapter/<domain>.rs` or deleted as each domain migrates (ADR-0041), and do not set the convention.

## Spans are invisible on their own

`lib_tracing::init` builds its `fmt` layers without span events (no `FmtSpan`), and the capture layer behind the Settings › Tracing page implements only `on_event`. So an `#[instrument]` span shows nowhere by itself: it only prefixes events emitted inside it, and its `err` emits an `ERROR` event when the function returns `Err`. A successful Load leaves no line at any level.

## What a reader sees

- **Console and log file:** filtered by the `log` Configuration (default `info`), `RUST_LOG` overriding, `calloop` capped at `warn`. At `info` a normal session shows startup lines from `lib-tracing`, `lib-locale` and dependencies, and the Bill Schedule line once a month.
- **Settings › Tracing page:** a ring of the last 1000 events (`LOG_CAPACITY`), recording our own crates at `debug` and dependencies at the console's level. Its level radios (Error, Warn, Info, Debug) filter the view only, retroactively, for the session; they are not a Preference and do not change what is recorded.

## The current convention

The `/tracing` skill asks for `debug` `#[instrument]` (with `err`) on functions doing database work, business logic, external calls or input handling, `info` for create, update and delete, and no `debug`/`trace` in hot paths. `docs/tracing.md` describes the levels in general terms. Neither says which VIPER layer logs, and no VIPER crate has applied either.
