## The Accounts module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. Column and field labels other screens share (Name,
## Institution, Type, Unit, Actions) come from the shared layer.

## The Settings page. `$count` is the Accounts, `$glyph` the Add button's leading plus sign and
## `$key` the key that opens Add.

accounts-count = { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}
accounts-kicker = by type
accounts-add-button = { $glyph } Add account
accounts-empty = No accounts yet. Press { $key } to add one.
accounts-summary-empty = no accounts yet
accounts-net-worth = net worth
accounts-held-separately = { $units } held separately
accounts-row-edit = edit
accounts-row-delete = delete

## The page's legend.

accounts-hint-row = row
accounts-hint-open-ledger = open ledger
accounts-hint-edit = edit
accounts-hint-delete = delete
accounts-hint-new = new

## The `:accounts` command's refusals, for the status line. `$command` is what was typed, `$name`
## the account it named and `$matches` the names it fits.

accounts-status-none = { $command } — no accounts
accounts-status-not-found = { $command } — no account named "{ $name }"
accounts-status-ambiguous = { $command } — "{ $name }" matches { $matches }

## The Add and Edit account Dialogs.

accounts-add-title = Add account
accounts-add-submit = Add account
accounts-edit-title = Edit account
accounts-edit-submit = Save
accounts-field-opening-balance = Opening balance
accounts-field-number = Account number
accounts-name-placeholder = e.g. Everyday Account
accounts-balance-placeholder = 0.00
accounts-number-placeholder = •••• •••• 1234

## The Edit Dialog's usage notice. `$opened` is the month and year the account opened.

accounts-edit-usage-notice = Opened { $opened } · { $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}. Renaming is safe. Unit and opening balance are fixed once an account exists.

## A refusal the Dialog shows under its fields.

accounts-error-gone = This account has since been deleted.
