## The Accounts page and its Add, Edit and Delete dialogs. Column and field labels that other
## screens share (Name, Institution, Type, Unit, Balance, Actions) come from the shared layer.

## The account count, used by the page summary and the status line.

desktop-accounts-count = { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}

## The page. `$key` is the key token that opens the Add dialog.

desktop-accounts-empty = No accounts yet. Press { $key } to add one.
desktop-accounts-summary-empty = no accounts yet
desktop-accounts-net-worth = net worth
desktop-accounts-held-separately = { $units } held separately
desktop-accounts-row-edit = edit
desktop-accounts-row-delete = delete

## The Add dialog. `.button` is the page button, whose leading `$glyph` is a plus sign.

desktop-accounts-add =
    .button = { $glyph } Add account
    .title = Add account
    .submit = Add account

desktop-accounts-edit =
    .title = Edit account
    .submit = Save

desktop-accounts-field-opening-balance = Opening balance
desktop-accounts-field-number = Account number
desktop-accounts-name-placeholder = e.g. Everyday Account

## The Edit dialog's usage notice. `$opened` is the month and year the account opened.

desktop-accounts-edit-usage-notice = Opened { $opened } · { $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}. Renaming is safe. Unit and opening balance are fixed once an account exists.

## The Delete dialog. `$balance` is the balance with its Unit and `$name` the account's name. The
## `<strong>` spans are the emphasised figures.

desktop-accounts-delete =
    .title = Delete account — { $name }
    .submit = Delete account

desktop-accounts-delete-warning = This account has a balance of <strong>{ $balance }</strong> and <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}.</strong> Deleting it cannot be undone.

desktop-accounts-delete-reference = { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
} will be permanently deleted · { $budgets ->
    [0] not referenced by any budget
    [one] referenced in { $budgets } budget
   *[other] referenced in { $budgets } budgets
}

desktop-accounts-delete-confirm-label = Type { $name } to confirm
