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

## The Documents page (`docs/ux/desktop-mockups/04-documents/`): the status line's legend and the `?` cheat sheet. The Library list's strings are the handoff's own; the index rail's are its scope keys.

documents-hint-scope = scope
documents-hint-list = list
documents-hint-inbox = inbox
documents-hint-link = link
documents-hint-show-in-folder = show in folder
documents-hint-sort = sort
documents-hint-import = import
documents-hint-accept = accept
documents-hint-undo = undo
documents-hint-accept-all = accept all strong matches
documents-hint-link-elsewhere = link elsewhere
documents-hint-follow = follow link
documents-hint-row = row
documents-hint-open = open
documents-hint-edit = edit
documents-hint-add = add
documents-hint-skip = skip

## The status line's right-hand note. `$size` is the stored size, `$file` the Ledger file name.

documents-status-stored = { $size } · stored beside { $file }

## The index rail. The two Financial Year labels take their years as text so no digit grouping is applied.

documents-rail-inbox = Inbox
documents-rail-all = All documents
documents-rail-type = Type
documents-rail-year = Financial year
documents-rail-fy = FY { $start }–{ $end }
documents-rail-earlier = Earlier

## The Library's header, toolbar and count line. `$count` is how many Documents are filed.

documents-library-title = Documents
documents-subline = { $count ->
    [one] { $count } file
   *[other] { $count } files
} · receipts, statements, policies and tax papers, linked to what they prove
documents-import-button = Import…
documents-add-button = + Add
documents-search-placeholder = / search names and text inside documents
documents-sort-newest = Newest
documents-sort-expiring = Expiring
documents-count-line = { $shown } of { $total } · { $review } to review
documents-empty-scope = Nothing is filed here yet.
documents-empty-search = No documents match.

## A Document row. The chips show what it links to; a Transaction's chip is its signed amount.

documents-not-linked = not linked
documents-flag-renews = Renews { $date }
documents-flag-ends = Ends { $date }
documents-flag-expires = Expires { $date }
documents-flag-revalue = Revalue by { $date }
documents-flag-renewed = Renewal was due { $date }
documents-flag-ended = Ended { $date }
documents-flag-expired = Expired { $date }
documents-flag-revalue-overdue = Revalue was due { $date }

## The detail pane. `$kind` is the file's extension in capitals, `$pages` a plural-selector number.

documents-detail-meta = { $kind } · { $pages ->
    [one] { $pages } page
   *[other] { $pages } pages
} · { $size } · page 1 of { $pages }
documents-detail-type = Type
documents-detail-date = Document date
documents-detail-year = Financial year
documents-detail-reminder = { $date } · reminder set
documents-detail-key-renews = Renews
documents-detail-key-ends = Ends
documents-detail-key-expires = Expires
documents-detail-key-revalue = Revalue by
documents-linked-to = Linked to
documents-link-transaction = Transaction
documents-link-inventory = Inventory
documents-link-account = Account
documents-link-payee = Payee
documents-link-bill = Bill
documents-link-add = + Link to transaction, account, item…
documents-open = Open
documents-show-in-folder = Show in folder
documents-link-elsewhere = Link elsewhere…

## The Inbox (4b). `$count` is a plural-selector number in the sublines and a text count in the buttons; `$date` is a formatted date.

documents-inbox-title = Inbox
documents-inbox-subline = { $count ->
    [one] { $count } unfiled
   *[other] { $count } unfiled
} · dropped, scanned or imported from a watched folder · each gets a suggested link
documents-watched-folder = Watched folder…
documents-watched-folder-toast = Watched folders are not available yet.
documents-accept-all = Accept all strong matches · { $count }
documents-inbox-col-file = File
documents-inbox-col-link = Suggested link
documents-inbox-empty = Nothing to file
documents-inbox-footnote = Accepted files move to their type and financial year; nothing is renamed on disk.
documents-inbox-below = { $count ->
    [one] { $count } more below
   *[other] { $count } more below
}
documents-drop-target = Drop files anywhere in the app to add them to the Inbox
documents-accept = Accept
documents-file-button = File…
documents-accept-next = Accept & next
documents-skip = Skip
documents-inbox-meta-image = Read from the image · check before accepting
documents-inbox-meta-file = Read from the file · check before accepting
documents-inbox-meta-unreadable = Nothing could be read from this file

## Where an Unfiled Document came from.

documents-source-scanned = Scanned { $date }
documents-source-emailed = Emailed { $date }
documents-source-downloads = Downloads · { $date }
documents-source-watched = Watched folder · { $date }
documents-source-dropped = Dropped { $date }
documents-source-import = Imported { $date }

## The Suggested Link column. `$days` is a plural-selector number.

documents-suggest-summary = { $kind } · { $who } · { $amount }
documents-suggest-summary-bare = { $kind } · { $amount }
documents-suggest-unreadable = Unreadable — no amount found
documents-suggest-unreadable-hint = File it by hand, or skip
documents-suggest-none = { $days ->
    [one] No suggestion — nothing within { $days } day
   *[other] No suggestion — nothing within { $days } days
}
documents-suggest-none-hint = File it by hand, or skip
documents-signals-all = amount, date and payee match
documents-signals-amount-date = amount and date match
documents-signals-amount-payee = amount and payee match
documents-signals-date-payee = date and payee match
documents-signals-amount = amount matches
documents-signals-payee = payee matches
documents-signals-date = date matches

