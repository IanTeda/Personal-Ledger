## The Budgets Dialogs still on the legacy host (#955). The page, the Switcher and Manage budgets
## read their text from `lib-budgets`' own Catalogue.

## The stat labels the Category detail (9d) shares with the page, and the History export's Toasts.
## `$path` is the file written.

desktop-budgets-stat-budgeted = Budgeted
desktop-budgets-stat-spent = Spent
desktop-budgets-stat-known = Known costs
desktop-budgets-history-pair = { $spent } / { $budget }
desktop-budgets-history-exported = Exported the budget history to { $path }
desktop-budgets-history-export-failed = Couldn't export the budget history: { $error }

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
desktop-hint-save = save

## The Fill dialog (9f): the Plan tab's button, the two sources with the total each gives, the
## diff against the plan and the footer. `$month` is already formatted.

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

## The New budget dialog (11c) and its edit mode: the fields, the four method cards (only
## Category limits can be chosen), the Account chips, Start from and the footer. `$budget` is the
## Budget on show.

desktop-budgets-new-title = New budget
desktop-budgets-new-title-edit = Edit budget — { $budget }
desktop-budgets-new-field-name = Name
desktop-budgets-new-name-placeholder = Holiday
desktop-budgets-new-field-method = Method
desktop-budgets-new-field-accounts = Accounts on budget
desktop-budgets-new-field-start = Start from
desktop-budgets-new-method-envelope = Envelope
desktop-budgets-new-method-envelope-tag = Zero-based
desktop-budgets-new-method-envelope-detail = Every dollar received is assigned to a category.
desktop-budgets-new-method-limits = Category limits
desktop-budgets-new-method-limits-tag = Caps
desktop-budgets-new-method-limits-detail = A monthly cap per category, measured against spending.
desktop-budgets-new-method-split = Percentage split
desktop-budgets-new-method-split-tag = 50/30/20
desktop-budgets-new-method-split-detail = Buckets measured against the income received.
desktop-budgets-new-method-project = Project
desktop-budgets-new-method-project-tag = One-off
desktop-budgets-new-method-project-detail = One total over a date range, split into lines.
desktop-budgets-new-method-later = coming later
desktop-budgets-new-accounts-help = Spending on these accounts counts in this budget. Accounts left off still count toward net worth.
desktop-budgets-new-accounts-none = No account takes transactions in this unit.
desktop-budgets-new-start-empty = Empty
desktop-budgets-new-start-copy = Copy categories from { $budget }
desktop-budgets-new-start-average = Last 3 months' spending
desktop-budgets-new-start-help = Each writes its amounts onward from this month. A category with no spending stays unbudgeted.
desktop-budgets-new-error-name = Another budget already has that name.
desktop-budgets-new-submit = Create budget
desktop-budgets-new-submit-edit = Save
