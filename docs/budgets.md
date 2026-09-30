# Budgets

A budget is a plan for your spending: "no more than 500.00 a month on groceries", say. Personal Ledger lets you keep several budgets over the same ledger, each with its own monthly amounts per category, and shows you how each month is going against them.

> **Heads up:** Budgets are built in the desktop app only, and for now they run on sample data that isn't saved between sessions. Only the Category limits method is built; Envelope, Percentage split and Project come later. Budgets are monthly and cover expense categories only. The terminal app's Budgets screen is a placeholder.

## What you can do

- Keep several named budgets, switch between them, and choose which one is the default
- Set a monthly amount per category, for one month or from a month onward
- See this month's budget, spending, unpaid bills and what's left, category by category
- Plan months ahead in a grid, and fill a month from last month or from your average spending
- Carry unspent money (or overspending) into the next month
- Look back over past months, and export that history as a CSV file
- Stop budgeting a category without losing its history
- Duplicate or archive a budget, and restore it later

## Terminology

- **Budget:** A named view over your ledger that tracks spending against a plan. It has a method, one unit and a set of on-budget accounts. A budget never owns transactions, so two budgets can count the same spending.
- **Method:** How a budget measures spending. Category limits is the only method built.
- **On-budget accounts:** The accounts whose spending a budget counts.
- **Default budget:** The one budget the Dashboard and the Budgets badge read. There is always exactly one.
- **Category Limit:** A monthly cap on one category's spending within one budget.
- **Budget Amount:** The figure a Category Limit holds for a month. An **Onward** amount applies from its month until you change it. A **Month-only** amount applies to that month alone.
- **Stop:** Ending a category's Budget Amounts from a chosen month. Nothing is deleted, and earlier months keep their amounts.
- **Unbudgeted:** Spending in a category that has no Budget Amount that month. It is always listed, never hidden.
- **Spent:** What went out in a category during the month on the budget's accounts. Refunds net off.
- **Known Costs:** The unpaid bills due in the month in that category. They count against what's left before you've paid them.
- **Rollover:** Whether a finished month carries into the next month's budget: None, Carry unspent, or Carry unspent & overspend.
- **Closed month:** A month before the current one. Its amounts can't be changed.

## Budgets concept

A budget is a container. Inside it, each category you choose to budget has a chain of Budget Amounts over time, and every figure you see is worked out from your transactions and bills when you look at it. Nothing is copied, so editing a transaction changes the budget straight away.

### Budgets and the default

You start with one budget, **Personal spending**. You can add more, for example one for the household accounts you share. One budget is always the default, and the Dashboard's budget list and the Budgets badge in the sidebar read it. The Monthly budget field on a category (see [Categories](categories.md)) always reads and writes Personal spending.

### Budget Amounts over time

Setting an amount never rewrites the past. When you save an amount you choose the month it starts in and whether it applies to that month only or from that month onward. An Onward amount runs until the next change you've made. A category with no amount is Unbudgeted; an amount of 0.00 is a real budget, and any spending is then over it.

Only the lowest-level categories hold amounts. A parent category shows the sum of its children.

### The three tabs

- **Progress** shows one month: what you budgeted, spent, still owe in bills, and have left.
- **Plan** is a six-month grid of amounts, for setting months ahead.
- **History** compares budget and spending over past months.

## Approach

1. Open Budgets and check the title shows the budget you want. Click the title or press `B` to switch.
2. On Progress, select a category and press `e` to set its amount, or press `c` to budget a category that has none.
3. Switch to Plan with `tab` to set amounts for the months ahead. Type into a cell, or press `f` to fill a whole month.
4. Through the month, watch Progress. A category in red has gone over. One marked "at risk" will go over once its unpaid bills are paid.
5. After a few months, open History to see which categories you keep overspending, and adjust the amounts.

## Worked example

It's 21 September and you budget 250.00 a month for Dining. You've spent 318.40, so Progress shows Dining in red, "68.40 over", and the header counts 1 category over budget.

You decide 250.00 is too tight. You select Dining, press `e`, type 300, choose October under Starting, leave Applies to on "That month onward", and save. The summary in the dialog confirms it before you save: September stays 250.00, October onward goes from 250.00 to 300.00, and October's total budgeted rises by 50.00.

September still shows as over, because its amount didn't change. On Plan, October's Dining cell now reads 300.00 in bold, and November and December follow it.

## Screens

