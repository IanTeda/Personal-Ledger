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

## Units and price sources.

desktop-settings-units-add =
    .title = Add unit
    .submit = Add

desktop-settings-units-edit =
    .title = Edit unit — { $code }
    .submit = Save

desktop-settings-units-delete =
    .title = Delete unit — { $code }
    .submit = Delete unit

desktop-settings-price-sources-heading = Price sources
desktop-settings-units-code-placeholder = e.g. usd
desktop-settings-units-name-placeholder = e.g. US Dollar
desktop-settings-units-group-source = Price source
desktop-settings-units-field-source = Source
desktop-settings-units-source-help = From the Price Sources table. + on that table adds another.
desktop-settings-units-field-symbol = Symbol
desktop-settings-units-symbol-placeholder = e.g. bitcoin
desktop-settings-units-symbol-help = { $source }'s id for this unit.
desktop-settings-units-field-quote = Quoted in
desktop-settings-units-quote-base = { $code } (base unit)
desktop-settings-units-test-price = Test price
desktop-settings-units-column-flags = Flags
desktop-settings-units-column-source = Source
desktop-settings-units-column-last-updated = Last updated
desktop-settings-units-not-fetched-yet = Not fetched yet
desktop-settings-units-flag-base = base
desktop-settings-units-column-price-source = Price source
desktop-settings-units-legend-base = base unit
desktop-settings-units-menu-edit-unit = Edit unit…
desktop-settings-units-menu-delete-unit = Delete unit…
desktop-settings-units-menu-reactivate-unit = Reactivate unit
desktop-settings-units-menu-test-source = Test price source
desktop-settings-units-menu-edit-source = Edit price source…
desktop-settings-units-menu-delete-source = Delete price source…

desktop-settings-units-usage-notice = Used by { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
} · { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
}. Renaming is safe; changing the code rewrites references.

desktop-settings-units-delete-warning = This unit is referenced by { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
} and { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
}. Deleting it cannot be undone.

desktop-settings-units-delete-held = { $name } · { $quantity } held in { $name } account
desktop-settings-units-delete-confirm-label = Type { $code } to confirm
desktop-settings-units-delete-new-base-label = New base unit
desktop-settings-units-deactivate =
    .title = Deactivate unit — { $code }
    .submit = Deactivate unit

desktop-settings-units-in-use = { $code } is used by { $uses }, so it can't be deleted.
desktop-settings-units-in-use-accounts = { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}
desktop-settings-units-in-use-bill-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}
desktop-settings-units-in-use-budgets = { $count ->
    [one] { $count } budget
   *[other] { $count } budgets
}
desktop-settings-units-in-use-help = Deactivating it keeps it on those and hides it from the unit pickers. You can reactivate it later.
desktop-settings-units-in-use-base = It is also the base unit, so it stays active until another unit takes over as base on the General page.
desktop-settings-units-inactive = inactive
desktop-settings-units-delete-new-base-help = { $code } is the base unit. Choose the unit that takes over before it is deleted.

## The Units detail pane docked at X-Large.

desktop-settings-units-pane-unit = Unit
desktop-settings-units-pane-none = —
desktop-settings-units-pane-used-by = Used by
desktop-settings-units-pane-used-by-value = { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
} · { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
}
desktop-settings-units-pane-accounts = Accounts
desktop-settings-units-pane-accounts-total = Total
desktop-settings-units-pane-accounts-none = No accounts use this unit.
desktop-settings-units-pane-title = { $code } · { $name }
desktop-settings-units-pane-meta = { $kind } · valued in { $base }
desktop-settings-units-pane-edit-unit = Edit unit
desktop-settings-units-pane-delete-unit = Delete unit
desktop-settings-units-pane-reactivate-unit = Reactivate unit
desktop-settings-units-pane-test = Test
desktop-settings-units-pane-edit-source = Edit source
desktop-settings-units-pane-delete-source = Delete source
desktop-settings-units-pane-add-source = Add price source
desktop-settings-units-pane-url = URL
desktop-settings-units-pane-last-price = Last price
desktop-settings-units-pane-source-meta = { $origin } · prices { $kinds } · { $key }
desktop-settings-units-pane-origin-built-in = built-in
desktop-settings-units-pane-origin-custom = custom
desktop-settings-units-pane-every-kind = every kind
desktop-settings-units-pane-key-needed = key needed
desktop-settings-units-pane-key-not-needed = key not needed
desktop-settings-units-pane-source-custom = { $name } · custom
desktop-settings-units-pane-manual-note = Prices you enter by hand.
desktop-settings-units-pane-values = Values
desktop-settings-units-pane-values-none = No units use this source.
desktop-settings-units-pane-symbol = symbol
desktop-settings-units-pane-quoted-in = quoted in { $code }
desktop-settings-units-pane-values-note = Each unit sets its own symbol in Edit unit.
desktop-settings-units-pane-values-note-custom = Each unit sets its own symbol in Edit unit. Deleting this source moves these units to Manual entry.
desktop-settings-units-pane-activity = Activity
desktop-settings-units-pane-activity-note = This session's Units outcomes, newest first. Errors still toast.
desktop-settings-units-activity-saved = Saved unit { $code }
desktop-settings-units-activity-deleted = Deleted unit { $code }
desktop-settings-units-activity-deactivated = Deactivated unit { $code }
desktop-settings-units-activity-reactivated = Reactivated unit { $code }
desktop-settings-units-activity-source-saved = Saved price source { $name }
desktop-settings-units-activity-source-deleted = Deleted price source { $name }

