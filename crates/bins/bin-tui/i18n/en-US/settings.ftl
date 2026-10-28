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

## Settings › Display's field labels.

tui-settings-setting-date-style = date style
tui-settings-setting-locale = locale

## The Edit Setting popup.

tui-setting-edit-title = negatives
tui-setting-edit-command = general.negatives

## The Base Unit Guard popup.

tui-setting-guard-prose = transactions are stored in their own units and are not touched. Every reported total is re-converted at the weekly USD close.

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
