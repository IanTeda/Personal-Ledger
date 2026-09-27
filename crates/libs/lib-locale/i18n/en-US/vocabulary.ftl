## Labels for enumerated domain values, shared by the desktop and the TUI. Each variant has one
## Message in fixed families, written in sentence case. The stored token (`as_str()`) is never
## shown.

account-kind-cash = Cash
account-kind-bank = Bank
account-kind-credit-card = Credit card
account-kind-investment = Investment
account-kind-loan = Loan

transaction-status-open = Open
transaction-status-cleared = Cleared
transaction-status-reconciled = Reconciled

transaction-flagged = Flagged

category-type-expense = Expense
category-type-income = Income

unit-kind-fiat = Fiat
unit-kind-crypto = Crypto
unit-kind-stock = Stock
unit-kind-precious-metal = Precious metal
unit-kind-other = Other

budget-period-weekly = Weekly
budget-period-monthly = Monthly
budget-period-quarterly = Quarterly
budget-period-yearly = Yearly

## The system-seeded placeholder Institution a Cash Account links to. The row stores a stable key,
## never this text, so each Client renders it in its own Locale.

institution-none = No institution

## Colour Theme names, shared by the desktop and the TUI. Brand names (Catppuccin, Gruvbox,
## Nord) are left untranslated.

colour-theme-modernist = Modernist
colour-theme-high-contrast = High Contrast
colour-theme-catppuccin = Catppuccin
colour-theme-gruvbox = Gruvbox
colour-theme-nord = Nord

colour-appearance-light = Light
colour-appearance-dark = Dark
colour-appearance-system = System

## The Colour Variant System resolved to. `$variant` is the Colour Appearance label for it.

colour-appearance-system-current = System (currently { $variant })
colour-appearance-system-undetected = System (not detected, using { $variant })
