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