- **Progress:** A strip of four totals (Budgeted, Spent, Known costs, Left to spend), then a row per category with its budget, spent, known costs, what's left and a bar. The bar's solid part is spent, the outlined part is known costs, and the tick marks how far through the month you are. `[` and `]` change the month.
- **Category detail:** Press `enter` on a Progress row (or a History cell). It shows the category's figures for that month, its largest transactions, and how often it has gone over. `t` opens those transactions in the Transactions screen, already filtered.
- **Plan:** A grid of categories by month. Closed months are greyed and read-only. A bold amount was set in that month. The Rollover column cycles with `r`. The footer shows each month's total and how much of your average income is unallocated.
- **Edit budget:** Amount, Starting month, Applies to, and Rollover, with a before-and-after summary.
- **Fill from…:** Fills one month from the month before, or from your average spending over the three months before it. It never overwrites a cell you've already set for that month.
- **Stop budgeting:** Ends a category's amounts from this month or next.
- **History:** A chart of budget against spending per month, then a table per category with its average and how many months it went over. `x` exports the table as a CSV file.
- **Switcher:** Lists your budgets with a one-line summary of each. Press `/` to search.
- **New budget:** Name, unit, on-budget accounts, and what to start from: empty, a copy of the budget you're on, or your last three months' spending.
- **Manage budgets:** Every budget in one table, with open, edit, duplicate, set default, archive and restore.

## Rules to know

- A budget has one unit, fixed when you create it, and can only include accounts in that unit.
- Only expense categories with no sub-categories can hold an amount.
- Closed months are read-only. The current and future months can be changed.
- Over budget means spent is more than the budget. Unpaid bills alone make a category "at risk", not over.
- Rollover carries a closed month's budget minus what you spent. The current month never carries.
- A category that gains a sub-category stops being budgeted from the current month, because a parent only adds up its children.
- The default budget can't be archived. Make another budget the default first.
- An archived budget can be viewed but not changed.

## Tips

- **Start small:** Budget the few categories where your spending varies most. The rest still show up as unbudgeted.
- **Use 0.00 on purpose:** It means "I intend to spend nothing here", which is different from leaving a category unbudgeted.
- **Irregular bills:** For a bill that falls due every few months, set a Month-only amount in the months it's due.
- **Shared and personal:** Two budgets can include the same account. Spending there counts in both.

## Scope

- Envelope, Percentage split and Project budgets: not built yet.
- Income targets: not built.
- Weekly, quarterly and yearly budget periods: not built; budgets are monthly.
- Budgets across more than one unit: not supported; a budget has one unit.
- Budget vs Actual reporting: see Reports, not built in the desktop app yet.
- Setting up bills: see [Bills](bills.md).

## Related

- [Categories](categories.md): budgets are set per category, and a category's Monthly budget field is its amount in Personal spending.
- [Bills](bills.md): unpaid bills appear in a budget as Known Costs.
- [Transactions](transactions.md): Spent is worked out from your transactions.
- [Accounts](accounts.md): a budget counts spending on its on-budget accounts only.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Budgets | `g` `b` | `g` `b` |

On the desktop Budgets screen: `B` opens the switcher, `n` starts a new budget, `1` `2` `3` (or `tab`) pick the tab, and `[` `]` change the month or range. The status line at the bottom lists the keys for whatever is open.

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Budgets. Ticked means built in at least one app; the tag says which.

- [x] BUD-001 (desktop): Keep multiple named budgets, each with one method, one unit and a set of on-budget accounts, with exactly one default
- [x] BUD-002 (desktop): Create a budget, starting empty, from a copy, or from the last three months' spending
- [x] BUD-003 (desktop): Rename a budget and change its on-budget accounts
- [x] BUD-004 (desktop): Switch between budgets, with a health summary for each
- [x] BUD-005 (desktop): Duplicate, set default, archive and restore budgets
- [x] BUD-006 (desktop): Set a category's monthly amount for one month or from a month onward, without rewriting closed months
- [x] BUD-007 (desktop): Stop budgeting a category from a chosen month
- [x] BUD-008 (desktop): Show budget, spent, known costs and amount left (or over) per category for a month
- [x] BUD-009 (desktop): Roll parents up from their children and list unbudgeted spending
- [x] BUD-010 (desktop): Carry unspent or overspent amounts into the next month (Rollover)
- [x] BUD-011 (desktop): Plan amounts months ahead in a grid
- [x] BUD-012 (desktop): Fill a month from the previous month or the three-month average
- [x] BUD-013 (desktop): Show budget history by month with average and over count
- [x] BUD-014 (desktop): Export budget history as CSV
- [x] BUD-015 (desktop): Open a category's transactions for a month from its budget
- [x] BUD-016 (desktop): Show the default budget's over-budget count in the sidebar and its progress on the Dashboard
- [x] BUD-017 (desktop): Read and write Personal spending from a category's Monthly budget field
- [ ] BUD-018: Save budgets in the ledger file
- [ ] BUD-019: Weekly, quarterly and yearly budget periods
- [ ] BUD-020: Envelope method
- [ ] BUD-021: Percentage split method
- [ ] BUD-022: Project method
- [ ] BUD-023: Budgets screen in the terminal app

## For developers

Curious how Budgets are structured in the codebase, or planning to change them? See the [Budgets development documentation](development/budgets.md).
