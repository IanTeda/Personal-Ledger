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

## The status line when a row's delete is asked for: it has no Dialog yet.

institutions-delete-not-built = delete institution — not yet built

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
