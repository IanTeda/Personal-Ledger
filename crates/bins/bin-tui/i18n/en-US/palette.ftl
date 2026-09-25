## The command popup (the shell's own command window). Command names and their `<...>`/`[...]`
## syntax are stable English ids, dispatched on `CommandId`, so they are passed in as arguments and
## never translated — only the domain headers, the descriptions, the argument previews and the
## footer hints are Messages.

## The resting-state group headers. Every other domain reuses a `nav-*` Message, or the shell's own
## `tui-view-balance-checks-title`, so only the file-level group needs one of its own.

tui-command-domain-quit = Quit

## The prompt row's match count, for example `3 of 57`.

tui-command-match-count = { $matches } of { $total }

## The info row when Enter lands on a command with nothing behind it yet. `$command` is the command
## as typed, including its leading colon.

tui-command-not-yet-built = { $command } — not yet built

## The footer hint strip. Each label sits beside its key, which the caller styles bold, so the key
## itself is never part of the Message.

tui-command-hint-select = select
tui-command-hint-complete = complete
tui-command-hint-run = run
tui-command-hint-history = history
tui-command-hint-close = close

## Command descriptions, written in lower case as the popup shows them. The two that every `off`/`on`
## pair shares are written once: `$key` is the chord that reveals inactive rows, `$command` the
## command being reversed.

tui-command-off-description = is_active = 0 — hides it from the list unless { $key }
tui-command-on-description = is_active = 1 — reverses { $command }

## Accounts.

tui-command-account-description = accounts grouped by type, per-unit subtotals, ledger
tui-command-account-new-description = add an account — opens the new popup, blank
tui-command-account-edit-description = edit the highlighted account
tui-command-account-delete-description = delete — a non-empty account needs a same-unit transfer target
tui-command-account-check-description = records a Balance Check, prints the variance

## Balance checks.

tui-command-check-list-description = balance checks and clearing
tui-command-check-new-description = record a balance check
tui-command-check-edit-description = edit the highlighted balance check
tui-command-check-delete-description = delete the highlighted balance check
tui-command-check-import-description = import balance checks from a CSV file

## Budgets.

tui-command-budget-list-description = budgets vs actual for the period
tui-command-budget-new-description = start tracking a category
tui-command-budget-edit-description = change the limit or period
tui-command-budget-delete-description = stop tracking a category

## Categories. `$into` and `$from` are the merge command's own argument tokens.

tui-command-category-description = categories tree — direct vs rollup, spend and transactions
tui-command-category-new-description = add a category — parent defaults to the tree selection
tui-command-category-edit-description = edit the highlighted category
tui-command-category-move-description = move — refuses cycles and a cross-root move with transactions
tui-command-category-rename-description = rename — nothing else references a category by name
tui-command-category-merge-description = reassigns transactions to { $into }, deletes { $from }
tui-command-category-archive-description = active = 0 — keeps every transaction and total
tui-command-category-tree-description = prints the subtree — scriptable/pipeable

## Dashboard, Help, Quit and Settings.

tui-command-dashboard-description = financial position — the default view
tui-command-help-description = browse every command
tui-command-quit-description = quit the app
tui-command-settings-description = groups, overrides and the settings table

## Payees.

tui-command-payee-description = payees ranked by spend, curation flags, record, matches
tui-command-payee-new-description = add a payee — opens the new popup, blank
tui-command-payee-edit-description = edit the highlighted payee
tui-command-payee-rename-description = rename — keeps the old name as a match
tui-command-payee-match-description = rename matches — add/edit/remove, live resolution test
tui-command-payee-match-add-description = adds an exact-text match — refuses on cross-payee collision
tui-command-payee-default-description = sets the default category — pre-fills it on transaction entry
tui-command-payee-delete-description = opens the delete popup — the database refuses a referenced payee

## Reports.

tui-command-report-list-description = spending report and charts
tui-command-report-account-balance-description = every account's current balance
tui-command-report-category-total-description = spending by category over a range
tui-command-report-payee-total-description = spending by payee over a range
tui-command-report-budget-variance-description = budgets vs actual for the current period
tui-command-report-balance-check-variance-description = asserted vs computed balance, by check

## Tags. `$key` is the chord that confirms the armed delete.

tui-command-tag-description = flat alphabetical list, summary box, lightweight delete
tui-command-tag-new-description = add a tag — opens the new popup, blank
tui-command-tag-edit-description = edit the highlighted tag
tui-command-tag-delete-description = arms the lightweight delete confirm — { $key } on the list confirms

## Transactions.

tui-command-txn-recent-description =
    { $count ->
        [one] last transaction, all accounts
       *[other] last { $count } transactions, all accounts
    }
tui-command-txn-new-description = add a transaction from anywhere
tui-command-txn-edit-description = edit the highlighted transaction
tui-command-txn-delete-description = delete — confirms by payee and amount

## Units.

tui-command-unit-description = Units screen with details, summary and prices
tui-command-unit-new-description = add a unit — code is permanent
tui-command-unit-edit-description = edit the highlighted unit
tui-command-unit-delete-description = delete — units in use can't be removed

## Argument previews: the fixed example shown beside an argument's own token in the popup's info
## row. Three are shared by several commands.

tui-command-preview-list-selection = the list selection
tui-command-preview-list-selection-inactive = the list selection, with { $key } held to see it
tui-command-preview-tree-selection = the tree selection

tui-command-preview-account-new = e.g. Everyday Spending, Mortgage Offset
tui-command-preview-account-edit = Everyday Spending · bank · AUD 4 210.65
tui-command-preview-account-delete = Everyday Spending · 1 284 txns — needs { $argument }
tui-command-preview-account-check = not yet designed — { $doc }

tui-command-preview-check-new = Everyday · last check 31 aug · $4,210.55
tui-command-preview-check-import = e.g. ./import/august-checks.csv

tui-command-preview-budget-list = current: SEP 2026 · optional, defaults to this period
tui-command-preview-budget-new = e.g. dining, groceries — one budget per category
tui-command-preview-budget-edit = dining · limit 300.00 · actual 412.00 · over by 112.00
tui-command-preview-budget-delete = dining · limit 300.00 — stops tracking, keeps past transactions

tui-command-preview-category-new = e.g. dining, groceries, salary
tui-command-preview-category-edit = groceries · expense · depth 3
tui-command-preview-category-move = e.g. expenses/food/daily — completes on full paths
tui-command-preview-category-rename = e.g. dining
tui-command-preview-category-merge = the surviving category
tui-command-preview-category-tree = defaults to both roots

tui-command-preview-payee-new = e.g. Woolworths, ATO, Telstra
tui-command-preview-payee-edit = Woolworths · -18 402.55 · 184 txns
tui-command-preview-payee-rename = e.g. Woolworths Group
tui-command-preview-payee-match = Woolworths · 2 matches
tui-command-preview-payee-match-add = e.g. WW Metro
tui-command-preview-payee-default = e.g. food/groceries
tui-command-preview-payee-delete = Woolworths · 184 txns · 2 matches — refused

tui-command-preview-tag-new = e.g. Japan Trip 2026, Tax Deductible
tui-command-preview-tag-edit = Japan Trip 2026 · active · 7 transactions

tui-command-preview-txn-new = optional · defaults to the current account, if any

tui-command-preview-unit-new = e.g. AUD, VDHG, BTC — must be unique, permanent once set
tui-command-preview-unit-edit = VDHG · etf · Vanguard Diversified High Growth
tui-command-preview-unit-delete = VDHG · in use by 1 account — can't be removed
