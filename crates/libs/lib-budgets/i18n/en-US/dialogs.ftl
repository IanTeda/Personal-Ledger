## The Budgets Dialogs (#955): the Category detail (9d), Edit budget (9e), Fill (9f), Stop budgeting (9g) and the
## New budget form (11c), read by their Presenters.

## The Category detail dialog (9d): the title, four stats, the bar's caption, the largest
## Transactions, the track record and the footer.

budgets-detail-title = { $category } — { $month }
budgets-detail-stat-over = Over
budgets-detail-stat-left = Left
budgets-detail-caption = { $spent }% spent · { $elapsed }% elapsed
budgets-detail-caption-unbudgeted = Unbudgeted · { $elapsed }% elapsed
budgets-detail-transactions = { $count ->
    [one] { $count } transaction · largest first
   *[other] { $count } transactions · largest first
}
budgets-detail-transactions-none = No transactions this month
budgets-detail-more = + { $count } more · { $amount }
budgets-detail-no-payee = —
budgets-detail-no-bills = no bill plans in this category
budgets-detail-bills = { $count ->
    [one] { $count } bill plan in this category
   *[other] { $count } bill plans in this category
}
budgets-detail-track = Over in { $over } of the last { $months ->
    [one] { $months } month
   *[other] { $months } months
} · average { $average }.
budgets-detail-track-none = No earlier months to compare.
budgets-detail-rollover-off = Rollover is off, so this overspend doesn't reduce { $next }.
budgets-detail-rollover-unspent = Rollover carries unspent only, so this overspend doesn't reduce { $next }.
budgets-detail-rollover-both = Rollover carries overspend, so this reduces { $next }.
budgets-detail-open-transactions = open in Transactions →
budgets-detail-close = Close

## The status-line legend while the Category detail dialog is open.


## The Edit budget dialog (9e): the title on a budgeted Category or on the picker, the fields, the
## before/after summary, the note and the footer. `$month` is already formatted.

budgets-detail-edit = Edit budget
budgets-limit-title = Edit budget — { $category }
budgets-limit-title-new = Budget a category
budgets-limit-field-category = Category
budgets-limit-field-amount = Amount per month
budgets-limit-field-starting = Starting
budgets-limit-field-span = Applies to
budgets-limit-field-rollover = Rollover
budgets-limit-month-current = { $month } (current)
budgets-limit-span-month-only = That month only
budgets-limit-span-onward = That month onward
budgets-limit-rollover-none = None
budgets-limit-rollover-unspent = Carry unspent
budgets-limit-rollover-both = Carry unspent & overspend
budgets-limit-error-amount = Enter an amount of 0.00 or more.
budgets-limit-error-archived = An archived budget is read-only.
budgets-limit-error-refused = That change couldn't be saved.
budgets-limit-error-closed-month = A closed month is read-only.
budgets-limit-error-not-leaf = Only an expense category with no sub-categories can have a budget amount.
budgets-limit-error-not-budgeted = That category has no budget amount this month.
budgets-limit-no-categories = Every expense category already has a budget amount this month.
budgets-limit-preview-unchanged = { $amount } unchanged
budgets-limit-preview-onward = { $month } → onward
budgets-limit-preview-month-only = { $month } only
budgets-limit-preview-total = Total budgeted, { $month }
budgets-limit-preview-change = { $before } → { $after }
budgets-limit-note = This is the same figure as Monthly budget on the category, so editing either updates both. Past months keep their amount.
budgets-limit-note-other = Past months keep their amount.
budgets-limit-stop = Stop budgeting…
budgets-limit-submit = Save budget

## The Stop budgeting dialog (9g). `$plans` is the Category's Bill Plan names, already joined.

budgets-stop-title = Stop budgeting { $category }
budgets-stop-field-from = From
budgets-stop-body = { $category } keeps its transactions. From that month its spending appears on Progress as unbudgeted. Past months keep their budget in History. Nothing is deleted.
budgets-stop-body-bills = Its bill plans ({ $plans }) still count as known costs.
budgets-stop-submit = Stop budgeting

## The status-line legend for Edit budget, Budget a category and Stop budgeting.


## The Fill dialog (9f): the Plan tab's button, the two sources with the total each gives, the
## diff against the plan and the footer. `$month` is already formatted.

budgets-fill-title = Fill { $month } from…
budgets-fill-source-previous = { $month } as budgeted
budgets-fill-source-previous-detail = an exact copy, without carries
budgets-fill-source-average = 3-month average spent
budgets-fill-source-average-detail = { $from } – { $to }, rounded to the nearest 10
budgets-fill-changes = Changes from the plan
budgets-fill-no-changes = Nothing would change.
budgets-fill-change = { $before } → { $after }
budgets-fill-kept = kept (edited)
budgets-fill-unchanged = { $count ->
    [one] { $count } other category unchanged
   *[other] { $count } other categories unchanged
}
budgets-fill-note = Fill writes this month only, for categories budgeted the month before, and never overwrites a cell that already has its own amount.
budgets-fill-submit = Fill { $month }

## The New budget dialog (11c) and its edit mode: the fields, the four method cards (only
## Category limits can be chosen), the Account chips, Start from and the footer. `$budget` is the
## Budget on show.

budgets-new-title = New budget
budgets-new-title-edit = Edit budget — { $budget }
budgets-new-field-name = Name
budgets-new-name-placeholder = Holiday
budgets-new-field-method = Method
budgets-new-field-accounts = Accounts on budget
budgets-new-field-start = Start from
budgets-new-method-envelope = Envelope
budgets-new-method-envelope-tag = Zero-based
budgets-new-method-envelope-detail = Every dollar received is assigned to a category.
budgets-new-method-limits = Category limits
budgets-new-method-limits-tag = Caps
budgets-new-method-limits-detail = A monthly cap per category, measured against spending.
budgets-new-method-split = Percentage split
budgets-new-method-split-tag = 50/30/20
budgets-new-method-split-detail = Buckets measured against the income received.
budgets-new-method-project = Project
budgets-new-method-project-tag = One-off
budgets-new-method-project-detail = One total over a date range, split into lines.
budgets-new-method-later = coming later
budgets-new-accounts-help = Spending on these accounts counts in this budget. Accounts left off still count toward net worth.
budgets-new-accounts-none = No account takes transactions in this unit.
budgets-new-start-empty = Empty
budgets-new-start-copy = Copy categories from { $budget }
budgets-new-start-average = Last 3 months' spending
budgets-new-start-help = Each writes its amounts onward from this month. A category with no spending stays unbudgeted.
budgets-new-error-name = Another budget already has that name.
budgets-new-error-name-required = Give the budget a name.
budgets-new-error-account-gone = An account in this budget no longer exists.
budgets-new-error-account-unit = An on-budget account must take transactions in the budget's unit.
budgets-new-submit = Create budget
budgets-new-submit-edit = Save

## The status-line legends while a Dialog is open. The chords come from the Dialog keymap or are
## the Dialog's own letters.

budgets-hint-transactions = transactions
budgets-hint-save = save
budgets-hint-source = source
budgets-hint-choose = choose
budgets-hint-next-field = next field
budgets-hint-confirm = confirm
budgets-hint-fill-confirm = fill

## The Toasts a confirmed Dialog raises before it closes.

budgets-toast-created = Created the budget { $budget }
budgets-toast-saved = Saved the budget { $budget }
budgets-toast-limit-saved = Saved the budget for { $category }
budgets-toast-stopped = Stopped budgeting { $category }
budgets-toast-filled = Filled { $month }
