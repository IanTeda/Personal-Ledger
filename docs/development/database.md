# Database

How the dev database is set up and seeded. The schema and the `mise run db` tasks are in CLAUDE.md's Setup section.

## The dev seed

Two tasks set up the dev database. `mise run db-fresh` drops it, creates it and runs the migrations, leaving no rows. `mise run db-seed` then adds to the migrated database without dropping it: it writes the Sample Ledger (`GLOSSARY.md`) through `lib-database`, then generated rows on top, so local runs have known data whose ids match the Sample Ledger the tests run on. Run `db-fresh` then `db-seed` for a clean seeded database. It holds everything `open_storage` asks of a Ledger (a base Unit, a Budget and the Document Types), though the shared dev DB also carries the Sync Server migrations, which a Client's migrator refuses. Point a Client at the file (`[Personal-Ledger] file`) to see the rows.

The Sample Ledger is a permanent fixture for developing and testing Personal Ledger, never offered to a person using it. It has one definition, `lib_database::seed::SampleLedger`: `NewLedger`'s rows (the Personal Institution, the base Unit, the empty Personal spending Budget and the default Document Types, as `:new` writes them) with each persisted domain's sample rows on top, every row dated from `today`, the Units' Weekly Prices included. Three entry points read it: `seed::sample` writes it to the dev DB for `db-seed`, `seed::sample_ledger(path, today)` creates a Ledger file through `LedgerCreationAdapter` and writes the sample on top, for tests that need a seeded file, and `FakeStores::sample(today)` holds the same rows in memory, for the tests' in-memory stores. A seed test checks that a `sample_ledger` file loaded back equals `FakeStores::sample` in every domain, so the three can't drift.

The Transactions are the joined sample's (`seed::sample_transactions`), not `lib_transactions::fake`'s generator alone, since the Bills and Documents samples add the Transactions their Paid entries and Links name. The joined sample, its Bills, Budgets and Documents mappings and the cross-domain seed tests live in `lib-database`'s `seed` (#1056).

## One seed per domain

A module's sample rows are the one sample for its domain, kept in its own `fake` module. A change to a persisted entity updates them, and the Sample Ledger and `db-seed` follow; there is no second, hand-written copy of the rows in `lib-database`.

The dev seed writes the Sample Ledger first, because the sample Accounts, Bill Plans and Budgets name sample Unit ids, then `seed::fake` adds generated Units and Transactions on top (25 of each from `db-seed`). The generators use the `fake` crate with a fixed RNG seed and seed-numbered ids clear of the sample's (Units from 100, past the sample's 1 to 3; Transactions from 100000), so a second run upserts the same rows. Only the generated prices' fetch times and the Transactions' dates follow the clock. A generated Transaction copies its Account and one Split's Category, Payee and Tags from a sample Transaction, so it names rows the Sample Ledger has. Each later domain adds its generator beside `fake_units` and `fake_transactions`.

Every write is an upsert, so running the seed twice leaves the same rows. The base Unit is the exception: `db-seed` sets it only when the Ledger has none, so a base you changed in a Client survives a re-seed. Everything else the Sample Ledger holds goes back to the sample's values; rows you added are kept.

## Not a new Ledger

`db-seed` is for development only. `open_storage` only opens a Ledger and never seeds one: a new Ledger is made by `lib-ledger-creation`'s `LedgerCreation` port (ADR-0054), which writes the Personal Institution, the base Unit, an empty Personal spending Budget and the default Document Types, and nothing else. `seed::sample_ledger(path, today)` creates a Ledger that way and writes the sample rows on top.

### The Sample Ledger's order

`seed::sample` and `seed::sample_ledger` write the Sample Ledger in foreign-key order, in one all-or-nothing transaction, so each domain's rows find the rows they name: the Units, Price Sources and base Unit (`UnitsAdapter::seed_in`), then the Institutions, the Accounts, the Categories, the Tags and the Payees (`lib_institutions`', `lib_accounts`', `lib_categories`', `lib_tags`' and `lib_payees`' `fake` samples, with `NewLedger`'s Personal Institution), then the Transactions, Bill Plans and Bill Schedule (`seed::sample_transactions`, `seed::sample_bills`, from `lib_transactions::fake::booked_stub_transactions` and `lib_bills`' own sample), then the Budgets (`seed::sample_budgets`, Personal spending being `NewLedger`'s with the sample's Accounts and Limits), the Locations, Areas and Items (`lib_inventory`'s sample), and last the Document Types (`NewLedger`'s) and the Documents and Document Links (`seed::sample_documents`), each as of `today`.

Opening a Ledger never gives it the sample, and completes no Ledger: `open_storage` writes nothing. A migrated file with no base Unit, no Budget or no Document Type is refused as an incomplete Ledger (`Error::IncompleteLedger`) and left as it was; it is created again with `:new` (ADR-0054). A Ledger with its Personal Institution deleted still opens.
