## The Categories module's Messages, read by its Presenters. Each Client composes this layer into
## its Locale; no Client catalogue keeps a copy. Column labels other screens share (Name, Type,
## Actions) and the Category Toasts come from the shared layer.

## The Settings page (2i): its kickers over the Expense and Income tables, the heading's meta, a
## parent row's note and its row actions. `$count` and `$levels` are plural-selector numbers.

categories-summary = { $count ->
    [one] { $count } category
   *[other] { $count } categories
} · { $levels ->
    [one] { $levels } level
   *[other] { $levels } levels
} deep
categories-kicker-expense = expense · { $count }
categories-kicker-income = income · { $count }
categories-add-button = + Add category
categories-subcount = { $count ->
    [one] { $count } subcategory
   *[other] { $count } subcategories
}
categories-row-sub = + sub
categories-row-edit = edit
categories-row-delete = delete
categories-note = Categories nest to any depth; only leaf categories can be picked on a transaction. Budget and spending figures live in Budgets and Reports; this is just the list you manage.

## The status line: a Delete asked of a parent.

categories-status-delete-children-first = delete or move its children first

## The page's legend.

categories-hint-row = row
categories-hint-expand = expand
categories-hint-edit = edit
categories-hint-delete = delete
categories-hint-new = new
categories-hint-sub = sub

## The Add and Edit category Dialogs (5b, 5c). `$parent` is the parent's name and `$kind` its type,
## one of the two type Messages below.

categories-add-title = Add category
categories-add-submit = Add category
categories-edit-title = Edit category
categories-edit-submit = Save
categories-name-placeholder = e.g. Subscriptions
categories-name-taken = Another category under that parent already has this name.
categories-type-expense = expense
categories-type-income = income
categories-field-parent = Parent category
categories-field-optional = (optional)
categories-parent-none = — none, top level —
categories-field-monthly-budget = Monthly budget
categories-budget-placeholder = leave blank to track only
categories-notice-default = Picking a parent will lock the type control to the parent's type.
categories-notice-nested = Nesting under { $parent } inherits its type ({ $kind }) — the type control locks once a parent is picked.
categories-edit-notice = Changes apply when you save.
categories-edit-parent-notice = This is a parent category — its budget shows the sum of its children's budgets.
categories-edit-usage-notice = This category has { $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}. Changing its type will affect these transactions.

## The Monthly budget field while it is read-only. `$amount` is the amount shown, `$empty` the
## placeholder for none and `$budget` the archived Budget's name.

categories-parent-budget-rollup = Shows rollup of children's budgets
categories-parent-budget-rollup-sum = { $amount } · rollup of children's budgets
categories-budget-archived = { $amount } · { $budget } is archived, so its amounts are read-only
categories-budget-none = —

## The Delete category Dialog (5d).

categories-delete-title = Delete category — { $name }
categories-delete-submit = Delete category
categories-delete-warning = This category has <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}.</strong> Deleting it cannot be undone.
categories-delete-reference = Splits will be moved to Uncategorised · { $budgets ->
    [0] no budget attached
    [one] { $budgets } budget attached
   *[other] { $budgets } budgets attached
}
categories-delete-confirm-label = Type { $name } to confirm

## Why a confirm was refused, shown in the Dialog.

categories-refusal-gone = This category has since been deleted.
categories-refusal-parent-gone = The parent category has since been deleted.
categories-refusal-type-mismatch = A category must be the same type as its parent.
categories-refusal-too-deep = Categories nest at most three levels deep.
categories-refusal-cycle = A category can't go under itself or one of its own subcategories.
categories-refusal-has-children = Delete or move its subcategories first.
categories-refusal-uncategorised-in-use = Uncategorised can't be deleted while transactions or bills use it.
categories-refusal-budget-not-a-leaf = Only an expense category with no subcategories can have a monthly budget.
categories-refusal-budget-read-only = The budget is read-only, so its monthly amount can't change.
categories-refusal-budget-unavailable = The monthly budget couldn't be saved.

## The command palette's descriptions of the Commands the keymap lists, in lower case as the palette shows them.

categories-command-new = add a Category
categories-command-edit = edit the selected Category
categories-command-delete = delete the selected Category
categories-command-new-child = add a Category under the selected one
