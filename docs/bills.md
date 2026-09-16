# Bills

Personal Ledger tracks recurring and one-off payment obligations, such as rent, subscriptions, and utilities, as Bills. This allows you to see what’s coming up before it’s due, what’s overdue, and what’s already been paid. This page explains the Bill workflow in Personal Ledger, the list of Bills, Bill Schedule entries, and the Anticipated Bill preview fit together, and how Bills show up inside Budget.

See CONTEXT.md for the one-paragraph canonical definition of each term, and ADR-0019 for why Bill Schedule entries are real, persisted rows linked to a Transaction rather than a lightweight, ledger-independent checklist. This document is where the fuller shape and workflow live, the same way docs/accounts.md covers Account Kinds.

## Bills Workflow

Personal Ledger Bills start with adding Bills to the Bill Planner list. The planned list is then used to populate the Bill Schedule over the plan period. Bill Schedule line items are then mirrored and displayed in the associated Account transactions and the budgets known expenses. Individual Bill Schedule line items are marked as paid and updated with the actual amount. The paid Bill is stored historically for future reference and verifying Bill Plan amounts.

## Why Bill Plan and Bill Schedule are Separate

A Bill on its own — “Telstra Internet, ~$89, due the 3rd of every month” — is a definition, not an individual planned Bill or record of anything that’s happened. What you actually pay, and when, is a series of separate dated events: January’s payment, February’s, March’s, each potentially a different amount, each with its own paid/unpaid status. As such, Personal Ledger keeps these as two things:

* Bill Plan — the recurring definition: what it’s for, roughly how much, which Account pays it, how often it recurs.
* Bill Schedule — one row per due date, generated ahead of time for the plan period from the Bill’s recurrence. A Monthly Bill has a new Bill Schedule entry every month. A One-shot Bill has exactly one schedule line item.

Similarly, this mirrors how a Loan Account’s interest rate is tracked as a history of dated entries rather than a single field rewritten in place, or how an Account is the umbrella and Account Kind is which specific shape it takes — one definition, many dated instances, each able to carry its own status and history.

## Bill Plan

A Bill is added to the Plan once and edited occasionally. It’s what you fill in when you set up “I pay this.”

| -- Column 1 -- | -- Column 2 -- |
|-----------------|-----------------|
| Field | Meaning |
|-------|---------|
| Name | A human-readable label (e.g. "Telstra Internet"). |
| Category | Exactly one, Expense-type only — the same restriction Budget already enforces, since a Bill is always an outgoing cost. |
| Unit | Fixed at creation, the currency the Bill is paid in. |
| Account | Mandatory — which Transaction/Credit Card Account this Bill is expected to be paid from. This is what lets the Anticipated Bill preview (see below) know which Account's list to appear in. |
| Payee | Optional — who the money goes to. |
| Planned Amount | An estimate of what's owed each time — see Fixed/Estimated below for how strictly it's treated. |
| Fixed or Estimated | Fixed for a Bill that's always the same (a Netflix subscription); Estimated for one that varies (an electricity bill) — the Amount is then a planning figure, not a promise. |
| Recurrence | Weekly, Fortnightly, Monthly (on a fixed day of the month), Quarterly, Annually, or One-shoot for a single, non-repeating due date. |
| Ends On | Optional, and only meaningful for a recurring Bill. Leave unset for a Bill that recurs indefinitely; set it when you know a Bill has a natural end (e.g. a 12-month contract). |
| Attention Lead | Optional day count, unset by default (meaning Overdue-only). When set, a Bill Schedule entry that's Due (not yet Overdue) but within this many days of its due date also shows up in Needs Attention — see docs/needs-attention.md. Set per Bill, since how much early notice is useful varies (rent versus a small subscription). |
| Active | Soft-delete, the same pattern as Account/Category/Payee/Tag — deactivating a Bill stops new Bill Schedule entries from being generated but leaves its history untouched. |


## Bill Schedule

Each Bill Schedule entry is given a unique UUIDv7 and number prefixed with the bill plan period name. It moves through a status as its due date approaches and passes:

| Status | Meaning |
|-------|---------|
| Upcoming | The due date falls in a future calendar month. |
| Due | The due date falls within the current calendar month, from the 1st up to and including the due day itself. |
| Overdue | The due day has passed this month and it's still unpaid. |
| Paid | Linked to the Transaction that settled it. |
| Skipped | Deliberately not paid — a recurring cycle you chose to skip, or a One-shoot Bill you cancelled. |

An Overdue entry doesn’t reset when the month rolls over — it just stays Overdue until you either pay or Skip it. Next month’s due date is a wholly separate Bill Schedule entry, starting its own Upcoming/Due cycle independently.

## Marking one as Paid

A Bill Schedule entry only ever becomes Paid by linking to a real Transaction — never a plain “mark as paid” toggle with nothing behind it. There are two ways to get there:

