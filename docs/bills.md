# Bills

Personal Ledger tracks recurring and one-off payment obligations like rent, subscriptions, and utilities as Bills. This lets you see what's coming up before it's due, what's overdue, and what you've already paid. Bills are still in the design phase, so this page shows the planned workflow.

> **Heads up:** Bills are not yet built. Nothing on this page is implemented — it describes the design for future development.

## What you can do

- Create bills for recurring or one-time payments
- Set up recurrence (weekly, monthly, quarterly, yearly, or one-time)
- Track whether each bill is paid, overdue, or upcoming
- Record payments against bills
- See upcoming bills in accounts and budgets
- Filter bills by status and history
- View spending totals and averages for each bill

## Terminology

- **Bill:** A recurring or one-time payment obligation (e.g. rent, internet subscription).
- **Bill Plan:** The definition of a bill (name, amount, recurrence, account, category).
- **Bill Schedule:** Individual due dates generated from a bill plan (one per billing cycle).
- **Anticipated Bill:** A preview row showing where a bill will land in your account's transaction list before it's paid.
- **Known Costs:** The total of unpaid bills in a budget, shown alongside actual spending.

## Bills concept

A Bill Plan is the definition: "Telstra Internet, ~$89, due the 3rd of every month". A Bill Schedule is what you actually pay — the individual due dates. January's payment, February's, March's — each is a separate Bill Schedule entry with its own date and paid/unpaid status.

Personal Ledger generates Bill Schedule entries ahead of time for the plan period so you can see what's coming. A monthly bill creates a new entry every month. A one-off bill creates exactly one entry. This separation keeps your bill definitions clean while tracking the history of what you actually paid.

### Bill Plan fields

When you create a bill, you fill in:

- **Name:** A label like "Telstra Internet".
- **Category:** Exactly one Expense category — bills are always money going out.
- **Unit:** The currency (fixed at creation).
- **Account:** Which account this bill is paid from (so Anticipated Bills can show in the right place).
- **Payee:** Optional — who the money goes to.
- **Planned Amount:** An estimate of what's owed each time.
- **Fixed or Estimated:** Fixed (Netflix) or Estimated (electricity) — estimates are planning figures, not promises.
- **Recurrence:** Weekly, Fortnightly, Monthly, Quarterly, Annually, or One-time.
- **Ends On:** Optional — when a recurring bill stops generating new entries.
- **Attention Lead:** Optional — days before due date to flag it in Needs Attention (different for rent vs a small subscription).
- **Active:** Deactivating a bill stops new entries but keeps its history.

### Bill Schedule statuses

Each Bill Schedule entry moves through statuses as its due date approaches and passes:

- **Upcoming:** The due date is in a future month.
- **Due:** The due date is this month, up to and including the due day.
- **Overdue:** The due day has passed this month and it's still unpaid.
- **Paid:** Linked to the real transaction that settled it.
- **Skipped:** Deliberately not paid — a recurring cycle you skipped, or a one-time bill you cancelled.

### Paying a bill

A bill never becomes "paid" with a checkbox. Instead, you link it to a real transaction. There are two ways:

1. **Create new:** Personal Ledger creates a transaction for you, seeded from the bill's category, payee, account, and amount, then links the bill to it.
2. **Merge existing:** You link the bill to a transaction you already recorded (e.g. from a CSV import).

Once linked, the transaction's actual date and amount are what count — the bill's "planned amount" was only ever an estimate.

### Anticipated Bills and Known Costs

Before a bill is paid, its next due date shows up as a shaded preview row in its account's transaction list — sitting where the real payment will eventually land. This "Anticipated Bill" preview disappears when you pay the bill and link the real transaction.

In a Budget, bills show up as "Known Costs" — the total of unpaid bills in that budget's category within the current period, shown alongside actual spending. An Overdue bill counts as committed spending but is visually distinct from a merely-Due one. Once a bill is paid, its linked transaction moves into the budget's ordinary spend figure, so it's never counted twice.

### Bill history

For each bill, Personal Ledger tracks the last amount paid, an average over time, and figures from the same period last year (calculated from paid entries only — a Skipped entry doesn't count as $0). No Bill Schedule entry is ever deleted, so you have a complete record of what was due, when, and what happened to it.

## Approach

1. Create a bill plan with a name, amount, category, account and recurrence pattern (weekly, fortnightly, monthly, quarterly, annually, or one-time).
2. Personal Ledger generates Bill Schedule entries ahead of time so you can see what's due and when.
3. As each due date approaches, check Needs Attention to see upcoming and overdue bills.
4. When you pay a bill, either create a new transaction directly from the bill, or link an existing transaction you've already recorded.
5. View bill history (last paid, average, same period last year) to understand your payment patterns.

## Worked example

You're setting up your bills. You create "Telstra Internet" with a planned amount of $89, monthly recurrence on the 3rd, paid from your Everyday account in the Utilities category. You create "Netflix" with a fixed amount of $16, monthly recurrence on the 15th. You create "Electricity" with an estimated amount of $150 (it varies), monthly recurrence on the 20th.

Personal Ledger generates Bill Schedule entries for the next few months. On the 5th of the month, you see the Telstra bill has moved to Overdue in Needs Attention. You open it, create a new transaction for $89 on the 4th (it was posted early), and link the bill. It's now Paid. On the 12th you manually create an electricity transaction for $145 (less than expected), then link the bill to it. On the 20th you decide to skip Netflix this month — you mark that Bill Schedule entry as Skipped. At month-end, your budget shows: Spent $234 (Telstra $89 + Electricity $145), Known Costs $150 (Electricity expected next cycle? no, just Telstra and Netflix of next month within current known costs?). The bill history for Telstra now shows last paid $89, average (if you had a year's history) and same month last year.

## Scope

- OS-level push notifications or a background reminder service are future considerations beyond the in-app Needs Attention dashboard.
- Relative-to-last-payment recurrence (e.g. "30 days after last paid") is future; V1 supports fixed calendar recurrence only.
- Dedicated Bill Planner screen layout is future.
- The exact cadence for populating future Bill Schedule entries is to be decided during implementation.

## Related

- **Accounts:** each bill is paid from one account.
- **Categories:** bills are classified by expense category.
- **Budgets:** bills appear as Known Costs within a budget's period.
- **Needs Attention:** Overdue and Due (within lead time) bills appear here.
- **Transactions:** linked transactions are the actual payments.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Bills | not in the terminal app yet | `g` `w` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Bills. Ticked means built in at least one app; the tag says which. None are ticked because Bills are not yet built.

- [ ] BIL-001: Create a bill plan with name, category, account, amount, recurrence, and end date
- [ ] BIL-002: List bills with filtering by active status and recurrence type
- [ ] BIL-003: Edit a bill plan (name, amount, recurrence, category, account)
- [ ] BIL-004: Deactivate a bill plan without losing history
- [ ] BIL-005: Generate Bill Schedule entries for a plan period
- [ ] BIL-006: Mark a Bill Schedule entry as Paid by linking or creating a transaction
- [ ] BIL-007: Mark a Bill Schedule entry as Skipped
- [ ] BIL-008: View Anticipated Bills (previews) in account transaction lists
- [ ] BIL-009: Show Known Costs (unpaid bills) within a budget's period
- [ ] BIL-010: Filter and view bill history with status, date range, category, payee, or account filters
- [ ] BIL-011: Calculate and display bill history (last paid, average, same period last year)
- [ ] BIL-012: Set Attention Lead per bill to flag upcoming bills in Needs Attention

## For developers

Curious how bills are structured in the codebase, or planning to change them? See the [Bills development documentation](development/bills.md).
