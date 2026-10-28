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
tui-settings-scope-sync-server = this device · synced · every 30 seconds
tui-settings-scope-backup = this device · local files · { $size }
tui-settings-scope-history = all devices
# `$capacity` is a formatted count.
tui-settings-scope-logs = this device · diagnostic · last { $capacity } entries
tui-settings-scope-about = version info
# A list page's note with its count, derived from its rows on each draw.
tui-settings-scope-units = ledger data · synced · { $count ->
    [one] { $count } unit
   *[other] { $count } units
}
# `$net_worth` is formatted, in `$base`, the base Unit's code.
tui-settings-scope-accounts = ledger data · synced · { $count ->
    [one] { $count } account
   *[other] { $count } accounts
} · net worth { $net_worth } { $base }

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
# A list page's keys.
tui-settings-hint-row = row
tui-settings-hint-add = add
tui-settings-hint-delete = delete
tui-settings-hint-open-ledger = open ledger
tui-settings-hint-open-units-view = open units view
tui-settings-hint-open-accounts-view = open accounts view

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
tui-settings-status-glyphs-ascii = ascii
tui-settings-hint-bar-shown = [x] show key hints
tui-settings-hint-bar-hidden = [ ] show key hints
# The note on a Configuration row, which saves to personal-ledger.conf.
tui-settings-config-file-note = config file
# The note on a Configuration row a higher source sets. `$source` is an environment variable's name
# or a file's path.
tui-settings-config-shadowed-note = set by { $source }
# With no user config directory, nothing is written.
tui-settings-display-note-config = Config file rows are read from personal-ledger.conf at start-up; change them there.
# `$path` is the user's personal-ledger.conf.
tui-settings-display-note-config-writes = Config file rows save to { $path }.
tui-settings-display-note-scope = ● rows sync with the ledger. The locale changes only in the file, then a restart.

## Why a field can't be set. `$field` is the name `:set` takes.

tui-settings-read-only-locale = { $field } is read-only: change it in personal-ledger.conf, then restart
tui-settings-read-only-config = { $field } is in personal-ledger.conf, and there's no user config directory to write it to
# `$source` is an environment variable's name or a file's path.
tui-settings-set-shadowed = { $field } is set by { $source }, which outranks your personal-ledger.conf: change it there

## `:set` refusals and the line echoing an open field as the command. `$set` is `:set` and
## `$fields` the field names it takes: command syntax, so never translated.

tui-settings-set-needs-field = { $set } needs a field: { $fields }
tui-settings-set-unknown-field = { $field } isn't a Settings field: { $set } takes { $fields }
tui-settings-set-bad-value = { $value } isn't a value { $field } takes
tui-settings-set-echo = — the same edit as a command

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
# A Configuration row saved. `$path` is the file written.
tui-settings-config-saved = { $field } saved to { $path }
# `$error` is the operating system's reason.
tui-settings-config-write-failed = Couldn't save { $field }: { $error }

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

## A list page's group label, `$label` the group's and `$count` its rows.

tui-settings-list-group = { $label } · { $count }

## Settings › Units (TUI-033): each block's heading and add key's action, and the notes on the
## selected row. `$code` is a Unit code.

tui-settings-units-heading = units
tui-settings-units-add-unit = add unit
tui-settings-units-add-source = add source
tui-settings-units-note-usage = { $code } is used by { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
} and { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
}. Renaming it is safe.
tui-settings-units-note-unused = No account holds { $code } yet. Renaming it is safe.
tui-settings-units-note-code = Changing its code changes every reference. Deleting asks you to type the code.

## Settings › Accounts (TUI-041). `$amount` is a holding's formatted quantity of its Unit.

tui-settings-accounts-heading = by type
tui-settings-accounts-add = add account
tui-settings-accounts-name = Name
tui-settings-accounts-institution = Institution
tui-settings-accounts-unit = Unit
tui-settings-accounts-balance = Balance
tui-settings-accounts-quantity = { $amount } u
tui-settings-accounts-note-form = Add and edit use the account form from the Accounts view.
tui-settings-accounts-note-net-worth = Unit balances convert to { $base } for net worth at the latest weekly close.

