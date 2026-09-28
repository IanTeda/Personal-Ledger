## The Payees page. Column labels other screens share (Name, Actions) come from the shared layer.

## The active payee count, used by the page summary and the status line.

desktop-payees-count = { $count ->
    [one] { $count } payee
   *[other] { $count } payees
}

## The page summary's second figure, the count in bold, and the status line's plain version.

desktop-payees-without-default = <strong>{ $count }</strong> without a default category
desktop-payees-status-without-default = { $count } without default category

## The page. `$glyph` is the Add button's leading plus sign, `$key` the key token that opens Add.

desktop-payees-add-button = { $glyph } Add payee
desktop-payees-empty = No payees yet. Press { $key } to add one.
desktop-payees-column-default-category = Default category
desktop-payees-column-match-rules = Match rules
desktop-payees-column-transactions = Transactions
desktop-payees-column-total = Total
desktop-payees-no-default-category = no default category
desktop-payees-inactive = inactive
desktop-payees-rule-count = { $count ->
    [one] { $count } rule
   *[other] { $count } rules
}
desktop-payees-row-edit = edit
desktop-payees-row-delete = delete
desktop-payees-footnote = Payees without a default category need one picked each time. Match rules drive renaming on import.

## The hint strip and the flashes for dialogs still to come.

desktop-hint-view-transactions = view transactions
desktop-status-add-payee-not-yet-built = add payee — not yet built
desktop-status-edit-payee-not-yet-built = edit payee — not yet built
desktop-status-delete-payee-not-yet-built = delete payee — not yet built
