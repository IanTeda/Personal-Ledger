# Payees

A payee is the person or business your money went to, or came from. Woolworths, your landlord, your employer: they're all payees. Personal Ledger keeps track of them so you can see who your money is going to.

> **Heads up:** this is still being built. The dedicated Payees screen isn't designed yet, so for now this page covers where payees show up in the Transactions screen, which is where you'll meet them day to day.

## What you can do

- Create payees with names (usually happens automatically)
- Rename payees and keep the history intact
- Find transactions by payee using search or filters
- See how much you've spent with each payee over time
- Deactivate payees without losing their transaction history

## Terminology

- **Payee:** The person or business money goes to or comes from (e.g. Woolworths, your landlord, your employer).
- **Payee Alias:** A previous name for a payee, kept when you rename it so old transaction entries still resolve correctly.

## Payees concept

You almost never create a payee yourself. The first time you type a new name on a transaction, Personal Ledger creates the payee for you. After that, it recognises the name and avoids duplicates — so "woolworths" and "Woolworths" are treated as the same payee. A payee is optional; plenty of things like cash withdrawals don't really have one. When you rename a payee, its prior name is kept as a Payee Alias, so old entries still resolve correctly.

## Approach

When recording a transaction, type the payee name and Personal Ledger creates it if it's new. To find transactions by payee, press `/` to search (the list narrows as you type) or press `f` to filter by payee. A chip at the top shows the filter is active; click its `✕` to clear it. The totals update to show spending with that payee over your current date range. To rename a payee, go to the Payees screen and edit it — your transactions all update to the new name automatically, and the old name is kept as an alias.

## Worked example

You record a shopping trip to Woolworths for $120. Personal Ledger auto-creates Woolworths as a payee. A week later you spend $85 at Woolworths again. Then you notice you sometimes type it as "Woolies" and want to standardize. You go to the Payees screen, rename Woolworths to "Woolies", and both transactions now show Woolies — the system kept "Woolworths" as an alias in case you type it again. Later you search for "Woolies" and both transactions come up, totalling $205.

## Screens

On the Transactions screen, the payee appears in its own column. The selected row shows its payee in bold. If you split a transaction across several categories, each part can have its own payee; the row shows the first with a "+2" note when others differ. The Payees screen (still in design) will show a list of all payees with spending totals and active status.

## Scope

- Finding a payee by fuzzy match (typo-tolerant) is a future consideration — for now, renaming creates an explicit alias.
- Cross-payee totals and trends are available through Reports.
- Payee merging (combining two payees into one) is not built yet.

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
- [ ] PAY-005: Dedicated payee list screen with spending totals
- [ ] PAY-006: Merge two payees into one

## For developers

Curious how payees are structured in the codebase, or planning to change them? See the [Payees development documentation](development/payees.md).