## Settings › the ledger-data list pages after Units and Accounts (TUI-040, TUI-042 to TUI-046).

tui-settings-sorted-a-z = a–z
# The `selected` box's title, naming the row.
tui-settings-selected = selected · { $name }
tui-settings-hint-remove = remove
tui-settings-hint-merge = merge
tui-settings-hint-move = move
tui-settings-hint-move-area = move area
tui-settings-hint-expand-collapse = expand/collapse
tui-settings-hint-add-sub = add sub
tui-settings-hint-add-location = location
tui-settings-hint-add-area = area
tui-settings-hint-open-categories-view = open categories view
tui-settings-hint-open-tags-view = open tags view
tui-settings-hint-open-payees-view = open payees view

tui-settings-scope-institutions = ledger data · synced · { $count ->
    [one] { $count } institution
   *[other] { $count } institutions
}
tui-settings-scope-categories = ledger data · synced · { $count ->
    [one] { $count } category
   *[other] { $count } categories
} · { $levels ->
    [one] { $levels } level deep
   *[other] { $levels } levels deep
}
tui-settings-scope-tags = ledger data · synced · { $count ->
    [one] { $count } tag
   *[other] { $count } tags
} · { $duplicates ->
    [one] { $duplicates } likely duplicate
   *[other] { $duplicates } likely duplicates
}
tui-settings-scope-payees = ledger data · synced · { $count ->
    [one] { $count } payee
   *[other] { $count } payees
}
# `$typed` and `$total` are formatted counts of files.
tui-settings-scope-documents = ledger data · synced · { $count ->
    [one] { $count } type
   *[other] { $count } types
} · { $typed } of { $total } files typed
# `$items` is a formatted count.
tui-settings-scope-inventory = ledger data · synced · { $locations ->
    [one] { $locations } location
   *[other] { $locations } locations
} · { $areas ->
    [one] { $areas } area
   *[other] { $areas } areas
} · { $items } items

## Institutions

tui-settings-institutions-add = add institution
tui-settings-institutions-name = Institution
tui-settings-institutions-types = Account types
tui-settings-institutions-unit = Unit
tui-settings-institutions-accounts = Accounts
tui-settings-institutions-selected = selected · { $name }
tui-settings-institutions-accounts-label = accounts
tui-settings-institutions-types-label = account types
tui-settings-institutions-unit-label = default unit
tui-settings-institutions-unit-effect = {" "}— new accounts here start in { $unit }
tui-settings-institutions-no-accounts = no accounts yet
tui-settings-institutions-note-delete = Deleting an institution that still holds accounts asks you to move them first.

## Categories

tui-settings-categories-expense = expense
tui-settings-categories-income = income
tui-settings-categories-add = add category
tui-settings-categories-name = Category
tui-settings-categories-subcategories = { $count ->
    [one] { $count } subcategory
   *[other] { $count } subcategories
}
tui-settings-categories-note = Categories nest to any depth. Only leaf categories can be picked on a transaction. Spend and budgets live in Reports and Budgets, not here.

## Tags

tui-settings-tags-add = add tag
tui-settings-tags-name = Tag
tui-settings-tags-duplicate = ⚑ looks like a duplicate of "{ $of }"
tui-settings-tags-merge-title = merge { $from } → { $to }
tui-settings-tags-merge-label = merge
tui-settings-tags-merge-effect = Every transaction tagged { $from } is retagged { $to }, then { $from } is removed. Opens the merge form from the Tags view, and u undoes it.
tui-settings-tags-note = Usage counts and last-used dates live in the Tags view, not here.

## Payees

tui-settings-payees-add = add payee
tui-settings-payees-name = Name
tui-settings-payees-category = Category
tui-settings-payees-rules = Match rules
tui-settings-payees-no-rules = — no rules
tui-settings-payees-rules-label = match rules
tui-settings-payees-alias-label = also known as
tui-settings-payees-renamed = {" "}· renamed { $date }
tui-settings-payees-default-label = default category
tui-settings-payees-default-effect = {" "}— filled in on import, still editable
tui-settings-payees-note = Renaming keeps the old name as an alias, so past imports still match.

## Documents

