## The shell chrome: the status line, the mode name, and the footer hint bar. Key tokens come from
## the configured keybindings and command names are stable English ids, so both are passed in as
## arguments and are never translated.

## View titles the shared navigation layer has no Message for. Every other title the status line
## shows reuses a `nav-*` Message.

tui-view-balance-checks-title = Balance checks
tui-view-units-title = Units & prices

## The status line: the ledger glyph, the product name, and the active view. `$glyph` is the glyph,
## `$name` the product name and `$view` the active view's title. The second form adds the mode,
## shown only when it is not the resting NORMAL state; `$mode` arrives already upper-cased.

tui-status-line = { $glyph } { $name } | { $view }
tui-status-line-with-mode = { $glyph } { $name } | { $view } · { $mode }

## The mode names, written in sentence case; the renderer upper-cases them.

tui-mode-command = Command
tui-mode-insert = Insert
tui-mode-edit = Edit
tui-mode-confirm = Confirm

## The footer hint bar at rest. Each label sits beside its key, which the caller styles.

tui-hint-command = command
tui-hint-search = search
tui-hint-help = help

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
