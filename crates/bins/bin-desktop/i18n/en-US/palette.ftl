## The command palette. Command names and syntax (`accounts new`) are stable English ids and are
## passed in; only the descriptions, the placeholder words and the hints are Messages.

## The palette's group headers for the commands with no noun of their own: the file-level ones and
## the Toast ones, and the Shell's own keys. Noun groups reuse the shared navigation Messages.

desktop-command-domain-ledger = Ledger
desktop-command-domain-toasts = Toasts
desktop-command-domain-navigation = Navigation

## Command descriptions, written in lower case as the palette shows them.

desktop-command-accounts-description = open Settings, Accounts
desktop-command-accounts-new-description = add an account: { $command } <account name>
desktop-command-accounts-edit-description = edit an account by name, or the selected row
desktop-command-accounts-delete-description = delete an account by name, or the selected row
desktop-command-categories-description = open Settings, Categories
desktop-command-open-description = open a ledger file
desktop-command-new-description = start a new ledger
desktop-command-close-description = close the open ledger
desktop-command-payees-description = open Settings, Payees
desktop-command-import-description = import a bank statement (sample statement for now)
desktop-command-settings-page-description = open a Settings page
desktop-command-tags-description = open Settings, Tags
desktop-command-units-description = open Settings, Units
desktop-command-institutions-description = open Settings, Institutions
desktop-command-dismiss-description = dismiss the newest Toast
desktop-command-toasts-on-description = show Toasts
desktop-command-toasts-off-description = send Toasts to the status line (Errors still show)
desktop-command-toasts-test-description = raise one Toast of each kind, a second apart

## The match count beside the input, for example `3 of 16`.

desktop-palette-match-count = { $matches } of { $total }

## The footer hint strip. `$key` is the key token.

desktop-palette-hint-select = { $previous }/{ $next } select
desktop-palette-hint-complete = { $key } complete
desktop-palette-hint-run = { $key } run
desktop-palette-hint-history = { $key } history
desktop-palette-hint-close = { $key } close

## `$theme` is a Colour Theme name, `$appearance` a Colour Appearance label.

desktop-command-colour-theme-description = use the { $theme } colour theme
desktop-command-colour-appearance-description = use the { $appearance } appearance
