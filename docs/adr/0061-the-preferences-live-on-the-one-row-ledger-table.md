# The Preferences live on the one-row `ledger` table

The [Settings › Ledger Preferences map](https://github.com/IanTeda/Personal-Ledger/issues/1340) made the Ledger name, the Owner and the Financial year start Preferences beside the base Unit (`GLOSSARY.md`), and [#1341](https://github.com/IanTeda/Personal-Ledger/issues/1341) decided where they are kept. ADR-0014 planned a typed singleton `preferences` table for Preferences, but it was never given a column: the base Unit moved to the one-row `ledger` table (ADR-0044), whose singleton is a constraint rather than a convention, and nothing reads `preferences`.

We keep every Preference on `ledger` and drop `preferences` and `lib_database::preferences`:

- **Columns.** `ledger` holds `base_unit_id`, `name TEXT NOT NULL CHECK (trim(name) <> '')`, `owner TEXT` (no Owner is `NULL`, never `''`) and `financial_year_start INTEGER NOT NULL CHECK (financial_year_start BETWEEN 1 AND 12)`, the month, since a Financial Year always starts on the 1st. None has an SQL default: every value comes from what creates the Ledger.
- **Two stores share the row.** A Ledger adapter (`adapter/ledger.rs`, its own store and `Domain::Ledger`) owns the name, the Owner and the Financial year start. The base Unit stays with the Units adapter, since the Units' rules (the base can't be deleted, the base select) need it beside the Units. Each adapter writes only its own columns.
- **Only creation inserts the row.** `LedgerCreationAdapter` inserts the whole row in its one transaction, and `seed::sample` inserts it with the Sample Ledger's Preferences when it is missing, keeping a row that exists. Every other write is an `UPDATE`, `SetBaseUnit` included. A file without the row is still refused as `IncompleteLedger`.
- **The Client works out the starting values.** `NewLedger` takes a `LedgerName`, an `Option<Owner>` and a `FinancialYearStart` (value types in `lib-core`). The Client prefills the name from the Ledger file's name without `.pldb` and the month from the Locale through `lib-locale`; the adapter derives nothing.
- **Older files are refused.** While the schema is in concept, `create_ledger_table.sql` is edited in place, so a Ledger file made before these columns fails its migration check and is created again with `:new`; nothing is backfilled.
- **The Settings ports.** `lib-settings`' `LedgerUnits` read becomes `LedgerReads`, adding the name, the Owner and the Financial year start, answered by `StoredSettingsReads` from the Units and Ledger stores. `LedgerPreferences` adds `set_name`, `set_owner` and `set_financial_year_start`, none refusable, since the value types have already checked the value. Each write is one commit through the Desktop's `send`, announcing `Domain::Ledger`, and one History entry.

## Considered options

- **Columns on `preferences`.** Rejected: its singleton is only a convention, its row needs a UUIDv7 made at first use, and it would split the four Preferences across two tables.
- **One store for the whole row, the base Unit included.** Rejected: the Units adapter would then read its own base through another store.
- **The three in the Units store.** Rejected: the Ledger name isn't Units data.
- **An `ALTER TABLE … ADD COLUMN` migration with defaults.** Rejected: it breaks the one-file-per-table convention while the schema is in concept, and needs SQL defaults no new Ledger should rely on.
- **The adapter deriving the name from its path.** Rejected: it hides a Preference's default in persistence, and the Dialog has to show the prefilled name anyway.
