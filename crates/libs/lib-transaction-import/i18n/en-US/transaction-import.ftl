## The Import "match payees" step (the Payees handoff's 6e), the Transaction Import module's one
## View. A Client composes this layer into its Locale; no Client catalogue keeps a copy. `$file` is
## the statement's file name, `$glyph` the create option's leading plus sign, `$name` a Payee's
## name and `$alias` a Payee Alias.

## The stepper, heading and subline.

transaction-import-step-upload = 1 upload
transaction-import-step-match = 2 match payees
transaction-import-step-confirm = 3 confirm
transaction-import-title = Match payees
transaction-import-subline = { $count ->
    [one] { $count } row from { $file } — review how each description maps to a payee and category before importing
   *[other] { $count } rows from { $file } — review how each description maps to a payee and category before importing
}

## The table.

transaction-import-column-raw = Raw description
transaction-import-column-payee = Payee
transaction-import-column-category = Category
transaction-import-column-status = Status
transaction-import-column-amount = Amount
transaction-import-create-payee = { $glyph } create "{ $name }"
transaction-import-choose-payee = choose payee…
transaction-import-choose-category = choose category…
transaction-import-status-rule-match = rule match
transaction-import-status-new-payee = new payee
transaction-import-status-matched = matched
transaction-import-status-needs-review = needs review

## The footer bar.

transaction-import-summary-rows = { $count ->
    [one] { $count } row
   *[other] { $count } rows
}
transaction-import-summary-matched = { $count } matched by rule
transaction-import-summary-new = { $count ->
    [one] { $count } new payee to create
   *[other] { $count } new payees to create
}
transaction-import-summary-review = { $count ->
    [one] { $count } to review
   *[other] { $count } to review
}
transaction-import-remember = remember new payees' rules for next time
transaction-import-back = back
transaction-import-continue = continue

## The status line: the legend's labels and the right-hand text.

transaction-import-hint-payee = payee
transaction-import-hint-category = category
transaction-import-hint-create-new-payee = create new payee
transaction-import-hint-remember-rules = remember rules
transaction-import-hint-continue = continue
transaction-import-hint-back = back
transaction-import-hint-choose = choose
transaction-import-hint-select = select
transaction-import-hint-close = close
transaction-import-status-pending = { $count ->
    [one] Review { $count } row before continuing
   *[other] Review { $count } rows before continuing
}
transaction-import-status-ready = ready to import

## The Toasts **continue** raises: the outcome, and why it was refused.

transaction-import-done = { $count ->
    [one] Imported { $count } transaction
   *[other] Imported { $count } transactions
}
transaction-import-error-account-gone = This statement's account no longer exists, so nothing was imported.
transaction-import-error-account-inactive = This statement's account is inactive, so nothing was imported.
transaction-import-error-name-taken = A payee named "{ $name }" already exists, so nothing was imported.
transaction-import-error-alias-taken = The alias "{ $alias }" already belongs to a payee, so nothing was imported.
