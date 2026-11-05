## The Delete category Dialog, the Category Deletion module's View, moved from `lib-categories`
## with its keys and text (#1004). A Client composes this layer into its Locale in place of
## `lib-categories`' copies. The Category Toasts come from the shared layer.

## The Delete category Dialog (5d). `$name` is the Category's name, `$count` how many of its
## Splits there are and `$budgets` how many Budgets hold a Limit for it.

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

## A confirm the Dialog's Category no longer allows.

categories-delete-refusal-gone = This category has since been deleted.
categories-refusal-has-children = Delete or move its subcategories first.
categories-refusal-uncategorised-in-use = Uncategorised can't be deleted while transactions or bills use it.
