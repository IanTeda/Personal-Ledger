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