## The Inbox detail pane.

documents-fact-merchant = Merchant
documents-fact-total = Total
documents-fact-none = —
documents-suggested-link = Suggested link
documents-suggested-detail = { $account } · { $documents }
documents-suggested-no-document = no document yet
documents-suggested-has-document = already has a document
documents-other-candidates = Other candidates
documents-other-none = { $days ->
    [one] Other candidates: none within { $days } day
   *[other] Other candidates: none within { $days } days
}

## Filing, undoing, opening a file and importing. `$title` is the Document's Title, `$count`, `$imported` and `$skipped` text counts, `$name` a file name and `$path` where it should be.

documents-toast-filed = Filed "{ $title }" · u undo
documents-toast-filed-many = Filed { $count } · u undo
documents-toast-undone = { $count ->
    [one] Put { $count } back in the Inbox.
   *[other] Put { $count } back in the Inbox.
}
documents-status-nothing-to-undo = Nothing to undo.
documents-status-no-strong = No strong matches to accept.
documents-status-opened = Opened { $name }.
documents-status-shown = Showing { $name } in its folder.
documents-status-missing = { $name } is not at { $path }.
documents-status-no-document = No document is selected.
documents-toast-imported = { $count ->
    [one] Imported { $count } file into the Inbox.
   *[other] Imported { $count } files into the Inbox.
}
documents-toast-import-partial = { $imported } imported, { $skipped } not imported.
documents-toast-unlinked = Unlinked { $record }.

## The command palette's descriptions of the Documents page's Commands.

documents-command-inbox = file the Documents waiting in the Inbox
documents-command-library = browse the filed Documents
documents-command-accept-all = accept every strong Inbox match
documents-command-add = add a Document you already know the meaning of
documents-command-import = import files into the Inbox

## The Documents page's Dialogs: Add, Edit, Import, Edit extracted facts, Accept all and the link picker. `$title` is a Document's Title, `$path` a path as typed, `$record` a linked record's name and `$count` a count.

documents-dialog-hint-cancel = cancel
documents-dialog-hint-confirm = confirm
documents-dialog-hint-next-field = next field
documents-dialog-hint-kind = kind
documents-dialog-add-title = Add document
documents-dialog-add-submit = Add document
documents-dialog-add-note = The file stays where it is. Add its links afterwards with the link key.
documents-dialog-edit-title = Edit document
documents-dialog-edit-submit = Save
documents-dialog-import-title = Import files
documents-dialog-import-submit = Import
documents-dialog-import-note = Each file lands in the Inbox, unfiled. Files are referenced where they are, never moved or renamed.
documents-dialog-field-path = File path
documents-dialog-field-path-placeholder = ~/Documents/receipt.pdf
documents-dialog-field-paths = File paths, one per line
documents-dialog-field-paths-placeholder = ~/Downloads/scan.pdf
documents-dialog-field-title = Title
documents-dialog-field-type = Document type
documents-dialog-field-date = Document date
documents-dialog-field-key-kind = Key date
documents-dialog-field-key-date = Key date on
documents-dialog-field-reminder = Remind me before it falls due
documents-dialog-key-none = None
documents-dialog-error-path-missing = There is no file at { $path }.
documents-dialog-error-unsupported = Only PDF, JPG and PNG files can be Documents.
documents-dialog-error-already = Already in the Library as { $title }.
documents-dialog-error-total = Enter an amount such as 212.40.
documents-dialog-toast-added = Added "{ $title }".
documents-dialog-toast-saved = Saved "{ $title }".
documents-dialog-facts-title = Edit extracted facts
documents-dialog-facts-submit = Save
documents-dialog-facts-note = Suggestions are recomputed when you save. Clear the total to mark the file unreadable.
documents-dialog-toast-facts-saved = Updated what was read from "{ $title }".
documents-dialog-accept-all-title = Accept all strong matches
documents-dialog-accept-all-body = { $count ->
    [one] File { $count } document with a strong suggestion?
   *[other] File { $count } documents with strong suggestions?
}
documents-dialog-accept-all-submit = File { $count }
documents-dialog-accept-all-note = Only matches where the amount, date and payee all agree. You can undo it with u.
documents-dialog-picker-kind-all = All
documents-dialog-picker-file-without-link = File without link
documents-dialog-picker-placeholder = Search transactions, accounts, payees, bills, items…
documents-dialog-picker-title-link = Link
documents-dialog-picker-title-file = File
documents-dialog-picker-title-follow = Follow
documents-dialog-picker-empty = Nothing matches.
documents-dialog-picker-type = Type
documents-dialog-picker-type-none = choose with ← →
documents-dialog-picker-hint-pick = ↑↓ move · tab kind · enter pick · esc close
documents-dialog-picker-hint-toggle = ↑↓ move · tab kind · enter link or unlink · esc close
documents-dialog-picker-hint-follow = ↑↓ move · enter go · esc close
documents-dialog-picker-need-type = Choose a document type first (← →).
documents-dialog-toast-linked = Linked { $record }.
