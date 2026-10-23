# Bills (development)

End-user documentation: [Bills](../bills.md).

## Overview

The Bills domain has two entities: a **Bill Plan**, the recurring definition, and a **Bill Schedule** entry, one dated instance of it that is Matched to a Split once paid. [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) and its amendments settled the design, and the [Desktop Bills Surface](https://github.com/IanTeda/Personal-Ledger/issues/364) map settled the details (#365 recurrence anchor, #366 statuses, #367 generation and edits, #368 settlement, #369 History figures, #381 History merged into the Schedule).

The desktop app builds the whole Bills surface (handoff `docs/ux/desktop-mockups/12-bills/`, screens 8a–8f, with 8f's History tab folded into the Schedule). Bill Plans and the Bill Schedule persist to SQLite through `lib-bills`' ports ([#777](https://github.com/IanTeda/Personal-Ledger/issues/777)), and the Bills View runs on the `lib-bills` VIPER module ([#778](https://github.com/IanTeda/Personal-Ledger/issues/778)): its Interactors, Presenters, keymap and Messages are the lib's, its Views and Router the Desktop's. The Pay Dialog is `lib-bill-payments`', the first cross-domain module ([#779](https://github.com/IanTeda/Personal-Ledger/issues/779)): Pay it directly writes the Transaction and the entry it settles through one atomic port, `BillPayments`, in one commit, and its Match panel runs `lib-bills`' `MatchBill`. The TUI has no Bills screen.

## Data model

ADR-0019 fixes the parts that are hardest to change later:

- **Bill Schedule entries are persisted rows, not computed on read.** A Transaction needs a stable identifier to Match against, and a recomputed instance has no identity that holds still across that link. The id is a UUIDv7 (`BillScheduleEntryId`), and `(bill_plan_id, due)` is unique: generation looks an entry up by its Bill Plan and due date (`EntryId`) before adding one, so a date never has two rows (ADR-0019 as amended).
- **Paid is only ever reached by a Match to a real Split**, one-to-one: by Pay (a Pending Transaction created from the Plan's defaults, dated today) or by Match (an existing unmatched Expense Split in the Plan's Unit within ±14 days of the due date, Payee matches first). There is deliberately no standalone "mark as paid" flag. Unmatch and Unskip reverse, and deleting a Matched Split Unmatches.
- **Statuses are derived by calendar month** (Upcoming/Due/Overdue) from `today`, with Paid and Skipped stored as the entry's resolution. Overdue and Due entries carry into every other month's Schedule, once; `schedule_rows(.., None, ..)` is All. Unresolved rows sort first (next due first), resolved after (most recent first). Needs Attention is a separate date rule: unresolved and due on or before today plus the Attention Lead.
- **Generation runs through the end of next calendar month**, always keeping at least one unresolved future entry per active Plan, and skips a due date whose Recurrence period already holds a Paid or Skipped entry.
- **Nothing is deleted.** An edit to Recurrence, First Due or Ends On, or deactivating, marks unresolved entries due today or later as superseded; Overdue, Paid and Skipped entries are never touched.

In memory the entry holds its Match as `Resolution::Paid(SplitRef)`. On disk the Match is the Split's optional, unique `splits.bill_schedule_entry_id`, which the Bills adapter writes and reads back as that resolution.

## Domain types

`lib-core` holds the ids, `BillPlanId` and `BillScheduleEntryId`. The domain types (`BillPlan`, `Recurrence`, `AmountKind`, `BillScheduleEntry`, `EntryId`, `SplitRef`, `Resolution`, `BillStatus`) and the rules over them (`bills.rs`, `history.rs`) live in `lib-bills`, which the Desktop's `src/bills/mod.rs` re-exports.

The module, as `docs/development/viper-architecture.md` lays one out:

- **Interactors** (`interactor/`) — `PopulateSchedule`, `AddBillPlan`, `EditBillPlan` (with its activation), `MatchBill` and the `MatchCandidates` read Pay opens on, `SkipBill`, `UnmatchBill` and `UnskipBill`. Each reads the Plans and entries, applies the rules to a copy and writes back only the rows that changed; refusals are `BillError`.
- **Presenters** (`presenter/`) — `BillsPagePresenter` (the header, tabs and status line, holding `SchedulePresenter` and `PlannerPresenter`, and generating a month the Client crosses into), `PlanFormPresenter` (8c) and `SkipPresenter` (8e).
- **`protocols.rs`** — the ports, `BillUnits` for Unit codes, `BillsPorts` (the set the Presenters share) and `BillsRouter` (the Plan form, Skip, Pay, `show_transaction`, `raise_toast`, `set_status`).
- **`keymap.rs`** — the page's default keys, `bills.`-namespaced where only Bills has the command; **`i18n/`** — the `bills-` Messages.

## Persistence

- **Ports** — `lib-bills`' `protocols.rs`: `BillPlanRepository` and `BillScheduleRepository` (together `BillsRepository`), and the read ports `BillTransactions` (Match candidates, a Paid entry's figures), `BillAccounts`, `BillCategories`, `BillPayees` and `BillUnits`. The Accounts, Categories and Payees ports return Bills' own read types (`BillAccount`, `BillCategory`, `BillPayee`), every row with its flags (`takes_transactions`, `is_expense`, `is_leaf`, `is_active`) and a Category's `path` filled by the answering side from the owning domain's rule, so Bills checks flags rather than receiving filtered lists. `fake.rs` holds the in-memory twins, fixed read rows with the stub's ids (`stub_accounts`, `stub_categories`, `stub_payees`), the stub (`InMemoryPorts::stub`) and `RecordingRouter`, and `contract.rs` (behind `contract-tests`) the suite both they and the adapter pass.
- **Schema** — `bill_plans` (a foreign key to `units` only), `bill_schedule_entries` (UUIDv7 `id`, `UNIQUE (bill_plan_id, due)`, the resolution, the Skipped snapshot amount and `superseded`), and `splits.bill_schedule_entry_id`.
- **Adapter** — `lib-database`'s `adapter/bills.rs`: rows in memory, writes through the shared batch, reload on failure. A Paid entry whose Split has gone with its Transaction loads as Unresolved. The Transactions adapter keeps a Split's Match when it rewrites the Transaction, and answers `BillTransactions`.
- **Generation** — `bin_desktop::start_on`, which `start` calls, runs `PopulateSchedule` over the loaded rows and commits the new entries before the Shell is built; the Bills page runs it again on its next event once a running Desktop crosses into a new calendar month.
- **Seeds** — the first launch and `mise run db-seed` write the stub Ledger's Bill Plans and entries (`lib_bills::default_bills`, with `seed_entry_id`'s fixed ids), after the Transactions their Paid entries name; `db-seed` then adds generated Bill Plans (`seed::fake_bills`). `default_bills` lays the Plans out against the read types and lists the payments that settle earlier cycles (`StubBills`); `lib-ledger` (`bills_seed.rs`) books them through `lib_bill_payments::PayBill::run_as`, under fixed Transaction ids. `fake`'s stub, which can't reach `lib-bill-payments`, books them the same way itself, and `lib-ledger`'s `the_bills_fake_stub_is_the_ledgers` keeps the two equal. The checks against the joined stub (the handoff's 8a) are `lib-ledger`'s `bills_seed_tests.rs`.
- **Stand-ins** — the `Ledger` read copy of Bills, the Bills forwarding shim, the Ledger-backed Accounts, Categories and Payees reads, the shared Bills store and the missing foreign keys are halfway shims, each with its removal condition in the VIPER doc's [During the transition](viper-architecture.md#during-the-transition).

## UI

- **Desktop** — `src/bills/`, the module's Desktop half:
  - `view/page.rs` — `BillsPage`, the page Presenter in an Entity: keys to the keymap, clicks to events, and the focused scope select's open list; `view/schedule.rs` (8a), `view/filters.rs` (the filter row and stat callout), `view/planner.rs` (8b) draw the view model.
  - `view/dialog.rs` — the Plan form (`view/dialog/plan_form.rs`, 8c) and Skip (`view/dialog/skip.rs`, 8e) in the Dialog host.
  - `router.rs` — `impl BillsRouter` and the entry point, `bills_page`. `show_transaction` calls `transactions::router::show`.
  - `ports.rs` — `LedgerBackedPorts`: the store shared with `LedgerStore`, the reads copied from the Ledger, the forwarding shim (`forward`) and `sync`, which commits and refreshes the Ledger read copy after each event.
  - The Pay Dialog (8d) is the Bill Payments module's: `src/bill_payments/view/dialog/pay.rs` draws `lib-bill-payments`' Presenter in the Dialog host, and `src/bill_payments/router.rs`' `open_pay` is what the Bills Router's `open_pay` calls. Its writes go through `lib-database`'s `BillPaymentsAdapter`, over the Transactions store `LedgerStore` holds and the Bills store the Bills ports hold.
  - `src/chrome/rail/primary.rs` — the Bills badge, the Needs Attention count; `src/view/dashboard/` — real Bill rows in Needs Attention from `bills::attention_entries`.
  - Messages in `lib-bills`' `i18n/en-US/bills.ftl`; the Pay Dialog's in `lib-bill-payments`' `i18n/en-US/bill-payments.ftl`.
- **TUI** — nothing.

## Traceability

Desktop locations for the ticked requirements.

| Requirement | Desktop location |
| --- | --- |
| BIL-001 | `interactor::AddBillPlan`, `presenter::PlanFormPresenter`, `bills/view/dialog/plan_form.rs` |
| BIL-003 | `interactor::EditBillPlan`, `bills/view/dialog/plan_form.rs` |
| BIL-004 | `interactor::EditBillPlan`, `presenter::PlannerPresenter`, `bills/view/planner.rs` |
| BIL-005 | `interactor::PopulateSchedule`, `presenter::SchedulePresenter`, `bills/view/schedule.rs` |
| BIL-006 | `lib_bill_payments::interactor::PayBill`, `lib_bill_payments::presenter::PayPresenter`, `interactor::MatchBill`, `bill_payments/view/dialog/pay.rs` |
| BIL-007 | `interactor::SkipBill`, `presenter::SkipPresenter`, `bills/view/dialog/skip.rs` |
| BIL-009 | `budgets::period_figures` (Known Costs), `view/budgets/progress.rs` |
| BIL-011 | `bills::history::plan_stats`, `presenter::SchedulePresenter`, `bills/view/filters.rs` |
| BIL-012 | `bills::needs_attention`, `chrome/rail/primary.rs`, `view/dashboard.rs` |

## Decisions

- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md) — materialised, Split-Matched Bill Schedule entries, and why Paid requires a real Transaction.
- [ADR-0020](../adr/0020-needs-attention-as-derived-view.md) — the contrast: Bill Schedule is materialised because it needs identity, Needs Attention is derived because it does not.
- `GLOSSARY.md` — Bill Plan, Bill Schedule, Match, Anticipated Bill, Known Costs (Bills), Needs Attention.

## Open questions and known gaps

- **Unmatch and Unskip have no UI.** The `UnmatchBill` and `UnskipBill` Interactors exist and are tested, but the handoff gives them no key or control, so no Presenter sends them yet.
- **Deleting a Matched Split Unmatches only through Account delete**, the one Desktop path that deletes a Transaction: the forwarding shim applies `bills::unmatch_transaction` in the same commit.
- **The Pay Dialog's Match list doesn't scroll the chosen row into view** ([#791](https://github.com/IanTeda/Personal-Ledger/issues/791)): its body scrolls, but `j`/`k` past the fold choose a row the person can't see.
- **No "carries a Bill" marker on the Transactions list.** `bills::carries_bill` exists; nothing draws it.
- **BIL-002 and BIL-010 are partial.** The Planner has no active/recurrence filter, and the Schedule has no Payee filter.
- **The financial year is a constant** (`bills::history::FINANCIAL_YEAR_START_MONTH`, July) until Settings' "Financial year starts" is a real Preference.
- **The Schedule's period nav is unbounded**; months past the generation horizon show computed previews.
- **Anticipated Bills (BIL-008)** is a later map. Known Costs (BIL-009) is built on the Budgets surface.
