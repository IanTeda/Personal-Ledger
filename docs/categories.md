# Categories

Categories are how you answer the question "where does my money actually go?" Every transaction gets a category, like Groceries, Rent or Salary, and Personal Ledger uses them to add things up for you. You make up the names, so they work the way you think.

## What you can do

- Create expense and income categories with names of your choice
- Nest categories up to three levels deep to keep a tidy overview
- Edit category names and budgets without losing transaction history
- Set a budget limit on any category
- Delete or archive categories
- See category totals and budget status at a glance

## Terminology

- **Category:** A label for a kind of spending or income (e.g. Groceries, Rent, Salary).
- **Expense:** A category for money going out.
- **Income:** A category for money coming in.
- **Parent category:** A category that contains other categories, creating a hierarchy.
- **Rollup:** The total of a parent category, which automatically adds up all its subcategories.

## Categories concept

Categories split into two types: Expense (money going out) and Income (money coming in). You pick the type when you create a category. Categories can sit inside other categories up to three levels deep, letting you keep the big picture tidy while being specific when you want. The parent category's total automatically adds up all its children, so you can see Housing total without doing sums yourself.

### Hierarchy and rollups

When you create a hierarchy like Housing > Utilities > Electricity, you file transactions only at the bottom level (under Electricity, not Utilities or Housing). The parent categories roll up the total automatically. A subcategory always inherits its parent's type, so everything under an Expense parent is an Expense too.

## Approach

To add a category, press `n` (or choose Add category) and fill in the name and type (Expense or Income). Optionally set a parent category to nest it, or leave it top-level. You can also set a monthly budget limit if you want to track spending against a target.

To edit a category, press `e` to rename it, move it to a different parent, or change its budget. The system updates your transactions and rollup totals automatically when you rename. Moving a category after you have transactions is allowed but changes how your past totals add up — think twice.

To delete, press `d` and confirm by typing the category name. Your transactions stay but no longer roll up into a deleted parent. If a budget referred to it, you'll see that in the confirmation.

## Worked example

You start tracking finances with a new Personal Ledger. You create top-level categories: Housing, Food, and Salary (Income). Under Housing you create Rent and Utilities. Under Utilities you create Electricity and Water. Over a month you record: rent $1,200 to Housing > Rent, electricity $45 to Housing > Utilities > Electricity, water $20 to Housing > Utilities > Water, groceries $300 to Food, and salary $3,000 to Salary. The Housing total shows $1,265 (rollup of Rent $1,200 + Utilities $65). Utilities itself shows $65 (rollup of Electricity $45 + Water $20).

## Screens

The Categories screen lists everything as a tree, with expenses in one section and income in another. Next to each category you see the monthly budget (or "no budget") and how much you've spent or received so far. If you've set a budget, a bar fills as you spend; going over turns it red. In the terminal app, press `g` then `c` to jump there. In the desktop app, use the menu or press `g` then `c`.

## Tips

- **Start small:** A dozen categories is plenty. You can always split one into two later.
- **Prefer broad over fussy:** If you'd never look at the difference between "Coffee" and "Cafes", make them one category.
- **Use tags for the odd ones out:** A holiday or home renovation cuts across lots of categories — use tags to total them without messing with your category list.
- **Set a budget where it matters:** You don't need one on everything. A few on the big spenders is a great start.

## Scope

- Cross-category totals and trends are available through Reports, not here — see Reports for more.
- Archiving (hiding) a category is a future consideration; for now, deletion is the only way to remove one.
- Splitting or merging categories after creation isn't built yet; work around it by creating a new one and reclassifying transactions manually.

## Related

- **Budgets:** set a limit on a category to track spending.
- **Transactions:** each transaction needs exactly one category.
- **Reports:** view spending totals across categories over time.
- **Tags:** for cross-cutting themes that don't fit as a category hierarchy.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Categories | `g` `c` | `g` `c` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Categories. Ticked means built in at least one app; the tag says which.

- [x] CAT-001 (TUI, desktop): Create a category with a code, name, description, type (Expense or Income), color and icon
- [x] CAT-002 (TUI, desktop): List categories with filtering by type and/or active status
- [x] CAT-003 (TUI, desktop): Edit a category's name, type, or active status
- [x] CAT-004 (TUI, desktop): Delete a category
- [x] CAT-005 (TUI, desktop): Create nested categories (parent/child hierarchy)
- [x] CAT-006 (TUI, desktop): View rollup totals for parent categories
- [ ] CAT-007: Archive (hide) a category without deleting it

## For developers

Curious how categories are structured in the codebase, or planning to change them? See the [Categories development documentation](development/categories.md).
