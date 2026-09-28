## The Import "match payees" step (the Payees handoff's 6e). `$file` is the statement's file name,
## `$glyph` the create option's leading plus sign.

## The title-bar context beside the Transactions label.

desktop-import-context = import { $file }

## The stepper, heading and subline.

desktop-import-step-upload = 1 upload
desktop-import-step-match = 2 match payees
desktop-import-step-confirm = 3 confirm
desktop-import-title = Match payees
desktop-import-subline = { $count ->
    [one] { $count } row from { $file } — review how each description maps to a payee and category before importing
   *[other] { $count } rows from { $file } — review how each description maps to a payee and category before importing
}

## The table.

desktop-import-column-raw = Raw description
desktop-import-column-payee = Payee
desktop-import-column-category = Category
desktop-import-column-status = Status
desktop-import-column-amount = Amount
desktop-import-create-payee = { $glyph } create "{ $name }"
desktop-import-choose-payee = choose payee…
desktop-import-choose-category = choose category…
desktop-import-status-rule-match = rule match
desktop-import-status-new-payee = new payee
desktop-import-status-matched = matched
desktop-import-status-needs-review = needs review

## The footer bar.

desktop-import-summary-rows = { $count ->
    [one] { $count } row
   *[other] { $count } rows
}
desktop-import-summary-matched = { $count } matched by rule
desktop-import-summary-new = { $count ->
    [one] { $count } new payee to create
   *[other] { $count } new payees to create
}
desktop-import-summary-review = { $count ->
    [one] { $count } needs review
   *[other] { $count } need review
}
desktop-import-remember = remember new payees' rules for next time
desktop-import-back = back
desktop-import-continue = continue

## The status line: the legend's labels and the right-hand text.

desktop-hint-payee = payee
desktop-hint-category = category
desktop-hint-create-new-payee = create new payee
desktop-hint-remember-rules = remember rules
desktop-hint-continue = continue
desktop-hint-back = back
desktop-import-status-pending = { $count ->
    [one] { $count } row needs review before continuing
   *[other] { $count } rows need review before continuing
}
desktop-import-status-ready = ready to import

## The Toast **continue** raises.

desktop-import-done = { $count ->
    [one] Imported { $count } transaction
   *[other] Imported { $count } transactions
}
