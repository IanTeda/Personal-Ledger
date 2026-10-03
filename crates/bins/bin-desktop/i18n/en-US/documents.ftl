## The Documents surface (`docs/ux/desktop/04-documents/`): the Library, its index rail and detail pane,
## and the Add, Import and Edit dialogs.

## The status line's legend. The Library list's strings are the handoff's own; the index rail's are
## its scope keys.

desktop-hint-scope = scope
desktop-hint-list = list
desktop-hint-inbox = inbox
desktop-hint-link = link
desktop-hint-show-in-folder = show in folder
desktop-hint-sort = sort
desktop-hint-import = import
desktop-hint-accept = accept
desktop-hint-undo = undo
desktop-hint-accept-all = accept all strong matches

## The status line's right-hand note. `$size` is the stored size, `$file` the Ledger file name.

desktop-documents-status-stored = { $size } · stored beside { $file }

## The index rail. `$count` is a plural-selector number; the two Financial Year labels take their
## years as text so no digit grouping is applied.

desktop-documents-rail-inbox = Inbox
desktop-documents-rail-all = All documents
desktop-documents-rail-type = Type
desktop-documents-rail-year = Financial year
desktop-documents-rail-fy = FY { $start }–{ $end }
desktop-documents-rail-earlier = Earlier

## The Document Types. The rail names them in the plural; the detail pane and the row's caps label
## in the singular.


## The Library's header, toolbar and count line. `$count` is how many Documents are filed.

desktop-documents-subline = { $count ->
    [one] { $count } file
   *[other] { $count } files
} · receipts, statements, policies and tax papers, linked to what they prove
desktop-documents-import-button = Import…
desktop-documents-add-button = + Add
desktop-documents-search-placeholder = / search names and text inside documents
desktop-documents-sort-newest = Newest
desktop-documents-sort-expiring = Expiring
desktop-documents-count-line = { $shown } of { $total } · { $review } need review
desktop-documents-empty-scope = Nothing is filed here yet.
desktop-documents-empty-search = No documents match.

## A Document row. The chips show what it links to; a Transaction's chip is its signed amount.

desktop-documents-not-linked = not linked
desktop-documents-flag-renews = Renews { $date }
desktop-documents-flag-ends = Ends { $date }
desktop-documents-flag-expires = Expires { $date }
desktop-documents-flag-revalue = Revalue by { $date }
desktop-documents-flag-renewed = Renewal was due { $date }
desktop-documents-flag-ended = Ended { $date }
desktop-documents-flag-expired = Expired { $date }
desktop-documents-flag-revalue-overdue = Revalue was due { $date }

## The detail pane. `$kind` is the file's extension in capitals, `$pages` a plural-selector number.

desktop-documents-detail-meta = { $kind } · { $pages ->
    [one] { $pages } page
   *[other] { $pages } pages
} · { $size } · page 1 of { $pages }
desktop-documents-detail-type = Type
desktop-documents-detail-date = Document date
desktop-documents-detail-year = Financial year
desktop-documents-detail-reminder = { $date } · reminder set
desktop-documents-detail-key-renews = Renews
desktop-documents-detail-key-ends = Ends
desktop-documents-detail-key-expires = Expires
desktop-documents-detail-key-revalue = Revalue by
desktop-documents-linked-to = Linked to
desktop-documents-link-transaction = Transaction
desktop-documents-link-inventory = Inventory
desktop-documents-link-account = Account
desktop-documents-link-payee = Payee
desktop-documents-link-bill = Bill
desktop-documents-link-add = + Link to transaction, account, item…
desktop-documents-open = Open
desktop-documents-show-in-folder = Show in folder

## The Inbox (4b). `$count` is a plural-selector number in the sublines and a text count in the
## buttons; `$date` is a formatted date.

desktop-documents-inbox-title = Inbox
desktop-documents-inbox-subline = { $count ->
    [one] { $count } unfiled
   *[other] { $count } unfiled
} · dropped, scanned or imported from a watched folder · each gets a suggested link
desktop-documents-watched-folder = Watched folder…
desktop-documents-watched-folder-toast = Watched folders are not available yet.
desktop-documents-accept-all = Accept all strong matches · { $count }
desktop-documents-inbox-col-file = File
desktop-documents-inbox-col-link = Suggested link
desktop-documents-inbox-empty = Nothing to file
desktop-documents-inbox-footnote = Accepted files move to their type and financial year; nothing is renamed on disk.
desktop-documents-inbox-below = { $count ->
    [one] { $count } more below
   *[other] { $count } more below
}
desktop-documents-drop-target = Drop files anywhere in the app to add them to the Inbox
desktop-documents-accept = Accept
desktop-documents-file-button = File…
desktop-documents-accept-next = Accept & next
desktop-documents-skip = Skip

## Where an Unfiled Document came from.

desktop-documents-source-scanned = Scanned { $date }
desktop-documents-source-emailed = Emailed { $date }
desktop-documents-source-downloads = Downloads · { $date }
desktop-documents-source-watched = Watched folder · { $date }
desktop-documents-source-dropped = Dropped { $date }
desktop-documents-source-import = Imported { $date }

## The Suggested Link column. `$days` is a text number.

desktop-documents-suggest-summary = { $kind } · { $who } · { $amount }
desktop-documents-suggest-summary-bare = { $kind } · { $amount }
desktop-documents-suggest-unreadable = Unreadable — no amount found
desktop-documents-suggest-unreadable-hint = File it by hand, or skip
desktop-documents-suggest-none = No suggestion — nothing within { $days } days
desktop-documents-suggest-none-hint = File it by hand, or skip
desktop-documents-signals-all = amount, date and payee match
desktop-documents-signals-amount-date = amount and date match
desktop-documents-signals-amount-payee = amount and payee match
desktop-documents-signals-date-payee = date and payee match
desktop-documents-signals-amount = amount matches
desktop-documents-signals-payee = payee matches
desktop-documents-signals-date = date matches

