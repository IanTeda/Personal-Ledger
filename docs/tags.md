# Tags

Tags are little labels you stick on transactions, for things that don't fit neatly into a category. Think "Japan trip", "tax deductible", or "shared". They let you find and add up spending across your whole ledger for a particular theme.

> **Heads up:** this is still being built. The dedicated Tags screen isn't designed yet, so for now this page covers where tags show up in the Transactions screen, which is where you'll meet them day to day.

## What you can do

- Add as many tags as you like to a transaction
- Create tags on the fly as you record transactions
- Find transactions by a single tag or combine with other filters
- Add up spending for a theme across all categories
- Filter and report by tags alongside categories and payees

## Terminology

- **Tag:** A label you add to transactions for themes that cut across categories (e.g. "Japan trip", "tax deductible").
- **Theme:** A concept tagged across multiple transactions and categories (e.g. a holiday, a project, or a tax category).

## Tags concept

Every transaction has exactly one category, but real life is messier. A trip to Japan touches flights, food, hotels and gifts — all different categories. And you can't put "tax deductible" on a single category because deductible purchases turn up everywhere. Tags solve this by ignoring the category structure and marking the transactions you care about, wherever they are. A transaction can have as many tags as you like, or none. Tags are flat (no tags inside tags), case-insensitive, and you control the exact wording.

## Approach

Add tags to a transaction as you record it, or edit them later. Type the tag name and Personal Ledger creates it if it's new — it remembers tags you've used before so you can reuse them without retyping. To find transactions by tag, press `f` to open the filter box and type into the Tag field. A chip at the top shows the filter is active. Combine tags with other filters (account, category, date range) in one go. The totals update to show spending for that tag over your current date range.

## Worked example

You take a trip to Japan in November and January, spending money across flights, accommodation, food, and gifts in different accounts and categories. You tag all the relevant transactions "japan-trip" as you enter them (some in November, some in January). Later, you filter by tag "japan-trip" and set the date range to cover both months. Personal Ledger shows you every expense associated with the trip, totalling $4,350, and you can see the breakdown by category (flights $1,800, accommodation $1,200, food $900, gifts $450) at a glance.

## Screens

On the Transactions screen, the Tags column shows a dash if there are none. If a transaction has tags, the first appears as a small chip with a "+2" note for any others. The Payees screen (still in design) will show all tags with a count of how many transactions use each.

## Rules to know

- A transaction can have as many tags as you like, or none at all.
- Tags are case-insensitive ("Japan-trip" and "japan-trip" are the same tag).
- The running total when filtering by tag shows only if everything on screen is in the same currency — mixed units displays "mixed units" instead.

## Scope

- Cross-tag rollups and reports are built into Reports; see Reports for totals across themes.
- Nested tags (tags inside tags) are not supported — keep them flat.
- Tag merging (combining two tags) is not built yet.

## Related

- **Transactions:** where tags attach to each transaction.
- **Categories:** what kind of spending, every transaction has one.
- **Payees:** who the money went to or came from.
- **Reports:** see themes across time, accounts, and categories.

## Tips

- **Keep the list short:** A handful of tags you actually use beats dozens you don't.
- **Name them to search easily:** "japan-trip-2026" is easier to find later than "trip".
- **Use tags for cross-category themes:** Groceries is a category; "christmas" is a tag for shopping that spans multiple categories.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Tags | `g` `g` | `g` `t` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Tags. Ticked means built in at least one app; the tag says which.

- [x] TAG-001 (TUI, desktop): Create tags on the fly as you record transactions
- [x] TAG-002 (TUI, desktop): Add multiple tags to a single transaction
- [x] TAG-003 (TUI, desktop): Filter transactions by tag, alone or combined with other filters
- [x] TAG-004 (TUI, desktop): View totals for spending tagged with a particular theme
- [ ] TAG-005: Dedicated tag list screen showing usage counts
- [ ] TAG-006: Merge two tags into one
- [ ] TAG-007: Rename a tag across all transactions

## For developers

Curious how tags are structured in the codebase, or planning to change them? See the [Tags development documentation](development/tags.md).
