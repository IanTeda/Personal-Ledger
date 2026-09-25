## The Payees view, three popups (new, edit, delete), and command palette entries.

## The Payees view title and main list.

tui-payees-title = Payees

## The payees list: column headers and key information.

tui-payees-column-payee = PAYEE
tui-payees-column-date = DATE
tui-payees-column-category = CATEGORY
tui-payees-column-amount = AMOUNT

## The payees list: record row labels and status indicators.

tui-payees-record-default = default
tui-payees-record-default-note = default follows the mix
tui-payees-record-matches = matches
tui-payees-record-inactive = inactive
tui-payees-record-status-with-count = { $count ->
    [one] 1 · { $status }
    *[other] { $count } · { $status }
}
tui-payees-record-missing = { $count ->
    [one] 1 · manage
    *[other] { $count } · manage
}

## The payees summary box: labels and field values.

tui-payees-summary-transactions = transactions
tui-payees-summary-categories = categories
tui-payees-summary-category-mix = CATEGORY MIX
tui-payees-summary-matches = matches
tui-payees-summary-default = default
tui-payees-summary-default-follows-mix = default follows the mix · { $percent }% { $amount }
tui-payees-summary-default-disagrees = disagrees with the mix · { $percent }% { $amount }
tui-payees-summary-default-exact = exactly one of the possibilities

## The payees list footer and messaging.

tui-payees-footer-default-set = default set · no transactions yet to check against
tui-payees-footer-message-cleared = any subsequent key clears a showing message

## Error messages for payee operations.

tui-payee-error-not-found = payee not found
tui-payee-error-duplicate-name = a payee with this name already exists
tui-payee-error-no-matches = no payee matches found
tui-payee-error-merge-same = can't merge a payee with itself

## The New payee popup.

tui-payee-new-title = new payee
tui-payee-new-field-name = name
tui-payee-new-field-default = default
tui-payee-new-note-default-usage = offered when posting a transaction from this payee
tui-payee-new-help-tab = next field
tui-payee-new-help-create = create
tui-payee-new-help-cancel = cancel

## The Edit payee popup.

tui-payee-edit-title = edit payee
tui-payee-edit-field-name = name
tui-payee-edit-field-default = default
tui-payee-edit-note-default-usage = offered when posting a transaction from this payee
tui-payee-edit-help-tab = next field
tui-payee-edit-help-save = save
tui-payee-edit-help-delete = delete
tui-payee-edit-help-cancel = cancel

## The Delete payee popup.

tui-payee-delete-title = delete payee
tui-payee-delete-field-confirm = confirm
tui-payee-delete-note-transactions = will be unassigned
tui-payee-delete-note-permanent = deletion is permanent
tui-payee-delete-help-delete = delete
tui-payee-delete-help-cancel = cancel

## The Matches popup: managing payee transaction matches.

tui-payee-matches-title = matches
tui-payee-matches-field-pattern = pattern
tui-payee-matches-field-matches = matches
tui-payee-matches-note-pattern-help = use patterns to auto-assign this payee
tui-payee-matches-help-tab = next field
tui-payee-matches-help-add = add pattern
tui-payee-matches-help-delete = delete
tui-payee-matches-help-cancel = cancel