## The Inbox detail pane.

desktop-documents-inbox-meta-image = Read from the image · check before accepting
desktop-documents-inbox-meta-file = Read from the file · check before accepting
desktop-documents-inbox-meta-unreadable = Nothing could be read from this file
desktop-documents-fact-merchant = Merchant
desktop-documents-fact-total = Total
desktop-documents-fact-none = —
desktop-documents-suggested-link = Suggested link
desktop-documents-suggested-detail = { $account } · { $documents }
desktop-documents-suggested-no-document = no document yet
desktop-documents-suggested-has-document = already has a document
desktop-documents-other-candidates = Other candidates
desktop-documents-other-none = Other candidates: none within { $days } days

## Accepting, undoing and the confirm. `$title` is the Document's Title, `$count` a text count.

desktop-documents-toast-filed = Filed "{ $title }" · u undo
desktop-documents-toast-filed-many = Filed { $count } · u undo
desktop-documents-toast-undone = { $count ->
    [one] Put { $count } back in the Inbox.
   *[other] Put { $count } back in the Inbox.
}
desktop-documents-status-nothing-to-undo = Nothing to undo.
desktop-documents-status-no-strong = No strong matches to accept.
desktop-documents-accept-all-title = Accept all strong matches
desktop-documents-accept-all-body = { $count ->
    [one] File { $count } document with a strong suggestion?
   *[other] File { $count } documents with strong suggestions?
}
desktop-documents-accept-all-submit = File { $count }
desktop-documents-accept-all-note = Only matches where the amount, date and payee all agree. You can undo it with u.

## The extracted-facts dialog.

desktop-documents-facts-title = Edit extracted facts
desktop-documents-facts-submit = Save
desktop-documents-facts-note = Suggestions are recomputed when you save. Clear the total to mark the file unreadable.
desktop-documents-error-total = Enter an amount such as 212.40.
desktop-documents-toast-facts-saved = Updated what was read from "{ $title }".

## Messages for opening a file. `$name` is the file name, `$path` where it should be.

desktop-documents-status-opened = Opened { $name }.
desktop-documents-status-shown = Showing { $name } in its folder.
desktop-documents-status-missing = { $name } is not at { $path }.
desktop-documents-status-no-document = No document is selected.
desktop-documents-status-not-yet-built = Not yet built.

## The Add document dialog, the Import dialog and the Edit dialog share these field labels.

desktop-documents-add-title = Add document
desktop-documents-add-submit = Add document
desktop-documents-import-title = Import files
desktop-documents-import-submit = Import
desktop-documents-edit-title = Edit document
desktop-documents-edit-submit = Save
desktop-documents-field-path = File path
desktop-documents-field-path-placeholder = ~/Documents/receipt.pdf
desktop-documents-field-paths = File paths, one per line
desktop-documents-field-paths-placeholder = ~/Downloads/scan.pdf
desktop-documents-field-title = Title
desktop-documents-field-type = Document type
desktop-documents-field-date = Document date
desktop-documents-field-key-kind = Key date
desktop-documents-field-key-date = Key date on
desktop-documents-field-reminder = Remind me before it falls due
desktop-documents-key-none = None
desktop-documents-error-path-missing = There is no file at { $path }.
desktop-documents-error-unsupported = Only PDF, JPG and PNG files can be Documents.
desktop-documents-error-already = Already in the Library as { $title }.
desktop-documents-error-no-title = Give the document a title.
desktop-documents-error-no-path = Enter the path to a file.
desktop-documents-import-note = Each file lands in the Inbox, unfiled. Files are referenced where they are, never moved or renamed.
desktop-documents-add-note = The file stays where it is. Add its links afterwards with the link key.

## Toasts for the dialogs. `$title` is the Document's Title, `$imported` and `$skipped` counts shown
## as text.

desktop-documents-toast-added = Added "{ $title }".
desktop-documents-toast-saved = Saved "{ $title }".
desktop-documents-toast-imported = { $count ->
    [one] Imported { $count } file into the Inbox.
   *[other] Imported { $count } files into the Inbox.
}
desktop-documents-toast-import-partial = { $imported } imported, { $skipped } not imported.
desktop-hint-link-elsewhere = link elsewhere
desktop-documents-link-elsewhere = Link elsewhere…

# The link picker (one palette-chrome modal) and following a link.
desktop-documents-picker-kind-all = All
desktop-documents-picker-file-without-link = File without link
desktop-documents-picker-placeholder = Search transactions, accounts, payees, bills, items…
desktop-documents-picker-title-link = Link
desktop-documents-picker-title-file = File
desktop-documents-picker-title-follow = Follow
desktop-documents-picker-empty = Nothing matches.
desktop-documents-picker-type = Type
desktop-documents-picker-type-none = choose with ← →
desktop-documents-picker-hint-pick = ↑↓ move · tab kind · enter pick · esc close
desktop-documents-picker-hint-toggle = ↑↓ move · tab kind · enter link or unlink · esc close
desktop-documents-picker-hint-follow = ↑↓ move · enter go · esc close
desktop-documents-picker-need-type = Choose a document type first (← →).
desktop-documents-toast-linked = Linked { $record }.
desktop-documents-toast-unlinked = Unlinked { $record }.
desktop-documents-link-remove = Remove link
desktop-hint-follow = follow link
desktop-hint-kind = kind
desktop-documents-drop-overlay = Drop to add to the Inbox
