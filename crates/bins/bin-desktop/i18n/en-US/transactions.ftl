## The Transactions page: its header, filter chips, table, footer and the filter popover. Column and
## field labels other screens share (Date, Account, Payee, Category, Amount, Status, From) come
## from the shared layer, and a Status word comes from the shared vocabulary.

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

## The filter chips. `$value` is the chosen account, category, payee, tag or status.

desktop-transactions-chip-account = account: { $value }
desktop-transactions-chip-category = category: { $value }
desktop-transactions-chip-payee = payee: { $value }
desktop-transactions-chip-tag = tag: { $value }
desktop-transactions-chip-status = status: { $value }
desktop-transactions-chip-all-accounts = all accounts
desktop-transactions-chip-any = any
desktop-transactions-chip-unknown = unknown

## The date chip. `$date`, `$from` and `$to` are formatted dates, and the end of a range may be the
## word for today.

desktop-transactions-date-this-year = this year
desktop-transactions-date-all = all dates
desktop-transactions-date-from = from { $date }
desktop-transactions-date-until = until { $date }
desktop-transactions-date-range = { $from } – { $to }

## The Status filter's first option, before the Transaction Statuses.

desktop-transactions-status-all = All

## The table. `$count` is the number of Splits a multi-Split Transaction has.

desktop-transactions-column-tags = Tags
desktop-transactions-column-running = Running
desktop-transactions-empty = No transactions match these filters.
desktop-transactions-split-summary = split · { $count }

## The footer. `$category` is the chosen Category's name followed by a space, or empty when no
## Category filter is set.

desktop-transactions-footer = { $shown } of { $of } { $category }{ $of ->
    [one] transaction
   *[other] transactions
} shown

desktop-transactions-total-label = Running total
desktop-transactions-total-mixed = mixed units

## The filter popover.

desktop-transactions-filter =
    .title = Filter transactions

desktop-transactions-filter-all-accounts = All accounts
desktop-transactions-filter-any-category = Any category
desktop-transactions-filter-any-payee = any payee
desktop-transactions-filter-any-tag = any tag
desktop-transactions-filter-no-start = no start
desktop-transactions-filter-no-end = no end
desktop-transactions-field-tag = Tag
desktop-transactions-field-to = To
