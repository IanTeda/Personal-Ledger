# Payees

A payee is the person or business your money went to, or came from. Woolworths, your landlord, your employer: they're all payees. Personal Ledger keeps track of them so you can see who your money is going to.

> **Heads up:** the desktop app's Payees screen is built, but it works on sample data for now: nothing you add, edit or import is saved when you close the app. Importing a real bank statement isn't built yet either; the import screen shows a sample statement so you can try the payee-matching step.

## What you can do

- Create payees with names (usually happens automatically)
- See every payee in one list, with how many transactions use it and how much has gone to or come from it
- Give a payee a default category, so new transactions for it start with that category filled in
- Add match rules, so a payee is recognised from the way it appears on a bank statement
- Rename payees and keep the history intact
- Delete a payee you've never used, or deactivate one you have
- Find transactions by payee using search or filters
- See how much you've spent with each payee over time
- Deactivate payees without losing their transaction history

## Terminology

- **Payee:** The person or business money goes to or comes from (e.g. Woolworths, your landlord, your employer).
- **Payee Alias:** A piece of text that points to a payee. It's either a previous name, kept when you rename the payee, or a match rule you add yourself. The Payees screen calls both "match rules".
- **Match rule:** Text such as `WOOLWORTHS` or `WW SUPERMARKET` that a bank statement line might contain. If a line contains it (capitals don't matter), the line belongs to that payee.
- **Default category:** The category a new transaction for this payee starts with. You can still change it on the transaction.

## Payees concept

You almost never create a payee yourself. The first time you type a new name on a transaction, Personal Ledger creates the payee for you. After that, it recognises the name and avoids duplicates — so "woolworths" and "Woolworths" are treated as the same payee. A payee is optional; plenty of things like cash withdrawals don't really have one. When you rename a payee, its prior name is kept as a Payee Alias, so old entries still resolve correctly.

Banks rarely write a payee's name the way you would. Woolworths might show up as `WOOLWORTHS 1234 SYDNEY` one week and `WW SUPERMARKET` the next. Match rules deal with that: add one rule for each way the payee appears, and when you import a statement, each line is matched to the payee whose rule it contains. If two rules fit, the longer one wins, so a short rule like `BP` can't grab a line a longer rule describes better. Each rule belongs to exactly one payee, and rules are always kept in capitals.

A default category saves you picking the same category every time. Changing it only affects new transactions. The ones you've already recorded keep the category they have.

You can only delete a payee that no transaction uses. Once a payee has transactions, you deactivate it instead. It stays on the list, dimmed and marked "inactive", and you can still filter transactions by it, but its match rules stop matching on import and it isn't counted in the payee totals. You can reactivate it any time.

## Approach

When recording a transaction, type the payee name and Personal Ledger creates it if it's new. To find transactions by payee, press `/` to search (the list narrows as you type) or press `f` to filter by payee. A chip at the top shows the filter is active; click its `✕` to clear it. The totals update to show spending with that payee over your current date range. To rename a payee, go to the Payees screen and edit it — your transactions all update to the new name automatically, and the old name is kept as an alias.

In the desktop app, press `g` `p` to open the Payees screen. Move up and down the list with `j` and `k`, and press `enter` to see the selected payee's transactions. Press `n` to add a payee, `e` to edit the selected one, or `d` to delete or deactivate it. In the add and edit dialogs, type a match rule and press `enter` to add it (that adds the rule, it doesn't save the dialog), or click a rule's `✕` to remove it. Deleting asks you to type the payee's name first, so you can't delete one by accident.

To import, type `:import` in the command box. Each statement line is shown with the payee and category it will get. Lines a match rule recognised are ready to go. For the rest, Personal Ledger suggests a tidied-up name as a new payee. Move to a line with `j` and `k`, press `p` to choose its payee or `c` to choose its category, or `n` to create the suggested payee. Lines still missing a payee or category are marked "needs review", and you can't continue until there are none. Press `r` to turn "remember new payees' rules for next time" on or off, `enter` to continue, or `esc` to go back to Transactions.

## Worked example

You record a shopping trip to Woolworths for $120. Personal Ledger auto-creates Woolworths as a payee. A week later you spend $85 at Woolworths again. Then you notice you sometimes type it as "Woolies" and want to standardize. You go to the Payees screen, rename Woolworths to "Woolies", and both transactions now show Woolies — the system kept "Woolworths" as an alias in case you type it again. Later you search for "Woolies" and both transactions come up, totalling $205.

Next month you import your bank statement. It has a line reading `WW SUPERMARKET 0412 NEWTOWN`. Woolies has a match rule `WW SUPERMARKET`, so the line goes to Woolies with its default category, Groceries, filled in. There's also a line reading `SP AUSSIE CANDLE CO`, which no rule matches. Personal Ledger suggests a new payee called "Aussie Candle Co". You press `c`, choose Household as the category, leave "remember" ticked and press `enter`. Aussie Candle Co is created with a match rule, so next month's statement recognises it by itself.

## Screens

On the Transactions screen, the payee appears in its own column. The selected row shows its payee in bold. If you split a transaction across several categories, each part can have its own payee; the row shows the first with a "+2" note when others differ. The desktop app's Payees screen lists every payee with its default category, how many match rules it has, how many transactions use it and their total. Payees with no default category are marked in red, and the heading says how many there are. Inactive payees are dimmed. Each row has `edit` and `delete` buttons. The import screen, opened with `:import`, shows one statement line per row with its payee, category, a status (rule match, new payee or needs review) and amount, and a summary along the bottom.

## Scope

- Finding a payee by fuzzy match (typo-tolerant) is a future consideration — for now, renaming creates an explicit alias.
- Cross-payee totals and trends are available through Reports.
- Payee merging (combining two payees into one) is not built yet.
- Importing a real statement file isn't built yet; the import screen uses a sample statement, and only its payee-matching step exists.
- The desktop app has no transaction form yet, so a default category is used only by the import screen for now.
- Transactions search doesn't look at match rules: searching finds a payee by its current name.

## Related

- **Transactions:** where payees appear in each transaction.
- **Categories:** what kind of spending, independent of payee.
- **Tags:** for extra labels that cut across payees and categories, like a holiday.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Payees | `g` `p` | `g` `p` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Payees. Ticked means built in at least one app; the tag says which.

- [x] PAY-001 (TUI, desktop): Create a payee with a name (usually auto-created on first transaction)
- [x] PAY-002 (TUI, desktop): List payees with filtering by active status
- [x] PAY-003 (TUI, desktop): Edit a payee's name and preserve prior names as aliases
- [x] PAY-004 (TUI, desktop): Deactivate a payee without deleting transactions
- [x] PAY-005 (desktop): Dedicated payee list screen with spending totals
- [ ] PAY-006: Merge two payees into one
- [x] PAY-007 (desktop): Set an optional default category that pre-fills new transactions for the payee
- [x] PAY-008 (desktop): Add and remove match rules that recognise a payee from bank statement text
- [x] PAY-009 (desktop): Delete a payee no transaction uses
- [x] PAY-010 (desktop): Match statement lines to payees and categories on import, suggesting new payees

## For developers

Curious how payees are structured in the codebase, or planning to change them? See the [Payees development documentation](development/payees.md).
