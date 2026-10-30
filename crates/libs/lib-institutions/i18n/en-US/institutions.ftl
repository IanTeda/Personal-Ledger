## The Institutions module's Messages, read by its Presenters. Each Client composes this layer into
## its Locale; no Client catalogue keeps a copy. Column labels other screens share (Institution,
## Actions) and the Account kinds come from the shared layer.

## The Settings page. `$count` is the Institutions, `$glyph` the Add button's leading plus sign.

institutions-count = { $count ->
    [one] { $count } institution
   *[other] { $count } institutions
}
institutions-add-button = { $glyph } Add institution
institutions-column-products = Products
institutions-row-edit = edit
institutions-row-delete = delete

## The row's right-click menu.

institutions-menu-edit = Edit institution…
institutions-menu-delete = Delete institution…

## The page's legend.

institutions-hint-row = row
institutions-hint-edit = edit
institutions-hint-delete = delete
institutions-hint-new = new

## The Add and Edit institution Dialog (2e, 16e). `$name` is the Institution as written.

institutions-add-title = Add institution
institutions-add-submit = Add institution
institutions-add-toast = Added { $name }
institutions-edit-title = Edit institution
institutions-edit-submit = Save institution
institutions-edit-toast = Saved { $name }
institutions-name-label = Institution name
institutions-name-placeholder = e.g. Commonwealth Bank
institutions-products = Products
institutions-usual-unit-label = Usual unit
institutions-usual-unit-none = None

## The Products chips, in the order the Add institution Dialog offers them. Lower case, as the
## table's Products cell joins them.

institutions-product-savings = savings
institutions-product-offset = offset
institutions-product-credit-card = credit card
institutions-product-loan = loan
institutions-product-investment = investment
institutions-product-crypto = crypto
institutions-product-retirement = retirement
institutions-product-cash = cash

## An Add refusal, under the name. `$name` is the Institution already holding the name, as it
## spells it; names are compared ignoring case and surrounding spaces.

institutions-error-name-taken = { $name } already exists
institutions-error-name-required = An institution needs a name
institutions-error-not-found = That institution is no longer on the ledger

## The Delete institution Dialog. `$name` is the Institution, `$count` the Accounts that name it or
## the ones left out of the list.

institutions-delete-title = Delete institution — { $name }
institutions-delete-submit = Delete institution
institutions-delete-warning = No account uses this institution. Deleting it cannot be undone.
institutions-delete-confirm-label = Type { $name } to confirm
institutions-delete-toast = Deleted { $name }

## What the Dialog says instead when an Account names the Institution: the Accounts, the first few
## by name, and a Close button in place of Delete institution.

institutions-delete-used-by = Used by { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}
institutions-delete-used-by-hint = Change those accounts' institution first, then delete this one.
institutions-delete-and-more = and { $count } more
institutions-delete-close = Close

## The command palette's descriptions of the Commands the keymap lists, in lower case as the palette shows them.

institutions-command-new = add an Institution
institutions-command-edit = edit the selected Institution
institutions-command-delete = delete the selected Institution
