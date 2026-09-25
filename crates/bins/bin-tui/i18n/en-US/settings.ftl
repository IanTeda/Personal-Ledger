## The Settings view title.

tui-settings-title = Settings

## The Groups section: heading and group names.

tui-settings-groups-heading = GROUPS
tui-settings-group-general = general
tui-settings-group-display = display
tui-settings-group-units-prices = units & prices
tui-settings-group-files-backup = files & backup
tui-settings-group-reconcile = reconcile
tui-settings-group-keys = keys
tui-settings-group-about = about

## Group row count tags.

tui-settings-group-count-general = 8
tui-settings-group-count-display = 7
tui-settings-group-count-units = 6
tui-settings-group-count-files = 5
tui-settings-group-count-reconcile = 4
tui-settings-group-count-keys = 12
tui-settings-group-count-about = —

## The "Where values live" section.

tui-settings-where-heading = WHERE VALUES LIVE
tui-settings-where-tag = H log
tui-settings-where-overrides = overrides
tui-settings-where-overrides-value = 3 rows
tui-settings-where-defaults = defaults
tui-settings-where-defaults-value = 42 in code
tui-settings-where-last-commit = last commit
tui-settings-where-last-commit-value = 14:02
tui-settings-where-note = kept in the database
tui-settings-where-config-source = ledger.db · table
tui-settings-where-bootstrap = bootstrap 4 keys

## The Reset section.

tui-settings-reset-heading = RESET
tui-settings-reset-tag = deletes the row
tui-settings-reset-hint-r = r this setting
tui-settings-reset-hint-group = R whole group

## The Settings list section.

tui-settings-list-heading = SETTINGS
tui-settings-list-heading-tag = 8
tui-settings-list-column-setting = SETTING
tui-settings-list-column-value = VALUE
tui-settings-list-overridden = 3 overridden

## Setting rows in the list.

tui-settings-setting-base-unit = base unit
tui-settings-setting-base-unit-value = AUD
tui-settings-setting-date-style = date style
tui-settings-setting-date-style-value = short
tui-settings-setting-negatives = negatives
tui-settings-setting-negatives-value = minus
tui-settings-setting-locale = locale
tui-settings-setting-week-starts = week starts
tui-settings-setting-week-starts-value = monday
tui-settings-setting-fiscal-year = fiscal year starts
tui-settings-setting-fiscal-year-value = 01 jul
tui-settings-setting-undo-depth = undo depth
tui-settings-setting-undo-depth-value = 50 commands
tui-settings-setting-number-format = number format
tui-settings-setting-number-format-value = (320 334.10)

## The "Selected" section.

tui-settings-selected-heading = SELECTED
tui-settings-selected-heading-tag = BASE UNIT
tui-settings-selected-explain = every total converts to this
tui-settings-selected-default = default
tui-settings-selected-default-value = AUD
tui-settings-selected-accepts = accepts
tui-settings-selected-accepts-value = active currency unit — 2 available
tui-settings-selected-changing = changing it
tui-settings-selected-changing-value = drives year-to-date and reports · re-converts every historical total

## The Settings table section.

tui-settings-table-heading = SETTINGS TABLE
tui-settings-table-heading-tag = 3 rows · 2 shown
tui-settings-table-column-key = KEY
tui-settings-table-column-value = VALUE
tui-settings-table-column-note = NOTE

## The Edit Setting popup.

tui-setting-edit-title = negatives
tui-setting-edit-command = general.negatives
tui-setting-edit-explain = how a negative amount prints everywhere
tui-setting-edit-value-label = value
tui-setting-edit-value-hint = ↔ choose · 3 options
tui-setting-edit-preview = preview
tui-setting-edit-preview-value = −320 334.10
tui-setting-edit-current = current
tui-setting-edit-current-value = (320 334.10) · DEFAULT
tui-setting-edit-applies-to = APPLIES TO
tui-setting-edit-applies-to-tag = EVERYWHERE AN AMOUNT PRINTS
tui-setting-edit-applies-to-net = net
tui-setting-edit-applies-to-net-value = −320 334.10
tui-setting-edit-applies-to-ledger = ledger row
tui-setting-edit-applies-to-ledger-value = Woolworths −184.20
tui-setting-edit-applies-to-export = csv export unaffected · machine format
tui-setting-edit-on-accept = on accept
tui-setting-edit-on-accept-value = upsert settings row
tui-setting-edit-on-accept-command = general.negatives = "minus"
tui-setting-edit-on-accept-hint = drop override, falling back to the shipped default
tui-setting-edit-typed-command = :set negatives minus
tui-setting-edit-help-tab = next field
tui-setting-edit-help-save = save
tui-setting-edit-help-cancel = cancel

## The Base Unit Guard popup.

tui-setting-guard-title = base unit
tui-setting-guard-command = general.base_unit
tui-setting-guard-current = current
tui-setting-guard-current-value = AUD
tui-setting-guard-proposed = proposed
tui-setting-guard-proposed-value = USD
tui-setting-guard-scope-heading = SCOPE OF CHANGE
tui-setting-guard-impact-accounts = accounts
tui-setting-guard-impact-accounts-tag = 42 in code
tui-setting-guard-impact-transactions = transactions
tui-setting-guard-impact-transactions-tag = 50 commands
tui-setting-guard-impact-prices = prices
tui-setting-guard-impact-prices-tag = 4
tui-setting-guard-prose = transactions are stored in their own units and are not touched. Every reported total is re-converted at the weekly USD close.
tui-setting-guard-help-tab = next field
tui-setting-guard-help-confirm = confirm
tui-setting-guard-help-cancel = cancel

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
