## The Dashboard view: header, net worth chart, income vs expense chart, pie chart and budget list.
## Mock data representing typical content, not real database figures.

## The dashboard header: main title and subtitle with date/accounts/currency.

desktop-dashboard-title = Financial position
desktop-dashboard-subtitle = { $date } · { $accounts } · { $currency }

## The figure row: NET POSITION and supporting metrics (30-DAY, ASSETS, LIABILITIES).

desktop-dashboard-net-position = NET POSITION
desktop-dashboard-metric-30-day = 30-DAY
desktop-dashboard-metric-assets = ASSETS
desktop-dashboard-metric-liabilities = LIABILITIES

## The net worth chart: title and axis label.

desktop-dashboard-net-worth-title = NET WORTH · 18 MONTHS
desktop-dashboard-net-worth-close = monthly close

## The income vs expense chart: title and legend.

desktop-dashboard-in-vs-out-title = IN VS OUT · 6 MONTHS
desktop-dashboard-in-vs-out-expense = ◀ expense
desktop-dashboard-in-vs-out-income = income ▶

## The pie chart ("where it went"): title and legend prefix (showing category name and percentage).

desktop-dashboard-where-it-went-title = WHERE IT WENT · 30 DAYS

## The budget list: title, period marker, and budget row legend.

desktop-dashboard-budgets-title = BUDGETS THIS PERIOD
desktop-dashboard-budgets-period = { $month } · day { $day }/{ $days } · { $percent }% elapsed
desktop-dashboard-budgets-legend = │ = period progress · bar = spent / budget

## The "Needs Attention" section: title and actionable items.

desktop-dashboard-needs-attention = NEEDS ATTENTION
desktop-dashboard-unreconciled = { $account } has { $count ->
    [one] { $count } unreconciled transaction
   *[other] { $count } unreconciled transactions
} →
desktop-dashboard-flagged-transactions = { $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
} flagged for review →
