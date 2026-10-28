## The Settings index page (TUI-047) and the page picker (TUI-048).

tui-settings-heading = Settings
tui-settings-page-count = { $count ->
    [one] { $count } page
   *[other] { $count } pages
}
tui-settings-legend = ● syncs with the ledger · ○ this device only
tui-settings-picker-title = Go to Settings Page
# The legend's two halves, one to a row under the docked index (TUI-050), which is too narrow for the one-line legend.
tui-settings-legend-synced = ● syncs with the ledger
tui-settings-legend-this-device = ○ this device only

## A page's breadcrumb heading, before the page's name.

tui-settings-breadcrumb-parent = Settings ›

## Page names, in the index's order.

tui-settings-page-general = General
tui-settings-page-display = Display
tui-settings-page-units = Units
tui-settings-page-institutions = Institutions
tui-settings-page-accounts = Accounts
tui-settings-page-categories = Categories
tui-settings-page-tags = Tags
tui-settings-page-payees = Payees
tui-settings-page-documents = Documents
tui-settings-page-inventory = Inventory
tui-settings-page-ai-connections = AI connections
tui-settings-page-sync-server = Sync server
tui-settings-page-backup = Backup
tui-settings-page-history = History
tui-settings-page-logs = Logs
tui-settings-page-about = About

## The one-line description beside each page's name on the index page.

tui-settings-description-general = ledger name, owner, financial year and base unit
tui-settings-description-display = appearance, colour theme, date style, glyphs and hint bar
tui-settings-description-units = currencies and other units, and their price sources
tui-settings-description-institutions = banks, brokers and lenders
tui-settings-description-accounts = every account, its institution and its unit
tui-settings-description-categories = income and expense categories
tui-settings-description-tags = free labels across transactions
tui-settings-description-payees = who you pay, their match rules and default category
tui-settings-description-documents = document types and their reminders
tui-settings-description-inventory = locations and their areas
tui-settings-description-ai-connections = model providers and assistant access
tui-settings-description-sync-server = server address, status and last sync
tui-settings-description-backup = local backups, export and Git backup
tui-settings-description-history = every change, with undo and restore
tui-settings-description-logs = log level and the live log
tui-settings-description-about = version, licences and links

## The scope note on the right of each page's heading.

tui-settings-scope-general = preferences · synced · ledger identity
tui-settings-scope-display = configuration · local · theme syncs
tui-settings-scope-ledger-data = ledger data · synced
tui-settings-scope-ai-connections = this device
tui-settings-scope-sync-server = this device · synced
tui-settings-scope-backup = this device · local files
tui-settings-scope-history = all devices
tui-settings-scope-logs = this device · diagnostic
tui-settings-scope-about = version info

## Hint bar labels, each after its key.

tui-settings-hint-page = page
tui-settings-hint-open = open
tui-settings-hint-first-last = first/last
tui-settings-hint-back = back
tui-settings-hint-close = close
tui-settings-hint-field = field
tui-settings-hint-edit = edit
tui-settings-hint-pages = pages
tui-settings-hint-picker = picker
tui-settings-hint-settings = settings
# At Large and X-Large, where the index is docked beside the page (TUI-050, TUI-051).
tui-settings-hint-focus-page = focus page
tui-settings-hint-index = index

## Settings › General's field labels.

tui-settings-general-ledger-name = ledger name
tui-settings-general-owner = owner
tui-settings-general-financial-year = financial year starts
tui-settings-general-base-unit = base unit

## The financial year's start months.

tui-settings-month-july = july
tui-settings-month-january = january
tui-settings-month-april = april

## Settings › General's THIS LEDGER box and its note.

tui-settings-general-this-ledger = this ledger
tui-settings-general-accounts = accounts
tui-settings-general-transactions = transactions
tui-settings-general-units = units
tui-settings-general-institutions = institutions
tui-settings-general-note = Ledger name, owner and financial year hold for this session until the Ledger saves Preferences. The base unit is the Units' own and saves at once.

## Settings › Display's field labels, values and notes.

tui-settings-setting-date-style = date style
tui-settings-setting-locale = locale
tui-settings-setting-terminal-colours = terminal colours
tui-settings-setting-status-glyphs = status glyphs
tui-settings-setting-hint-bar = hint bar
tui-settings-terminal-colours-theme = theme rgb
tui-settings-terminal-colours-palette = terminal palette
tui-settings-status-glyphs-unicode = unicode
tui-settings-hint-bar-shown = [x] show key hints
# The note on a Configuration row Settings can't write yet.
tui-settings-config-file-note = config file
tui-settings-display-note-config = Config file rows are read from personal-ledger.conf at start-up; change them there.
tui-settings-display-note-scope = ● rows sync with the ledger. The locale changes only in the file, then a restart.

