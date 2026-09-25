# Tags (development)

End-user documentation: [Tags](../tags.md).

## Overview

A Tag is a freeform, cross-cutting label attached many-to-many directly to Transactions, independent of their Category and Payee. [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) settled the domain model and vocabulary and **deferred schema, UI and reporting to a future build effort**.

That deferral is the thing to know before working here: there is **no `tags` table, no `lib-core` type and no `lib-database` module**. Everything that exists is UI built against fixture data — which, in the TUI, is a fairly complete screen. Nothing persists.

## Data model

Not yet built. No migration exists in `migrations/client/`, and nothing in `lib-database` references Tags.

ADR-0015 fixes these properties for whoever builds the schema:

- **Globally unique, case-insensitively.** The same shape as `payees.name`, which uses `TEXT NOT NULL UNIQUE COLLATE NOCASE`.
- **Many-to-many with Transaction**, so a join table is required. Note the end-user documentation and the TUI model attach Tags at the **Split** level in places and the Transaction level in others — see [Open questions](#open-questions-and-known-gaps).
- **`is_active` soft-delete lifecycle**, matching Unit, Category, Account and Payee.
- **No rename aliases**, unlike Payee. Nothing auto-creates a Tag from parsed import text that would later need reconciling under a rename, so a plain in-place rename is enough and no `tag_aliases` table is needed.
- **Never sum across Units.** A Tag's total only sums across Transactions sharing one Unit; a Tag spanning mixed Units reports per-Unit subtotals. This extends the rule `docs/ux/tui/accounts/README.md` already established for type-grouped subtotals.

## Domain types

None. There is no `lib_core::Tag`, and no `Label` implementation in `lib-locale` — a Tag's name is user data, not a translated Message, so it correctly has none.

## Persistence

Not yet built.

## UI

Both Clients render Tags from in-memory fixtures.

- **TUI** — the more complete of the two. `crates/bins/bin-tui/src/tag/` (`mod.rs`, `fixture.rs`) holds the model and data; the screen is `view/tags.rs`. The left pane is a flat, alphabetically sorted list, because a Tag has no classification dimension to group by. The right pane (issue #133, from the "Tags right pane" map, issue #131) implements `docs/ux/tui/tags/README.md`'s `9a` spec with three widgets: **Tagged spend** (a monthly sparkline mirroring `view::accounts::render_balance_chart`), **Where it lands** (a proportional category breakdown), and a summary box for the selected Tag. Its data comes from the fixtures landed by issue #132.
- **Desktop** — `crates/bins/bin-desktop/src/tags.rs` is a small `gpui`-free stub seeded from the TUI's fixture (which leads with "Japan Trip 2026") plus the neutral "shared" chip the Transactions mockup shows. `Tag` is `{ id: u32, name: String }`. There is a `Noun::Tags` nav entry and a command-palette entry, but **no desktop Tags screen** — `view/` has no `tags` module. Tags surface on the desktop only as chips in the Transactions views.

## Traceability

Omitted: nothing is built, and [tags.md](../tags.md) carries no requirement checklist. Add the table here when `TAG-` requirements are written and something is ticked.

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — Tag as an independent, many-to-many Transaction label rather than a grouping over Category or Payee, including why the rejected alternative cannot express a cross-cutting total over an untagged Category.
- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — the contrast that explains why Tag needs no aliases.
- `CONTEXT.md` — Tag, Category, Payee, Split, Unit.

## Open questions and known gaps

- **Nothing persists.** Schema, CRUD and reporting are all unbuilt, by ADR-0015's own scoping.
- **Transaction or Split?** ADR-0015 says Tags attach to Transactions, and [transactions.md](transactions.md)'s schema sketch has `split_tags` joining to a Split instead. These are different designs with different totals behaviour, and the discrepancy needs resolving before a migration is written.
- **The desktop has no Tags screen**, despite the nav entry and command that navigate to it.
- **Per-Unit subtotal behaviour is specified but unimplemented**, since there is nothing to total.
- **The two Clients' stubs are independent** and share no model.
