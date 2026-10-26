# Categories

Categories are how you answer the question "where does my money actually go?" Every transaction gets a category, like Groceries, Rent or Salary, and Personal Ledger uses them to add things up for you. You make up the names, so they work the way you think.

> **Heads up:** in the desktop app, the categories you add, edit, move and delete are saved, so they are still there when you reopen it. The terminal app's Categories screen still works on sample data, and nothing you change there is saved. Archiving a category isn't built yet.

## What you can do

- Create expense and income categories with names of your choice
- Nest categories up to three levels deep to keep a tidy overview
- Edit category names and budgets without losing transaction history
- Set a budget limit on any category
- Delete a category, moving its transactions and bills to Uncategorised
- Keep your categories after you close the desktop app
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

To add a category, press `n` (or choose Add category) and fill in the name and type (Expense or Income). Optionally set a parent category to nest it, or leave it top-level. You can also set a monthly budget if you want to track spending against a target. It becomes that category's amount in the Personal spending budget, from the current month onward (see [Budgets](budgets.md)).

To edit a category, press `e` to rename it, move it to a different parent, or change its budget. The Monthly budget field shows this month's amount in the Personal spending budget. Changing it applies from this month onward, and clearing it stops budgeting the category from this month. Earlier months keep their amounts. On a category with sub-categories the field is read-only and shows the sum of its children, and it is read-only for every category while Personal spending is archived. The system updates your transactions and rollup totals automatically when you rename. Moving a category after you have transactions is allowed but changes how your past totals add up — think twice.

To delete, press `d` and confirm by typing the category name. The delete box says how many transactions use the category and how many budgets are attached, and warns that it can't be undone. Its transactions and bills move to Uncategorised, and any payee that defaulted to the category loses that default. A category with sub-categories can't be deleted until you delete or move its children first, and Uncategorised can't be deleted while transactions or bills still use it.

In the desktop app the add, edit and delete boxes save in one step with the rest of what you changed, and a name another category under the same parent already has is refused as you type.

## Worked example

You start tracking finances with a new Personal Ledger. You create top-level categories: Housing, Food, and Salary (Income). Under Housing you create Rent and Utilities. Under Utilities you create Electricity and Water. Over a month you record: rent $1,200 to Housing > Rent, electricity $45 to Housing > Utilities > Electricity, water $20 to Housing > Utilities > Water, groceries $300 to Food, and salary $3,000 to Salary. The Housing total shows $1,265 (rollup of Rent $1,200 + Utilities $65). Utilities itself shows $65 (rollup of Electricity $45 + Water $20).

## Screens

The Categories screen lists everything as a tree, with expenses in one section and income in another. Next to each category you see the monthly budget (or "no budget") and how much you've spent or received so far. If you've set a budget, a bar fills as you spend; going over turns it red. In the terminal app, press `g` then `c` to jump there. In the desktop app, Categories is a page in Settings: press `g` then `s` and pick Categories, or run `:categories`. There the tree shows names and actions only; the monthly budget is set in the edit dialog.

## Tips

- **Start small:** A dozen categories is plenty. You can always split one into two later.
- **Prefer broad over fussy:** If you'd never look at the difference between "Coffee" and "Cafes", make them one category.
- **Use tags for the odd ones out:** A holiday or home renovation cuts across lots of categories — use tags to total them without messing with your category list.
- **Set a budget where it matters:** You don't need one on everything. A few on the big spenders is a great start.

## Scope

- Cross-category totals and trends are available through Reports, not here — see Reports for more.
- Archiving (hiding) a category isn't built; for now, deletion is the only way to remove one.
- A Ledger file made before categories were saved can't be opened by this version; start a fresh Ledger (see the New Ledger workflow).
- Splitting or merging categories after creation isn't built yet; work around it by creating a new one and reclassifying transactions manually.

## Related

- **[Budgets](budgets.md):** set a monthly amount on a category to track spending against it.
- **Transactions:** each transaction needs exactly one category.
- **Reports:** view spending totals across categories over time.
- **Tags:** for cross-cutting themes that don't fit as a category hierarchy.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Categories | `g` `c` | Settings › Categories (`:categories`) |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Categories. Ticked means built in at least one app; the tag says which.

- [x] CAT-001 (TUI, desktop): Create a category with a name, a type (Expense or Income) and, optionally, a parent
- [x] CAT-002 (TUI, desktop): List categories as a tree, expenses in one section and income in another
- [x] CAT-003 (TUI, desktop): Edit a category's name, type, parent or monthly budget
- [x] CAT-004 (TUI, desktop): Delete a category
- [x] CAT-005 (TUI, desktop): Create nested categories (parent/child hierarchy), up to three levels deep
- [x] CAT-006 (TUI): View rollup totals for parent categories
- [ ] CAT-007: Archive (hide) a category without deleting it
- [x] CAT-008 (desktop): Keep categories after the app closes
- [x] CAT-009 (desktop): Deleting a category moves its transactions and bills to Uncategorised and clears it as a payee's default category

## For developers

Curious how categories are structured in the codebase, or planning to change them? See the [Categories development documentation](development/categories.md).