tui-settings-documents-order = in type-filter order
tui-settings-documents-add = add type
tui-settings-documents-type = Type
tui-settings-documents-tracks = Tracks
tui-settings-documents-remind = Remind
tui-settings-documents-tax-year = Tax year
tui-settings-documents-files = Files
tui-settings-documents-none = —
tui-settings-documents-yes = yes
tui-settings-documents-no = no
tui-settings-documents-renews = renews
tui-settings-documents-ends = ends
tui-settings-documents-expires = expires
tui-settings-documents-lead-days = { $count ->
    [one] { $count } day before
   *[other] { $count } days before
}
tui-settings-documents-lead-months = { $count ->
    [one] { $count } month before
   *[other] { $count } months before
}
tui-settings-documents-note-tracks = Tracks: the date a document of this type carries (renews, ends, expires).
tui-settings-documents-note-remind = Remind: when that date raises an Event.
tui-settings-documents-note-tax-year = Tax year: files group by tax year.
tui-settings-documents-note-remove = Removing a type that still has files asks where to move them. { $untyped } untyped stay in the Inbox.

## Inventory

tui-settings-inventory-heading = locations
tui-settings-inventory-add-location = add location
tui-settings-inventory-add-area = add area
tui-settings-inventory-name = Location · Area
# `$items` is a formatted count.
tui-settings-inventory-counts = { $areas ->
    [one] { $areas } area
   *[other] { $areas } areas
} · { $items } items
tui-settings-inventory-note-register = Address and cover live on the register in Inventory, not here.
tui-settings-inventory-note-remove = Removing an area with items asks where to move them.

## Logs

tui-settings-logs-level = level
tui-settings-logs-level-error = error
tui-settings-logs-level-warn = warn
tui-settings-logs-level-info = info
tui-settings-logs-level-debug = debug
tui-settings-logs-clear = clear logs
tui-settings-logs-empty = No log entries yet.
tui-settings-logs-all-hidden = No entries at { $level } or above. Choose a lower level to see { $count ->
    [one] { $count } hidden entry
   *[other] { $count } hidden entries
}.
tui-settings-logs-clear-title = Clear logs
tui-settings-logs-clear-warning = Remove every captured log entry from this page? New entries keep arriving, and the console and log file keep their own copies.
tui-settings-logs-clear-confirm = clear
tui-settings-logs-clear-cancel = cancel
tui-settings-logs-cleared = Logs cleared
tui-settings-hint-level = level
tui-settings-hint-scroll = scroll
tui-settings-hint-newest-oldest = newest/oldest
tui-settings-hint-clear = clear
tui-settings-hint-copy-line = copy line

## About

tui-settings-about-description = A local-first personal ledger for expenses, investments and assets.
tui-settings-about-version = Personal Ledger v{ $version }
tui-settings-about-built-with = Built with Rust + ratatui + SQLite
tui-settings-about-author = Author
tui-settings-about-repository = Repository
tui-settings-about-documentation = Documentation
tui-settings-about-issues = Report an issue
tui-settings-about-license = License
tui-settings-about-license-text = Distributed under the GPL-3.0 License
tui-settings-about-copyright = © 2025–2026 { $author }. All rights reserved.
tui-settings-hint-link = link
tui-settings-hint-open-browser = open in browser
tui-settings-hint-copy-link = copy link

## Edit overlays (Sync Server, Set up Git backup)

tui-settings-overlay-hint-test = test
tui-settings-overlay-hint-next = next field
tui-settings-overlay-hint-save = save
tui-settings-overlay-hint-cancel = cancel
tui-settings-overlay-untested = untested
tui-settings-overlay-passed = test passed
tui-settings-overlay-failed = test failed
tui-settings-overlay-untested-note = Not tested yet. ctrl t tests these values.
tui-settings-hint-close-dialog = close dialog

## Sync Server

