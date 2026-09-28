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
desktop-status-pay-bill-not-yet-built = pay bill — not yet built
desktop-status-skip-bill-not-yet-built = skip bill — not yet built
desktop-status-add-bill-plan-not-yet-built = add bill plan — not yet built
desktop-status-bill-not-actionable = this bill isn't actionable