## The Add, Edit and Delete price source Dialogs (16z·custom, 16z·edit, 16z·del).

desktop-settings-price-source-add =
    .title = Add price source
    .submit = Add price source

desktop-settings-price-source-edit =
    .title = Edit price source — { $name }
    .submit = Save

desktop-settings-price-source-delete =
    .title = Delete price source — { $name }
    .submit = Delete price source

desktop-settings-price-source-field-updates = Updates
desktop-settings-price-source-field-address = Address
desktop-settings-price-source-field-request = Request
desktop-settings-price-source-field-price-path = Price path
desktop-settings-price-source-field-auth = Authentication
desktop-settings-price-source-field-key-param = Key parameter
desktop-settings-price-source-field-api-key = API key
desktop-settings-price-source-name-placeholder = e.g. Alpha Vantage
desktop-settings-price-source-address-placeholder = https://
desktop-settings-price-source-request-placeholder = /query?symbol={ "{" }symbol{ "}" }
desktop-settings-price-source-price-path-placeholder = /price
desktop-settings-price-source-key-param-placeholder = apikey
desktop-settings-price-source-name-help = Shown in the Price Sources table and the unit picker.
desktop-settings-price-source-request-help = { "{" }symbol{ "}" } is the unit's symbol and { "{" }quote{ "}" } the unit it's quoted in.
desktop-settings-price-source-price-path-help = A JSON pointer to the price in the response.
desktop-settings-price-source-key-note = API keys aren't saved yet: storing them encrypted is still to be explored.
desktop-settings-price-source-updates-every-15-minutes = Every 15 min
desktop-settings-price-source-updates-hourly = Hourly
desktop-settings-price-source-updates-daily = Daily
desktop-settings-price-source-updates-manual-only = Manual only
desktop-settings-price-source-auth-none = None
desktop-settings-price-source-auth-api-key = API key
desktop-settings-price-source-test-with-symbol = Test with symbol
desktop-settings-price-source-test = Test price source
desktop-settings-price-source-test-not-built = Testing isn't built yet.
desktop-settings-price-source-built-in = { $name } · built-in
desktop-settings-price-source-built-in-help = Built-in sources update with the app, so their address isn't edited here.
desktop-settings-price-source-usage = { $count ->
    [0] No units use this source.
    [one] Values 1 unit: { $codes }.
   *[other] Values { $count } units: { $codes }.
}
desktop-settings-price-source-delete-warning = This price source values { $count ->
    [one] 1 unit
   *[other] { $count } units
}. Deleting it cannot be undone.
desktop-settings-price-source-delete-fallback = { $count ->
    [one] { $codes } falls back to Manual entry and keeps its last price.
   *[other] { $codes } fall back to Manual entry and keep their last prices.
}
desktop-settings-price-source-delete-confirm-label = Type { $name } to confirm

## Institutions. `$glyph` is the plus sign on the add button.

desktop-settings-institutions-add =
    .button = { $glyph } Add institution
    .title = Add institution
    .submit = Add institution

desktop-settings-institutions-column-account-types = Account type
desktop-settings-institutions-name-label = Institution name
desktop-settings-institutions-name-placeholder = e.g. Commonwealth Bank
desktop-settings-institutions-account-types = Account types
desktop-settings-institutions-default-unit = Default unit
desktop-settings-institutions-no-units = No units available
desktop-settings-account-type-savings = savings
desktop-settings-account-type-offset = offset

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

## The Accounts page's kicker over its tables, uppercased where drawn.

desktop-settings-accounts-kicker = by type
