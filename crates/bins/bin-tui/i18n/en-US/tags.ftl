## The Tags view title and main list.

tui-tag-title = Tags
tui-tags-heading = tags { $visible } of { $total }

## The Tags list: tag row status indicators.

tui-tag-row-inactive = · inactive

## The Tags summary box: labels and field values.

tui-tags-summary-active = active
tui-tags-summary-active-value-on = [×] offered when tagging
tui-tags-summary-active-value-off = [ ] not offered
tui-tags-summary-tagged = tagged
tui-tags-summary-tagged-count = { $count ->
    [0] none
    [one] 1 transaction
    *[other] { $count } transactions
}
tui-tags-summary-created = created
tui-tags-summary-updated = updated

## The Tags list: empty state when no tags match the filter.

tui-tags-empty-filter = no tags match the current filter

## The Tagged spend section (right pane).

tui-tag-right-pane-spend-heading = TAGGED SPEND
tui-tag-right-pane-spend-footer-empty = no transactions yet
tui-tag-right-pane-spend-footer = first used { $first } · peak { $peak } { $peak_amount } · { $latest } { $latest_amount }

## The Where it lands section (right pane).

tui-tag-right-pane-lands-heading = WHERE IT LANDS
tui-tag-right-pane-lands-heading-tag = { $count ->
    [one] 1 category
    *[other] { $count } categories
}
tui-tag-right-pane-lands-empty = no categories yet
tui-tag-right-pane-lands-rollup = { $count } more
tui-tag-right-pane-lands-statement = a tag crosses the tree — that is what it is for

## The Transactions section (right pane).

tui-tag-right-pane-txn-heading = TRANSACTIONS
tui-tag-right-pane-txn-heading-tag = { $visible } of { $total } · newest first
tui-tag-right-pane-txn-empty = no transactions yet
tui-tag-right-pane-txn-footer-overlap = { $overlap } of { $total } carry another tag — rows overlap
tui-tag-right-pane-txn-footer-empty = no transactions yet
tui-tag-right-pane-txn-not-yet-built = opening filtered Transactions — not yet built

## Transaction column headers for the right pane.

tui-tag-txn-column-date = DATE
tui-tag-txn-column-payee = PAYEE
tui-tag-txn-column-category = CATEGORY
tui-tag-txn-column-tags = T
tui-tag-txn-column-amount = AMOUNT

## Delete confirm flow (inline in the Tags view heading).

tui-tag-delete-confirm = delete "{ $name }"{ $notice } — y confirms, any other key cancels
tui-tag-delete-notice-transactions = , { $count ->
    [one] 1 transaction will lose this tag
    *[other] { $count } transactions will lose this tag
}

## The New tag popup.

tui-tag-new-title = new tag
tui-tag-new-command = :tag new
tui-tag-new-field-name = name
tui-tag-new-field-active = active
tui-tag-new-active-note = offered when tagging
tui-tag-new-note-unique = name must be globally unique, regardless of case
tui-tag-new-note-clash = a tag named "{ $name }" already exists
tui-tag-new-help-tab = next field
tui-tag-new-help-create = create
tui-tag-new-help-create-next = create & add another
tui-tag-new-help-cancel = cancel

## The Edit tag popup.

tui-tag-edit-title = edit tag
tui-tag-edit-command = :tag edit
tui-tag-edit-field-name = name
tui-tag-edit-field-active = active
tui-tag-edit-active-note = clear to deactivate
tui-tag-edit-note-unique = renaming is always safe — nothing derives from the name
tui-tag-edit-note-clash = a tag named "{ $name }" already exists
tui-tag-edit-help-tab = next field
tui-tag-edit-help-save = save
tui-tag-edit-help-deactivate = deactivate
tui-tag-edit-help-cancel = cancel
