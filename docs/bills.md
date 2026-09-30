# Bills

Bills are the payments you know are coming, like rent, subscriptions and utilities. You write each one down once as a Bill Plan, and Personal Ledger lays out every due date for you. That way you can see what's coming, what's overdue and what you've already paid, and every payment is tied to a real transaction.

> **Heads up:** Bills are built in the desktop app only, and for now they run on sample data that isn't saved between sessions. Anticipated Bills in an account's transaction list are still to come. The terminal app has no Bills screen yet.

## What you can do

- Write down a recurring or one-off payment once, as a Bill Plan
- See each month's due dates, and which are upcoming, due, overdue, paid or skipped
- Pay a bill by creating its transaction, or by matching a transaction you've already recorded
- Skip a cycle, or cancel a one-off bill
- Get warned early about the bills that matter, with a per-bill Attention Lead
- Look back over your bill history, with last paid, average and same-period-last-year figures

## Terminology

- **Bill:** Loosely, the feature as a whole. The things you actually work with are Bill Plans and Bill Schedule entries.
- **Bill Plan:** The definition of a bill: its name, amount, recurrence, account, category and payee.
- **Bill Schedule:** The individual due dates Personal Ledger lays out from each Bill Plan, one entry per due date.
- **Match:** Tying a Bill Schedule entry to the transaction that paid it. An entry matches one transaction, and a transaction settles one entry.
- **Anticipated Bill:** A preview row showing where an unpaid bill will land in its account's transaction list. Not built yet.
- **Known Costs:** The total of unpaid bills in a budget, shown next to actual spending. See [Budgets](budgets.md).

## Bills concept

A Bill Plan is the definition: "Telstra Internet, about $89, due on the 3rd of every month". The Bill Schedule is what you actually pay: January's payment, February's, March's, each its own entry with its own due date and status.

Personal Ledger lays out Bill Schedule entries through the end of next month, and always keeps at least one future entry for each active Bill Plan. Entries are never deleted, so the Bill Schedule doubles as a complete record of what was due, what you paid, what you skipped and what you missed.

### Bill Plan fields

When you add a Bill Plan, you fill in:

- **Name:** A label like "Telstra Internet".
- **Category:** Exactly one expense category, because bills are always money going out.
- **Unit:** The currency. It's fixed once the Bill Plan is saved.
- **Account:** The account the bill is paid from. Only accounts in the Bill Plan's unit are offered.
- **Payee:** Optional. Who the money goes to.
- **Planned amount:** What you expect to pay each time.
- **Fixed or Estimated:** Fixed for a set price (Netflix), Estimated when it varies (electricity). An estimate is a planning figure, not a promise.
- **Recurrence:** Weekly, Fortnightly, Monthly, Quarterly, Annually or One-shot.
- **First Due:** The first due date, which every later due date steps from. It starts as today. A Monthly, Quarterly or Annual bill due on the 29th, 30th or 31st falls on the last day of a shorter month, then goes back to its own day.
- **Ends On:** Optional. The last date (inclusive) a recurring bill can fall due. It can't be before First Due, and it isn't offered for a One-shot bill.
- **Attention Lead:** Optional. How many days before the due date the bill shows in Needs Attention. With no lead, it shows from its due day.
- **Active:** On the Edit dialog only. Turning a Bill Plan off stops new due dates but keeps its history.

Changing the Recurrence, First Due or Ends On, or turning a Bill Plan off, only replaces the unpaid entries due today or later. Overdue, paid and skipped entries stay exactly as they are.

### Bill Schedule statuses

- **Upcoming:** The due date is in a future month.
- **Due:** The due date is this month, up to and including the due day.
- **Overdue:** The due day has passed and it's still unpaid. An overdue entry from an earlier month is also carried into the current month until you pay or skip it, and it still shows in its own month.
- **Paid:** Matched to the transaction that settled it.
- **Skipped:** Deliberately not paid: a recurring cycle you skipped, or a One-shot bill you cancelled.

Weekly and Fortnightly bills can have several Due entries in the same month.

### Paying a bill

A bill never becomes Paid with a tick box. It's always matched to a real transaction, so your bills can't drift away from your balances. The Pay dialog offers two ways:

1. **Pay it directly:** Personal Ledger creates a Pending transaction for you, dated today and filled in from the Bill Plan's account, payee, category and amount. You can change the amount and date first.
2. **Match existing transaction:** Pick a transaction you've already recorded, for example from a CSV import. The dialog lists unmatched expense transactions in the Bill Plan's unit from within 14 days of the due date, with ones to the same payee first.

Once matched, the transaction's own date and amount are what count. The planned amount was only ever an estimate.

### Needs Attention and the Bills badge

An unpaid bill shows in Needs Attention on the Dashboard once its due date is within its Attention Lead, even if that's early next month, and stays there until you pay or skip it. The number on the Bills entry in the side rail is how many bills need your attention, not how many there are.

### Bill history

Your bill history lives on the Schedule tab. Press `0` (or click **All**) to switch from one month to every Bill Schedule entry ever generated. Unpaid bills always sit at the top, next due first, and paid and skipped ones follow, most recent first. You can filter by status, Bill Plan, category and account. When you narrow it to one Bill Plan, a summary shows:

- **Last paid:** The payment with the latest transaction date.
- **Average:** The average payment over the last complete financial year. Skipped entries are left out rather than counted as $0.
- **Same period last year:** A year back from the last payment. For a Weekly or Fortnightly bill it's the total paid in that month last year. A One-shot bill doesn't show it.

