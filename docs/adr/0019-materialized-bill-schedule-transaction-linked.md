# Materialized, Transaction-linked Bill Schedule entries

Personal Ledger's Bill Planner needs both a recurring definition (`Bill`) and individual dated instances of it (`Bill Schedule`) that a Transaction can reference once paid, that carry their own status (Upcoming/Due/Overdue/Paid/Skipped), and that a Budget's Known Costs (Bills) figure and paid-history stats ("last paid," average, same time last year) can be queried against — see `GLOSSARY.md`. We considered computing these dated instances virtually from a Bill's Recurrence on every read, the same way Budget's current period bounds are computed live in `budget_period.rs` rather than stored, but rejected it: a Transaction needs a stable identifier to link against when it settles a Bill Schedule entry, and a virtual, recomputed-on-demand instance has no identity to hold still across that link. Instead, Bill Schedule entries are persisted rows, each with its own `RowID` (UUIDv7, matching every other entity in this codebase), generated ahead of the due date by a populate process rather than derived at read time.

This is a genuine trade-off, not the obvious default: Bill Schedule entries need active maintenance — a process that keeps enough future entries populated, most likely triggered on Client startup or on Bill create/edit, since there's no always-on background service on Desktop/TUI — rather than being derivable purely from the Bill's Recurrence at read time. It also means a Bill's Recurrence or `ends_on` can't be treated as pure metadata: editing either can require regenerating not-yet-Paid future entries. In exchange, a Transaction can hold a plain, stable foreign key (`bill_schedule_id`) the same way it already does for Category/Payee/Account, with no special-cased "virtual reference" concept anywhere else in the system.

Separately, and for the same underlying reason a Bill Schedule entry exists as a real row at all: it only ever reaches **Paid** by linking to a real Transaction — either created fresh from the Bill's defaults, or an existing Transaction edited to link — never by a standalone "mark as paid" flag with no corresponding ledger entry. A lighter, ledger-independent checklist (recording "paid: yes/no" without ever touching a real Account's Transactions) was the more obvious naive design for a bill-tracking feature, but it would let Bill Schedule data silently diverge from the actual Balance/Transaction data everywhere else in the Ledger, undermining the single-source-of-truth spend figures Budget, reports, and reconciliation all depend on. Enforcing the link keeps Bill Schedule data trustworthy by construction rather than by convention.

## Amendment: generation horizon, Bill Plan edits and superseded entries

Settled in [#367](https://github.com/IanTeda/Personal-Ledger/issues/367), on the [Desktop Bills Surface](https://github.com/IanTeda/Personal-Ledger/issues/364) map.

**Horizon.** The populate process keeps Bill Schedule entries generated through the end of next calendar month, and always keeps at least one unresolved future entry per active Bill Plan, so a Quarterly or Annually Bill Plan still shows its next due date and Anticipated Bill.

**When it runs.** On Client start, on Bill Plan create, edit or activation (for that Bill Plan only), and once when a running Client crosses into a new calendar month. Navigating to a future period never generates rows; periods past the horizon show computed previews.

**Identity across Clients.** Every Client generates offline, so two Clients would otherwise each create their own row for the same due date. A Bill Schedule entry's `RowID` is therefore a deterministic UUIDv5 of its Bill Plan's id and its due date, backed by a unique `(bill_plan_id, due_date)` constraint: both Clients produce the same row, and the last-write-wins Change Set merge ([ADR-0009](0009-lww-sqlite-change-set-log.md)) folds them together field by field. This is the one deliberate exception to "every `RowID` is UUIDv7".

**Superseded, never deleted.** An edit to a Bill Plan's Recurrence, First Due or Ends On may replace only unresolved entries due today or later that no Transaction links to. They are not deleted but marked `superseded`, which every view and figure ignores; if a later edit brings the same due date back, generation clears the flag on the same deterministic row rather than fighting a delete. Paid, Skipped and past-due entries (Overdue included) are never touched, so "every Bill Schedule entry is retained permanently" holds literally.

**Settled periods.** Generation skips a due date that falls in a Recurrence period (the 7- or 14-day step for Weekly and Fortnightly; the calendar month, quarter or year for Monthly, Quarterly and Annually) already holding a Paid or Skipped entry, so moving First Due after a cycle is settled never bills that cycle twice.

**Live defaults.** An unresolved entry holds no copy of its Bill Plan's Planned Amount, Fixed/Estimated flag, Account, Payee or Category; it reads them live, so editing the Bill Plan updates every open entry, Overdue ones included. A Paid entry takes its figures from its linked Transaction. A Skipped entry keeps a snapshot of the Planned Amount at the moment it was skipped, for the history view.

**Active and Ends On.** Turning a Bill Plan's Active off supersedes its unresolved future entries and keeps its Overdue ones, which still need resolving. Reaching Ends On removes nothing; generation simply finds no further due dates. Reactivating generates from today forward and does not backfill the due dates that fell while it was inactive.

## Amendment: settlement by Match, at the Split

Settled in [#368](https://github.com/IanTeda/Personal-Ledger/issues/368), on the [Desktop Bills Surface](https://github.com/IanTeda/Personal-Ledger/issues/364) map.

**Match, not link or merge.** The domain act of tying a Bill Schedule entry to real money is to **Match** it; `docs/bills.md`'s "Merge existing" is retired because Merge already names folding one Tag into another. The Pay dialog's two paths are **Pay it directly** (create a Transaction and Match it) and **Match existing transaction**, and the future Transactions-side gesture of pairing an Anticipated Bill with a real Transaction is also Match.

**At the Split, one-to-one.** What is Matched is a Split, not a whole Transaction: the foreign key is an optional, unique `bill_schedule_id` on the Split, so one bank payment covering two Bill Plans settles two entries through two Splits, while each Split and each entry take part in at most one Match. A Transaction "carries a Bill" when any of its Splits is Matched. A Paid entry reads its amount from the Matched Split and its date from that Split's Transaction.

**Candidates.** The Match list offers Expense Splits in the Bill Plan's Unit, not already Matched, whose Transaction is dated within 14 days either side of the due date, from any Account or Payee. They are ordered by Payee match, then Account match, then closeness of amount, then closeness of date; the first is pre-selected only when its Payee matches the Bill Plan's.

**Pay it directly.** Creates a Transaction dated today by default (editable), with Transaction Status Pending, holding one Split with the Bill Plan's Category, Payee and Account and an Amount pre-filled from the Planned Amount, which must be greater than zero; that Split is Matched to the entry.

**Skip.** Any unresolved entry may be Skipped, a One-shot Bill Plan's included, where Skipping means the bill is cancelled.

**Reversal.** Paid is undone by **Unmatch**, which clears the link, leaves the Transaction in place (one created by Pay it directly too) and returns the entry to its derived Upcoming, Due or Overdue status. Skipped is undone by **Unskip**. Deleting a Matched Split or its Transaction Unmatches the entry the same way, so it returns to Needs Attention. Editing a Matched Transaction's amount, date or Payee needs no handling: a Paid entry reads them live, and nothing re-checks that the Payee still matches.
