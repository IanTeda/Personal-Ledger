---
name: tracing
description: Add or review tracing::instrument spans and log statements in this repo. Use when instrumenting a new function, writing an Interactor or adapter, or deciding what level/fields a trace event should have.
---

# Tracing

Who logs what is decided in ADR-0047 (`docs/adr/0047-interactors-log-ledger-changes-and-presenters-never-log.md`). A log exists to rebuild what happened to the Ledger before a failure, for the maintainer and for a user attaching it to a bug report. It is not for monitoring.

## By VIPER layer

- **Interactor:** one `info!` per intent that changes the Ledger, with its outcome: done, or refused with the refusal. A refusal is `info`, not `warn`. Reads (filters, searches, actuals) never log.
- **Presenter, View, Router:** never log.
- **Adapter and writer (`lib-database`):** `#[tracing::instrument(level = "debug", skip_all, err)]` on functions returning `Result` (load, reload, seed, open), `debug!` for how a change reached storage, `error!` for a failed or lost write.
- **Bin:** startup and wiring (Ledger opened, window opened), `error!` for what fails there.
- The Sync Server and `lib-database`'s per-entity modules (`categories/`, `payees/`, …) are outside this; don't copy their style into a feature crate.

## Levels

- **ERROR** — work lost or not done: a failed write, a Ledger that won't open, a window that won't open.
- **WARN** — system trouble that recovered: a Colour Theme pair below contrast, a Configuration fallback, a retry. Never a refusal.
- **INFO** — a change to the Ledger and its outcome, startup and shutdown.
- **DEBUG** — how storage was reached: loads, reloads, row counts, writer jobs.
- **TRACE** — not used in the Clients; the Tracing page never records it.

## An Interactor

Explicit events, not `#[instrument]` (span events are off, so a span alone prints nothing):

```rust
pub fn run(self, units: &mut dyn UnitRepository, /* … */) -> Result<UnitRow, DeleteUnitRefusal> {
    let outcome = self.apply(units, /* … */);
    match &outcome {
        Ok(_) => tracing::info!(unit_id = %self.id, "Deleted a Unit"),
        Err(refusal) => tracing::info!(unit_id = %self.id, %refusal, "Refused to delete a Unit"),
    }
    outcome
}
```

## What a field may hold

- Ids, domain and kind names, counts, dates, the Ledger file path, a refusal's own text.
- Never a Money amount, a name (Payee, Account, Bill, …), notes or any user-entered text, a token, password or Configuration secret. This holds at every level, `debug` included: the Tracing page records `debug` and its contents may be shared.
- `skip_all` on `#[instrument]`, then add back only fields that meet this rule.

## Avoid

- Logging per key press, per render or per read.
- `#[instrument]` in a feature crate.
- Turning on span events (`FmtSpan`).

## Before finishing

- Each Interactor that writes logs one `info!` for done and for refused; no read logs.
- Adapter functions returning `Result` carry `err`.
- No field holds an amount, a name or user-entered text.
