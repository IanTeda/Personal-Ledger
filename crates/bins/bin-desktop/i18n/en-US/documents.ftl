## The Documents surface (`docs/ux/desktop/Documents/`): the Library, its index rail and detail pane,
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

desktop-documents-type-receipts = Receipts
desktop-documents-type-statements = Statements
desktop-documents-type-tax = Tax
desktop-documents-type-insurance = Insurance
desktop-documents-type-warranties = Warranties & manuals
desktop-documents-type-contracts = Contracts
desktop-documents-type-identity = Identity
desktop-documents-type-bills = Bills
desktop-documents-type-one-receipt = Receipt
desktop-documents-type-one-statement = Statement
desktop-documents-type-one-tax = Tax
desktop-documents-type-one-insurance = Insurance
desktop-documents-type-one-warranty = Warranty or manual
desktop-documents-type-one-contract = Contract
desktop-documents-type-one-identity = Identity
desktop-documents-type-one-bill = Bill

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

## The Inbox, until its own view is built.

desktop-documents-inbox-title = Inbox
desktop-documents-inbox-subline = { $count ->
    [one] { $count } unfiled
   *[other] { $count } unfiled
} · dropped, scanned or imported · each gets a suggested link
desktop-documents-inbox-placeholder = The Inbox list is not built yet.

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
