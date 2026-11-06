# Creating a Ledger is the only way one starts

`open_storage` decided a Ledger was new when it had no base Unit, and wrote the stub seed and the Personal Institution into it. Starting a Client no longer creates a Ledger file, so the only path into that branch was `:new`, and once the stub seed goes (#817) the Personal Institution alone writes no base Unit, so "no base Unit" could no longer mark a Ledger as new.

We're splitting creating from opening. Creating a Ledger, the `lib-ledger-creation` workflow behind `:new` (#912), makes the file, migrates it and writes what every new Ledger starts with (the Personal Institution, the base Unit the person chooses in the Create ledger Dialog, an empty Personal spending Budget in that Unit and the default Document Types) in one transaction through its `LedgerCreation` port, removing the file if any of it fails. Opening a Ledger only opens: a migrated file with no base Unit is refused as an incomplete Ledger with an error Toast, never seeded or completed. A new Ledger is never recognised, because it is created. The backfills for Ledgers written before Budgets or Documents persisted stay in `open_storage`, since those are complete Ledgers missing a later domain, not new ones.

## Considered options

Keeping "no base Unit means new" and seeding the starting rows on open was rejected: opening would still create, so a file cut short or made by hand would silently become a Ledger with a base Unit nobody chose, and the base Unit, which fixes every Account's Unit, must be a conscious choice. The cost accepted is that a file left without a base Unit by an older build's interrupted first launch can no longer be opened; it is created again with `:new`.

## Amendment: opening refuses any file missing what `:new` writes (2026-11-07)

Settled in [#1071](https://github.com/IanTeda/Personal-Ledger/issues/1071), under the [Sample Ledger spec](https://github.com/IanTeda/Personal-Ledger/issues/1070).

Opening refuses any migrated file missing what `:new` writes: a file with no base Unit, no Budget or no Document Type is refused as an incomplete Ledger (`Error::IncompleteLedger`) before anything is written, and is left as it was, so `open_storage` writes nothing. The Personal Institution isn't checked, because a person may delete it. The backfill this decision kept for Ledgers written before Budgets or Documents persisted is removed: in concept those are only dev files, reset by `mise run db`. The cost accepted is that such a file no longer opens; it is created again with `:new`.
