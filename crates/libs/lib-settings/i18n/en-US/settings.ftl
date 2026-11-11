## The Settings frame: its pages, the This device and This Ledger groups, the Medium index page and
## the frame's status-line hints. The pages' own Messages join as each page moves here.

## The groups' headings, their scope lines (a page count and whether they sync) and what each holds.

settings-group-this-device = This device
settings-group-this-ledger = This ledger
settings-group-this-device-scope = { $count ->
    [one] { $count } page
   *[other] { $count } pages
} · never synced
settings-group-this-ledger-scope = { $count ->
    [one] { $count } page
   *[other] { $count } pages
} · synced with the Ledger
settings-group-this-device-description = Stays on this computer and never syncs. Applies whichever Ledger is open, or none.
settings-group-this-ledger-description = Saved in the Ledger and synced with it, so every device that opens it shares them.

# The page heading at Large and X-Large, beside the docked index: `Settings - Display`.
settings-page-heading = { $settings } - { $page }

# The Medium index page's count beside its heading.
settings-index-count = { $count ->
    [one] { $count } page
   *[other] { $count } pages
}

## Each page's name, as the index, the picker and its heading show it.

settings-section-general = General
settings-section-display = Display
settings-section-ledger = Ledger
settings-section-units = Units
settings-section-institutions = Institutions
settings-section-accounts = Accounts
settings-section-categories = Categories
settings-section-tags = Tags
settings-section-payees = Payees
settings-section-documents = Documents
settings-section-inventory = Inventory
settings-section-sync-server = Sync server
settings-section-data-backup = Data & backup
settings-section-tracing = Tracing (Logs)
settings-section-about = About

## Each page's one-line description under its name on the Medium index page.

settings-description-general = on start, toasts, locale and the ledger folder
settings-description-display = date format, row density, status glyphs, colour theme
settings-description-ledger = ledger name, owner and start of the financial year
settings-description-units = currencies and other units, and the default
settings-description-institutions = banks, brokers and lenders
settings-description-accounts = every account, its institution and its unit
settings-description-categories = income and expense categories
settings-description-tags = free labels across transactions
settings-description-payees = who you pay and who pays you
settings-description-documents = document types and where files are kept
settings-description-inventory = item types and depreciation
settings-description-sync-server = server address, sign-in, interval
settings-description-data-backup = backups, export and the ledger file
settings-description-tracing = log level and the live log
settings-description-about = version, licences and credits

## Each page's scope note, right of its heading. `$unit` and `$size` are the backup's Unit code and
## store size, `$entries` how many log entries the capture keeps.

settings-scope-this-device = this device · never synced
settings-scope-ledger = preferences · synced · ledger identity
settings-scope-synced-change-sets = synced · change sets
settings-scope-ledger-data = ledger data · synced
settings-scope-sync-server = synced · every { $seconds ->
    [one] { $seconds } second
   *[other] { $seconds } seconds
}
settings-scope-data-backup = local files · { $unit } · { $size }
settings-scope-tracing = this device · diagnostic · last { $entries ->
    [one] { $entries } entry
   *[other] { $entries } entries
}
settings-scope-about = version info

## The frame's status-line hints: the index's, a page's at Medium, and the open picker's.

settings-hint-page = page
settings-hint-open = open
settings-hint-pages = pages
settings-hint-picker = picker
settings-hint-settings = settings
settings-hint-close = close

# Choosing a This Ledger page with no Ledger open. `$open` and `$new` are the commands to type.
settings-status-refused-no-ledger = No Ledger open: { $open } or { $new } first

## A This device page's status-line hints.

settings-hint-field = field
settings-hint-change = change
settings-hint-on-off = on / off
settings-hint-test = test
settings-hint-toggle = toggle
settings-hint-index = index
settings-hint-colour = colour
settings-hint-choose = choose

