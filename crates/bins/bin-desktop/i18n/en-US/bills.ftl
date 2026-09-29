## The Bills page (8a–8f): the header, tab row and Schedule tab.

## The header. `$glyph` is the Add button's leading plus sign.

desktop-bills-add-button = { $glyph } Add bill plan
desktop-bills-tab-schedule = Schedule
desktop-bills-tab-planner = Planner
desktop-bills-tab-history = History

## The Schedule tab's meta line: the period's row count, then its Overdue (carried rows included),
## Due and Paid counts, then its planned total (`$total`, already formatted).

desktop-bills-schedule-entries = { $count ->
    [one] { $count } schedule entry this period
   *[other] { $count } schedule entries this period
}
desktop-bills-schedule-overdue = <strong>{ $count }</strong> overdue
desktop-bills-schedule-due = <strong>{ $count }</strong> due
desktop-bills-schedule-paid = { $count } paid
desktop-bills-schedule-planned = { $total } planned
desktop-bills-total-mixed = mixed units

## The Schedule table.

desktop-bills-column-bill = Bill
desktop-bills-column-account = Account
desktop-bills-column-planned = Planned
desktop-bills-column-due = Due
desktop-bills-column-status = Status
desktop-bills-estimated = estimated
desktop-bills-status-paid = paid
desktop-bills-status-skipped = skipped
desktop-bills-status-overdue = overdue
desktop-bills-status-due = due
desktop-bills-status-upcoming = upcoming
desktop-bills-row-pay = pay
desktop-bills-row-skip = skip
desktop-bills-row-view-transaction = view transaction →
desktop-bills-row-excluded = excluded from history
desktop-bills-row-not-actionable = not yet actionable
desktop-bills-schedule-empty = No bills fall due this period.
desktop-bills-schedule-footnote = These rows are generated ahead of time from the Bill Plans on the Planner tab — one per due date, never deleted. Overdue rows, and Due rows inside their Bill Plan's attention lead (⚑), also surface in Needs Attention on the Dashboard.

## The Planner tab (8b): its meta line, table and footnote.

desktop-bills-planner-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}
desktop-bills-planner-inactive = <strong>{ $count }</strong> inactive
desktop-bills-column-name = Name
desktop-bills-column-category = Category
desktop-bills-column-recurs = Recurs
desktop-bills-column-lead = Lead
desktop-bills-column-active = Active
desktop-bills-recurrence-weekly = Weekly
desktop-bills-recurrence-fortnightly = Fortnightly
desktop-bills-recurrence-monthly = Monthly
desktop-bills-recurrence-quarterly = Quarterly
desktop-bills-recurrence-annually = Annually
desktop-bills-recurrence-one-shot = One-shot
# `$days` is the Attention Lead in days.
desktop-bills-lead-days = { $days }d
desktop-bills-active-yes = yes
desktop-bills-active-no = no
desktop-bills-row-edit = edit
desktop-bills-planner-empty = No bill plans yet.
desktop-bills-planner-footnote = Editing a plan only changes future Schedule rows. LEAD is the Attention Lead — days before the due date a still-Due (not yet Overdue) row also joins Needs Attention; blank means Overdue-only. An inactive plan generates no new rows, but its history stays intact under History.
desktop-bills-status-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}

## The tabs not built yet.

desktop-bills-tab-not-yet-built = This tab is not yet built.

## The status line: the period and its row count; and the messages for keys whose dialog isn't
## built yet, or whose row can't take them.

desktop-bills-status-period = { $period } · { $count ->
    [one] { $count } entry
   *[other] { $count } entries
}
desktop-hint-pay = pay
desktop-hint-skip = skip
desktop-hint-switch-view = switch view
desktop-hint-period = period
desktop-status-skip-bill-not-yet-built = skip bill — not yet built
desktop-status-bill-not-actionable = this bill isn't actionable

## The Add and Edit bill plan dialog (8c). `$name` is the Plan's stored name.

desktop-bills-plan-add-title = Add bill plan
desktop-bills-plan-edit-title = Edit bill plan — { $name }
desktop-bills-plan-add-submit = Add bill plan
desktop-bills-plan-edit-submit = Save
desktop-bills-plan-name-placeholder = e.g. Telstra Internet
desktop-bills-plan-field-category = Category (expense only)
desktop-bills-plan-field-payee = Payee
desktop-bills-plan-no-accounts = No account takes this unit
desktop-bills-plan-field-amount = Planned amount
desktop-bills-plan-field-amount-kind = Amount is
desktop-bills-amount-fixed = Fixed
desktop-bills-amount-estimated = Estimated
desktop-bills-plan-field-recurrence = Recurrence
desktop-bills-plan-field-first-due = First due
desktop-bills-plan-field-ends-on = Ends on
desktop-bills-plan-ends-on-placeholder = Recurs indefinitely
desktop-bills-plan-field-lead = Attention lead
desktop-bills-plan-lead-explainer = days before the due date a Due row joins Needs Attention; blank means Overdue only
desktop-bills-plan-lead-placeholder = none
desktop-bills-plan-active = Active — generate future Schedule rows
desktop-bills-plan-error-name = A bill plan needs a name.
desktop-bills-plan-error-category = Choose an expense category with no subcategories.
desktop-bills-plan-error-account = Choose an account in this unit that takes transactions.
desktop-bills-plan-error-amount = Enter an amount greater than zero.
desktop-bills-plan-error-ends-before-first-due = Ends on can't be before first due.
desktop-bills-plan-error-lead = Enter a whole number of days.

## The Pay dialog (8d). `$name` is the Bill Plan's name and `$due` the entry's due date, both
## already formatted; `$amount` is the Plan's planned amount, formatted with its unit.

desktop-bills-pay-title = Pay — { $name }, { $due }
desktop-bills-pay-mode-direct = Pay it directly
desktop-bills-pay-mode-match = Match existing transaction
desktop-bills-pay-candidates = Unmatched transactions
desktop-bills-pay-none-of-these = None of these — pay it directly instead
desktop-bills-pay-no-candidates = No unmatched expense within 14 days of the due date.
desktop-bills-pay-match-notice = Matching marks this entry Paid using that transaction's real date and amount — the plan's { $amount } estimate is discarded for this cycle. It won't create a duplicate transaction.
desktop-bills-pay-direct-notice = Creates a transaction from the plan's category, payee and account, and marks this entry Paid.
desktop-bills-pay-field-amount = Amount
desktop-bills-pay-field-date = Date
desktop-bills-pay-no-payee = No payee
desktop-bills-pay-submit-direct = Create transaction & mark paid
desktop-bills-pay-submit-match = Match & mark paid
desktop-bills-pay-error-amount = Enter an amount greater than zero.
desktop-bills-pay-error-gone = This bill can no longer be paid.
desktop-hint-switch-panel = switch panel
