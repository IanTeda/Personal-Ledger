## The Documents module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. Column labels other screens share (Actions) come from
## the shared layer.

## The Settings › Documents page (`docs/ux/desktop-mockups/16-settings/`, frame 16p, as amended by
## #479): the Ledger's user-managed Document Types. `$typed` is Filed Documents, `$all` every Document.

documents-types-count = { $count ->
    [one] { $count } type
   *[other] { $count } types
}
documents-types-scope = { $types } · { $files }
documents-types-files-typed = { $all ->
    [one] { $typed } of { $all } file typed
   *[other] { $typed } of { $all } files typed
}

## The kicker, Add button and column headings. `$glyph` is the Add button's leading plus sign.

documents-types-kicker = Document types · in type-filter order
documents-types-add-button = { $glyph } Add document type
documents-types-column-type = Type
documents-types-column-tracks-date = Tracks date
documents-types-column-remind = Remind
documents-types-column-financial-year = Financial year
documents-types-column-files = Files

## Cells.

documents-types-tracks-renews = Renews
documents-types-tracks-ends = Ends
documents-types-tracks-expires = Expires
documents-types-tracks-revalue = Revalue
documents-types-remind-days = { $count ->
    [one] { $count } day before
   *[other] { $count } days before
}
documents-types-remind-months = { $count ->
    [one] { $count } month before
   *[other] { $count } months before
}
documents-types-none = —
documents-types-yes = Yes
documents-types-no = No
documents-types-flag-default = default
documents-types-row-edit = edit
documents-types-row-remove = remove

## The four notes beneath the table, one sentence each.

documents-types-note-tracks-date = <strong>Tracks date</strong> gives every document of the type a Key Date of that kind, and Remind raises a notification that long before it; Remind does nothing without a Tracks date.
documents-types-note-financial-year = <strong>Financial year</strong> lets the type be filtered and shown by financial year in Documents.
documents-types-note-removal = <strong>Removing</strong> a type that still has files asks which type to move them to.
documents-types-note-default = <strong>Other</strong> is the fallback type: it can't be removed, though it can be renamed and reordered.

## The status line's keys, and the hint shown when `x` is pressed on Other.

documents-types-hint-row = row
documents-types-hint-reorder = reorder
documents-types-hint-edit = edit
documents-types-hint-remove = remove
documents-types-hint-new = new
documents-types-hint-default-kept = Other can't be removed

## The command palette's descriptions of the Commands the keymap lists, in lower case as the palette shows them.

documents-types-command-new = add a Document type
documents-types-command-edit = edit the selected Document type
documents-types-command-remove = remove the selected Document type

## The Add, Edit and Remove Document type Dialogs (#478). `$files` is a count of Filed files.

documents-types-dialog-add-title = Add Document type
documents-types-dialog-edit-title = Edit Document type
documents-types-dialog-remove-title = Remove Document type
documents-types-dialog-default-title = Other can't be removed
documents-types-dialog-add-submit = Add type
documents-types-dialog-save = Save
documents-types-dialog-close = Close

documents-types-field-name = Name
documents-types-field-name-placeholder = e.g. Leases
documents-types-field-name-hint = { $max ->
    [one] Up to { $max } character, and different from every other type.
   *[other] Up to { $max } characters, and different from every other type.
}
documents-types-field-tracks-date = Tracks date
documents-types-field-tracks-date-hint = The kind of Key Date every document of this type carries.
documents-types-field-remind = Remind
documents-types-field-remind-hint = How long before the Key Date to raise a reminder. Needs a Tracks date.
documents-types-field-financial-year = Financial year
documents-types-field-financial-year-hint = Lets the type be filtered and shown by financial year.

documents-types-option-none = None
documents-types-option-days = { $count } d
documents-types-option-months = { $count } mo

documents-types-error-name-empty = A name is required.
documents-types-error-name-too-long = { $max ->
    [one] Names are at most { $max } character.
   *[other] Names are at most { $max } characters.
}
documents-types-error-name-taken = Another type already has that name.

documents-types-edit-usage = { $files ->
    [one] Used by { $files } Filed file.
   *[other] Used by { $files } Filed files.
} { $dated ->
    [one] { $dated } file has a date.
   *[other] { $dated } files have dates.
} Changing the kind relabels them; clearing it makes the dates inert.

documents-types-remove-with-files = { $name } has { $files ->
    [one] { $files } Filed file.
   *[other] { $files } Filed files.
} Move them to:
documents-types-remove-destination = Destination
documents-types-remove-moved-note = Moved files keep their dates, which go hidden and inert if the destination tracks no date.
documents-types-remove-confirm-move = { $files ->
    [one] Move { $files } file and remove
   *[other] Move { $files } files and remove
}
documents-types-remove-no-files = No Filed files use { $name }, so nothing moves.
documents-types-remove-refused = That type can't be removed now. Check its destination and try again.
documents-types-remove-confirm = Remove type

documents-types-default-notice = Other is the default type: the fallback for documents with no type and the pre-selected destination when another type is removed. You can still rename and reorder it.