## Why a field can't be set. `$field` is the name `:set` takes.

tui-settings-read-only-locale = { $field } is read-only: change it in personal-ledger.conf, then restart
tui-settings-read-only-config = { $field } is in personal-ledger.conf, which Settings can't write yet

## A field open in place: its sub-rows' labels and values.

tui-settings-edit-this-year = this year runs
tui-settings-edit-was = was
tui-settings-edit-changes = changes
tui-settings-edit-on-enter = on enter
tui-settings-edit-chosen = chosen
# `$first` and `$last` are dates.
tui-settings-edit-year-range = { $first } – { $last }
# `$value` is the old value, `$range` its year's dates.
tui-settings-edit-was-year = { $value } · { $range }
tui-settings-edit-financial-year-changes = year-to-date · budget years · reports
tui-settings-edit-holds-for-session = holds for this session
tui-settings-edit-base-on-enter = asks you to type the code, then saves

## The hint bar while a field is open. `$value` is the old value.

tui-settings-hint-choose = choose
tui-settings-hint-toggle = toggle
tui-settings-hint-save = save
tui-settings-hint-keep = keep { $value }
tui-settings-hint-next-field = next field

## The base unit guard (TUI-021). `$from` and `$to` are Unit codes.

tui-settings-guard-title = base unit
tui-settings-guard-change = { $from } → { $to }
tui-settings-guard-prose-units = Transactions keep their own units and don't change.
tui-settings-guard-prose-totals = Every reported total is converted again into { $to }.
tui-settings-guard-transactions = transactions
tui-settings-guard-accounts = accounts
tui-settings-guard-converted = converted again
tui-settings-guard-missing = missing prices
tui-settings-guard-then = then
# `$count` is a formatted count.
tui-settings-guard-transactions-value = { $count } · unchanged
# `$count` and `$already` are formatted counts.
tui-settings-guard-accounts-value = { $count } · { $already } already in { $to }
tui-settings-guard-converted-value = { $months ->
    [one] { $months } month of totals
   *[other] { $months } months of totals
}
# `$range` is the first and last month without prices.
tui-settings-guard-missing-value = { $months ->
    [one] { $months } month · { $range }
   *[other] { $months } months · { $range }
}
tui-settings-guard-then-value = saves to this Ledger at once
tui-settings-guard-gaps = { $months ->
    [one] That { $months } month shows as a gap until it has prices.
   *[other] Those { $months } months show as gaps until they have prices.
}
tui-settings-guard-price-sources = open price sources first — settings › units
tui-settings-guard-type = type { $to } to confirm
tui-settings-guard-hint-change = change base unit
tui-settings-guard-hint-price-sources = price sources
tui-settings-guard-hint-cancel = cancel
# The Toast once the base unit is set.
tui-settings-toast-base-set = { $to } is now the base unit

## Locale display descriptions (where the effective Locale came from).

tui-settings-locale-from-system = from your system
tui-settings-locale-from-config = from config
tui-settings-locale-from-default = the default
tui-settings-locale-unsupported = { $requested } is not supported

## Date style labels for the Settings display.

tui-settings-date-style-default = locale default
tui-settings-date-style-short = short
tui-settings-date-style-medium = medium
tui-settings-date-style-long = long
tui-settings-date-style-iso = iso

## The Display group's Colour Theme rows and their list popup.

tui-settings-setting-colour-theme = colour theme
tui-settings-setting-colour-appearance = appearance
tui-settings-colour-theme-note = enter picks
tui-settings-colour-popup-theme-title = COLOUR THEME
tui-settings-colour-popup-appearance-title = APPEARANCE

## Why this Client ignores the Colour Theme: `$key` is the Configuration key.

tui-settings-colour-terminal-note = { $key } is on: this terminal draws its own colours. Your choice still syncs to your other devices.

## `$roles` is the overridden Colour Role keys, comma-separated.

tui-settings-colour-overrides = { $roles } overridden by [theme] in your configuration file. Restart to change them.

## A `[theme]` pair below its contrast rule. `$subject` and `$surface` are Colour Role or
## calculated colour keys; `$ratio` and `$required` are contrast ratios.

tui-settings-colour-contrast = { $subject } on { $surface } is { $ratio }:1, needs { $required }:1

## The footer while the colour list popup is open. `$back` is the key that reverts.

tui-footer-colour-popup = j/k preview · enter keep · { $back } revert

## The rows' note while `$key` (the Configuration key) makes this terminal draw its own colours.

tui-settings-colour-terminal-row-note = syncs; { $key } draws terminal colours here
