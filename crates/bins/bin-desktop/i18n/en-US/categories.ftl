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
