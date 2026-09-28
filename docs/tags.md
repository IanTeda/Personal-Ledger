# Tags

Tags are little labels you stick on transactions, for things that don't fit neatly into a category. Think "Japan trip", "tax deductible", or "shared". They let you find and add up spending across your whole ledger for a particular theme.

## What you can do

- Add as many tags as you like to a transaction
- Create tags on the fly as you record transactions
- Find transactions by a single tag or combine with other filters
- Add up spending for a theme across all categories
- Filter and report by tags alongside categories and payees
- See every tag on one screen, with how many transactions use it, their total and when it was last used
- Rename, recolour, deactivate or remove a tag, and merge two tags into one

## Terminology

- **Tag:** A label you add to transactions for themes that cut across categories (e.g. "Japan trip", "tax deductible").
- **Split:** One part of a transaction, with its own category, payee and tags. A simple transaction has one.
- **Theme:** A concept tagged across multiple transactions and categories (e.g. a holiday, a project, or a tax category).

## Tags concept

Every transaction has exactly one category, but real life is messier. A trip to Japan touches flights, food, hotels and gifts — all different categories. And you can't put "tax deductible" on a single category because deductible purchases turn up everywhere. Tags solve this by ignoring the category structure and marking the transactions you care about, wherever they are. A transaction can have as many tags as you like, or none. Tags are flat (no tags inside tags), and you control the exact wording. Two tags can't differ only in case, spaces or punctuation, so "Work Trip" and "work-trip" can't both exist. A tag can have a colour if you'd like one, to make it easier to spot.

## Approach

Add tags to a transaction as you record it, or edit them later. Type the tag name and Personal Ledger creates it if it's new — it remembers tags you've used before so you can reuse them without retyping. To find transactions by tag, press `f` to open the filter box and type into the Tag field. A chip at the top shows the filter is active. Combine tags with other filters (account, category, date range) in one go. The totals update to show spending for that tag over your current date range.

In the desktop app, the Tags screen (`g` `t`) is where you look after the tags themselves. Press `n` to add one, `e` to edit the selected tag, `x` to remove it and `m` to merge it into another. Press `enter` to see the transactions that carry it. Renaming or recolouring a tag changes it everywhere at once, because transactions refer to the tag rather than copying its name. If two tags look like duplicates (which can happen when data comes in from elsewhere), the screen flags them and offers to merge them.

## Worked example

You take a trip to Japan in November and January, spending money across flights, accommodation, food, and gifts in different accounts and categories. You tag all the relevant transactions "japan-trip" as you enter them (some in November, some in January). Later, you filter by tag "japan-trip" and set the date range to cover both months. Personal Ledger shows you every expense associated with the trip, totalling $4,350, and you can see the breakdown by category (flights $1,800, accommodation $1,200, food $900, gifts $450) at a glance.

## Screens

On the Transactions screen, the Tags column shows a dash if there are none. If a transaction has tags, the first appears as a small chip, with its colour if it has one, and a "+2" note for any others.

The desktop app's Tags screen lists every tag, most used first, with a colour swatch, how many transactions use it, their total and when it was last used. A tag whose transactions use more than one currency shows "mixed units" instead of a total. Inactive tags are dimmed and marked "inactive". A likely duplicate is highlighted with a "looks like a duplicate of" badge, and the heading says how many there are with a "merge them" link. Each row has `edit` and `remove` buttons.

The Add and Edit dialogs ask for a name and an optional colour: pick none, one of six presets, or type any hex colour. Edit also has an Active checkbox. The Remove dialog tells you how many transactions lose the tag and, if any do, asks you to type the tag's name to confirm. The Merge dialog has two dropdowns (the tag to merge and the tag to keep), each showing how many transactions use it, and spells out what will happen before you confirm.

## Rules to know

- A transaction can have as many tags as you like, or none at all.
- Tags are case-insensitive ("Japan-trip" and "japan-trip" are the same tag), and spaces and punctuation don't count either: "work trip" is refused while "work-trip" exists.
- Removing a tag takes it off every transaction and deletes it. The transactions themselves keep their amount, category and payee. It can't be undone.
- Merging moves every transaction from one tag to the other, then deletes the first. The tag you keep keeps its own name, colour and active setting. It can't be undone.
- An inactive tag isn't offered when you tag a transaction, but its name is still taken and it still shows on transactions that already carry it.
- A transaction counts once for a tag even if several of its splits carry it.
- The running total when filtering by tag shows only if everything on screen is in the same currency — mixed units displays "mixed units" instead.

## Scope

- Cross-tag rollups and reports are built into Reports; see Reports for totals across themes.
- Nested tags (tags inside tags) are not supported — keep them flat.
- The dedicated Tags screen is in the desktop app only; the terminal app's Tags screen doesn't manage tags yet. It does show and set a tag's colour: in its new and edit popups, press `space` on the colour field to step through the presets (and back to none), or type a hex colour.
- The desktop app has no transaction form yet, so you can't add a tag to a transaction there.
- Everything is sample data for now; changes aren't saved.

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
- [x] TAG-005 (desktop): Dedicated tag list screen showing usage counts
- [x] TAG-006 (desktop): Merge two tags into one
- [x] TAG-007 (desktop): Rename a tag across all transactions
- [x] TAG-008 (desktop, terminal): Give a tag an optional colour
- [x] TAG-009 (desktop): Remove a tag, untagging every transaction that carries it
- [x] TAG-010 (desktop): Deactivate and reactivate a tag
- [x] TAG-011 (desktop): Flag likely duplicate tags and offer to merge them

## For developers

Curious how tags are structured in the codebase, or planning to change them? See the [Tags development documentation](development/tags.md).
