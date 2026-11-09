## The Settings index's detail pane (#1109): what each page is for, the two kinds of setting, and the
## label of each read-only value a page's summary quotes. A value that is a setting's own label
## (On start, Date format, Base unit, …) reuses the Message the page draws it with.

## What each page is for, in a sentence or two.

settings-purpose-general = How Personal Ledger starts on this computer and where it looks for Ledger files. Nothing here changes a Ledger.
settings-purpose-display = How Personal Ledger looks on this computer: the date format, row density, status glyphs and colour theme. Nothing here changes a Ledger.
settings-purpose-tracing = The log level and the live log of what Personal Ledger is doing on this computer, for when something goes wrong. Nothing here changes a Ledger.
settings-purpose-about = Which version of Personal Ledger this is, what it is built with, and its licences and credits.
settings-purpose-ledger = The Ledger's own identity: its name, owner, the month its financial year starts and its base unit. Every device that opens the Ledger shares them.
settings-purpose-units = The currencies and other units the Ledger counts in, their precision and price sources, and which one is the base unit.
settings-purpose-institutions = The banks, brokers and lenders that hold your accounts, each with the account types it offers and its usual unit.
settings-purpose-accounts = Every account in the Ledger with its institution, its unit and its balance, grouped by type.
settings-purpose-categories = The income and expense categories that transactions are sorted into, nested up to three levels deep.
settings-purpose-tags = Free labels you can put on any transaction, whatever its category. Merge or remove the likely duplicates here.
settings-purpose-payees = Who you pay and who pays you, with the default category and the match rules that file a transaction under each.
settings-purpose-documents = The document types the library files by, which of them track a date and remind you, and where the files are kept.
settings-purpose-inventory = The locations and areas you keep your belongings in, and the items in them, with their value and depreciation.
settings-purpose-sync-server = The Sync Server this Ledger syncs through: its address, whether it is connected and when it last synced.
settings-purpose-data-backup = Where the Ledger file lives on this computer, when it was last backed up, and exporting it as CSV.

## The two kinds of setting, one paragraph each.

settings-pane-this-device = This device settings live in personal-ledger.conf on this computer. Another computer keeps its own, and they work with no Ledger open.
settings-pane-this-ledger = This Ledger settings live in the Ledger file and sync with it. Change one here and every device that opens the Ledger sees it. They need a Ledger open.

## The labels of a summary's values.

settings-summary-locale = Locale
settings-summary-appearance = Appearance
settings-summary-log-level = Log level
settings-summary-log-entries = Entries kept
settings-summary-version = Version
settings-summary-built-with = Built with
settings-summary-units = Units
settings-summary-institutions = Institutions
settings-summary-accounts = Accounts
settings-summary-net-worth = Net worth
settings-summary-categories = Categories
settings-summary-levels-deep = Levels deep
settings-summary-tags = Tags
settings-summary-likely-duplicates = Likely duplicates
settings-summary-payees = Payees
settings-summary-without-default = Without a default category
settings-summary-document-types = Document types
settings-summary-files-typed = Files typed
settings-summary-locations = Locations
settings-summary-areas = Areas
settings-summary-items = Items
# `$typed` of `$all` Documents have a type.
settings-summary-typed-of = { $typed } of { $all }
# A figure with its Unit: `83,995.87 aud`.
settings-summary-amount = { $amount } { $code }
