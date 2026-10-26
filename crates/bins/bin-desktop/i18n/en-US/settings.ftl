## Settings, Display: the date style control and the read-only Locale.

desktop-display-date-style-label = Date format
desktop-display-date-style-default = Locale
desktop-display-date-style-short = Short
desktop-display-date-style-medium = Medium
desktop-display-date-style-long = Long
desktop-display-date-style-iso = ISO

desktop-display-locale-label = Language and region
desktop-display-locale-from-system = { $locale }, from your system
desktop-display-locale-from-config = { $locale }, from config
desktop-display-locale-from-default = { $locale }, the default
desktop-display-locale-fallback = { $requested } is not supported, so { $locale } is used.
desktop-display-locale-hint = Set with the locale setting or the --locale flag, then restart.

## Settings: the page, its index rail and the section headings. Terms such as Configuration,
## Preferences and Change Sets are the ones `GLOSSARY.md` defines.

desktop-settings-index-footer = preferences sync · configuration local

# The page heading at Large and X-Large, beside the docked index: `Settings - Display`.
desktop-settings-page-heading = { $settings } - { $page }

## Settings at Medium (DESKTOP-119, DESKTOP-120): the index page's count and each page's one-line
## description, and the page picker's filter prompt.

desktop-settings-index-count = { $count ->
    [one] { $count } page
   *[other] { $count } pages
}
desktop-settings-description-general = ledger name, owner and start of the financial year
desktop-settings-description-display = date format, separators, row density, colour theme
desktop-settings-description-units = currencies and other units, and the default
desktop-settings-description-institutions = banks, brokers and lenders
desktop-settings-description-accounts = every account, its institution and its unit
desktop-settings-description-categories = income and expense categories
desktop-settings-description-tags = free labels across transactions
desktop-settings-description-payees = who you pay and who pays you
desktop-settings-description-documents = document types and where files are kept
desktop-settings-description-inventory = item types and depreciation
desktop-settings-description-sync-server = server address, sign-in, interval
desktop-settings-description-data-backup = backups, export and the ledger file
desktop-settings-description-tracing = log level and the live log
desktop-settings-description-about = version, licences and credits

desktop-settings-section-general = General
desktop-settings-section-display = Display
desktop-settings-section-units = Units
desktop-settings-section-institutions = Institutions
desktop-settings-section-inventory = Inventory
desktop-settings-section-sync-server = Sync server
desktop-settings-section-data-backup = Data & backup
desktop-settings-section-tracing = Tracing (Logs)
desktop-settings-section-about = About

## Each section's scope note. `$unit` and `$size` are the backup's Unit code and store size.

desktop-settings-scope-general = ledger identity
desktop-settings-scope-synced-change-sets = synced · change sets
desktop-settings-scope-ledger-data = ledger data · synced
desktop-settings-scope-display = client-scoped · never synced
desktop-settings-scope-sync-server = synced · every { $seconds ->
    [one] { $seconds } second
   *[other] { $seconds } seconds
}
desktop-settings-scope-data-backup = local files · { $unit } · { $size }
desktop-settings-scope-tracing = this device · diagnostic · last { $entries ->
    [one] { $entries } entry
   *[other] { $entries } entries
}
desktop-settings-scope-about = version info
desktop-settings-scope-units = { $count ->
    [one] { $count } unit
   *[other] { $count } units
} · synced
desktop-settings-scope-institutions = { $count ->
    [one] { $count } institution
   *[other] { $count } institutions
} · synced

## General. The summary labels name what the ledger holds.

desktop-settings-general-name = Ledger name
desktop-settings-general-owner = Owner
desktop-settings-general-financial-year = Financial year starts
desktop-settings-general-base-unit = Base unit
desktop-settings-general-base-unit-option = { $code } — { $name }
desktop-settings-general-summary-title = This ledger
desktop-settings-general-summary-accounts = accounts
desktop-settings-general-summary-transactions = transactions
desktop-settings-general-summary-units = units
desktop-settings-general-summary-institutions = institutions
desktop-settings-general-note = General settings are ledger-scoped Preferences — they travel with the ledger as Change Sets. Client-only Configuration lives under Display.

## Display. `$glyphs` is the run of status glyphs the option draws.

desktop-settings-display-row-density = Row density
desktop-settings-display-status-glyphs = Status glyphs
desktop-settings-display-start-minimised = Start Sidebar minimised
desktop-settings-display-preview = Preview
desktop-settings-display-note = Display settings are Configuration, not Preferences — they are read from personal-ledger.conf at start-up and written back here. Ledger-scoped Preferences (like the default Unit for new entries) live under Units and sync as Change Sets.
desktop-settings-display-colour-theme = Colour theme
desktop-settings-display-colour-appearance = Appearance

## `$roles` is the overridden Colour Role keys, comma-separated.

desktop-settings-display-colour-overrides = { $roles } overridden by [theme] in your configuration file. Restart to change them.

## A `[theme]` pair below its contrast rule. `$subject` and `$surface` are Colour Role or
## calculated colour keys; `$ratio` and `$required` are contrast ratios.

desktop-settings-display-colour-contrast = { $subject } on { $surface } is { $ratio }:1, needs { $required }:1
desktop-settings-density-compact = compact
desktop-settings-density-regular = regular
desktop-settings-density-roomy = roomy
desktop-settings-glyphs-unicode = unicode — { $glyphs }
desktop-settings-glyphs-ascii = ascii fallback — { $glyphs }

## Sync server, Data & backup and Tracing.

desktop-settings-sync-url = Server URL
desktop-settings-sync-last = Last sync
desktop-settings-sync-status = Status
desktop-settings-sync-connected = connected
desktop-settings-sync-now = Sync now
desktop-settings-backup-location = Store location
desktop-settings-backup-last = Last backup
desktop-settings-backup-now = Backup now
desktop-settings-backup-export = Export ledger (CSV)
desktop-settings-tracing-level-error = error
desktop-settings-tracing-level-warn = warn
desktop-settings-tracing-level-info = info
desktop-settings-tracing-level-debug = debug
desktop-settings-tracing-clear = Clear logs
desktop-settings-tracing-empty = No log entries yet.
desktop-settings-tracing-all-hidden = No entries at { $level } or above. Choose a lower level to see { $hidden ->
    [one] { $hidden } hidden entry
   *[other] { $hidden } hidden entries
}.
desktop-settings-tracing-clear-title = Clear logs
desktop-settings-tracing-clear-warning = Remove every captured log entry from this page? New entries keep arriving, and the console and log file keep their own copies.
desktop-settings-tracing-toast-cleared = Logs cleared

## About.

desktop-settings-about-version = Personal Ledger v{ $version }
desktop-settings-about-built-with = Built with Rust + GPUI + SQLite

## The pages that moved in from the primary rail, until each is rebuilt to the handoff.

desktop-settings-page-placeholder = Not yet rebuilt as a Settings page.

## The status-line legend's fold key on a tree page (Inventory).

desktop-hint-expand = expand
