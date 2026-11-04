## The Remove tag and Merge tags Dialogs, the Tag Removal module's Views, moved from `lib-tags`
## with their keys and text (#1002). A Client composes this layer into its Locale in place of
## `lib-tags`' copies. The Tag Toasts come from the shared layer.

## The Remove tag dialog (7d). Removing always deletes the Tag (#353). `$name` is the Tag's name,
## `$count` how many Transactions carry it.

tags-remove-title = Remove tag — { $name }
tags-remove-submit = Remove tag
tags-remove-unused = No transactions use <strong>{ $name }</strong>. Removing it deletes the tag.
tags-remove-used = This removes <strong>{ $name }</strong> from <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> and deletes the tag. The transactions keep their amount, category and payee. This can't be undone.
tags-remove-confirm-label = Type { $name } to confirm

## The Merge tags dialog (7e, #354). `$name` is a Tag's name and `$count` how many Transactions
## carry it; `$source` and `$target` are the Tags being merged. The callout reads accordingly when
## the target has no colour.

tags-merge-title = Merge tags
tags-merge-source-label = Merge this tag
tags-merge-target-label = into this tag
tags-merge-option = { $name } ({ $count ->
    [one] { $count } txn
   *[other] { $count } txns
})
tags-merge-callout = All <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> tagged "{ $source }" will be retagged "{ $target }" instead, using { $target }'s colour. "{ $source }" is then deleted. This can't be undone, but the transactions themselves are never touched beyond their tag.
tags-merge-callout-no-colour = All <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> tagged "{ $source }" will be retagged "{ $target }" instead, which has no colour. "{ $source }" is then deleted. This can't be undone, but the transactions themselves are never touched beyond their tag.
tags-merge-choose = Choose the tag to merge and the tag to keep.
tags-merge-submit = Merge into "{ $target }"
tags-merge-submit-empty = Merge

## A confirm a Dialog's Tag no longer allows.

tags-refusal-gone = This tag has since been removed.
tags-refusal-merge-into-itself = A tag can't be merged into itself.
