## Shell chrome: the mode badge, status line, top bar and the no Ledger open content. Key tokens (`esc`, `g x`)
## and command names (`:open`) are passed in, never translated.

## The mode badge. Written in sentence case; the renderer upper-cases it.

desktop-mode-normal = Normal
desktop-mode-insert = Insert
desktop-mode-search = Search
desktop-mode-command = Command
desktop-mode-dialog = Dialog
desktop-mode-filter = Filter
desktop-mode-import = Import

## A page's own hint strip, beside its keys.

desktop-hint-row = row
desktop-hint-open = open
desktop-hint-edit = edit
desktop-hint-add = add
desktop-hint-filter = filter
desktop-hint-scroll = scroll
desktop-hint-index = index

## The right-hand side of the status line while a command surface is open. `$key` is the key token.

desktop-status-close-command-window = { $key } close command window
desktop-status-close-file-explorer = { $key } close file explorer
# The Data folder picker's keys (DESKTOP-187). `$folder` is the name of the folder Use picks, `$key` the key that cancels.
desktop-status-pick-folder = j/k move · l/enter into folder · h up · ctrl-enter use { $folder } · { $key } cancel

## Flashes on the status line. `$keys` is the typed chord, `$command` the command as typed (`:accounts
## delete`), `$name` what the user typed after it and `$matches` the names it matched.

desktop-status-not-a-jump = { $keys } is not a jump
desktop-status-command-not-yet-built = :{ $command } — not yet built
desktop-status-command-refused = :{ $command } — not available here right now

## With no Ledger open: the flash refusing anything that needs one, and the status line's right-hand
## side in place of the open Ledger's file. `$open` and `$new` are the command names as typed.

desktop-status-refused-no-ledger = No Ledger open: { $open } or { $new } first
desktop-status-no-ledger-file = No ledger open…
desktop-status-no-configuration-file = no configuration file

## Why start-up or `:open` couldn't open a Ledger file, the `$reason` of the Toast saying so.

desktop-ledger-open-missing = no such file
desktop-ledger-open-not-a-file = not a file
desktop-ledger-open-empty = the file is empty
desktop-ledger-open-unreadable = it isn't a Ledger file, or can't be read
desktop-ledger-open-out-of-date = it was made by an earlier version and can't be upgraded
desktop-ledger-open-newer = it was made by a newer version; update Personal Ledger to open it
desktop-ledger-open-not-a-ledger = it's a database, but not a Ledger file
desktop-ledger-open-incomplete = it isn't a complete Ledger; create it again with :new

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

## The Dashboard with no Ledger open. `$open` and `$new` are the command names as typed, and the
## tags mark them as the clickable spans.

desktop-no-ledger-title = No ledger open
desktop-no-ledger-hint = Run <open>{ $open }</open> to open a ledger file, or <new>{ $new }</new> to start one.

## Toasts. The line above the stack counting Toasts held back out of view.

desktop-toast-more = +{ $count } more

## A write that didn't reach the Ledger file. The Units are put back as stored, or, when they
## can't be read back either, left as shown.

desktop-toast-write-failed-on-close = Last change to { $name } wasn't saved
desktop-toast-write-failed-units = Couldn't save the change to units, so they are back as last saved
desktop-toast-write-failed-units-unsaved = Couldn't save the change to units or reload them, so what you see isn't saved
desktop-toast-write-failed-institutions = Couldn't save the change to institutions, so they are back as last saved
desktop-toast-write-failed-institutions-unsaved = Couldn't save the change to institutions or reload them, so what you see isn't saved
desktop-toast-write-failed-accounts = Couldn't save the change to accounts, so they are back as last saved
desktop-toast-write-failed-accounts-unsaved = Couldn't save the change to accounts or reload them, so what you see isn't saved
desktop-toast-write-failed-categories = Couldn't save the change to categories, so they are back as last saved
desktop-toast-write-failed-categories-unsaved = Couldn't save the change to categories or reload them, so what you see isn't saved
desktop-toast-write-failed-tags = Couldn't save the change to tags, so they are back as last saved
desktop-toast-write-failed-tags-unsaved = Couldn't save the change to tags or reload them, so what you see isn't saved
desktop-toast-write-failed-payees = Couldn't save the change to payees, so they are back as last saved
desktop-toast-write-failed-payees-unsaved = Couldn't save the change to payees or reload them, so what you see isn't saved
desktop-toast-write-failed-transactions = Couldn't save the change to transactions, so they are back as last saved
desktop-toast-write-failed-transactions-unsaved = Couldn't save the change to transactions or reload them, so what you see isn't saved
desktop-toast-write-failed-bills = Couldn't save the change to bills, so they are back as last saved
desktop-toast-write-failed-bills-unsaved = Couldn't save the change to bills or reload them, so what you see isn't saved
desktop-toast-write-failed-budgets = Couldn't save the change to budgets, so they are back as last saved
desktop-toast-write-failed-budgets-unsaved = Couldn't save the change to budgets or reload them, so what you see isn't saved
desktop-toast-write-failed-inventory = Couldn't save the change to the inventory, so it is back as last saved
desktop-toast-write-failed-inventory-unsaved = Couldn't save the change to the inventory or reload it, so what you see isn't saved
desktop-toast-write-failed-documents = Couldn't save the change to documents, so they are back as last saved
desktop-toast-write-failed-documents-unsaved = Couldn't save the change to documents or reload them, so what you see isn't saved
