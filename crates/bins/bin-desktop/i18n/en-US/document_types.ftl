## The Settings › Documents page (`docs/ux/desktop/16-settings/`, frame 16p, as amended by #479):
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
desktop-document-types-note-default = <strong>Other</strong> is the fallback type: it can't be removed or renamed.

## The status line's keys and the hint shown when `x` is pressed on Other.

desktop-hint-reorder = reorder
desktop-document-types-hint-default-kept = Other can't be removed
