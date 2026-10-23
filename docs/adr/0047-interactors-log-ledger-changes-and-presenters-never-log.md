# Interactors log Ledger changes; Presenters never log

Issue [#466](https://github.com/IanTeda/Personal-Ledger/issues/466) asked whether the Desktop's logging was too verbose. The inventory (`docs/research/desktop-tracing-inventory.md`) found the opposite: the Desktop bin has eight log events, the VIPER feature crates none, and the spans the Clients do run print nothing unless they fail, because no layer turns on span events. The `/tracing` skill was written for the async per-entity database code and says nothing about VIPER layers. The logs' readers are the maintainer debugging a live session and, second, an end user attaching a log to a bug report; nobody monitors or alerts on a Client. So a log exists to rebuild what happened to the Ledger before a failure.

## Decision

- **Interactors log each intent that changes the Ledger, at `info`**: create, update, delete, pay, generate, with the outcome, done or refused ("Delete Payee 7: refused, has Transactions"). A refusal is a normal outcome, so it is `info`, not `warn`.
- **Reads are not logged** at any level, Interactor or Presenter, since a filter or search can run on every keystroke and leaves the Ledger as it was.
- **Presenters, Views and Routers never log.** Their state (selection, focus, which Dialog is open) is UI churn that rebuilds no failure.
- **Adapters and the writer log how a change reached storage, at `debug`,** and a lost or failed write at `error`. Their `Result`-returning functions keep `#[tracing::instrument(level = "debug", err)]` for the failure event; feature crates use explicit `info!`/`debug!` events, not `#[instrument]`. Span events (`FmtSpan`) stay off.
- **`warn`** is for system trouble that recovered (a Colour Theme pair below contrast, a Configuration fallback), **`error`** for work lost or not done.
- **A log line carries ids, domain and kind names, counts and dates only:** never a Money amount, a name (Payee, Account, Bill, …), notes or other user-entered text, or a Configuration secret, at any level. The Ledger file path is allowed. This is what makes a log safe to attach to a bug report.
- **`info` stays the `log` default,** and the Settings › Tracing page keeps recording our crates at `debug` with its level radios as a session view filter.
- The rule covers the feature crates and both Clients, which run the same Interactors. The Sync Server, read by an operator, keeps its request-level spans. `lib-database`'s per-entity modules, which no Client calls, are legacy and don't set the convention.

## Considered Options

- **Only the edges log** (adapters, writer, bin startup). Rejected: the writer knows only that a Bills job ran, not that the person paid Bill 12, so a bug report can't be rebuilt from it.
- **Every layer logs, Presenter events included.** Rejected: one line per key press and selection change buries the few lines that matter, and the Tracing page's 1000-entry ring would hold seconds of history.
- **Turn on span-close events for timings.** Rejected: it doubles the lines for timings nobody has asked for.

## Consequences

- Feature crates take a `tracing` dependency. It is a logging facade, not I/O, so they stay I/O-free.
- The migrated crates with Interactors (`lib-units`, `lib-bills`, `lib-bill-payments`) need their Interactors instrumented, and every later slice does it as it migrates.
- With no `log_file_path` by default, a user launching from a desktop launcher still has only the Tracing page's ring; a Save logs action on that page is the follow-up for bug reports.
