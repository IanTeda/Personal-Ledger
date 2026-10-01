## The Tags page (7a).

## The active tag count, used by the page summary and the status line.

desktop-tags-count = { $count ->
    [one] { $count } tag
   *[other] { $count } tags
}

## The page summary's duplicates clause, the count in bold, before the "merge them" link; and the
## status line's plain version.

desktop-tags-likely-duplicates = <strong>{ $count }</strong> likely { $count ->
    [one] duplicate
   *[other] duplicates
} —
desktop-tags-status-duplicates = { $count ->
    [one] { $count } likely duplicate
   *[other] { $count } likely duplicates
}

## The page. `$glyph` is the Add button's leading plus sign, `$key` the key token that opens Add,
## `$name` the suggested merge target's name.

desktop-tags-add-button = { $glyph } Add tag
desktop-tags-empty = No tags yet. Press { $key } to add one.
desktop-tags-column-tag = Tag
desktop-tags-inactive = inactive
desktop-tags-duplicate-of = looks like a duplicate of "{ $name }"
desktop-tags-row-edit = edit
desktop-tags-row-remove = remove
desktop-tags-footnote = Tags are free-form and can be applied to any number of transactions. Removing a tag untags its transactions — it never deletes them. Flagged tags differ from another tag only in case, spacing or punctuation; merge them to fold them together.

## The Add tag dialog (7b), and the name, colour and hex fields the Edit dialog shares. `$name` is
## the Tag already holding the name once case, spaces and punctuation are ignored.

desktop-tags-add-title = Add tag
desktop-tags-add-submit = Add tag
desktop-tags-name-placeholder = e.g. work-trip
desktop-tags-field-colour = Colour
desktop-tags-colour-none = none
desktop-tags-hex-placeholder = #RRGGBB
desktop-tags-add-callout = Tag names are matched ignoring case, spaces and punctuation — "Work Trip" can't be added while "work-trip" exists.
desktop-tags-error-name-taken = { $name } already exists
desktop-tags-error-no-letter-or-digit = A tag name needs a letter or a digit
desktop-tags-error-hex = A colour is # and six hex digits, e.g. #4A7C9E

## The Edit tag dialog (7c). `$name` is the Tag's name before this edit, `$count` how many
## Transactions carry it.

desktop-tags-edit-title = Edit tag — { $name }
desktop-tags-edit-submit = Save
desktop-tags-field-active = Active
desktop-tags-active-hint = inactive tags aren't offered when tagging a split
desktop-tags-edit-callout = { $count ->
    [one] { $count } transaction uses this tag.
   *[other] { $count } transactions use this tag.
} Renaming or recolouring updates all of them immediately — nothing needs re-tagging.

## The Remove tag dialog (7d). Removing always deletes the Tag (#353). `$name` is the Tag's name,
## `$count` how many Transactions carry it.

desktop-tags-remove-title = Remove tag — { $name }
desktop-tags-remove-submit = Remove tag
desktop-tags-remove-unused = No transactions use <strong>{ $name }</strong>. Removing it deletes the tag.
desktop-tags-remove-used = This removes <strong>{ $name }</strong> from <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> and deletes the tag. The transactions keep their amount, category and payee. This can't be undone.
desktop-tags-remove-confirm-label = Type { $name } to confirm

## The Merge tags dialog (7e, #354). `$name` is a Tag's name and `$count` how many Transactions
## carry it; `$source` and `$target` are the Tags being merged. The callout reads accordingly when
## the target has no colour.

desktop-tags-merge-title = Merge tags
desktop-tags-merge-source-label = Merge this tag
desktop-tags-merge-target-label = into this tag
desktop-tags-merge-option = { $name } ({ $count ->
    [one] { $count } txn
   *[other] { $count } txns
})
desktop-tags-merge-callout = All <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> tagged "{ $source }" will be retagged "{ $target }" instead, using { $target }'s colour. "{ $source }" is then deleted. This can't be undone, but the transactions themselves are never touched beyond their tag.
desktop-tags-merge-callout-no-colour = All <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong> tagged "{ $source }" will be retagged "{ $target }" instead, which has no colour. "{ $source }" is then deleted. This can't be undone, but the transactions themselves are never touched beyond their tag.
desktop-tags-merge-choose = Choose the tag to merge and the tag to keep.
desktop-tags-merge-submit = Merge into "{ $target }"
desktop-tags-merge-submit-empty = Merge

## The Settings page (2j): its kicker over the A–Z list and the row's direct merge action. The
## heading's meta, Add button, row actions and footnote are the Tags page's own messages.

desktop-settings-tags-kicker = a–z
desktop-settings-tags-row-merge = merge
