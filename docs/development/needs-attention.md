# Needs Attention (development)

End-user documentation: [Needs Attention](../needs-attention.md).

## Overview

Needs Attention surfaces four kinds of "this wants action" fact: open Tasks, Flagged Transactions, Transactions that are Cleared but not yet Reconciled, and unresolved Bill Schedule entries due on or before today plus their Bill Plan's Attention Lead (unset = 0).

[ADR-0020](../adr/0020-needs-attention-as-derived-view.md) is the whole design, and its central decision shapes everything here: **Needs Attention is a derived view, not a table.** Only Task — the one entry with no existing home elsewhere in the Ledger — is a persisted row. The other three facts already live as status on an existing row, so Needs Attention is a live, on-demand query recomputed on every view and never cached.

Nothing is persisted. There is no `tasks` table, no query, and no `lib-database` module. The desktop dashboard shows real Bill rows from the Bills stub (`bills::attention_entries`) alongside sample rows for the other three sources; the TUI's widget is entirely fake.

## Data model

Not yet built.

**Task** is the only thing that will ever be persisted here. The other three facts are read from existing columns:

| Fact | Source |
| --- | --- |
| Open Task | A `tasks` row (unbuilt) |
| Flagged Transaction | `transactions.flagged` |
| Cleared but not Reconciled | `transactions.status` |
| Bill inside its Attention Lead | A Bill Schedule entry's due date and resolution, and its Bill Plan's Attention Lead (desktop stub only — see [bills.md](bills.md)) |

ADR-0020's reasoning for rejecting a materialised Attention Item table is worth keeping in mind before anyone proposes one again. Unlike a Bill Schedule entry, which needs a stable identity for a Transaction to link against ([ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md)), none of the Flagged / Cleared-not-Reconciled / Overdue-Bill facts need a *new* identity. Materialising them would mean detecting "is there already a row for this still-unresolved fact" on every scan — an ADR-0019-shaped dedup problem — purely to duplicate state the Ledger already tracks, with the duplicate free to drift from its source.

The trade-off ADR-0020 accepts: **Needs Attention can never be queried as its own list-with-history** the way Bill Schedule can. There is no stored record of what needed attention last week, only what needs it now. A fact resolves itself off the list the instant its underlying state changes, with nothing to separately dismiss or garbage-collect.

## Domain types

None yet. `lib_core::TransactionStatus` (`transaction_status.rs`) supplies the Cleared and Reconciled values two of the four facts are read from, and it already exists.

## Persistence

Not yet built. No `tasks` migration, and no query module. Two of the four inputs — Task and Bill Schedule — have no persistence yet, so the joining query cannot be written in full even if someone wanted to.

## UI

Both Clients render a Needs Attention box on the dashboard from fake data.

- **TUI** — `crates/bins/bin-tui/src/view/dashboard.rs`: `render_needs_attention` draws a heading, an item list and a "more" row, fed by an `AttentionItem` slice from `fake_attention_items()`. Covered by the test `needs_attention_shows_heading_items_and_the_more_row`, which asserts against an 80×10 `TestBackend`.
- **Desktop** — `crates/bins/bin-desktop/src/view/dashboard.rs`: a `needs_attention()` widget rendering a heading from `msg::desktop_dashboard_needs_attention()` above real Bill rows from `bills::attention_entries` (#377) and stub rows for the rest. `rail/primary.rs` shows the Bill count as the Bills badge.

Neither reads persisted data, and neither has a dedicated screen — Needs Attention exists only as a dashboard panel.

## Traceability

Omitted: nothing is built. The domain's requirement prefix is `NAT`. Add the table here when `NAT-` requirements are written and something is ticked.

## Decisions

- [ADR-0020](../adr/0020-needs-attention-as-derived-view.md) — Needs Attention as a derived view, why only Task persists, and the accepted loss of history.
- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) — the deliberate contrast in the other direction.
- `CONTEXT.md` — Task, Transaction, Bill Plan, Bill Schedule, Transaction Status.

## Open questions and known gaps

- **Everything.** No `tasks` table, no derived query, no real data in either dashboard widget.
- **Two of the four inputs are not persisted.** Task is unbuilt, and Bill Schedule exists only as the desktop stub ([bills.md](bills.md)), so only the two Transaction-sourced facts could be queried today.
- **Recomputing on every view has no measured cost.** ADR-0020 chose "never cached" on correctness grounds; nothing has profiled it against a large Ledger.
