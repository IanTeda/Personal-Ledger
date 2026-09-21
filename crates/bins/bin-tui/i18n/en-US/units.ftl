## The Units view title and main list.

tui-units-title = Units & Prices

## The Units list heading and column headers.

tui-units-list-heading = UNITS
tui-units-list-heading-tag = { $visible } OF { $total }
tui-units-column-code = CODE
tui-units-column-type = TYPE
tui-units-column-last = LAST

## The Units summary section.

tui-units-summary-heading = SUMMARY
tui-units-summary-units-held = UNITS HELD
tui-units-summary-last-market-price = LAST MARKET PRICE
tui-units-summary-last-market-value = LAST MARKET VALUE
tui-units-summary-priced = priced
tui-units-summary-week-change = week change
tui-units-summary-52w-range = 52w range
tui-units-summary-symbol = symbol · priced in
tui-units-summary-precision = precision
tui-units-summary-accounts = accounts

## The Weekly Close candlestick chart section.

tui-units-weekly-close-heading = WEEKLY CLOSE · VDHG
tui-units-weekly-close-tag = SEP 25 – SEP 26 · CANDLESTICK

## The Weekly Prices table section.

tui-units-weekly-prices-heading = WEEKLY PRICES
tui-units-weekly-prices-tag = W/C MONDAY · { $units_held } UNITS
tui-units-weekly-prices-column-wc = W/C
tui-units-weekly-prices-column-close = CLOSE
tui-units-weekly-prices-column-change = Δ%
tui-units-weekly-prices-column-market-value = MARKET VALUE

## The command/keybind hints section.

tui-units-hints-heading = KEYS · COMMANDS
tui-units-hint-j-k = j/k unit
tui-units-hint-tab = Tab list ↔ prices
tui-units-hint-n = n new
tui-units-hint-e = e edit
tui-units-hint-d = d delete
tui-units-hint-p = p set price
tui-units-hint-window = [ ] 12m window

## The command line hints for units commands.

tui-units-command-new = :unit new \<code\> \<type\> \<precision\>  — add a unit
tui-units-command-edit = :unit edit VDHG  — name, symbol, precision, active
tui-units-command-price = :price set VDHG \<date\> \<close\>  ·  :price import \<path.csv\>

## The New Unit popup.

tui-unit-new-title = new unit
tui-unit-new-command = :unit new
tui-unit-new-field-code = code
tui-unit-new-field-name = name
tui-unit-new-field-type = type
tui-unit-new-field-symbol = symbol
tui-unit-new-field-source = source
tui-unit-new-field-priced-in = priced in
tui-unit-new-field-qty-precision = qty precision
tui-unit-new-field-price-precision = price precision
tui-unit-new-field-active = active
tui-unit-new-hint-symbol = optional, your own reference
tui-unit-new-hint-source = tab to pick another unit
tui-unit-new-hint-priced-in = tab to pick another unit
tui-unit-new-hint-qty-precision = decimals held
tui-unit-new-hint-price-precision = permanent
tui-unit-new-warning = CODE AND PRICE PRECISION CANNOT CHANGE ONCE A TRANSACTION EXISTS

## The Edit Unit popup.

tui-unit-edit-title = edit unit
tui-unit-edit-command = :unit edit
tui-unit-edit-field-code = code
tui-unit-edit-field-name = name
tui-unit-edit-field-type = type
tui-unit-edit-field-symbol = symbol
tui-unit-edit-field-priced-in = priced in
tui-unit-edit-field-qty-precision = qty precision
tui-unit-edit-field-active = active
tui-unit-edit-note-code = referenced by { $count ->
    [one] 1 transaction
    *[other] { $count } transactions
}
tui-unit-edit-note-type = fixed while prices exist
tui-unit-edit-locked-note-symbol = no
tui-unit-edit-title-tag = { $accounts ->
    [one] 1 account
    *[other] { $accounts } accounts
} · { $transactions ->
    [one] 1 txn
    *[other] { $transactions } txns
} · { $prices ->
    [one] 1 price
    *[other] { $prices } prices
}
tui-unit-edit-precision-warning = QTY PRECISION IS LOWER THAN THE LOWEST HOLDING · SOME UNITS MAY NOT ROUND

## The Delete Unit popup.

tui-unit-delete-title = delete unit
tui-unit-delete-confirm = confirm

## Footer hints for popups.

tui-unit-new-help-tab = tab next field
tui-unit-new-help-create = ^s create
tui-unit-new-help-cancel = esc cancel

tui-unit-edit-help-tab = tab next field
tui-unit-edit-help-save = ^s save
tui-unit-edit-help-cancel = esc cancel

tui-unit-delete-help-delete = ^s delete
tui-unit-delete-help-cancel = esc cancel
