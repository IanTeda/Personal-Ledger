## Settings, Display: the date style control. The read-only Locale is on General.

desktop-display-date-style-label = Date format
desktop-display-date-style-default = Locale
desktop-display-date-style-short = Short
desktop-display-date-style-medium = Medium
desktop-display-date-style-long = Long
desktop-display-date-style-iso = ISO

desktop-settings-general-locale-label = Language and region
desktop-settings-general-locale-from-system = { $locale }, from your system
desktop-settings-general-locale-from-config = { $locale }, from config
desktop-settings-general-locale-from-default = { $locale }, the default
desktop-settings-general-locale-fallback = { $requested } is not supported, so { $locale } is used.
desktop-settings-general-locale-hint = Set with the locale setting or the --locale flag, then restart.

## Settings: the page, its index rail and the section headings. Terms such as Configuration,
## Preferences and Change Sets are the ones `GLOSSARY.md` defines.

## The index's two groups (ADR-0051): each heading, its page count and sync scope, and the line
## under it on the Medium index page.

desktop-settings-group-this-device = This device
desktop-settings-group-this-ledger = This ledger
desktop-settings-group-this-device-scope = { $count ->
    [one] { $count } page
   *[other] { $count } pages
} · never synced
desktop-settings-group-this-ledger-scope = { $count ->
    [one] { $count } page
   *[other] { $count } pages
} · synced with the Ledger
desktop-settings-group-this-device-description = Stays on this computer and never syncs. Applies whichever Ledger is open, or none.
desktop-settings-group-this-ledger-description = Saved in the Ledger and synced with it, so every device that opens it shares them.

# The page heading at Large and X-Large, beside the docked index: `Settings - Display`.
desktop-settings-page-heading = { $settings } - { $page }

## Settings at Medium (DESKTOP-119, DESKTOP-120): the index page's count and each page's one-line
## description, and the page picker's filter prompt.

desktop-settings-index-count = { $count ->
    [one] { $count } page
   *[other] { $count } pages
}
desktop-settings-description-general = on start, toasts, locale and the ledger folder
desktop-settings-description-display = date format, row density, status glyphs, colour theme
desktop-settings-description-ledger = ledger name, owner and start of the financial year
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
desktop-settings-section-ledger = Ledger
desktop-settings-section-units = Units
desktop-settings-section-institutions = Institutions
desktop-settings-section-inventory = Inventory
desktop-settings-section-sync-server = Sync server
desktop-settings-section-data-backup = Data & backup
desktop-settings-section-tracing = Tracing (Logs)
desktop-settings-section-about = About

## Each section's scope note. `$unit` and `$size` are the backup's Unit code and store size.

desktop-settings-scope-this-device = this device · never synced
desktop-settings-scope-ledger = preferences · synced · ledger identity
desktop-settings-scope-synced-change-sets = synced · change sets
desktop-settings-scope-ledger-data = ledger data · synced
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

## General (DESKTOP-178): On start, Start view, the read-only Ledger folder and the THIS DEVICE box, whose
## rows are the Ledger On start reopens and the config file Settings writes to.

desktop-settings-general-on-start = On start
desktop-settings-general-on-start-reopen = Reopen last Ledger
desktop-settings-general-on-start-none = Start with no Ledger open
desktop-settings-general-on-start-hint = Reopens the Ledger open when Personal Ledger last quit, or none if none was. A file named by --file or file in the configuration file always wins.
## Start view: the View the Desktop lands on as it starts. The nouns' own names are the rail's.
desktop-settings-general-start-view = Start view
desktop-settings-general-start-view-last = Last view
## `$view` is the start_view value the configuration file holds, which isn't one of the choices.
desktop-settings-general-start-view-unlisted = The configuration file names { $view }, which isn't one of these.
desktop-settings-general-start-view-hint = Where Personal Ledger opens, after On start. A View that needs a Ledger opens the Dashboard when none is open.
desktop-settings-general-ledger-folder = Ledger folder
desktop-settings-general-ledger-folder-hint = Where :open and :new start browsing. Set by data in the configuration file.
desktop-settings-general-this-device = This device
desktop-settings-general-last-ledger = last ledger
desktop-settings-general-configuration = configuration
desktop-settings-general-none = none
desktop-settings-general-note = General settings belong to this device and never sync. What travels with the Ledger is under Ledger.

## A device's settings in personal-ledger.conf (ADR-0053). `$key` is the setting's key in the file,
## such as on_start; `$source` is the environment variable or configuration file that sets it.
desktop-settings-device-shadowed-note = Set by { $source }
desktop-settings-device-shadowed = { $key } is set by { $source }, which outranks your personal-ledger.conf: change it there
desktop-settings-device-no-file = { $key } can't be saved: there's no home folder to keep personal-ledger.conf in
desktop-settings-device-saved = { $key } saved to { $path }
desktop-settings-device-write-failed = Couldn't save { $key } to personal-ledger.conf: the log says why

## Ledger. The summary labels name what the ledger holds.

desktop-settings-ledger-name = Ledger name
desktop-settings-ledger-owner = Owner
desktop-settings-ledger-financial-year = Financial year starts
desktop-settings-ledger-base-unit = Base unit
desktop-settings-ledger-base-unit-option = { $code } — { $name }
desktop-settings-ledger-summary-title = This ledger
desktop-settings-ledger-summary-accounts = accounts
desktop-settings-ledger-summary-transactions = transactions
desktop-settings-ledger-summary-units = units
desktop-settings-ledger-summary-institutions = institutions
desktop-settings-ledger-note = Ledger settings are ledger-scoped Preferences that travel with the ledger as Change Sets. What belongs to this device is under General and Display.

## Display. `$glyphs` is the run of status glyphs the option draws.

desktop-settings-display-row-density = Row density
desktop-settings-display-status-glyphs = Status glyphs
desktop-settings-display-start-minimised = Start Sidebar minimised
desktop-settings-display-preview = Preview
desktop-settings-display-note = These belong to this device and never sync: the Colour Theme, Appearance and date style are Client-scoped Preferences, the rest Configuration read from personal-ledger.conf. What travels with the Ledger is under Ledger.
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

