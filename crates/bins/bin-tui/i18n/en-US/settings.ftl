## The Groups section: heading and group names.

tui-settings-groups-heading = GROUPS

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

## Setting rows in the list.

tui-settings-setting-base-unit = base unit
tui-settings-setting-base-unit-value = AUD
tui-settings-setting-date-style = date style
tui-settings-setting-negatives = negatives
tui-settings-setting-locale = locale
tui-settings-setting-week-starts = week starts
tui-settings-setting-week-starts-value = monday
tui-settings-setting-fiscal-year = fiscal year starts
tui-settings-setting-fiscal-year-value = 01 jul
tui-settings-setting-undo-depth = undo depth
tui-settings-setting-undo-depth-value = 50 commands

## The "Selected" section.

tui-settings-selected-explain = every total converts to this

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
