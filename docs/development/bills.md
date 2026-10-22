# Bills (development)

End-user documentation: [Bills](../bills.md).

## Overview

The Bills domain has two entities: a **Bill Plan**, the recurring definition, and a **Bill Schedule** entry, one dated instance of it that is Matched to a Split once paid. [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) and its amendments settled the design, and the [Desktop Bills Surface](https://github.com/IanTeda/Personal-Ledger/issues/364) map settled the details (#365 recurrence anchor, #366 statuses, #367 generation and edits, #368 settlement, #369 History figures, #381 History merged into the Schedule).

The desktop app builds the whole Bills surface (handoff `docs/ux/desktop-mockups/12-bills/`, screens 8a–8f, with 8f's History tab folded into the Schedule). Bill Plans and the Bill Schedule persist to SQLite through `lib-bills`' ports ([#777](https://github.com/IanTeda/Personal-Ledger/issues/777)), while the Bills View still writes through `lib-ledger`'s `bill_commands` and the Desktop's forwarding shim. The TUI has no Bills screen.

## Data model

ADR-0019 fixes the parts that are hardest to change later:

- **Bill Schedule entries are persisted rows, not computed on read.** A Transaction needs a stable identifier to Match against, and a recomputed instance has no identity that holds still across that link. The id is a UUIDv7 (`BillScheduleEntryId`), and `(bill_plan_id, due)` is unique: generation looks an entry up by its Bill Plan and due date (`EntryId`) before adding one, so a date never has two rows (ADR-0019 as amended).
- **Paid is only ever reached by a Match to a real Split**, one-to-one: by Pay (a Pending Transaction created from the Plan's defaults, dated today) or by Match (an existing unmatched Expense Split in the Plan's Unit within ±14 days of the due date, Payee matches first). There is deliberately no standalone "mark as paid" flag. Unmatch and Unskip reverse, and deleting a Matched Split Unmatches.
- **Statuses are derived by calendar month** (Upcoming/Due/Overdue) from `today`, with Paid and Skipped stored as the entry's resolution. Overdue and Due entries carry into every other month's Schedule, once; `schedule_rows(.., None, ..)` is All. Unresolved rows sort first (next due first), resolved after (most recent first). Needs Attention is a separate date rule: unresolved and due on or before today plus the Attention Lead.
- **Generation runs through the end of next calendar month**, always keeping at least one unresolved future entry per active Plan, and skips a due date whose Recurrence period already holds a Paid or Skipped entry.
- **Nothing is deleted.** An edit to Recurrence, First Due or Ends On, or deactivating, marks unresolved entries due today or later as superseded; Overdue, Paid and Skipped entries are never touched.

In memory the entry holds its Match as `Resolution::Paid(SplitRef)`. On disk the Match is the Split's optional, unique `splits.bill_schedule_entry_id`, which the Bills adapter writes and reads back as that resolution.

## Domain types

`lib-core` holds the ids, `BillPlanId` and `BillScheduleEntryId`. The domain types (`BillPlan`, `Recurrence`, `AmountKind`, `BillScheduleEntry`, `EntryId`, `SplitRef`, `Resolution`, `BillStatus`) and rules live in `lib-bills`, which the Desktop's `src/bills/mod.rs` re-exports.

## Persistence

- **Ports** — `lib-bills`' `protocols.rs`: `BillPlanRepository` and `BillScheduleRepository`, and the read ports `BillTransactions` (Match candidates, a Paid entry's figures), `BillAccounts`, `BillCategories` and `BillPayees`. `fake.rs` holds the in-memory twins, and `contract.rs` (behind `contract-tests`) the suite both they and the adapter pass.
- **Schema** — `bill_plans` (a foreign key to `units` only), `bill_schedule_entries` (UUIDv7 `id`, `UNIQUE (bill_plan_id, due)`, the resolution, the Skipped snapshot amount and `superseded`), and `splits.bill_schedule_entry_id`.
- **Adapter** — `lib-database`'s `adapter/bills.rs`: rows in memory, writes through the shared batch, reload on failure. A Paid entry whose Split has gone with its Transaction loads as Unresolved. The Transactions adapter keeps a Split's Match when it rewrites the Transaction, and answers `BillTransactions`.
- **Generation** — `bin_desktop::start_on`, which `start` calls, runs `lib_bills::populate_stored` over the loaded rows and commits the new entries before the Shell is built.
- **Seeds** — the first launch and `mise run db-seed` write the stub Ledger's Bill Plans and entries (`lib_bills::default_bills`, with `seed_entry_id`'s fixed ids), after the Transactions their Paid entries name; `db-seed` then adds generated Bill Plans (`seed::fake_bills`).

## UI

- **Desktop** — on the Ledger read copy of the Bills ports:
  - `src/bills/mod.rs` — the `gpui`-free model and rules: recurrence stepping, `populate` and `horizon`, `status`, `needs_attention` and `attention_entries`, `insert_plan`/`edit_plan`/`set_active`, `pay`, `match_candidates`/`preselected_candidate`/`match_split`, `skip`, `unmatch`/`unskip`/`unmatch_transaction`, `carries_bill`, `schedule_rows`, `period_summary` and `planner_order`, plus the `BillsTab` and Shell-owned `BillsDialog` enums. Start here; it is unit-tested without a window.
  - `src/bills/form.rs` (8c), `src/bills/pay_form.rs` (8d) and `src/bills/history.rs` (the Schedule's `BillFilters` and the stat callout's `plan_stats`) — the dialogs' and filters' pure state.
  - `src/view/bills/` — `mod.rs` the page chrome and tabs, `schedule.rs` (8a), `planner.rs` (8b), `plan_dialog.rs` (8c), `pay_dialog.rs` (8d), `skip_dialog.rs` (8e), `filters.rs` (the Schedule's filter row and stat callout, from 8f).
  - `src/shell.rs` — `handle_bills_key` (`n`/`e`/`p`/`s`/`[`/`]`/`0`/`f`/`1`–`5`), `handle_bills_tab_key`, `handle_bills_dialog_key`, the `open_*_bill_*_dialog` openers, and `open_bill_transaction`, which hands a Paid row off to its Transaction.
  - `src/chrome/rail/primary.rs` — the Bills badge, the Needs Attention count; `src/view/dashboard.rs` — real Bill rows in Needs Attention from `bills::attention_entries`.
  - Messages in `i18n/en-US/bills.ftl`.
- **TUI** — nothing.

## Traceability

Desktop locations for the ticked requirements.

| Requirement | Desktop location |
| --- | --- |
| BIL-001 | `bills::insert_plan`, `bills/form.rs`, `view/bills/plan_dialog.rs` |
| BIL-003 | `bills::edit_plan`, `view/bills/plan_dialog.rs` |
| BIL-004 | `bills::set_active`, `view/bills/planner.rs` |
| BIL-005 | `bills::populate`, `bills::schedule_rows`, `view/bills/schedule.rs` |
| BIL-006 | `bills::pay`, `bills::match_split`, `bills/pay_form.rs`, `view/bills/pay_dialog.rs` |
| BIL-007 | `bills::skip`, `view/bills/skip_dialog.rs` |
| BIL-009 | `budgets::period_figures` (Known Costs), `view/budgets/progress.rs` |
| BIL-011 | `bills::history::plan_stats`, `view/bills/filters.rs` |
| BIL-012 | `bills::needs_attention`, `chrome/rail/primary.rs`, `view/dashboard.rs` |

## Decisions

- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) — materialised, Split-Matched Bill Schedule entries, and why Paid requires a real Transaction.
- [ADR-0020](../adr/0020-needs-attention-as-derived-view.md) — the contrast: Bill Schedule is materialised because it needs identity, Needs Attention is derived because it does not.
- `GLOSSARY.md` — Bill Plan, Bill Schedule, Match, Anticipated Bill, Known Costs (Bills), Needs Attention.

## Open questions and known gaps

- **Unmatch and Unskip have no UI.** `bills::unmatch` and `bills::unskip` exist and are tested, but the handoff gives them no key or control.
- **Deleting a Matched Split Unmatches only through Account delete**, the one Desktop path that deletes a Transaction: the forwarding shim applies `bills::unmatch_transaction` in the same commit.
- **No "carries a Bill" marker on the Transactions list.** `bills::carries_bill` exists; nothing draws it.
- **BIL-002 and BIL-010 are partial.** The Planner has no active/recurrence filter, and the Schedule has no Payee filter.
- **The financial year is a constant** (`bills::history::FINANCIAL_YEAR_START_MONTH`, July) until Settings' "Financial year starts" is a real Preference.
- **The Schedule's period nav is unbounded**; months past the generation horizon show computed previews.
- **Generation doesn't run when the month turns** while the Desktop is open; it runs on start and on a Bill Plan add or edit.
- **Anticipated Bills (BIL-008)** is a later map. Known Costs (BIL-009) is built on the Budgets surface.
