## The Categories view, three popups (new, edit, move), and command palette entries.

## The Categories view title.

tui-categories-title = Categories

## The tree list: header showing visible count, total count, depth and folded count.

tui-categories-tree-header = tree { $visible } of { $total } · depth { $depth } · { $folded } folded

## The tree column headers and symbols.

tui-categories-tree-column-n = N
tui-categories-tree-column-rollup = 12M
tui-categories-tree-node-archived = · archived
tui-categories-tree-node-leaf-count = —

## The summary box: label/value pairs shown for the selected node.

tui-categories-summary-merged = direct · rollup 12m
tui-categories-summary-direct = direct 12m
tui-categories-summary-rollup = rollup 12m
tui-categories-summary-kind-depth = kind · depth
tui-categories-summary-children-leaf = none · leaf
tui-categories-summary-children-parent = { $count } · parent
tui-categories-summary-transactions-none = none
tui-categories-summary-transactions-direct = { $count } · direct
tui-categories-summary-first-last-none = none
tui-categories-summary-first-last = { $first } · { $last }
tui-categories-summary-note = note
tui-categories-summary-active-offered = [×] · offered
tui-categories-summary-active-not-offered = [ ] · not offered

## The spend chart heading showing which series is plotted.

tui-categories-chart-direct = DIRECT SPEND
tui-categories-chart-subtree = SUBTREE SPEND
tui-categories-chart-date-range = { $first } – { $last }
tui-categories-chart-avg = avg { $value }

## The transactions list: column headers and footer.

tui-categories-transactions-title = TRANSACTIONS
tui-categories-transactions-heading = { $shown } of { $total } · newest first
tui-categories-transactions-column-date = DATE
tui-categories-transactions-column-account = ACCOUNT
tui-categories-transactions-column-category = CATEGORY
tui-categories-transactions-column-payee = PAYEE
tui-categories-transactions-column-amount = AMOUNT
tui-categories-transactions-footer = { $direct } direct · { $subtree } in subtree  ·  enter open txn

## The New category popup.

tui-category-new-title = new category
tui-category-new-field-name = name
tui-category-new-field-parent = parent
tui-category-new-field-note = note
tui-category-new-field-active = active
tui-category-new-field-kind = kind
tui-category-new-field-depth = depth
tui-category-new-note-kind = kind cannot be set here
tui-category-new-note-parent-required = an unresolved parent
tui-category-new-note-sibling-clash = case-insensitive sibling clash
tui-category-new-note-completion = completion
tui-category-new-note-no-matches = no matches · tab
tui-category-new-placeholder-parent = \<parent\>
tui-category-new-preview-parent = expenses/food · inherited from root
tui-category-new-help-tab = next field
tui-category-new-help-create = create
tui-category-new-help-create-and-add = create & add another
tui-category-new-help-cancel = cancel

## The Edit category popup.

tui-category-edit-title = edit category
tui-category-edit-field-name = name
tui-category-edit-field-parent = parent
tui-category-edit-field-note = note
tui-category-edit-field-active = active
tui-category-edit-field-kind = kind
tui-category-edit-field-depth = depth
tui-category-edit-note-completion = completion
tui-category-edit-note-no-matches = no matches · tab
tui-category-edit-placeholder-parent = \<parent\>
tui-category-edit-help-tab = next field
tui-category-edit-help-save = save
tui-category-edit-help-deactivate = deactivate
tui-category-edit-help-merge = merge
tui-category-edit-help-cancel = cancel

## The Move category popup.

tui-category-move-title = move category
tui-category-move-field-target = target
tui-category-move-field-confirm = confirm
tui-category-move-field-children = children
tui-category-move-field-transactions = transactions
tui-category-move-field-active = active
tui-category-move-note-sibling-clash = case-insensitive sibling clash
tui-category-move-note-completion = completion
tui-category-move-note-no-matches = no matches · tab
tui-category-move-placeholder-target = \<parent\>
tui-category-move-help-tab = complete parent
tui-category-move-help-new = new parent
tui-category-move-help-move = move
tui-category-move-help-cancel = cancel

## Errors when creating, editing or moving categories.

tui-category-error-not-found = category not found
tui-category-error-creation-failed = failed to create category
tui-category-error-edit-failed = failed to edit category
tui-category-error-move-failed = failed to move category

## The command palette: Categories domain commands and their help text.

tui-command-category = browse the expense/income tree, plot spend by month
tui-command-category-new = add a child category · opens the new popup, blank
tui-command-category-new-placeholder = \<name\>
tui-command-category-new-preview = e.g. Groceries, Rent

tui-command-category-edit = edit the highlighted category
tui-command-category-edit-placeholder = \<name\>
tui-command-category-edit-preview = Food · parent Expenses

tui-command-category-move = move to a different parent
tui-command-category-move-placeholder = \<parent\>
tui-command-category-move-preview = Expenses/Food to Expenses/Household

tui-command-category-archive = archive the highlighted category
tui-command-category-archive-placeholder = \<name\>
tui-command-category-archive-preview = Food

tui-command-category-unarchive = unarchive a category
tui-command-category-unarchive-placeholder = \<name\>
tui-command-category-unarchive-preview = Food

tui-command-category-merge = merge — not yet built