tui-settings-sync-url = server url
tui-settings-sync-status = status
tui-settings-sync-connected = connected
tui-settings-sync-last-sync = last sync
tui-settings-hint-sync-now = sync now
tui-settings-sync-edit-title = edit sync server
tui-settings-sync-url-note = https:// is required. Include the port if it isn't 443.
tui-settings-sync-token = access token
tui-settings-sync-token-note = From the server's admin page. Kept in the OS keychain, never synced.
tui-settings-sync-save-note = Save unlocks after a passing test. Saving reconnects and syncs.
tui-settings-sync-failed-https = can't test · not https
tui-settings-sync-failed-https-detail = The server URL must start with https:// and name a host.
tui-settings-sync-failed-token = can't test · no access token
tui-settings-sync-failed-token-detail = Paste the access token from the server's admin page.
tui-settings-sync-failed-refused = couldn't connect · connection refused
tui-settings-sync-failed-refused-detail = { $host } didn't answer. Check the address and port, and that the server is running.
tui-settings-sync-passed = connected · { $host }
tui-settings-sync-passed-detail = The server answered and accepted the token. Save to reconnect and sync.

## Backup

tui-settings-backup-local = local
tui-settings-backup-store = store location
tui-settings-backup-last-snapshot = last snapshot
tui-settings-backup-back-up-now = back up now
tui-settings-backup-export = export ledger (csv)
tui-settings-backup-git = git backup
tui-settings-backup-visibility = visibility
tui-settings-backup-private = private · checked { $time }
tui-settings-backup-contents = contents
tui-settings-backup-contents-value = ledger text dump · { $count ->
    [one] { $count } document
   *[other] { $count } documents
} via LFS
tui-settings-backup-schedule = schedule
tui-settings-backup-schedule-value = after changes · at most hourly
tui-settings-backup-push-now = push now
tui-settings-hint-back-up = back up
tui-settings-hint-export-csv = export csv
tui-settings-hint-edit-git-backup = edit git backup
tui-settings-hint-push = push

## Set up Git backup

tui-settings-git-setup-title = set up git backup
tui-settings-git-remote = remote
tui-settings-git-remote-note = Use a private repository. SSH (git@…) or HTTPS.
tui-settings-git-branch = branch
tui-settings-git-authentication = authentication
tui-settings-git-auth-ssh = SSH key
tui-settings-git-auth-token = Access token
tui-settings-git-ssh-key = SSH key
tui-settings-git-token = access token
tui-settings-git-schedule = push schedule
tui-settings-git-schedule-after-changes = after changes
tui-settings-git-schedule-hourly = hourly
tui-settings-git-schedule-daily = daily
tui-settings-git-schedule-manual = manual
tui-settings-git-schedule-note-after-changes = After changes waits for 5 minutes of quiet, then pushes at most hourly.
tui-settings-git-schedule-note-hourly = Pushes at most once an hour, when something changed.
tui-settings-git-schedule-note-daily = Pushes once a day, when something changed.
tui-settings-git-schedule-note-manual = Pushes only when you press P on the Backup page.
tui-settings-git-commit-message = commit message
tui-settings-git-commit-prefix = yyyy-mm-dd hh:mm ·
tui-settings-git-commit-default = ledger backup
tui-settings-git-commit-note = The push's date and time go in front automatically.
tui-settings-git-save = save & push
tui-settings-git-save-note = Saving makes the first push ({ $size }).
tui-settings-git-failed-remote = can't test · not a Git remote
tui-settings-git-failed-remote-detail = Use an SSH (git@…) or HTTPS address.
tui-settings-git-failed-branch = can't test · no branch
tui-settings-git-failed-branch-detail = Name the branch to push to, for example main.
tui-settings-git-failed-token = can't test · no access token
tui-settings-git-failed-token-detail = Paste an access token with push rights to the repository.
tui-settings-git-failed-unreachable = couldn't connect · the remote didn't answer
tui-settings-git-failed-unreachable-detail = { $remote } didn't answer. Check the address, and your key or token.
tui-settings-git-failed-public = refused · the repository is public
tui-settings-git-failed-public-detail = Git backup only pushes to a private repository. Make it private on the host, then test again.
tui-settings-git-failed-lfs = refused · no Git LFS
tui-settings-git-failed-lfs-detail = The remote doesn't support Git LFS, which Documents are pushed through. Turn LFS on, or use another host.
tui-settings-git-passed = ready · repository is empty
tui-settings-git-passed-detail = authenticated as { $user } · repository is private · supports LFS · dry-run push to { $branch } succeeded

