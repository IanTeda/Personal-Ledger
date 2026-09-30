## The Budgets page (9a–9g): the shared header, tab strip and the Progress tab (9a).

## The header: the method tag beside the title, the primary action, and the tab strip.

desktop-budgets-method-limits = Limits
desktop-budgets-edit-plan = Edit plan
desktop-budgets-tab-progress = Progress
desktop-budgets-tab-plan = Plan
desktop-budgets-tab-history = History
desktop-budgets-legend-spent = spent
desktop-budgets-legend-known = known costs
desktop-budgets-legend-elapsed = elapsed
desktop-budgets-legend-over = over

## The meta line under the title: `$month` (already formatted), then the day of the month for the
## current month only, the over-budget count, the unbudgeted spend and the at-risk count.

desktop-budgets-meta-period = { $month }
desktop-budgets-meta-day = day { $day } of { $days }
desktop-budgets-meta-over = <strong>{ $count }</strong> over budget
desktop-budgets-meta-at-risk = { $count } at risk
desktop-budgets-meta-unbudgeted = { $amount } unbudgeted

## The stat strip: a label, a figure and a line of detail under it.

desktop-budgets-stat-budgeted = Budgeted
desktop-budgets-stat-spent = Spent
desktop-budgets-stat-known = Known costs
desktop-budgets-stat-left = Left to spend
desktop-budgets-stat-budgeted-detail = { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
desktop-budgets-stat-carried = { $amount } carried in
desktop-budgets-stat-spent-detail = { $spent }% of budget · { $elapsed }% of period
desktop-budgets-stat-known-detail = { $count ->
    [one] { $count } unpaid bill due this period
   *[other] { $count } unpaid bills due this period
}
desktop-budgets-stat-known-link = Schedule →
desktop-budgets-stat-left-per-day = ≈ { $amount } / day for { $days ->
    [one] { $days } day
   *[other] { $days } days
}
desktop-budgets-stat-left-over = over budget

## The Progress table.

desktop-budgets-column-budget = Budget
desktop-budgets-column-spent = Spent
desktop-budgets-column-known = Known
desktop-budgets-column-left = Left
desktop-budgets-column-progress = Progress
desktop-budgets-row-rollup = rollup
desktop-budgets-row-carried = incl. { $amount } carried
desktop-budgets-row-unbudgeted = unbudgeted
desktop-budgets-row-over = { $amount } over
desktop-budgets-row-at-risk = at risk
desktop-budgets-row-edit = edit
desktop-budgets-row-set = set
desktop-budgets-progress-empty = Nothing is budgeted or spent this period.

## The Plan tab (9b): the meta line, the range nav, the grid's columns and footers.

desktop-budgets-plan-meta = Monthly amounts per category · past months are read-only · bold = differs from the category default
desktop-budgets-plan-range = { $from } – { $to } { $year }
desktop-budgets-plan-range-years = { $from } { $from_year } – { $to } { $to_year }
desktop-budgets-plan-now = now
desktop-budgets-plan-column-rollover = Rollover
desktop-budgets-plan-rollover-unspent = carry unspent
desktop-budgets-plan-rollover-both = carry both
desktop-budgets-plan-other = Other
desktop-budgets-plan-empty = No category has a budget amount in these months.
desktop-budgets-plan-total = Total budgeted
desktop-budgets-plan-unallocated = Unallocated of { $average } avg. income
desktop-budgets-status-plan = { $range } · { $count ->
    [one] { $count } category
   *[other] { $count } categories
}

## The status-line legend on the Plan tab, in Normal and Insert modes.

desktop-hint-cell = cell
desktop-hint-rollover = rollover
desktop-hint-range = range
desktop-hint-clear = clear
desktop-hint-save-cell = save cell
desktop-hint-next-month = next month
desktop-hint-this-month-only = this month only

## The tabs still to be built.

desktop-budgets-tab-later = This tab is built in a later ticket.

## The status line's right-hand side.

desktop-budgets-status-period = { $month } · { $count ->
    [one] { $count } category
   *[other] { $count } categories
}

## The Category detail dialog (9d): the title, four stats, the bar's caption, the largest
## Transactions, the track record and the footer.

desktop-budgets-detail-title = { $category } — { $month }
desktop-budgets-detail-stat-over = Over
desktop-budgets-detail-stat-left = Left
desktop-budgets-detail-caption = { $spent }% spent · { $elapsed }% elapsed
desktop-budgets-detail-caption-unbudgeted = Unbudgeted · { $elapsed }% elapsed
desktop-budgets-detail-transactions = { $count ->
    [one] { $count } transaction · largest first
   *[other] { $count } transactions · largest first
}
desktop-budgets-detail-transactions-none = No transactions this month
desktop-budgets-detail-more = + { $count } more · { $amount }
desktop-budgets-detail-no-payee = —
desktop-budgets-detail-no-bills = no bill plans in this category
desktop-budgets-detail-bills = { $count ->
    [one] { $count } bill plan in this category
   *[other] { $count } bill plans in this category
}
desktop-budgets-detail-track = Over in { $over } of the last { $months ->
    [one] { $months } month
   *[other] { $months } months
} · average { $average }.
desktop-budgets-detail-track-none = No earlier months to compare.
desktop-budgets-detail-rollover-off = Rollover is off, so this overspend doesn't reduce { $next }.
desktop-budgets-detail-rollover-unspent = Rollover carries unspent only, so this overspend doesn't reduce { $next }.
desktop-budgets-detail-rollover-both = Rollover carries overspend, so this reduces { $next }.
desktop-budgets-detail-open-transactions = open in Transactions →
desktop-budgets-detail-close = Close

## The status-line legend while the Category detail dialog is open.

desktop-hint-close = close
desktop-hint-transactions = transactions
