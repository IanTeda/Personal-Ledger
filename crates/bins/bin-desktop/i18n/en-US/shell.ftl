## Shell chrome: the mode badge, status line, top bar and the empty state. Key tokens (`esc`, `g x`)
## and command names (`:open`) are passed in, never translated.

## The mode badge. Written in sentence case; the renderer upper-cases it.

desktop-mode-normal = Normal
desktop-mode-insert = Insert
desktop-mode-search = Search
desktop-mode-command = Command
desktop-mode-dialog = Dialog
desktop-mode-filter = Filter
desktop-mode-import = Import

## The shell-wide hint strip. Each label sits beside its key, which the caller styles.

desktop-hint-command = command
desktop-hint-search = search
desktop-hint-help = help
desktop-hint-toggle-sidebar = toggle sidebar

## A page's own hint strip, beside its keys.

desktop-hint-row = row
desktop-hint-open-ledger = open ledger
desktop-hint-open = open
desktop-hint-edit = edit
desktop-hint-delete = delete
desktop-hint-remove = remove
desktop-hint-merge = merge
desktop-hint-new = new
desktop-hint-add = add
desktop-hint-filter = filter
desktop-hint-next-field = next field
desktop-hint-apply = apply
desktop-hint-cancel = cancel
desktop-hint-reset = reset
desktop-hint-colour = colour
desktop-hint-choose = choose
desktop-hint-toggle-active = toggle active
desktop-hint-page = page
desktop-hint-field = field
desktop-hint-change = change
desktop-hint-toggle = toggle
desktop-hint-scroll = scroll
desktop-hint-index = index
desktop-hint-level = level
desktop-hint-clear-logs = clear logs
desktop-hint-pages = pages
desktop-hint-picker = picker
desktop-hint-settings = settings

## The right-hand side of the status line while a command surface is open. `$key` is the key token.

desktop-status-close-command-window = { $key } close command window
desktop-status-close-file-explorer = { $key } close file explorer

## Flashes on the status line. `$keys` is the typed chord, `$command` the command as typed (`:accounts
## delete`), `$name` what the user typed after it and `$matches` the names it matched.

desktop-status-not-a-jump = { $keys } is not a jump
desktop-status-no-accounts = { $command } — no accounts
desktop-status-no-account-named = { $command } — no account named "{ $name }"
desktop-status-account-ambiguous = { $command } — "{ $name }" matches { $matches }
desktop-status-command-not-yet-built = :{ $command } — not yet built
desktop-status-delete-children-first = delete or move its children first

## Status-line flashes for actions whose flow is not designed or wired yet.

desktop-status-open-transaction-not-yet-built = open transaction — not yet built
desktop-status-add-transaction-not-yet-built = add transaction — not yet built
desktop-status-edit-transaction-not-yet-built = edit transaction — not yet built
desktop-status-edit-institution-not-yet-built = edit institution — not yet built
desktop-status-delete-institution-not-yet-built = delete institution — not yet built
desktop-status-sync-now-not-implemented = sync now — not implemented
desktop-status-backup-now-not-implemented = backup now — not implemented
desktop-status-export-ledger-not-implemented = export ledger — not implemented

## The top bar. `$time` is the time of the last sync.

desktop-topbar-synced = Synced { $time }

## The top bar's Undo button and its list of recent actions, newest first. The actions are
## representative until the Ledger keeps an Undo history.

desktop-topbar-undo-recent = Recent actions
desktop-topbar-undo-action-categorise = Categorise 3 splits as Groceries
desktop-topbar-undo-action-add-transaction = Add transaction at Woolworths
desktop-topbar-undo-action-rename-payee = Rename payee Coles Express
desktop-topbar-undo-action-tag = Tag 2 splits Japan Trip 2026
desktop-topbar-undo-action-delete-transaction = Delete transaction at Bunnings
desktop-topbar-undo-action-import = Import statement.csv
desktop-status-undo-not-yet-built = undo — not yet built

## The cold-start empty state. `$open` and `$new` are the command names as typed, and the tags mark
## them as the clickable spans.

desktop-empty-state-title = No ledger open
desktop-empty-state-hint = Run <open>{ $open }</open> to load a ledger file, or <new>{ $new }</new> to start one.

## Toasts. The line above the stack counting Toasts held back out of view.

desktop-toast-more = +{ $count } more

## A write that didn't reach the Ledger file. The Units are put back as stored, or, when they
## can't be read back either, left as shown.

desktop-toast-write-failed-units = Couldn't save the change to units, so they are back as last saved
desktop-toast-write-failed-units-unsaved = Couldn't save the change to units or reload them, so what you see isn't saved
desktop-toast-write-failed-transactions = Couldn't save the change to transactions, so they are back as last saved
desktop-toast-write-failed-transactions-unsaved = Couldn't save the change to transactions or reload them, so what you see isn't saved
desktop-toast-write-failed-bills = Couldn't save the change to bills, so they are back as last saved
desktop-toast-write-failed-bills-unsaved = Couldn't save the change to bills or reload them, so what you see isn't saved
