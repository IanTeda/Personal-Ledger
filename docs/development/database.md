# Database

How the dev database is set up and seeded. The schema and the `mise run db` tasks are in CLAUDE.md's Setup section.

## The dev seed

Two tasks set up the dev database. `mise run db-fresh` drops it, creates it and runs the migrations, leaving no rows. `mise run db-seed` then adds to the migrated database without dropping it: it writes each persisted domain's stub seed through `lib-database`, so local runs have known data whose ids match the stub Ledger while the domains move to VIPER one by one. Run `db-fresh` then `db-seed` for a clean seeded database. Point a Client at the file (`[Personal-Ledger] file`) to see the rows.

The seed lives in `lib_database::seed::stub`, which calls each persisted domain's stub seed directly and writes it through that domain's adapter. Today that is Units, Price Sources and the base Unit, from `lib-units`' `InMemoryPorts::stub`, and the Transactions with their Splits and Tags. The Transactions are the stub Ledger's (`Ledger::stub_on`), not `lib_transactions::fake`'s generator alone, since the Bills and Documents stubs add the Transactions their Paid entries and Links name; `lib-database` takes them from `lib-ledger` until those domains persist (#774). Each later domain adds its seed to `seed::stub` as it migrates.

## One seed per domain

A module's stub seed is the one seed for its domain. A change to a persisted entity updates that seed, and `db-seed` follows; there is no second, hand-written copy of the rows in `lib-database`.

The dev seed writes the stub seed first, because the stub Accounts, Bill Plans and Budgets name stub Unit ids, then `seed::fake` adds generated Units and Transactions on top (25 of each from `db-seed`). The generators use the `fake` crate with a fixed RNG seed and seed-numbered ids clear of the stub's (Units from 100, past the stub's 1 to 3; Transactions from 100000), so a second run upserts the same rows. Only the prices' fetch times and the Transactions' dates follow the clock. A generated Transaction copies its Account and one Split's Category, Payee and Tags from a stub Transaction, so it names rows the stub Ledger has. Each later domain adds its generator beside `fake_units` and `fake_transactions`.

Every write is an upsert, so running the seed twice leaves the same rows. The base Unit is the exception: `db-seed` sets it only when the Ledger has none, so a base you changed in a Client survives a re-seed. Everything else the stub seeds (its Units, Price Sources and Transactions) goes back to the stub's values; rows you added are kept.

## Not the first-launch seed

`db-seed` is for development only. The Clients seed an empty Ledger themselves at start-up (`open_storage`), a separate halfway shim removed once every domain persists.

### The first-launch seed's order

`open_storage` seeds a new Ledger file in foreign-key order, so each domain's rows find the rows they name: the stub's Units, Price Sources and base Unit (`UnitsAdapter::seed`), then the Institutions, the Accounts, the Categories, the Tags and the Payees (`lib_institutions`', `lib_accounts`', `lib_categories`', `lib_tags`' and `lib_payees`' `fake` stubs), then the Transactions, Bill Plans and Bill Schedule dated from that first launch (`seed::stub_transactions`, `seed::stub_bills`, from `lib_transactions::fake::booked_stub_transactions` and `lib_bills`' own stub, not `lib-ledger`'s joined stub), then the Budgets (`seed::stub_budgets`), the Locations, Areas and Items (`lib_inventory`'s stub), and last the Document Types, Documents and Document Links (`seed::stub_documents`), each as of that day. `db-seed` writes them in the same order.

A Ledger file that already exists isn't given the stub. Two domains seed an empty table on open instead: an existing Ledger with no Budgets gains an empty default Budget, and one with no Document Types the nine default ones, with no Documents.