1. Pay it directly — Personal Ledger creates a new Transaction for you, seeded from the Bill’s Category, Payee, Account, and Amount, and links it to this Bill Schedule entry.
2. Merge or Link an existing Transaction — if you’d already entered the payment separately (or it came in through a CSV/Balance Check import), you point that Transaction at this Bill Schedule entry instead of creating a duplicate.

Either way, once linked, the Transaction’s own date and amount are what count — the Bill’s Amount was only ever an estimate to plan against.

Wherever that Transaction shows up afterwards (its Account’s list, reports), it’s visually marked as carrying a Bill, so you can tell at a glance it’s a recurring obligation rather than a one-off purchase. This is unrelated to the ordinary Flagged marker you can put on any Transaction for your own follow-up — a Transaction can carry a Bill, be Flagged, or both, independently.

## Bill History

For each Bill, Personal Ledger shows the last amount paid, an average over time, and the same period last year — computed from Paid entries’ actual Transaction amounts only. A Skipped entry doesn’t count as a $0 payment; it’s left out of the average entirely, since skipping a bill one cycle isn’t the same as it costing nothing.

No Bill Schedule entry is ever deleted, regardless of what happens to its Bill afterwards — deactivating a Bill, or it reaching its Ends On date, only stops future entries from being generated; every entry already generated stays exactly as it was, whether Paid, Skipped, or left Overdue. This makes Bill Schedule itself the historical record: a full, permanent list of what was due, when, and what happened to it.

That list can be filtered when reviewing history by any combination of:

* Status — e.g. only Paid, or only Skipped, to see what you’ve actually let lapse over time.
* Bill — every past entry for one specific Bill, the raw data behind its “last paid / average / same time last year” figures (see above).
* Category, Payee, or Account — the same filters already available elsewhere in Personal Ledger (e.g. Budget’s own Category/active-status filtering), so Bill history reviews the same way the rest of the Ledger does.
* Date range — e.g. everything due last financial year.

## Anticipated Bill — seeing what’s coming, inline

Before a Bill Schedule entry is paid, its next due date shows up as a shaded preview row directly inside its target Account’s transaction list — sitting where the real payment will eventually land, so you can see it coming while reviewing your Account like any other line, without needing to check a separate screen. It isn’t a real Transaction: it disappears the moment the Bill Schedule entry resolves to Paid, replaced by the one real, linked Transaction — the two never sit in the list side by side.

## Known Costs (Bills) in Budget

Bill and Budget stay independent entities — a Bill isn’t attached to any particular Budget — but a Budget’s progress view adds a Known Costs (Bills) figure alongside its actual spend, summing every unpaid (Due or Overdue) Bill Schedule entry that matches the Budget’s Category and Unit within its current period.

Example — a “$300/month for Utilities” Budget, with a Telstra Internet Bill (~$89, due the 3rd) and an Electricity Bill (~$150 estimated, due the 20th), viewed on the 10th of the month:

| -- Column 1 -- |-- Column 2 -- |-- Column 3 -- |
|-|--|--|
| Description | Amount | Note |
| Spent so far | $0.00 | No Utilities Transactions yet this period |
| Known Costs (Bills) | $239.00 | $89.00 Overdue (Telstra, due the 3rd, unpaid) + $150.00 Due (Electricity, due the 20th) |
| Remaining after known costs | $61.00 | $300 limit − $239 known costs |


An Overdue entry stays inside the Known Costs total — it’s still unpaid, committed spending — but is visually distinguished from a merely-Due one, so you can tell “coming up” apart from “already late.” Once a Bill Schedule entry is Paid, its linked Transaction’s actual amount moves into the Budget’s ordinary spend figure instead, so it’s never counted in both places at once.

## Reminders

Overdue Bill Schedule entries — and Due ones within a Bill’s own Attention Lead, see above — feed into Needs Attention, the one cross-Ledger list of everything wanting action, surfaced on each Client’s own Dashboard (TUI and Desktop) whenever you have it open. See docs/needs-attention.md for the fuller picture, including how Bills sit alongside Flagged and unreconciled Transactions there. A merely Upcoming or plain-Due (outside its Attention Lead) Bill Schedule entry isn’t part of Needs Attention — it’s still visible in Bill Planner and Budget’s Known Costs (Bills) as usual, just not flagged as needing action yet.

## What’s deliberately out of scope here

This document captures the shape agreed so far — it is not a functional spec, and nothing here has been built yet (no migrations, tables, or screens exist for Bill or Bill Schedule). Left for later, once that work is actually scoped:

* OS-level push notifications or a background reminder service — a Future Consideration beyond the in-app Dashboard surfacing described above.
* Relative-to-last-payment recurrence (e.g. “30 days after it was last paid”) — V1 supports fixed calendar recurrence only.
* Any dedicated Bill Planner screen layout for the TUI or Desktop app.
* The exact cadence/trigger for the process that keeps future Bill Schedule entries populated — ADR-0019 records why entries must be materialised ahead of time, not when that process runs.

A planned Bill can be merged with an actual transaction, linking and marking the bill as paid
