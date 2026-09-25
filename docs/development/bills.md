# Bills (development)

End-user documentation: [Bills](../bills.md).

## Overview

The Bills domain has two entities: a **Bill**, the recurring definition, and a **Bill Schedule** entry, an individual dated instance of it that a Transaction links to once paid. [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) settled the design; **none of it is built**.

What exists in the codebase today is navigation only: a `Noun::Bills` entry and a command-palette command that navigates to it. There is no table, no `lib-core` type, no `lib-database` module and no screen in either Client.

## Data model

Not yet built. ADR-0019 fixes the parts that are hardest to change later:

- **Bill Schedule entries are persisted rows, not computed on read.** Each carries its own `RowID` (UUIDv7, matching every other entity), generated ahead of its due date by a populate process. The rejected alternative — deriving dated instances virtually from the Bill's Recurrence on every read, the way `budget_period.rs` computes Budget period bounds live — fails because a Transaction needs a stable identifier to link against, and a recomputed instance has no identity that holds still across that link.
- **A Bill Schedule entry carries its own status**: Upcoming, Due, Overdue, Paid or Skipped.
- **`transactions.bill_schedule_id`** will be a plain nullable foreign key, the same shape the table already uses for Category, Payee and Account. No special-cased "virtual reference" concept is needed anywhere.
- **Paid is only ever reached by linking a real Transaction** — either created fresh from the Bill's defaults, or an existing Transaction edited to link. There is deliberately **no standalone "mark as paid" flag**. A ledger-independent checklist was the more obvious naive design and was rejected because it would let Bill Schedule data silently diverge from the Balance and Transaction data that Budget, reports and reconciliation all depend on.

The cost of this design, which ADR-0019 states plainly as a trade-off rather than a free win: Bill Schedule entries need **active maintenance**. Something must keep enough future entries populated, most likely on Client startup or on Bill create/edit, since Desktop and TUI have no always-on background service. It also means a Bill's Recurrence and `ends_on` are not pure metadata — editing either can require regenerating not-yet-Paid future entries.

## Domain types

None yet. `lib_core::BudgetPeriod` (`budget_period.rs`) is the closest existing analogue for recurrence handling and is worth reading before designing a Bill's Recurrence, but it is a different concept and is not reused.

## Persistence

Not yet built. No migration exists in `migrations/client/`, and `lib-database` has no bills module.

## UI

No Bills screen exists in either Client.

- **Desktop** — `Noun::Bills` in `nav.rs` (label via `lib_locale::msg::nav_bills()`), and a `bills` command in `command.rs` whose effect is `CommandEffect::Navigate(Noun::Bills)`. `view/` has no `bills` module, so the entry points lead nowhere built. Elsewhere, "bill" appears only in Transactions seed data — memo strings like "quarterly bill" and the "Bill looks high" special row in `transactions.rs` and `transaction_rows.rs`, which are mock content rather than Bills domain code.
- **TUI** — nothing. No `view/bills.rs`, no fixture module.

A Bill Schedule entry's status is one of the four facts Needs Attention surfaces, so [needs-attention.md](needs-attention.md) depends on this domain being built.

## Traceability

Omitted: nothing is built. Add the table here when `BIL-` requirements are written and something is ticked.

## Decisions

- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) — materialised, Transaction-linked Bill Schedule entries, and why Paid requires a real Transaction.
- [ADR-0020](../adr/0020-needs-attention-as-derived-view.md) — the contrast: Bill Schedule is materialised because it needs identity, Needs Attention is derived because it does not.
- `CONTEXT.md` — Bill, Bill Schedule, Recurrence, Transaction, Budget.

## Open questions and known gaps

- **Everything.** No schema, no types, no persistence, no screens.
- **The populate process has no home.** ADR-0019 names Client startup or Bill create/edit as the likely triggers but does not settle it, and neither Client has anywhere that work currently belongs.
- **Regeneration semantics on edit are unspecified** — exactly which not-yet-Paid entries are regenerated when a Recurrence or `ends_on` changes, and what happens to an entry already linked to a Transaction.
- **The nav entry and command navigate to a screen that does not exist.**
- **Budget's Known Costs (Bills) figure** described on the end-user page depends on this domain and is equally unbuilt.
