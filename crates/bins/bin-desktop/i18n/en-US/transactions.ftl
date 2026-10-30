## The Transactions page: its header, table and footer. Column labels other screens share (Date,
## Account, Payee, Category, Amount, Status) come from the shared layer. The filter chips' and the
## filter popover's Messages are `lib-transactions`'.

## The header. `.button` is the primary button, whose key hint is a separate token.

desktop-transactions-add =
    .button = add transaction

desktop-transactions-count-line = { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
} across { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
}

desktop-transactions-clear-filters = clear filters
desktop-transactions-search-placeholder = / search payee or memo

## The table. `$count` is the number of Splits a multi-Split Transaction has.

desktop-transactions-column-tags = Tags
desktop-transactions-column-running = Running
desktop-transactions-empty = No transactions match these filters.
desktop-transactions-split-summary = split · { $count }

## The footer. `$category` is the chosen Category's name followed by a space, or empty when no
## Category filter is set.

desktop-transactions-footer = { $of ->
    [one] { $shown } of { $of } { $category }transaction shown
   *[other] { $shown } of { $of } { $category }transactions shown
}

desktop-transactions-total-label = Running total
desktop-transactions-total-mixed = mixed units
