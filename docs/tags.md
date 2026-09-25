# Tags

Tags are little labels you stick on transactions, for the things that don't fit neatly into a category. Think "Japan trip", "tax deductible" or "shared". They let you find and add up spending across your whole ledger for a particular theme.

> **Heads up:** this is still being built. The dedicated Tags screen isn't designed yet, so for now this page covers where tags show up in the Transactions screen, which is where you'll meet them day to day.

## Why tags, when you have categories?

Every transaction has exactly one category, and that's what keeps your books tidy. But real life is messier. A trip to Japan touches flights, food, hotels and gifts, which are all different categories. And you can't put "tax deductible" on a category, because deductible purchases turn up in all sorts of them.

That's where tags come in. A tag ignores the category structure and just marks the transactions you care about, wherever they are.

## How tags work

- **A transaction can have as many tags as you like,** or none at all.
- **Tags are flat.** There are no tags inside tags. A tag is just a name.
- **Capital letters don't matter.** "Japan-trip" and "japan-trip" are the same tag.
- **Your own words stay as typed.** Tags are yours, so they're never translated or changed.

## Seeing tags on your transactions

Open Transactions and look at the **Tags** column. If a transaction has no tags, you'll see a dash. If it has some, the first tag shows as a small chip, with a "+2" style note for any others.

## Finding transactions by tag

- **Filter:** press `f` to open the filter box, then type into the **Tag** field. A "tag" chip appears at the top of the screen to show the filter is on. Click its `✕` to clear just that one, or use **clear filters** to clear them all.
- **Combine it with other filters.** The filter box lets you set everything at once, so you can ask for something like "everything tagged japan-trip, in this account, since January" in one go.

The totals at the bottom of the screen update to match what you're looking at, so filtering by a tag is also how you add up a theme. Keep in mind that it only covers the date range you've chosen, which starts as "this year". If you're totalling a trip that started last year, widen the dates first.

## One currency at a time

The running total is only shown when everything on screen is in the same currency (or unit). If your filtered list mixes currencies, the total says "mixed units" instead. Personal Ledger won't quietly add dollars and yen together, because that would give you a number that looks right and isn't.

## Tags, categories and payees

These three do different jobs, and they work best together:

- **Category:** what kind of spending or income it was. Every transaction has one.
- **Payee:** who the money went to or came from.
- **Tag:** an optional extra label that cuts across both.

See [Categories](categories.md) and [Payees](payees.md) for more.

## Tips

- **Keep the list short.** A handful of tags you actually use beats dozens you don't.
- **Name them the way you'd search for them.** "japan-trip-2026" is easier to find later than "trip".
- **Use a category when it's one thing, and a tag when it cuts across.** Groceries is a category, and "christmas" is a tag.

## Getting around

Press `g` then `t` to jump to Tags from anywhere. For the full set of keys, see [Getting around](getting-around.md).

## For developers

Curious how tags are structured in the codebase, or planning to change them? See the [Tags development documentation](development/tags.md).
