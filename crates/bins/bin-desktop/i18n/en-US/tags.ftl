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
desktop-tags-merge-link = merge them
desktop-tags-status-duplicates = { $count ->
    [one] { $count } likely duplicate
   *[other] { $count } likely duplicates
}

## The page. `$glyph` is the Add button's leading plus sign, `$key` the key token that opens Add,
## `$name` the suggested merge target's name.

desktop-tags-add-button = { $glyph } Add tag
desktop-tags-empty = No tags yet. Press { $key } to add one.
desktop-tags-column-tag = Tag
desktop-tags-column-transactions = Transactions
desktop-tags-column-total = Total
desktop-tags-column-last-used = Last used
desktop-tags-inactive = inactive
desktop-tags-duplicate-of = looks like a duplicate of "{ $name }"
desktop-tags-row-edit = edit
desktop-tags-row-remove = remove
desktop-tags-footnote = Tags are free-form and can be applied to any number of transactions. Removing a tag untags its transactions — it never deletes them. Flagged tags differ from another tag only in case, spacing or punctuation; merge them to fold them together.
