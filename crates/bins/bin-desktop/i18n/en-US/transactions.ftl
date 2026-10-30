## The Transactions page: its header, table and footer. Column labels other screens share (Date,
## Account, Payee, Category, Amount, Status) come from the shared layer. The text the page's
## Presenter writes (the count line, the footer's count, the Split summary, the legend and the
## filter chips and popover) is `lib-transactions`'.

## The header. `.button` is the primary button, whose key hint is a separate token.

desktop-transactions-add =
    .button = add transaction

desktop-transactions-clear-filters = clear filters
desktop-transactions-search-placeholder = / search payee or memo

## The table.

desktop-transactions-column-tags = Tags
desktop-transactions-column-running = Running
desktop-transactions-empty = No transactions match these filters.

## The footer.

desktop-transactions-total-label = Running total
desktop-transactions-total-mixed = mixed units