A figure with nothing to go on shows a dash.

## Approach

1. On the Bills screen's Planner tab, press `n` to add a Bill Plan, and fill in its amount, category, account, recurrence and First Due.
2. Check the Schedule tab to see the month's due dates. Use `[` and `]` to move between months.
3. Keep an eye on Needs Attention and the Bills badge for bills coming due.
4. When you pay a bill, select its row on the Schedule tab and press `p`. Either pay it directly or match a transaction you've already recorded.
5. If you're not paying a cycle, press `s` to skip it.
6. On the Schedule tab, press `0` to see every entry and filter to one Bill Plan to look back at what you paid, and how it compares with last year.

## Screens

The Bills screen has two tabs. Press `tab` to move between them.

- **Schedule:** One month of due dates (or, with `0`, all of them), with each bill's account, planned and actual amount, due date, paid date and status, and the due, overdue and paid counts and planned total. Overdue and due bills from other months always show too, so nothing unpaid is out of sight. Skipped bills show a dash for the actual amount and paid date. Press `p` to pay the selected bill and `s` to skip it, `[` and `]` to change month, `1` to `5` to toggle the Paid, Skipped, Overdue, Due and Upcoming chips, and `f` to move through the Bill, Category and Account filters. Press `enter` on a paid row to open its transaction.
- **Planner:** Your Bill Plans, with category, account, planned amount, recurrence, Attention Lead and whether each is active. Press `n` to add one, and `e` or `enter` to edit the selected one.

Use `j` and `k` to move between rows on every tab.

## Worked example

It's 1 March and you're setting up your bills. You add three Bill Plans, all paid from your Everyday account:

- Telstra Internet: $89 Fixed, Monthly, First Due 3 March, category Utilities.
- Netflix: $16 Fixed, Monthly, First Due 15 March, category Entertainment.
- Electricity: $150 Estimated, Monthly, First Due 20 March, category Utilities, with an Attention Lead of 5 days.

The Schedule tab shows three Due entries for March, with a planned total of $255.

On 5 March, Telstra has gone Overdue and shows in Needs Attention. Your bank took the payment on the 4th, and you've already imported it. You select Telstra, press `p`, choose Match existing transaction, and pick the $89 payment to Telstra from the 4th. Telstra is now Paid.

On 15 March, Electricity shows in Needs Attention, five days ahead of its due date. You pay it on the 18th, when the bill comes in at $145. You press `p`, choose Pay it directly, and change the amount to $145. Personal Ledger records a $145 transaction for you, and Electricity is Paid at $145, not the $150 you planned.

You've decided to take a break from Netflix this month, so you select it and press `s`. March's Netflix entry is Skipped, and April's is still there as normal.

At the end of March, the Schedule tab shows 2 paid and 1 skipped entry, and you spent $234 on bills: $89 for Telstra and $145 for Electricity. On the Schedule tab, showing All and filtered to Electricity, Last paid reads $145.

## Scope

- The terminal app has no Bills screen yet.
- Bills run on sample data in the desktop app and aren't saved yet.
- Anticipated Bills in an account's transaction list are future.
- The Planner tab lists every Bill Plan, active first. You can't yet filter it by active status or recurrence.
- The Schedule tab doesn't filter by payee or by a date range yet.
- Unmatching a paid bill or unskipping a skipped one isn't on screen yet.
- Operating system notifications or a background reminder service are future. Opening the app is what shows Needs Attention.
- Recurrence relative to your last payment (for example "30 days after last paid") is future. Bills use a fixed calendar recurrence.

## Related

- **Accounts:** each Bill Plan is paid from one account.
- **Categories:** each Bill Plan has one expense category.
- **Transactions:** the matched transactions are the actual payments, and a paid bill opens its transaction.
- **[Budgets](budgets.md):** unpaid bills show as Known Costs in a budget's month.
- **[Needs Attention](needs-attention.md):** unpaid bills show here from their due date minus their Attention Lead.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Bills | not in the terminal app yet | `g` `w` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Bills. Ticked means built in at least one app; the tag says which.

- [x] BIL-001 (desktop): Create a bill plan with name, category, account, amount, recurrence, and end date
- [ ] BIL-002: List bills with filtering by active status and recurrence type
- [x] BIL-003 (desktop): Edit a bill plan (name, amount, recurrence, category, account)
- [x] BIL-004 (desktop): Deactivate a bill plan without losing history
- [x] BIL-005 (desktop): Generate Bill Schedule entries for a plan period
- [x] BIL-006 (desktop): Mark a Bill Schedule entry as Paid by linking or creating a transaction
- [x] BIL-007 (desktop): Mark a Bill Schedule entry as Skipped
- [ ] BIL-008: View Anticipated Bills (previews) in account transaction lists
- [x] BIL-009 (desktop): Show Known Costs (unpaid bills) within a budget's period
- [ ] BIL-010: Filter and view bill history with status, date range, category, payee, or account filters
- [x] BIL-011 (desktop): Calculate and display bill history (last paid, average, same period last year)
- [x] BIL-012 (desktop): Set Attention Lead per bill to flag upcoming bills in Needs Attention

## For developers

Curious how bills are structured in the codebase, or planning to change them? See the [Bills development documentation](development/bills.md).
