## The Settings › Documents page (`docs/ux/desktop-mockups/16-settings/`, frame 16p, as amended by #479):
## the Ledger's user-managed Document Types.

## The heading's meta. `$typed` is Filed Documents, `$all` every Document.

desktop-document-types-count = { $count ->
    [one] { $count } type
   *[other] { $count } types
}
desktop-document-types-scope = { $types } · { $files }
desktop-document-types-files-typed = { $typed } of { $all } files typed

## The kicker, Add button and column headings. `$glyph` is the Add button's leading plus sign.

desktop-document-types-kicker = Document types · in type-filter order
desktop-document-types-add-button = { $glyph } Add document type
desktop-document-types-column-type = Type
desktop-document-types-column-tracks-date = Tracks date
desktop-document-types-column-remind = Remind
desktop-document-types-column-financial-year = Financial year
desktop-document-types-column-files = Files

## Cells.

desktop-document-types-tracks-renews = Renews
desktop-document-types-tracks-ends = Ends
desktop-document-types-tracks-expires = Expires
desktop-document-types-tracks-revalue = Revalue
desktop-document-types-remind-days = { $count ->
    [one] { $count } day before
   *[other] { $count } days before
}
desktop-document-types-remind-months = { $count ->
    [one] { $count } month before
   *[other] { $count } months before
}
desktop-document-types-none = —
desktop-document-types-yes = Yes
desktop-document-types-no = No
desktop-document-types-flag-default = default
desktop-document-types-row-edit = edit
desktop-document-types-row-remove = remove

## The four notes beneath the table, one sentence each.

desktop-document-types-note-tracks-date = <strong>Tracks date</strong> gives every document of the type a Key Date of that kind, and Remind raises a notification that long before it; Remind does nothing without a Tracks date.
desktop-document-types-note-financial-year = <strong>Financial year</strong> lets the type be filtered and shown by financial year in Documents.
desktop-document-types-note-removal = <strong>Removing</strong> a type that still has files asks which type to move them to.
desktop-document-types-note-default = <strong>Other</strong> is the fallback type: it can't be removed, though it can be renamed and reordered.

## The status line's keys and the hint shown when `x` is pressed on Other.

desktop-hint-reorder = reorder
desktop-document-types-hint-default-kept = Other can't be removed

## The Add, Edit and Remove Document type dialogs (#478). `$files` is a count of Filed files.

desktop-document-types-dialog-add-title = Add Document type
desktop-document-types-dialog-edit-title = Edit Document type
desktop-document-types-dialog-remove-title = Remove Document type
desktop-document-types-dialog-default-title = Other can't be removed
desktop-document-types-dialog-add-submit = Add type
desktop-document-types-dialog-save = Save
desktop-document-types-dialog-close = Close

desktop-document-types-field-name = Name
desktop-document-types-field-name-placeholder = e.g. Leases
desktop-document-types-field-name-hint = Up to 40 characters, and different from every other type.
desktop-document-types-field-tracks-date = Tracks date
desktop-document-types-field-tracks-date-hint = The kind of Key Date every document of this type carries.
desktop-document-types-field-remind = Remind
desktop-document-types-field-remind-hint = How long before the Key Date to raise a reminder. Needs a Tracks date.
desktop-document-types-field-financial-year = Financial year
desktop-document-types-field-financial-year-hint = Lets the type be filtered and shown by financial year.

desktop-document-types-option-none = None
desktop-document-types-option-days = { $count } d
desktop-document-types-option-months = { $count } mo

desktop-document-types-error-name-empty = A name is required.
desktop-document-types-error-name-too-long = Names are at most { $max } characters.
desktop-document-types-error-name-taken = Another type already has that name.

desktop-document-types-edit-usage = { $files ->
    [one] Used by { $files } Filed file.
   *[other] Used by { $files } Filed files.
} { $dated ->
    [one] { $dated } file has a date.
   *[other] { $dated } files have dates.
} Changing the kind relabels them; clearing it makes the dates inert.

desktop-document-types-remove-with-files = { $name } has { $files ->
    [one] { $files } Filed file.
   *[other] { $files } Filed files.
} Move them to:
desktop-document-types-remove-destination = Destination
desktop-document-types-remove-moved-note = Moved files keep their dates, which go hidden and inert if the destination tracks no date.
desktop-document-types-remove-confirm-move = { $files ->
    [one] Move { $files } file and remove
   *[other] Move { $files } files and remove
}
desktop-document-types-remove-no-files = No Filed files use { $name }, so nothing moves.
desktop-document-types-remove-confirm = Remove type

desktop-document-types-default-notice = Other is the default type: the fallback for documents with no type and the pre-selected destination when another type is removed. You can still rename and reorder it.
