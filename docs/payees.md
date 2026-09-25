# Payees

A payee is the person or business your money went to, or came from. Woolworths, your landlord, your employer: they're all payees. Personal Ledger keeps track of them so you can see who your money is going to.

> **Heads up:** this is still being built. The dedicated Payees screen isn't designed yet, so for now this page covers where payees show up in the Transactions screen, which is where you'll meet them day to day.

## You don't have to set them up

You almost never create a payee yourself. The first time you type a new name on a transaction, Personal Ledger creates the payee for you. After that, it recognises the name, so you don't end up with five spellings of the same shop. Matching ignores capital letters, so "woolworths" and "Woolworths" are the same payee.

A payee is optional. Plenty of things, like a cash withdrawal, don't really have one.

## Seeing payees on your transactions

Open Transactions and you'll find the payee in its own column, right in the middle of each row. The row you've selected shows its payee in bold, so it's easy to spot.

If you split a transaction across several categories, each part can have its own payee. In that case the row shows the first payee, with a "+2" style note when other, different payees are involved.

## Finding transactions by payee

There are two quick ways to narrow your transactions down to a payee:

- **Search:** press `/` and start typing. The list narrows as you type, matching payee names and descriptions.
- **Filter:** press `f` to open the filter box, then type into the **Payee** field. A "payee" chip appears at the top of the screen to show the filter is on. Click its `✕` to clear just that one, or use **clear filters** to clear them all.

The totals at the bottom of the screen update to match what you're looking at, so filtering by a payee is also a quick way to see how much you've spent with them. Just keep in mind that it only covers the date range you've chosen, which starts as "this year".

## Payees, categories and tags

These three do different jobs, and they work best together:

- **Payee:** who the money went to or came from.
- **Category:** what kind of spending or income it was.
- **Tag:** an extra label for anything that cuts across the other two, like a holiday.

See [Categories](categories.md) and [Tags](tags.md) for more.

## Getting around

Press `g` then `p` to jump to Payees from anywhere. For the full set of keys, see [Getting around](getting-around.md).

## For developers

Curious how payees are structured in the codebase, or planning to change them? See the [Payees development documentation](development/payees.md).
