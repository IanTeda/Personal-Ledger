## Settings, Display: the date style control and the read-only Locale.

desktop-display-date-style-label = Date format
desktop-display-date-style-default = Locale default
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
## Preferences and Change Sets are the ones `CONTEXT.md` defines.

desktop-settings-scope-preferences = preferences · synced
desktop-settings-index-footer = preferences sync · configuration local

desktop-settings-section-general = General
desktop-settings-section-display = Display
desktop-settings-section-units = Units
desktop-settings-section-institutions = Institutions
desktop-settings-section-sync-server = Sync server
desktop-settings-section-data-backup = Data & backup
desktop-settings-section-tracing = Tracing (Logs)
desktop-settings-section-about = About

## Each section's scope note. `$unit` and `$size` are the backup's Unit code and store size.

desktop-settings-scope-general = ledger identity
desktop-settings-scope-synced-change-sets = synced · change sets
desktop-settings-scope-display = client-scoped · never synced
desktop-settings-scope-sync-server = synced · every { $seconds ->
    [one] { $seconds } second
   *[other] { $seconds } seconds
}
desktop-settings-scope-data-backup = local files · { $unit } · { $size }
desktop-settings-scope-tracing = diagnostic · last { $entries ->
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
desktop-settings-density-compact = compact
desktop-settings-density-regular = regular
desktop-settings-density-roomy = roomy
desktop-settings-glyphs-unicode = unicode — { $glyphs }
desktop-settings-glyphs-ascii = ascii fallback — { $glyphs }

## Units and price sources. `$glyph` is the plus sign on an add button.

desktop-settings-units-add =
    .button = { $glyph } Add unit
    .title = Add unit
    .submit = Add

desktop-settings-units-edit =
    .title = Edit unit — { $code }
    .submit = Save

desktop-settings-units-delete =
    .title = Delete unit — { $code }
    .submit = Delete unit

desktop-settings-price-sources-heading = Price sources
desktop-settings-price-sources-add = { $glyph } Add price source
desktop-settings-units-code-placeholder = e.g. usd
desktop-settings-units-name-placeholder = e.g. US Dollar
desktop-settings-units-column-flags = Flags
desktop-settings-units-column-source = Source
desktop-settings-units-column-last-updated = Last updated
desktop-settings-units-flag-base = base
desktop-settings-units-flag-default = default
desktop-settings-units-test = test
desktop-settings-unit-kind-currency = currency
desktop-settings-unit-kind-cryptocurrency = cryptocurrency
desktop-settings-unit-kind-custom = custom

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

## About.

desktop-settings-about-version = Personal Ledger v{ $version }
desktop-settings-about-built-with = Built with Rust + GPUI + SQLite
