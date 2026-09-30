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

## The tabs still to be built.

desktop-budgets-tab-later = This tab is built in a later ticket.

## The status line's right-hand side.

desktop-budgets-status-period = { $month } · { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
