## The Budgets page (9a–9c): the shared header, tab strip and the Progress tab (9a).

## The header: the method tag beside the title, the primary action, and the tab strip.

budgets-method-limits = Limits
budgets-edit-plan = Edit plan
budgets-plan-add = + Budget a category
budgets-history-export = Export CSV
budgets-fill-button = Fill { $month } from…
budgets-tab-progress = Progress
budgets-tab-plan = Plan
budgets-tab-history = History
budgets-legend-spent = spent
budgets-legend-known = known costs
budgets-legend-elapsed = elapsed
budgets-legend-over = over

## The meta line under the title: `$month` (already formatted), then the day of the month for the
## current month only, the over-budget count, the unbudgeted spend and the at-risk count.

budgets-meta-period = { $month }
budgets-meta-day = day { $day } of { $days }
budgets-meta-over = <strong>{ $count }</strong> over budget
budgets-meta-at-risk = { $count } at risk
budgets-meta-unbudgeted = { $amount } unbudgeted

## The stat strip: a label, a figure and a line of detail under it.

budgets-stat-budgeted = Budgeted
budgets-stat-spent = Spent
budgets-stat-known = Known costs
budgets-stat-left = Left to spend
budgets-stat-budgeted-detail = { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
budgets-stat-carried = { $amount } carried in
budgets-stat-spent-detail = { $spent }% of budget · { $elapsed }% of period
budgets-stat-known-detail = { $count ->
    [one] { $count } unpaid bill due this period
   *[other] { $count } unpaid bills due this period
}
budgets-stat-known-link = Schedule →
budgets-stat-left-per-day = ≈ { $amount } / day for { $days ->
    [one] { $days } day
   *[other] { $days } days
}
budgets-stat-left-over = over budget

## The Progress table, whose Budget and Spent headings also head the History export's columns.

budgets-column-budget = Budget
budgets-column-spent = Spent
budgets-column-known = Known
budgets-column-left = Left
budgets-column-progress = Progress
budgets-row-rollup = rollup
budgets-row-carried = incl. { $amount } carried
budgets-row-unbudgeted = unbudgeted
budgets-row-over = { $amount } over
budgets-row-at-risk = { $amount } at risk
budgets-row-parent = ▾ { $name }
budgets-row-edit = edit
budgets-row-set = set
budgets-progress-empty = Nothing is budgeted or spent this period.

## The Plan tab (9b): the meta line, the range nav, the grid's columns and footers.

budgets-plan-meta = Monthly amounts per category · past months are read-only · bold = amount set this month
budgets-plan-range = { $from } – { $to } { $year }
budgets-plan-range-years = { $from } { $from_year } – { $to } { $to_year }
budgets-plan-month-now = { $month } · now
budgets-plan-column-rollover = Rollover
budgets-plan-rollover-unspent = carry unspent
budgets-plan-rollover-both = carry both
budgets-plan-other = Other
budgets-plan-empty = No category has a budget amount in these months.
budgets-plan-total = Total budgeted
budgets-plan-unallocated = Unallocated of { $average } avg. income

## The History tab (9c): the meta line, the chart's pairs and caption, and the table's AVG and
## OVER columns.

budgets-history-meta = Budget vs spent by month · the average and OVER count use closed months only
budgets-history-empty = No category has a budget amount yet.
budgets-history-pair = { $spent } / { $budget }
budgets-history-month-to-date = { $month } · to date
budgets-history-month-over = { $month } · over
budgets-history-cell-over = ▲ { $amount }
budgets-history-caption = { $months ->
    [one] over in { $over } of the last { $months } closed month
   *[other] over in { $over } of the last { $months } closed months
}
budgets-history-caption-none = no closed months in this range
budgets-history-column-avg = Avg
budgets-history-column-over = Over
budgets-history-over-count = { $over }/{ $months }

## The status line's right-hand side, per tab.

budgets-status-period = { $month } · { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
budgets-status-plan = { $range } · { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
budgets-status-history = { $count ->
    [one] { $shown } of { $count } category shown
   *[other] { $shown } of { $count } categories shown
}

## The status-line legends: each tab's, the Plan cell editor's, the Switcher's and Manage's.

budgets-hint-row = row
budgets-hint-open = open
budgets-hint-edit-budget = edit budget
budgets-hint-budget-category = budget a category
budgets-hint-stop-budgeting = stop budgeting
budgets-hint-switch-budget = switch budget
budgets-hint-period = period
budgets-hint-switch-view = switch view
budgets-hint-cell = cell
budgets-hint-edit = edit
budgets-hint-rollover = rollover
budgets-hint-clear = clear
budgets-hint-fill = fill
budgets-hint-range = range
budgets-hint-save-cell = save cell
budgets-hint-cancel = cancel
budgets-hint-next-month = next month
budgets-hint-this-month-only = this month only
budgets-hint-month = month
budgets-hint-export = export
budgets-hint-new-budget = new budget
budgets-hint-close = close
budgets-hint-duplicate = duplicate
budgets-hint-set-default = set default
budgets-hint-archive = archive

## The Switcher popover (11b): the search field, a row's default marker and health line, the
## archived section and the footer. `$date` is already formatted.

budgets-switcher-search = find a budget
budgets-switcher-default = default
budgets-switcher-health = { $over } over · { $left } left to spend
budgets-switcher-health-at-risk = { $over } over · { $left } left to spend · { $risk } at risk
budgets-switcher-archived = Archived
budgets-switcher-archived-on = archived { $date }
budgets-switcher-none = No budget matches.
budgets-switcher-new = + New budget
budgets-switcher-manage = Manage budgets…

## Manage budgets (11f): the header counts, the table, its row actions, a refused action and the
## three notes. `$budget` is the default Budget's name.

budgets-manage-title = Manage budgets
budgets-manage-counts = { $active } active · { $archived } archived · opens to <strong>{ $budget }</strong>
budgets-manage-column-budget = Budget
budgets-manage-column-method = Method
budgets-manage-column-period = Period
budgets-manage-column-accounts = Accounts
budgets-manage-column-scope = Scope
budgets-manage-column-actions = Actions
budgets-manage-period-monthly = Monthly
budgets-manage-accounts = { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}
budgets-manage-scope = { $count ->
    [one] { $count } category
   *[other] { $count } categories
}
budgets-manage-action-open = open
budgets-manage-action-edit = edit
budgets-manage-action-duplicate = duplicate
budgets-manage-action-default = set default
budgets-manage-action-archive = archive
budgets-manage-action-restore = restore
budgets-manage-error-default = The default budget can't be archived. Set another default first.
budgets-manage-error-archived = An archived budget is read-only.
budgets-manage-error-refused = That change couldn't be saved.
budgets-manage-note-views-title = Budgets are views
budgets-manage-note-views = A budget never owns or copies transactions. A transaction counts in every budget whose accounts and categories cover it.
budgets-manage-note-switching-title = Switching
budgets-manage-note-switching = Switching keeps the tab you are on, and the last budget opened is remembered.
budgets-manage-note-archive-title = Archive, not delete
budgets-manage-note-archive = Archiving affects only the budget, never the ledger. Its history is kept and it can be restored.

## The command palette's descriptions of the Budgets commands the keymap lists.

budgets-command-switch = switch to another budget
budgets-command-new = create a budget
budgets-command-edit = rename this budget or change its accounts
budgets-command-manage = every budget, with duplicate, default and archive
budgets-command-duplicate = copy this budget with its amounts
budgets-command-set-default = make this the budget the dashboard reads
budgets-command-archive = archive this budget, keeping its history
budgets-command-restore = restore this archived budget
budgets-command-next-tab = show the next tab
budgets-command-progress = show the Progress tab
budgets-command-plan = show the Plan tab
budgets-command-history = show the History tab
budgets-command-previous-period = step the month or range back
budgets-command-next-period = step the month or range forward
budgets-command-budget-category = budget a category
budgets-command-fill = fill a month from the one before or the average
budgets-command-export = export the History table as CSV
