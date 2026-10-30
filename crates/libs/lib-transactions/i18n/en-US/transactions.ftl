## The Transactions module's Messages, read by its Presenters. Each Client composes this layer into
## its Locale; no Client catalogue keeps a copy. Column and field labels other screens share
## (Account, Category, Payee, From, Status) come from the shared layer, and a Status word comes
## from the shared vocabulary.

## The filter chips. `$value` is the chosen account, category, payee, tag or status.

transactions-chip-account = account: { $value }
transactions-chip-category = category: { $value }
transactions-chip-payee = payee: { $value }
transactions-chip-tag = tag: { $value }
transactions-chip-status = status: { $value }
transactions-chip-all-accounts = all accounts
transactions-chip-any = any
transactions-chip-unknown = unknown

## The date chip. `$date`, `$from` and `$to` are formatted dates, and the end of a range may be the
## word for today.

transactions-date-this-year = this year
transactions-date-all = all dates
transactions-date-from = from { $date }
transactions-date-until = until { $date }
transactions-date-range = { $from } – { $to }

## The Status filter's first option, before the Transaction Statuses.

transactions-status-all = All

## The filter popover: its title, the selects' first options, the text fields' labels and
## placeholders, and its two buttons.

transactions-filter-title = Filter transactions
transactions-filter-all-accounts = All accounts
transactions-filter-any-category = Any category
transactions-filter-any-payee = any payee
transactions-filter-any-tag = any tag
transactions-filter-no-start = no start
transactions-filter-no-end = no end
transactions-filter-field-tag = Tag
transactions-filter-field-to = To
transactions-filter-apply = apply
transactions-filter-reset = reset
