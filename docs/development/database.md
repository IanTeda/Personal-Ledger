# Database

How the dev database is set up and seeded. The schema and the `mise run db` tasks are in CLAUDE.md's Setup section.

## The dev seed

Two tasks set up the dev database. `mise run db-fresh` drops it, creates it and runs the migrations, leaving no rows. `mise run db-seed` then adds to the migrated database without dropping it: it writes each persisted domain's stub seed through `lib-database`, so local runs have known data whose ids match the stub Ledger while the domains move to VIPER one by one. Run `db-fresh` then `db-seed` for a clean seeded database. Point a Client at the file (`[Personal-Ledger] file`) to see the rows.

The seed lives in `lib_database::seed::stub`, which calls each persisted domain's stub seed directly and writes it through that domain's adapter. Today that is Units, Price Sources and the base Unit, from `lib-units`' `InMemoryPorts::stub`. Each later domain adds its seed to `seed::stub` as it migrates.

## One seed per domain

A module's stub seed is the one seed for its domain. A change to a persisted entity updates that seed, and `db-seed` follows; there is no second, hand-written copy of the rows in `lib-database`.

The dev seed writes the stub seed first, because the stub Accounts, Bill Plans and Budgets name stub Unit ids, then `seed::fake` adds generated Units on top (25 from `db-seed`). The generator uses the `fake` crate with a fixed RNG seed and seed-numbered ids from 100 up, clear of the stub's 1 to 3, so a second run upserts the same rows. Only the prices' fetch times follow the clock. Each later domain adds its generator beside `fake_units`.

Every write is an upsert, so running the seed twice leaves the same rows.

## Not the first-launch seed

`db-seed` is for development only. The Clients seed an empty Ledger themselves at start-up (`open_storage`), a separate halfway shim removed once every domain persists.
