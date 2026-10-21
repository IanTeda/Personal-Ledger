# Database

How the dev database is set up and seeded. The schema and the `mise run db` tasks are in CLAUDE.md's Setup section.

## The dev seed

`mise run db-seed` drops the dev database, runs the Client migrations and writes each persisted domain's stub seed through `lib-database`, so local runs have known data whose ids match the stub Ledger while the domains move to VIPER one by one. Point a Client at the file (`[Personal-Ledger] file`) to see the rows.

The seed lives in `lib_database::seed::stub`, which calls each persisted domain's stub seed directly and writes it through that domain's adapter. Today that is Units, Price Sources and the base Unit, from `lib-units`' `InMemoryPorts::stub`. Each later domain adds its seed to `seed::stub` as it migrates.

## One seed per domain

A module's stub seed is the one seed for its domain. A change to a persisted entity updates that seed, and `db-seed` follows; there is no second, hand-written copy of the rows in `lib-database`.

The dev seed uses the stub seed, not random `fake` data, because the stub Accounts, Bill Plans and Budgets name stub Unit ids. Bulk `fake` data waits until no domain reads the stub.

Every write is an upsert, so running the seed twice leaves the same rows.

## Not the first-launch seed

`db-seed` is for development only. The Clients seed an empty Ledger themselves at start-up (`open_storage`), a separate halfway shim removed once every domain persists.
