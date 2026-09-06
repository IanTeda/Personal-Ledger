# `Shell`/`View` replaces the breadcrumb-stack `App`/`Screen` navigation model

`docs/ux/tui/README.md` (a Claude Design handoff) specifies a modal, command-driven shell —
status line, one full-bleed view region, command line, keybind hint bar — with a floating `:`
command window as the *only* navigation affordance and no persistent nav chrome. This is a
different shape from the breadcrumb `Vec<Box<dyn Screen>>` stack `App` locked in under
ADR-0003: that stack renders a visible `A > B > C` breadcrumb and is driven by per-screen
`Action::OpenX` variants triggered from a menu-of-areas dashboard, neither of which the new
handoff has room for.

We chose to replace the navigation layer rather than extend it in place: `App` becomes `Shell`,
the `Screen` trait becomes `View` (`screen/` → `view/`), the breadcrumb is dropped, and all
navigation routes through the shell's command window and a shared action registry (palette,
keybind column, and help sheet all generated from one source, per the handoff's grammar). This
amends the navigation-shape portion of ADR-0003; the Elm/Component split it locked in (a
top-level `Action` enum, per-screen `update`/`view` behind a small trait) is unchanged and
still the shape `View` implements.

The nine existing, real, data-wired screens (Accounts, Categories, Transactions, Payees,
Balance Checks, Budgets, Reports, Settings, CSV import) and the five feasibility-cycle chart
demos (`line_chart.rs`, `doughnut_chart.rs`, `candlestick_chart.rs`, `divergent_chart.rs`,
`table.rs`) are not migrated as part of this decision. The old `app.rs`/`screen/` module is left
compiling but disconnected from `main.rs` — dead code, not a live fallback path — until each
screen is individually redesigned and migrated into `view/` behind its own Claude Design
handoff, the same process this handoff (shell + dashboard + command window) is the first
instance of.

## Considered Options

Keeping `App`/`Screen` as-is and layering a `Shell` wrapper around the existing stack (status
line/hint bar replacing the breadcrumb row, command window pushing onto the same stack) was
rejected: the handoff's navigation model has no stack visible to the user and no per-screen
`Open` actions, so preserving the old stack underneath would mean maintaining two competing
navigation shapes at once for no reader's benefit.
