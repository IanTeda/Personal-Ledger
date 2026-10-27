## The shell chrome: the status line, the mode name, and the footer hint bar. Key tokens come from
## the configured keybindings and command names are stable English ids, so both are passed in as
## arguments and are never translated.

## View titles the shared navigation layer has no Message for. Every other title the status line
## shows reuses a `nav-*` Message.

tui-view-balance-checks-title = Balance checks
tui-view-units-title = Units & prices

## The status line: the product name alone, or with the mode when it is not the resting NORMAL
## state. `$name` is the product name; `$mode` arrives already upper-cased.

tui-status-line-with-mode = { $name } · { $mode }

## The status line's right side once a Ledger is open: only the time of the last sync (`$time`).

tui-status-synced = Synced { $time }

## The mode names, written in sentence case; the renderer upper-cases them.

tui-mode-normal = Normal
tui-mode-command = Command
tui-mode-insert = Insert
tui-mode-edit = Edit
tui-mode-confirm = Confirm

## The footer hint bar at rest. Each label sits beside its key, which the caller styles.

tui-hint-command = command
tui-hint-search = search
tui-hint-help = help

## The footer's right-hand label while no Ledger is open. Once one is, the label is its Ledger
## file's name.

tui-footer-no-ledger = No Ledger open

## The Dashboard's place while no Ledger is open: two lines, centred. `$new` and `$open` are the
## `:new` and `:open` command names, never translated; the renderer bolds the tagged spans.

tui-empty-state-title = No Ledger open.
tui-empty-state-hint = Use <new>{ $new }</new> to create a Ledger or <open>{ $open }</open> to open one.

## Status-line messages from the Ledger file commands. `$open` and `$new` are the `:open` and `:new`
## command names, never translated, and `$path` the path as typed.

tui-status-no-ledger-open = No Ledger open: { $open } or { $new } first
tui-status-no-ledger-file = No Ledger file configured: { $open } needs a path
tui-status-new-needs-path = { $new } needs a path for the new Ledger file
tui-status-ledger-file-exists = { $path } already exists: { $open } it instead

## The reason `toast-ledger-open-failed` gives when `:open` names a path with no file there.

tui-ledger-file-missing = there's no file there

## The reason `toast-ledger-open-failed` gives when a file is there but won't open as a Ledger file.

tui-ledger-file-unreadable = it isn't a Ledger file, or can't be read

## The footer while an overlay is open: the same hints, plus one close hint for the overlay. `$key`
## is the configured back key and `$noun` the form's own noun.

tui-footer-close-command-window = { $key } close command window
tui-footer-close-form = { $key } close { $noun } form
tui-footer-close-dialog = { $key } close dialog

## The nouns the close-form hint takes.

tui-form-noun-unit = unit
tui-form-noun-category = category
tui-form-noun-edit = edit
tui-form-noun-account = account
tui-form-noun-tag = tag
tui-form-noun-payee = payee

## Flashes on the footer when a jump chord lands on a command with nothing behind it yet.
## `$command` is the command as typed, including its leading colon.

tui-footer-not-yet-built = { $command } — not yet built

## Toasts. The line above the stack counting Toasts held back out of view.

tui-toast-more = +{ $count } more

## A failed save of the Units, after which they are read back as stored.

tui-toast-write-failed-units = Couldn't save the change to units, so they are back as last saved
tui-toast-write-failed-units-unsaved = Couldn't save the change to units or reload them, so what you see isn't saved