# Settings › AI Connections (TUI-034). Status is a glyph and a word, never colour alone.
tui-settings-scope-ai-connections-counts = this device · { $providers ->
    [one] { $providers } provider
   *[other] { $providers } providers
} · { $assistants ->
    [one] { $assistants } assistant
   *[other] { $assistants } assistants
}
tui-settings-ai-providers = model providers
tui-settings-ai-add-provider = add provider
tui-settings-ai-provider = provider
tui-settings-ai-model = model
tui-settings-ai-key = key
tui-settings-ai-status = status
tui-settings-ai-default = default
tui-settings-ai-connected = ● connected
tui-settings-ai-key-rejected = ■ key rejected
tui-settings-ai-idle = ○ idle
tui-settings-ai-providers-note = Keys stay in the OS keychain and never sync. Only the transactions you select are sent.
tui-settings-ai-assistants = assistant access · mcp server
tui-settings-ai-add-assistant = add assistant
tui-settings-ai-assistant = assistant
tui-settings-ai-access = access
tui-settings-ai-last-seen = last seen
tui-settings-ai-server = server
tui-settings-ai-server-on = ● on
tui-settings-ai-read-only = read only
tui-settings-ai-read-write-not-offered = read & write · not yet offered
tui-settings-ai-approval = approval
tui-settings-ai-approval-value = ask before each change
# `$time` is a formatted time of day.
tui-settings-ai-seen-today = today { $time }
tui-settings-ai-server-note = The server listens on this device only. Revoking ends the session at once.
tui-settings-hint-ai-block = providers ↔ assistants
tui-settings-hint-test = test
tui-settings-hint-make-default = make default
tui-settings-hint-remove-revoke = remove · revoke

# Settings › AI Connections › Add / Edit model provider.
tui-settings-provider-add-title = add model provider
tui-settings-provider-edit-title = edit model provider
tui-settings-provider-provider = provider
tui-settings-provider-model = model
tui-settings-provider-key = api key
tui-settings-provider-key-note = Kept in the OS keychain. It never syncs and is never shown again.
tui-settings-provider-save-note = Save unlocks after a passing test. Only the transactions you select are sent.
tui-settings-provider-failed-model = can't test · no model
tui-settings-provider-failed-model-detail = Name the model to use, for example claude-sonnet-5-5.
tui-settings-provider-failed-key = can't test · no api key
tui-settings-provider-failed-key-detail = Paste the key from the provider's console.
tui-settings-provider-failed-rejected = refused · key rejected
tui-settings-provider-failed-rejected-detail = { $provider } didn't accept the key. Check it was copied whole, then test again.
tui-settings-provider-passed = connected · { $provider }
tui-settings-provider-passed-detail = The key was accepted and { $model } answered. Save to keep the key in the keychain.

# Settings › History (TUI-037). `$count` is the stack's size.
tui-settings-scope-history-changes = all devices · last 30 days · { $count ->
    [one] { $count } change
   *[other] { $count } changes
}
tui-settings-history-next-undo = next undo
tui-settings-history-area = area
tui-settings-history-area-all = all
tui-settings-history-area-transactions = txn
tui-settings-history-area-budgets = budget
tui-settings-history-area-settings = settings
tui-settings-history-kind-transactions = txn
tui-settings-history-kind-budgets = budgets
tui-settings-history-kind-tags = tags
tui-settings-history-kind-bills = bills
tui-settings-history-kind-inventory = inventory
tui-settings-history-kind-accounts = accounts
tui-settings-history-newest-first = newest first
tui-settings-history-time = time
tui-settings-history-change = change
tui-settings-history-device = device
tui-settings-history-this-device = this device
tui-settings-history-restore-to-here = enter restore to here
tui-settings-history-redo-to-here = enter redo to here
tui-settings-history-current-state = ▸ current state
# `$count` is how many undone changes can be redone.
tui-settings-history-redoable = { $count ->
    [one] { $count } change can be redone · a new edit clears it
   *[other] { $count } changes can be redone · a new edit clears them
}
tui-settings-history-synced-note = Synced changes undo on every device.
tui-settings-hint-undo = undo
tui-settings-hint-redo = redo
tui-settings-hint-restore = restore to here
tui-settings-hint-area-filter = area filter
