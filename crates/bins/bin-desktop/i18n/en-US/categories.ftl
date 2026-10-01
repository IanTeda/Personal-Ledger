## The Categories page and its Add, Edit and Delete dialogs.

## The Add dialog.

desktop-categories-add =
    .title = Add category
    .submit = Add category

desktop-categories-edit =
    .title = Edit category
    .submit = Save

desktop-categories-delete =
    .title = Delete category — { $name }
    .submit = Delete category

desktop-categories-name-placeholder = e.g. Subscriptions
desktop-categories-field-parent = Parent category
desktop-categories-parent-none = — none, top level —
desktop-categories-field-monthly-budget = Monthly budget
desktop-categories-budget-placeholder = leave blank to track only
desktop-categories-notice-default = Picking a parent will lock the type control to the parent's type.

## Edit dialog notices.

desktop-categories-parent-budget-rollup = Shows rollup of children's budgets
desktop-categories-parent-budget-rollup-sum = { $amount } · rollup of children's budgets
desktop-categories-budget-archived = { $amount } · { $budget } is archived, so its amounts are read-only
desktop-categories-edit-parent-notice = This is a parent category — its budget shows the sum of its children's budgets.
desktop-categories-edit-notice = Changes apply when you save.

## Delete dialog messages.

desktop-categories-delete-warning = This category has <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}.</strong> Deleting it cannot be undone.

desktop-categories-delete-reference = Splits will be moved to Uncategorised · { $budgets ->
    [0] no budget attached
    [one] 1 budget attached
   *[other] { $budgets } budgets attached
}

desktop-categories-delete-confirm-label = Type { $name } to confirm

## The Settings page (2i): its kickers over the Expense and Income tables, the heading's meta, a
## parent row's note and its row actions. `$count` and `$levels` are plural-selector numbers.

desktop-settings-categories-kicker-expense = expense · { $count }
desktop-settings-categories-kicker-income = income · { $count }

desktop-settings-categories-summary = { $count ->
    [one] { $count } category
   *[other] { $count } categories
} · { $levels ->
    [one] { $levels } level
   *[other] { $levels } levels
} deep

desktop-settings-categories-subcount = { $count ->
    [one] { $count } subcategory
   *[other] { $count } subcategories
}

desktop-settings-categories-row-sub = + sub
desktop-settings-categories-add = + Add category
desktop-settings-categories-note = Categories nest to any depth; only leaf categories can be picked on a transaction. Budget and spending figures live in Budgets and Reports; this is just the list you manage.

## The status-line legend while the page has focus.

desktop-hint-expand = expand
desktop-hint-sub = sub
