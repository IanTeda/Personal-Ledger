## The Documents Dialogs not yet on `lib-documents`' Presenters (#968): Add, Edit, Import, Edit
## facts, Accept all and the link picker. The page's own Messages are `lib-documents`'.

## The Accept all confirm. `$count` is a text count.

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
desktop-hint-kind = kind
desktop-documents-drop-overlay = Drop to add to the Inbox
