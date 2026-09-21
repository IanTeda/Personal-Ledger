## Shell chrome: the mode badge, status line, top bar and the empty state. Key tokens (`esc`, `g x`)
## and command names (`:open`) are passed in, never translated.

## The mode badge. Written in sentence case; the renderer upper-cases it.

desktop-mode-normal = Normal
desktop-mode-insert = Insert
desktop-mode-search = Search
desktop-mode-command = Command
desktop-mode-dialog = Dialog
desktop-mode-filter = Filter
desktop-mode-help = Help

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
desktop-hint-new = new
desktop-hint-add = add
desktop-hint-filter = filter
desktop-hint-next-field = next field
desktop-hint-apply = apply
desktop-hint-cancel = cancel
desktop-hint-reset = reset

## The right-hand side of the status line while a command surface is open. `$key` is the key token.

desktop-status-close-command-window = { $key } close command window
desktop-status-close-file-explorer = { $key } close file explorer

## Flashes on the status line. `$keys` is the typed chord, `$command` the command as typed (`:accounts
## delete`), `$name` what the user typed after it and `$matches` the names it matched.

desktop-status-not-a-jump = { $keys } is not a jump
desktop-status-no-accounts = { $command } — no accounts
desktop-status-no-account-named = { $command } — no account named "{ $name }"
desktop-status-account-ambiguous = { $command } — "{ $name }" matches { $matches }

## The top bar. `$time` is the time of the last sync.

desktop-topbar-synced = synced { $time }

## The cold-start empty state. `$open` and `$new` are the command names as typed, and the tags mark
## them as the clickable spans.

desktop-empty-state-title = No ledger open
desktop-empty-state-hint = Run <open>{ $open }</open> to load a ledger file, or <new>{ $new }</new> to start one.
