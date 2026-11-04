## The Dashboard's Presenter: the Needs Attention Bill rows and the budget list. The page's fixed
## labels and its stub figures are the Desktop's, until #144 gives them data.

## A Bill Schedule entry in Needs Attention: `$due` in the date style, `$amount` with its Unit
## code when that isn't the base Unit.

dashboard-bill-overdue = { $plan } overdue since { $due }, { $amount } →
dashboard-bill-due = { $plan } due { $due }, { $amount } →

## The budget list: the default Budget's month (`$month` already formatted), its day and elapsed
## share, each bar's figures, and the line shown with no bars.

dashboard-budgets-period = { $month } · day { $day }/{ $days } · { $percent }% elapsed
dashboard-budget-figures = { $spent } / { $budget }
dashboard-budgets-none = No budget amounts this month.
