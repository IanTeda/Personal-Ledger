## The Dashboard's own labels, captions and tags. Written in sentence case; the renderer upper-cases
## the headings and tags it shows in capitals. The figures beside them are still wireframe mock data
## and are not Messages. The box titles the desktop shares (`dashboard-net-position-title`,
## `dashboard-budgets-this-period-title`, `dashboard-needs-attention-title`) come from the shared
## layer.

## The headline trio beside the net position figure. `$days` is the window in days.

tui-dashboard-stat-days = { $days ->
    [one] 1 day
    *[other] { $days } days
}
tui-dashboard-stat-assets = Assets
tui-dashboard-stat-liabilities = Liabilities

## The net-worth chart.

tui-dashboard-net-worth-title = Net worth

## The income-vs-expense bars: the heading, its right-hand tag naming the window in months, and the
## two captions under the diverging bars.

tui-dashboard-income-vs-expense-title = Income vs expense
tui-dashboard-income-vs-expense-tag = { $months }m divergent
tui-dashboard-caption-expense = expense
tui-dashboard-caption-income = income

## The spending pie chart: the heading with its window in days, and the right-hand tag.

tui-dashboard-where-it-went-title = Where it went · { $days }d
tui-dashboard-pie-chart-tag = Pie chart

## Budgets this period: the right-hand tag naming the period and how much of it has elapsed, the
## wireframe period label itself, and the caption explaining the bars. `$tick` is the tick glyph.

tui-dashboard-budget-period-tag = { $period } · { $percent }% elapsed
tui-dashboard-budget-period-label = { $month } · day { $day }/{ $days }
tui-dashboard-budget-caption = { $tick } = period progress · bar = actual / limit

## Needs attention: the right-hand column heading, the trailing row hinting at a fuller list, and
## the wireframe items themselves.

tui-dashboard-attention-command-heading = Command
tui-dashboard-attention-more = ...more
tui-dashboard-attention-unreconciled =
    { $count ->
        [one] { $count } unreconciled transaction
       *[other] { $count } unreconciled transactions
    }
tui-dashboard-attention-variance = balance check variance { $amount }
