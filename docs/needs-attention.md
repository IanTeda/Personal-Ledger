# Needs Attention

Personal Ledger surfaces one always-current list of everything that wants a user's action — an unreconciled Transaction, a Transaction flagged for review, a Bill that's overdue, or a plain money-related to-do the user jotted down themselves — so there's one place to check rather than hunting across Accounts, Bills, and Reports separately. This document explains how the one thing a user creates directly (Task) relates to the three things the Ledger surfaces on its own, and how the combined list behaves.

See `GLOSSARY.md` for the one-paragraph canonical definition of Task and Needs Attention, and [ADR-0020](adr/0020-needs-attention-as-derived-view.md) for why Needs Attention is computed fresh on every view rather than stored as its own table. This document is where the fuller shape and workflow live, the same way `docs/bills.md` covers Bill Plan and Bill Schedule.

## Why Task is the only persisted thing here

Needs Attention has four sources, but only one of them is new data:

- **Task** — a plain to-do a user typed in themselves. Nothing else in the Ledger already tracks "I need to call the bank about this," so it needs a real, persisted row.
- **Flagged Transactions**, **Cleared-but-not-Reconciled Transactions**, and **Bill Schedule entries inside their Attention Lead or Overdue** — each is already a fact living on an existing row: a Transaction's Flagged marker, a Transaction's Status, a Bill Schedule entry's own status.

Rather than copying those three facts into a second table of their own, Needs Attention reads them directly off the rows that already carry them, every time the list is viewed. This mirrors the Anticipated Bill preview (`docs/bills.md`) — a computed view standing in for something that isn't its own stored row — rather than Bill Schedule, which *is* materialized because a Transaction needs a stable id to link against. See [ADR-0020](adr/0020-needs-attention-as-derived-view.md) for the full reasoning, including the alternative (a single materialized "Attention Item" table) that was considered and rejected.

## Task

A Task is created directly by the user, whenever a money-related to-do doesn't already fit anywhere else in the Ledger.

| Field | Meaning |
| ----- | ------- |
| Description | Free text — what needs doing (e.g. "call the bank about this charge"). |
| Linked Transaction | Optional. Points at one Transaction for context. Independent of that Transaction's Flagged marker — linking doesn't require Flagged, and Flagging doesn't create a Task. |
| Status | Open or Done. |

### Task lifecycle

A Task starts Open and is marked Done once it's handled — there's no separate "Dismissed" state, since done is done the same way there's no "un-flag with a reason" on a Transaction. A Done Task's row is kept, not deleted, the same soft-delete convention as Account/Category/Payee/Tag/Bill Plan's `is_active` — Needs Attention simply filters to Open Tasks, the same way it only ever shows unresolved facts from the other three sources.

## What shows up in Needs Attention

Every time the list is viewed, it's built fresh from four sources:

| Source | Included when | Disappears when |
| ------ | -------------- | ---------------- |
| Task | Status is Open | Marked Done |
| Transaction | Flagged | Un-flagged |
| Transaction | Status is Cleared, not yet Reconciled | Moved to Reconciled (a plain Open Transaction hasn't even been confirmed yet, so it isn't included) |
| Bill Schedule entry | Unresolved, and due on or before today plus that Bill Plan's Attention Lead (unset = 0) | Paid, or Skipped |

Flagged Transactions and Cleared-but-not-Reconciled Transactions are shown as aggregated counts (e.g. "14 unreconciled transactions", "3 transactions flagged for review") rather than one row per Transaction. Bill Schedule entries inside their Attention Lead or Overdue are shown individually by Bill Plan name instead — each is a distinct, recognisable obligation worth naming on its own, the same way Budget's Known Costs (Bills) itemises by Bill Plan rather than reporting one lump sum, rather than an interchangeable stack the way plain Transactions are. In the desktop app these Bill rows are real, and the Bills entry in the side rail carries the same count as a badge; the other three sources are still sample rows.

### Bills and Attention Lead

A Bill Schedule entry's Due/Overdue status (`docs/bills.md`) is unchanged by any of this — it's still computed purely from the due date. Attention Lead is a date rule, separate from status: an unresolved entry counts once its due date is on or before today plus the lead, even while it's still Upcoming because it falls due early next month. It's an optional, per-Bill Plan day count (unset by default, meaning zero: the entry counts from its due day) rather than a single global setting, since how early a bill deserves attention varies from one Bill Plan to the next — a large recurring rent payment might warrant a week's notice, a small subscription none at all.

### Example

A Telstra Internet Bill Plan, due the 25th, has no Attention Lead set (counted from its due day); an Insurance Bill Plan, due the 20th, has an Attention Lead of 5 days. Viewed on the 16th:

- Telstra, Due the 25th — **not** shown in Needs Attention, even though it's coming up; it's visible on the Bills Schedule tab as usual.
- Insurance, Due the 20th, within its own 5-day Attention Lead from the 15th onward — **shown** in Needs Attention by name, ahead of actually being late.
- Rent, due the 1st of next month with a 5-day Attention Lead — not shown on the 16th, but viewed on the 28th it's **shown** while still Upcoming, because the lead crosses the month boundary.

## Relationship to Bills

`docs/bills.md`'s Needs Attention section describes Bill Schedule entries surfacing on the Dashboard once inside their Attention Lead — that surfacing *is* Needs Attention; the two documents describe the same list from each entity's own vantage point. Needs Attention adds Flagged Transactions, Cleared-but-not-Reconciled Transactions, and Tasks alongside it, so a user checks one list rather than a Bills-specific one and everything else separately.

## What's deliberately out of scope here

This document captures the shape agreed so far — it is not a functional spec. The desktop Dashboard's Bill rows follow the rule above on sample data; nothing else is built yet (no migration, table, or screen exists for Task). Left for later, once that work is actually scoped:

- The exact dashboard layout and truncation rule for the summary widget (how many rows show before "...more"), and the design of a dedicated, fully browsable Needs Attention view — the TUI's existing dummy dashboard code has a placeholder command that should be renamed from `:to-do` to `:actions` wherever it's referenced, but its screen isn't designed here.
- Any formal Alert/Outstanding severity tier — purely a display decision (already sketched as `is_alert` in the dummy dashboard code), not a domain concept.
- A calendar-driven "end of month" system category — dropped for simplicity; a user wanting a recurring monthly check-in creates their own Task manually. Task itself carries no Recurrence the way a Bill Plan does.
- OS-level push notifications or a background reminder service — same Future Consideration `docs/bills.md` already defers; opening the app is what surfaces Needs Attention in V1.

## For developers

Curious how needs attention are structured in the codebase, or planning to change them? See the [Needs Attention development documentation](development/needs-attention.md).
