# `ratatui` + `crossterm` for the TUI, custom `Canvas` rendering for doughnut and divergent charts

The TUI feasibility cycle (`docs/product-requirements.md` FC-TUI-001–004) needs a Rust TUI
framework that can render line, doughnut, candlestick, and divergent charts plus tables,
working identically on Linux, macOS, and Windows. A primary-source survey (see
`docs/research/tui-charting-libraries.md`) compared `ratatui`, `cursive`, and `iocraft`.
We chose `ratatui`: it is the clear maintenance leader (22.5k GitHub stars, MIT, actively
published), whereas `cursive` has no charting capability at all and its crates.io publish
has been stale since August 2024, and `iocraft` has no chart widgets or `Canvas`-equivalent
drawing primitive to build on. For the terminal backend we chose `crossterm`, the only one
of the surveyed options confirmed to work across Linux, macOS, and Windows — `termion` is
Unix-only, which would break the FC-TUI-004 Windows requirement outright.

`ratatui`'s built-in `Chart` and `Table` widgets cover line charts and tables natively. For
the other two chart types, no adequate off-the-shelf crate exists: `tui-piechart` renders a
solid pie with no hollow centre, so it doesn't actually produce a doughnut, and no
diverging-bar crate exists anywhere on crates.io. We chose to build both directly on
`ratatui`'s `Canvas` widget rather than reach for a mismatched or nonexistent third-party
crate, keeping the charting layer on one consistent, self-owned foundation instead of mixing
off-the-shelf and custom code. The divergent chart demo is intentionally minimal — a plain
two-directional bar layout from a zero-line, proving the mechanism rather than matching the
Concept cycle's eventual visual polish — since this is feasibility-cycle scope.

Candlestick charts are the one case with a purpose-built crate, `chandelier`, but it is very
young (~3 months old, 217 downloads at time of research) and carries real
maintenance/reliability risk. Rather than committing to it or ruling it out up front, the
FC-TUI-002 candlestick demonstration ticket prototypes with `chandelier` first — the
cheapest way to find out whether that risk is real for our use case — and falls back to
custom `Canvas` rendering only if it proves inadequate.