## A device's settings in personal-ledger.conf (ADR-0053). `$key` is the setting's key in the file,
## such as on_start; `$source` is the environment variable or configuration file that sets it, and
## `$path` the file a setting was saved to.

settings-device-shadowed-note = Set by { $source }
settings-device-shadowed = { $key } is set by { $source }, which outranks your personal-ledger.conf: change it there
settings-device-no-file = { $key } can't be saved: there's no home folder to keep personal-ledger.conf in
settings-device-saved = { $key } saved to { $path }
settings-device-write-failed = Couldn't save { $key } to personal-ledger.conf: the log says why

## General (DESKTOP-178): On start, Start view, Toasts, the Locale, the read-only Ledger folder, and
## the THIS DEVICE box, whose rows are the Ledger On start reopens and the config file Settings
## writes to. Toasts' own label, choices and note are `lib-locale`'s, shared with the TUI.

settings-general-on-start = On start
settings-general-on-start-reopen = Reopen last Ledger
settings-general-on-start-none = Start with no Ledger open
settings-general-on-start-hint = Reopens the Ledger open when Personal Ledger last quit, or none if none was. A file named by --file or file in the configuration file always wins.
settings-general-start-view = Start view
# `$view` is the start_view value the configuration file holds, which isn't one of the choices.
settings-general-start-view-unlisted = The configuration file names { $view }, which isn't one of these.
settings-general-start-view-hint = Where Personal Ledger opens, after On start. A View that needs a Ledger opens the Dashboard when none is open.
settings-general-locale-label = Language and region
settings-general-locale-hint = How dates, numbers and money read. System follows this computer. A change applies when Personal Ledger next starts.
# The Locale choices (DESKTOP-178). System follows the operating system; `$tag` is the Locale it resolves to now, such as en-AU.
settings-general-locale-system = System
settings-general-locale-system-now = now { $tag }
settings-general-locale-en-au = English (Australia)
settings-general-locale-en-gb = English (United Kingdom)
settings-general-locale-en-us = English (United States)
# A chosen Locale applies at the next start (DESKTOP-185). `$chosen` is the tag the file now names, `$effective` the tag in effect for this run.
settings-general-locale-pending = Saved. { $chosen } applies when Personal Ledger next starts; until then it reads { $effective }.
# A flag, an environment variable or a higher configuration file sets the Locale (DESKTOP-186). `$source` names it, such as --locale en-GB.
settings-general-locale-overridden = { $source } overrides this for this run, so Personal Ledger reads { $effective }. Your choice is kept for when it's gone.
settings-general-ledger-folder = Ledger folder
settings-general-ledger-folder-hint = Where :open and :new start browsing. Set by data in the configuration file.
settings-general-this-device = This device
settings-general-last-ledger = last ledger
settings-general-configuration = configuration
settings-general-none = none
settings-general-note = General settings belong to this device and never sync. What travels with the Ledger is under Ledger.

## Display: the date style, Row density, Status glyphs, Start Sidebar minimised, the PREVIEW and the
## Colour Theme group. `$glyphs` is the run of status glyphs the option draws.

settings-display-date-style-label = Date format
settings-display-date-style-default = Locale
settings-display-date-style-short = Short
settings-display-date-style-medium = Medium
settings-display-date-style-long = Long
settings-display-date-style-iso = ISO
settings-display-row-density = Row density
settings-display-density-compact = compact
settings-display-density-regular = regular
settings-display-density-roomy = roomy
settings-display-status-glyphs = Status glyphs
settings-display-glyphs-unicode = unicode — { $glyphs }
settings-display-glyphs-ascii = ascii fallback — { $glyphs }
settings-display-start-minimised = Start Sidebar minimised
settings-display-preview = Preview
settings-display-note = These belong to this device and never sync: each is saved to personal-ledger.conf, except Start Sidebar minimised, which the app remembers itself. What travels with the Ledger is under Ledger.
settings-display-colour-theme = Colour theme
settings-display-colour-appearance = Appearance
