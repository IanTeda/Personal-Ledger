# A hybrid Elm/Component architecture for the TUI, async from the start

`ratatui`'s own documentation names three application architecture patterns (see
`docs/research/tui-application-architecture.md`): The Elm Architecture (TEA), Component
Architecture, and Flux. Each has a documented gap for Personal Ledger's purposes. TEA
centralises all state behind a single `Model`/`update`, so one function ends up knowing
about every message app-wide — workable for a small demo, but Personal Ledger's TUI is
already specified to need five-plus real screens past the feasibility cycle (accounts,
transactions, categories, budgets, balance checks — see `docs/product-requirements.md`
FR.4–FR.38), not a hypothetical future requirement to design around speculatively. Component
Architecture solves that by giving each screen its own state behind a `Component` trait, but
ratatui's own docs describe no story for composing components together. Flux's docs are the
thinnest of the three, state no tradeoffs, and its own example doesn't even use Flux's
distinguishing multi-store shape, so we ruled it out without further consideration.

We chose a **hybrid**: a top-level `Action`/`Message` enum and a top-level `App` that owns
which screen is active, delegating `update` and `view` to a per-screen `Model` implementing a
small trait. This keeps TEA's predictable, centrally-routed message flow while giving each
screen the same local cohesion Component Architecture offers — at the cost of being a
combination ratatui's own docs don't walk through directly; no tutorial matches it exactly.
The official Component `cargo-generate` template (the only ratatui-provided scaffolding with
generator tooling) is shaped around the pure Component trait rather than this hybrid, so we're
treating it as a one-off reference for project layout and tooling conventions (Cargo.toml
shape, CI config, error-handling choices) rather than generating from it directly — its trait
shape doesn't fit and adapting it in place would be more work than hand-writing the hybrid
from ratatui's TEA tutorial.

We're also adopting ratatui's documented async extension (`tokio::select!` multiplexing
tick/render/input, plus a channel for background tasks to report results back) from the first
ticket, rather than starting synchronous and retrofitting later. `lib-database`'s `sqlx` is
already compiled with the `runtime-tokio-rustls` feature, and the server binary already runs
on `tokio`, so once FC-TUI-005 (the real end-to-end SQLite demo) wires the TUI to
`lib-database`, async becomes unavoidable regardless of this decision. Retrofitting an event
loop from sync to async touches every screen's plumbing at once; building on the async
skeleton from the start avoids a rewrite of a cost that's already certain to land.
