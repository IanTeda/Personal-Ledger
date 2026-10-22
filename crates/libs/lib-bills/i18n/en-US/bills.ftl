## The Bills module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. The Pay Dialog's stay in the Desktop's catalogue until
## it moves to `lib-bill-payments`.

## The Bills page (8a–8e): the header and the tab row. `$glyph` is the Add button's leading plus
## sign.

bills-add-button = { $glyph } Add bill plan
bills-tab-schedule = Schedule
bills-tab-planner = Planner

## The Schedule tab's meta line: the period's (or All's) row count, then its Overdue (carried rows
## included), Due and Paid counts, then its planned total (`$total`, already formatted).

bills-schedule-entries = { $count ->
    [one] { $count } schedule entry this period
   *[other] { $count } schedule entries this period
}
bills-schedule-entries-all = { $count ->
    [one] { $count } schedule entry in all
   *[other] { $count } schedule entries in all
}
bills-schedule-overdue = <strong>{ $count }</strong> overdue
bills-schedule-due = <strong>{ $count }</strong> due
bills-schedule-paid = { $count } paid
bills-schedule-planned = { $total } planned
bills-total-mixed = mixed units

## The Schedule table.

bills-column-bill = Bill
bills-column-account = Account
bills-column-planned = Planned
bills-column-due = Due
bills-column-status = Status
bills-column-actual = Actual
bills-column-paid = Paid
bills-estimated = estimated
bills-status-paid = paid
bills-status-skipped = skipped
bills-status-overdue = overdue
bills-status-due = due
bills-status-upcoming = upcoming
bills-row-pay = pay
bills-row-skip = skip
bills-row-view-transaction = view transaction →
bills-row-excluded = excluded from history
bills-row-not-actionable = not yet actionable
bills-schedule-empty = No bills fall due this period.
bills-schedule-footnote = These rows are generated ahead of time from the Bill Plans on the Planner tab — one per due date, never deleted. Overdue rows, and Due rows inside their Bill Plan's attention lead (⚑), also surface in Needs Attention on the Dashboard.
## A cell with nothing to show.
bills-none = —

## The Planner tab (8b): its meta line, table and footnote.

bills-planner-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}
bills-planner-inactive = <strong>{ $count }</strong> inactive
bills-column-name = Name
bills-column-category = Category
bills-column-recurs = Recurs
bills-column-lead = Lead
bills-column-active = Active
bills-recurrence-weekly = Weekly
bills-recurrence-fortnightly = Fortnightly
bills-recurrence-monthly = Monthly
bills-recurrence-quarterly = Quarterly
bills-recurrence-annually = Annually
bills-recurrence-one-shot = One-shot
# `$days` is the Attention Lead in days.
bills-lead-days = { $days }d
bills-active-yes = yes
bills-active-no = no
bills-row-edit = edit
bills-planner-empty = No bill plans yet.
bills-planner-footnote = Editing a plan only changes future Schedule rows. LEAD is the Attention Lead — days before the due date a still-Due (not yet Overdue) row also joins Needs Attention; blank means Overdue-only. An inactive plan generates no new rows, but its history stays intact on the Schedule.

## The Schedule tab's filter row and stat callout (carried over from 8f's History tab). `$from` and
## `$to` are the Average's financial year as two-digit years (FY25–26).

bills-filter-status-paid = Paid
bills-filter-status-skipped = Skipped
bills-filter-status-overdue = Overdue
bills-filter-status-due = Due
bills-filter-status-upcoming = Upcoming
bills-filter-all-bills = All bills
bills-filter-all-categories = All categories
bills-filter-all-accounts = All accounts
bills-history-last-paid = Last paid
bills-history-average = Average FY{ $from }–{ $to }
bills-history-no-payments-last-fy = no payments last FY
bills-history-same-month = Same month last year
bills-history-last-year = Last year
bills-history-same-month-payments = Same month last year ({ $count ->
    [one] { $count } payment
   *[other] { $count } payments
})
bills-history-no-payments-yet = No payments yet
bills-filter-empty = No schedule rows match these filters.
bills-history-footnote = Skipped rows keep their planned figure for context but are left out of the average, last-paid and same-period-last-year figures above.

## The status line: each tab's legend, the period (or All) and its filtered row count of the
## unfiltered, the Planner's Plan count, and the message for a key whose row can't take it.

bills-hint-row = row
bills-hint-pay = pay
bills-hint-skip = skip
bills-hint-period = period
bills-hint-all = all
bills-hint-status-chips = status
bills-hint-filters = filters
bills-hint-switch-view = switch view
bills-hint-edit = edit
bills-hint-new = new
bills-period-all = All
bills-status-period = { $period } · { $shown } of { $count ->
    [one] { $count } entry
   *[other] { $count } entries
}
bills-status-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}
bills-status-not-actionable = this bill isn't actionable

## The Add and Edit bill plan dialog (8c). `$name` is the Plan's stored name.

bills-plan-add-title = Add bill plan
bills-plan-edit-title = Edit bill plan — { $name }
bills-plan-add-submit = Add bill plan
bills-plan-edit-submit = Save
bills-plan-name-placeholder = e.g. Telstra Internet
bills-plan-field-category = Category (expense only)
bills-plan-field-payee = Payee (optional)
bills-plan-payee-none = — none, ask every time —
bills-plan-no-accounts = No account takes this unit
bills-plan-field-amount = Planned amount
bills-plan-amount-placeholder = 0.00
bills-plan-field-amount-kind = Amount is
bills-amount-fixed = Fixed
bills-amount-estimated = Estimated
bills-plan-field-recurrence = Recurrence
bills-plan-field-first-due = First due
bills-plan-field-ends-on = Ends on
bills-plan-ends-on-placeholder = Recurs indefinitely
bills-plan-field-lead = Attention lead
bills-plan-lead-explainer = days before the due date a Due row joins Needs Attention; blank means Overdue only
bills-plan-lead-placeholder = none
bills-plan-active = Active — generate future Schedule rows
bills-plan-error-name = A bill plan needs a name.
bills-plan-error-category = Choose an expense category with no subcategories.
bills-plan-error-account = Choose an account in this unit that takes transactions.
bills-plan-error-amount = Enter an amount greater than zero.
bills-plan-error-ends-before-first-due = Ends on can't be before first due.
bills-plan-error-lead = Enter a whole number of days.
bills-plan-error-gone = This bill plan no longer exists.

## The Skip dialog (8e). `$name` is the Bill Plan's name and `$due` the entry's due date, both
## already formatted.

bills-skip-title = Skip this cycle — { $name }
bills-skip-body = The { $due } entry is marked Skipped, not Paid. It's left out of { $name }'s average entirely — a skipped cycle isn't the same as it costing nothing. The next cycle's entry is generated independently and isn't affected.
bills-skip-body-one-shot = The { $due } entry is marked Skipped, not Paid: this one-time bill is cancelled. It's left out of { $name }'s figures entirely rather than counted as costing nothing.
bills-skip-submit = Skip this cycle
bills-skip-error-gone = This bill can no longer be skipped.
