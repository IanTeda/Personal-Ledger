## The Tags module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. Column labels other screens share (Name, Actions) and
## the Tag Toasts come from the shared layer.

## The active tag count, used by the page summary and the status line.

tags-count = { $count ->
    [one] { $count } tag
   *[other] { $count } tags
}

## The page summary's duplicates clause, the count in bold; and the status line's plain version.

tags-likely-duplicates = { $count ->
    [one] <strong>{ $count }</strong> likely duplicate
   *[other] <strong>{ $count }</strong> likely duplicates
}
tags-status-duplicates = { $count ->
    [one] { $count } likely duplicate
   *[other] { $count } likely duplicates
}

## The Settings page (2j): its kicker over the A–Z list, the Add button, the empty list, each
## row's tags and actions, and the footnote. `$glyph` is the Add button's leading plus sign, `$key`
## the key token that opens Add, `$name` the suggested merge target's name.

tags-kicker = a–z
tags-add-button = { $glyph } Add tag
tags-empty = No tags yet. Press { $key } to add one.
tags-column-tag = Tag
tags-inactive = inactive
tags-duplicate-of = looks like a duplicate of "{ $name }"
tags-row-merge = merge
tags-row-edit = edit
tags-row-remove = remove
tags-footnote = Tags are free-form and can be applied to any number of transactions. Removing a tag untags its transactions — it never deletes them. Flagged tags differ from another tag only in case, spacing or punctuation; merge them to fold them together.

## The status-line legend while the page has focus.

tags-hint-row = row
tags-hint-edit = edit
tags-hint-remove = remove
tags-hint-merge = merge
tags-hint-new = new

## The Add tag dialog (7b), and the name, colour and hex fields the Edit dialog shares. `$name` is
## the Tag already holding the name once case, spaces and punctuation are ignored.

tags-add-title = Add tag
tags-add-submit = Add tag
tags-name-placeholder = e.g. work-trip
tags-field-colour = Colour
tags-colour-none = none
tags-hex-placeholder = #RRGGBB
tags-add-callout = Tag names are matched ignoring case, spaces and punctuation — "Work Trip" can't be added while "work-trip" exists.
tags-error-name-taken = { $name } already exists
tags-error-no-letter-or-digit = A tag name needs a letter or a digit
tags-error-hex = A colour is # and six hex digits, e.g. #4A7C9E

## The Edit tag dialog (7c). `$name` is the Tag's name before this edit, `$count` how many
## Transactions carry it.

tags-edit-title = Edit tag — { $name }
tags-edit-submit = Save
tags-field-active = Active
tags-active-hint = inactive tags aren't offered when tagging a split
tags-edit-callout = { $count ->
    [one] Used by { $count } transaction.
   *[other] Used by { $count } transactions.
} Renaming or recolouring updates all of them immediately — nothing needs re-tagging.

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
