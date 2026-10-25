## The Institutions module's Messages, read by its Presenters. Each Client composes this layer into
## its Locale; no Client catalogue keeps a copy. Column labels other screens share (Institution,
## Actions) and the Account kinds come from the shared layer.

## The Settings page. `$count` is the Institutions, `$glyph` the Add button's leading plus sign.

institutions-count = { $count ->
    [one] { $count } institution
   *[other] { $count } institutions
}
institutions-add-button = { $glyph } Add institution
institutions-column-account-types = Account type
institutions-row-edit = edit
institutions-row-delete = delete

## The status line when a row's edit or delete is asked for: neither has a Dialog yet.

institutions-edit-not-built = edit institution — not yet built
institutions-delete-not-built = delete institution — not yet built

## The page's legend.

institutions-hint-row = row
institutions-hint-edit = edit
institutions-hint-delete = delete
institutions-hint-new = new

## The Add institution Dialog (2e). `$name` is the Institution as written.

institutions-add-title = Add institution
institutions-add-submit = Add institution
institutions-add-toast = Added { $name }
institutions-name-label = Institution name
institutions-name-placeholder = e.g. Commonwealth Bank
institutions-account-types = Account types
institutions-product-savings = savings
institutions-product-offset = offset
institutions-default-unit = Default unit
institutions-no-units = No units available

## An Add refusal, under the name. `$name` is the Institution already holding the name, as it
## spells it; names are compared ignoring case and surrounding spaces.

institutions-error-name-taken = { $name } already exists
institutions-error-name-required = An institution needs a name
