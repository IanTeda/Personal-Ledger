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

## The Edit budget dialog (9e): the title on a budgeted Category or on the picker, the fields, the
## before/after summary, the note and the footer. `$month` is already formatted.

desktop-budgets-plan-add = + Budget a category
desktop-budgets-detail-edit = Edit budget
desktop-budgets-limit-title = Edit budget — { $category }
desktop-budgets-limit-title-new = Budget a category
desktop-budgets-limit-field-category = Category
desktop-budgets-limit-field-amount = Amount per month
desktop-budgets-limit-field-starting = Starting
desktop-budgets-limit-field-span = Applies to
desktop-budgets-limit-field-rollover = Rollover
desktop-budgets-limit-month-current = { $month } (current)
desktop-budgets-limit-span-month-only = That month only
desktop-budgets-limit-span-onward = That month onward
desktop-budgets-limit-rollover-none = None
desktop-budgets-limit-rollover-unspent = Carry unspent
desktop-budgets-limit-rollover-both = Carry unspent & overspend
desktop-budgets-limit-error-amount = Enter an amount of 0.00 or more.
desktop-budgets-limit-error-archived = An archived budget is read-only.
desktop-budgets-limit-error-refused = That change couldn't be saved.
desktop-budgets-limit-no-categories = Every expense category already has a budget amount this month.
desktop-budgets-limit-preview-unchanged = unchanged
desktop-budgets-limit-preview-onward = { $month } → onward
desktop-budgets-limit-preview-month-only = { $month } only
desktop-budgets-limit-preview-total = Total budgeted, { $month }
desktop-budgets-limit-preview-change = { $before } → { $after }
desktop-budgets-limit-note = This is the same figure as Monthly budget on the category, so editing either updates both. Past months keep their amount.
desktop-budgets-limit-note-other = Past months keep their amount.
desktop-budgets-limit-stop = Stop budgeting…
desktop-budgets-limit-submit = Save budget

## The Stop budgeting dialog (9g). `$plans` is the Category's Bill Plan names, already joined.

desktop-budgets-stop-title = Stop budgeting { $category }
desktop-budgets-stop-field-from = From
desktop-budgets-stop-body = { $category } keeps its transactions. From that month its spending appears on Progress as unbudgeted. Past months keep their budget in History. Nothing is deleted.
desktop-budgets-stop-body-bills = Its bill plans ({ $plans }) still count as known costs.
desktop-budgets-stop-submit = Stop budgeting

## The status-line legend for Edit budget, Budget a category and Stop budgeting.

desktop-hint-edit-budget = edit budget
desktop-hint-budget-category = budget a category
desktop-hint-stop-budgeting = stop budgeting
desktop-hint-save = save

## The Fill dialog (9f): the Plan tab's button, the two sources with the total each gives, the
## diff against the plan and the footer. `$month` is already formatted.

desktop-budgets-fill-button = Fill { $month } from…
desktop-budgets-fill-title = Fill { $month } from…
desktop-budgets-fill-source-previous = { $month } as budgeted
desktop-budgets-fill-source-previous-detail = an exact copy, without carries
desktop-budgets-fill-source-average = 3-month average spent
desktop-budgets-fill-source-average-detail = { $from } – { $to }, rounded to the nearest 10
desktop-budgets-fill-changes = Changes from the plan
desktop-budgets-fill-no-changes = Nothing would change.
desktop-budgets-fill-change = { $before } → { $after }
desktop-budgets-fill-kept = kept (edited)
desktop-budgets-fill-unchanged = { $count ->
    [one] { $count } other category unchanged
   *[other] { $count } other categories unchanged
}
desktop-budgets-fill-note = Fill writes this month only, for categories budgeted the month before, and never overwrites a cell that already has its own amount.
desktop-budgets-fill-submit = Fill { $month }
desktop-hint-fill = fill
desktop-hint-source = source
