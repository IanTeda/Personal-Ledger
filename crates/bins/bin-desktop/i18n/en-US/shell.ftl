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
desktop-hint-toggle-active = toggle active

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
desktop-status-merge-tags-not-yet-built = merge tags — not yet built
desktop-status-test-price-source-not-yet-built = test price source — not yet built
desktop-status-edit-price-source-not-yet-built = edit price source — not yet built
desktop-status-delete-price-source-not-yet-built = delete price source — not yet built
desktop-status-add-price-source-not-yet-built = add price source — not yet built
desktop-status-edit-institution-not-yet-built = edit institution — not yet built
desktop-status-delete-institution-not-yet-built = delete institution — not yet built
desktop-status-sync-now-not-implemented = sync now — not implemented
desktop-status-backup-now-not-implemented = backup now — not implemented
desktop-status-export-ledger-not-implemented = export ledger — not implemented

## The top bar. `$time` is the time of the last sync.

desktop-topbar-synced = synced { $time }

## The cold-start empty state. `$open` and `$new` are the command names as typed, and the tags mark
## them as the clickable spans.

desktop-empty-state-title = No ledger open
desktop-empty-state-hint = Run <open>{ $open }</open> to load a ledger file, or <new>{ $new }</new> to start one.

## Toasts. The line above the stack counting Toasts held back out of view.

desktop-toast-more = +{ $count } more
