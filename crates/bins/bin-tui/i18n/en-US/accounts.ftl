## The Accounts view, three popups (new, edit, delete), and command palette entries.

## The Accounts view title.

tui-accounts-title = Accounts

## The Accounts view column headers.

tui-accounts-column-balance = BALANCE
tui-accounts-column-ledger = LEDGER
tui-accounts-column-date = DATE
tui-accounts-column-payee = PAYEE
tui-accounts-column-amount = AMOUNT

## The New account popup.

tui-account-new-title = new account
tui-account-new-field-name = name
tui-account-new-field-type = type
tui-account-new-field-unit = unit
tui-account-new-field-starting-balance = starting bal
tui-account-new-field-active = active
tui-account-new-note-unit-1 = unit cannot change afterwards — to hold a
tui-account-new-note-unit-2 = second unit, make a second account
tui-account-new-error-no-matches = no matches · tab
tui-account-new-placeholder-completion = completion
tui-account-new-help-tab = next field
tui-account-new-help-create = create
tui-account-new-help-create-and-add = create & add another
tui-account-new-help-cancel = cancel

## The Edit account popup.

tui-account-edit-title = edit account
tui-account-edit-field-name = name
tui-account-edit-field-type = type
tui-account-edit-field-unit = unit
tui-account-edit-field-active = active
tui-account-edit-field-opening-balance = starting bal
tui-account-edit-note-unit-fixed = fixed at creation
tui-account-edit-heading-computed = computed
tui-account-edit-field-balance-now = balance now
tui-account-edit-field-transactions = transactions
tui-account-edit-field-created = created
tui-account-edit-help-tab = next field
tui-account-edit-help-save = save
tui-account-edit-help-deactivate = deactivate
tui-account-edit-help-delete = delete
tui-account-edit-help-cancel = cancel

## The Delete account popup.

tui-account-delete-title = delete
tui-account-delete-field-holds = holds
tui-account-delete-field-transactions = transactions
tui-account-delete-field-move-to = move to
tui-account-delete-field-candidates = candidates
tui-account-delete-heading-after = after
tui-account-delete-field-into = into
tui-account-delete-field-balance = its balance
tui-account-delete-field-moves = moves
tui-account-delete-field-loses = loses
tui-account-delete-field-confirm = confirm
tui-account-delete-help-tab = next field
tui-account-delete-help-delete = delete
tui-account-delete-help-deactivate = deactivate
tui-account-delete-help-cancel = cancel

## Errors when creating, editing or deleting accounts. These become part of a Message
## that includes the error title.

tui-account-error-not-found = account not found
tui-account-error-requires-transfer = holds transactions or Balance Checks — choose a same-unit account to move them into
tui-account-error-transfer-unit-mismatch = the transfer target must share the account's unit
tui-account-error-transfer-is-source = can't transfer an account's transactions to itself
tui-account-error-transfer-inactive = the transfer target must be active

## The command palette: Accounts domain commands and their help text.

tui-command-account = accounts grouped by type, per-unit subtotals, ledger
tui-command-account-new = add an account — opens the new popup, blank
tui-command-account-new-placeholder = \<name\>
tui-command-account-new-preview = e.g. Everyday Spending, Mortgage Offset

tui-command-account-edit = edit the highlighted account
tui-command-account-edit-placeholder = \<acct\>
tui-command-account-edit-preview = Everyday Spending · bank · AUD 4 210.65

tui-command-account-delete = delete — a non-empty account needs a same-unit transfer target
tui-command-account-delete-placeholder = \<acct\>
tui-command-account-delete-preview = Everyday Spending · 1 284 txns — needs [into \<acct\>]

tui-command-account-off = is_active = 0 — hides it from the list unless za
tui-command-account-off-placeholder = \<acct\>
tui-command-account-off-preview = the list selection

tui-command-account-on = is_active = 1 — reverses account off
tui-command-account-on-placeholder = \<acct\>
tui-command-account-on-preview = the list selection, with za held to see it

tui-command-account-check = records a Balance Check, prints the variance
tui-command-account-check-placeholder = \<amount\>
tui-command-account-check-preview = not yet designed — docs/ux/tui/accounts/README.md
