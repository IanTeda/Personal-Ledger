# Transactions — Development

This document covers how transactions are structured in the codebase. Start with the [end-user documentation](../transactions.md) if you're unfamiliar with the domain.

## Overview

Personal Ledger transactions are single-entry records, not double-entry — a Transaction is one row linking to exactly one Account or Credit Card Account, on a date, with one or more Splits representing category breakdowns. This design is recorded in [ADR-0001](../adr/0001-single-entry-not-double-entry.md).

Each Transaction carries:
- A stable `RowID` (UUIDv7-based)
- A date
- A link to exactly one Account or Credit Card Account
- A Transaction Status (Open, Cleared, or Reconciled)
- A Flagged marker (independent of status)
- One or more Splits

Each Split carries:
- An amount
- A Category
- An optional Payee
- Zero or more Tags
- Shared transaction date and account from its Transaction

## The module

`lib-transactions` is a VIPER module for what the Desktop's Transactions View does today: list, filter and search ([#923](https://github.com/IanTeda/Personal-Ledger/issues/923), `docs/development/viper-architecture.md`). It has no Interactors, because no Client adds, edits or deletes a Transaction yet; the writers are cross-domain modules (`lib-bill-payments`, `lib-account-deletion`, `lib-transaction-import`, `lib-tag-removal`, `lib-category-deletion`) and the Categories handover. It is `gpui`-free and I/O-free and depends on no other feature crate.

- **`protocols.rs`**: `TransactionRepository` (every Transaction, one by id, `upsert_transaction` whole, `delete_transaction`), and the five read ports for other domains' rows: `TransactionAccounts` (id, name, Unit), `TransactionCategories` (id, name, parent and path, in tree order), `TransactionPayees`, `TransactionTags` and `TransactionUnits` (a Unit's code). Each port's methods carry a `transaction_` prefix, and only the fields the engine, rows and filter selects read. `TransactionsPorts` bundles them with the repository for the page, and returns owned rows since a shared store's `RefCell` can't lend a `&dyn` getter.
- **`query.rs`**: the query engine, moved from `lib-ledger`: `TransactionFilters` and its constructors (`defaults`, `for_account`, `for_category`, `for_budget_category`, `for_payee`, `for_tag`), `StatusFilter`, `query`, `Visible`, the running balance and the footer totals.
- **`presenter/`**: `TransactionsPagePresenter` (selection, Search, the applied filters, reveal and show-filtered, the count line, rows and footer) and `FilterPresenter`, the popover's draft, apply, reset, select stepping and chip clears. The page routes events to the filter while the popover is open, and `handle` returns a `FilterOutcome` (`Applied` or `Cancelled`) when it closes.
- **`keymap.rs`**: the page's keys on `lib-viper`'s shared command names, plus `transactions.filter` on `f`, and a second keymap, `TRANSACTIONS_FILTER_BINDINGS`, for the open filter popover's own `transactions.reset_filters` on `ctrl+r`, which the page Presenter holds and offers keys to through `filter_key`. `n`, `e` and `Enter` are bound and answer "not yet built". `/` is the Shell's: the page reads the Shell's Search chord for its legend and takes the Search through `start_search`.
- **`msg.rs`** and `i18n/`: the module's Messages.
- **`fake.rs`**: `InMemoryTransactions`, the in-memory read ports and the sample seed (`fake::sample_transactions`, behind a `SeedNames` lookup, since `lib-tags` depends on `lib-transactions`). `fake::bill_transactions` and `fake::document_transactions` (`fake/booked.rs`) are the rows the Bills and Documents seeds name by id: the Bills' payments and Match candidate, and the Documents' sample purchases. `lib_database::seed::sample_transactions` joins the generator to the other samples, and `lib_database::seed::sample_transactions` books the named rows after it from `FIRST_BOOKED`.
- **`contract.rs`** (behind `contract-tests`): the suite the fake and the adapter both pass: Splits and Tags round-trip, delete, a failed write, and the read ports through `ReadsFactory<Row>`, which `lib-database`'s `TransactionReadsFactory` implements once for all five adapters.

## Persistence

- **Schema** — `transactions` (id, date, account, description, status, flagged), `splits` keyed `(transaction_id, position)` with ADR-0019's optional, unique `bill_schedule_entry_id`, and `split_tags` keyed `(transaction_id, position, tag_position)`. None has a foreign key to `accounts`, `categories`, `payees` or `tags`, which nothing persists yet; each table says why.
- **Adapter** — `lib-database`'s `adapter/transactions.rs`, `TransactionsAdapter`: rows in memory, writes through the shared batch, reload on failure (ADR-0046). A Transaction is saved whole, its Splits and Tags written afresh, and a rewrite keeps a Split's Match to a Bill Schedule entry, which is the Bills adapter's to write. It also answers `lib-bills`' `BillTransactions` read port (Match candidates, a Paid entry's figures).
- **Store** — `adapter/store.rs`' `TransactionsStore`, which the bins take, and `SharedTransactionsStore`, the handle the Open Ledger and `lib-database`'s `BillPaymentsAdapter` both hold, so Pay it directly's Transaction joins the same commit as its entry.
- **Seeds** — the first launch and `mise run db-seed` write the sample Transactions (`seed::sample_transactions`, from `lib_transactions::fake::booked_sample_transactions`, the rows the Bills and Documents name included), then `db-seed` adds generated ones (`seed::fake_transactions`).
- **Balances** — an Account's Balance is its starting balance plus its Splits, computed on read and never stored (summed in Rust, since `Money` is stored as text).

## Desktop

- `src/transactions/view/`: the page's View. `page.rs` is the Entity that holds the page Presenter and takes keys, clicks and scroll; `header.rs`, `table.rs`, `footer.rs` and `filter_popover.rs` draw its view models, and `hints.rs` the status line. Glyphs and row density stay here, and `LedgerChanged` for Accounts, Categories, Payees, Tags, Transactions and Units repaints it.
- The page's ports are `lib-database`'s `StoredTransactionsPorts`, which `LedgerPorts` hands it ([#1076](https://github.com/IanTeda/Personal-Ledger/issues/1076)), answering from the Transactions store and, through `StoredTransactionReads`, the Accounts, Categories, Payees, Tags and Units stores. Every writer of Transactions is a cross-domain module's port over the store: Pay through `lib-bill-payments`', Account delete through `lib-account-deletion`'s (#854), the Import through `lib-transaction-import`'s ([#943](https://github.com/IanTeda/Personal-Ledger/issues/943)), Tag remove and merge through `lib-tag-removal`'s (#1003) and Category delete through `lib-category-deletion`'s (#1005); the Categories handover writes the store from the Categories ports. The forwarding shim went with slice 10's close-out ([#1006](https://github.com/IanTeda/Personal-Ledger/issues/1006)).
- `src/transactions/router.rs`: the module's entry point, `show(cx, id)` and `show_filtered(cx, filters)`. Every hand-off into the page calls one: the Bills Router's `show_transaction`, Documents' Transaction link, Accounts' `open_ledger`, Payees' and Tags' `Enter` (`for_payee`, `for_tag`) and the Budgets detail's "open in Transactions". `Reveal` still carries some of them, and the page turns it into Presenter events. The entry point finds the page through a gpui global installed in `Root::new`.
- `src/transaction_import/`: the Import step, the Transaction Import module's View ([#943](https://github.com/IanTeda/Personal-Ledger/issues/943)), which the root Router shows in place of the list on `:import`, the Noun staying Transactions. Its Entity holds `lib-transaction-import`'s Presenter while the step is open, over ports on the shared Payees, Transactions, Accounts and Categories stores and `lib-database`'s `TransactionImportAdapter`, so the step reads no read copy. A commit names the Transactions and the Payees it wrote, which the input door announces, and the step calls the Transactions entry point's `first_row`, putting the page back on its first row.
- Bills reads Transactions from the same store (`TransactionsStore::bill_splits_between`, `bill_paid_split`), and the Payees, Categories, Tags and Accounts counts are free functions over the rows the store holds (`category_splits`, `payee_splits`, [#1123](https://github.com/IanTeda/Personal-Ledger/issues/1123)). Budgets, Documents and the Dashboard read it through their own stores' reads too; no read copy of the Transactions is kept ([#1028](https://github.com/IanTeda/Personal-Ledger/issues/1028)), and every writer announces `Domain::Transactions`, which is what redraws the pages reading them.
- Each stand-in has its removal condition in the VIPER doc's [During the transition](viper-architecture.md#during-the-transition).

## Status workflow

Transaction Status (Open → Cleared → Reconciled) is a simple field, not a state machine enforced at the database level. The intended rule, that a Reconciled transaction's other fields (amount, category, payee, date, account) cannot change until its Status moves back to Open or Cleared, is not enforced yet: it becomes a Transactions Interactor rule when a form that edits a Transaction is built.

The Status change is always an explicit user action — the system never infers or auto-applies status changes (e.g. no automatic Cleared just because a Balance Check matched it).

## Flagged marker

Flagged is an independent boolean field on Transaction, separate from Status. A Transaction can be Flagged at any point in the reconciliation workflow. This is distinct from a Transaction "carrying a Bill" (a derived fact of a Bill Schedule link), which is a separate concept recorded in Bill Schedule, not on Transaction itself.

## Payee auto-creation

When a Split is created with a payee name that doesn't exist, the Payee is auto-created from the name (see [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md)). The insert flow handles this transparently — the name is resolved to or creates a Payee, and the Split links to its id. The TUI/Desktop app typically handles prompting the user for confirmation or offering a merge with an existing Payee before the insert happens.

## Splitting across accounts (same-Unit transfers)

A Transaction cannot move value between two accounts directly — a Split is always against the Transaction's Account. Transferring between two Accounts of the same Unit is an ordinary pair of Transactions: one posting a negative amount to the source account and a positive amount to the destination, with a pair of tags or a shared description so the user can see they're related.

Cross-Unit transfers are not possible in V1 (see `GLOSSARY.md` — Unit, and `product-requirements.md` — Constraints).

## Moving a transaction to another account

Moving a Transaction to another Account is allowed only when both Accounts share the same Unit. Balances are derived from the Splits, so a move needs no balance update. No Client offers the move yet.

## What's not yet built

- **Reconciliation screen:** the workflow for formally matching transactions against a statement and marking them Reconciled is future design work (TXN-009).
- **CSV import:** reading a statement from a CSV file (TXN-010). The Import step and its commit are built (`lib-transaction-import`), but its only `StatementSource` is the seeded sample statement.
- **Bank-feed sync:** pulling transactions directly from a bank (future consideration, out of scope for V1).

## Related codebase areas

- `lib-core` — `TransactionId`, `TransactionStatus`, `Money`
- `lib-transactions` — the types, ports, query engine, Presenters, keymap, fake and contract suite
- `lib-database` — `adapter/transactions.rs` and the shared writer
- `lib-bill-payments` — Pay it directly, which writes a Transaction and the Bill Schedule entry it settles in one commit
- `lib-transaction-import` — the Import: its rules, Presenter and atomic port, which writes an Import's new Payees and its Transactions in one commit
- `bin-tui` — the Transactions View, a wireframe on its fixture
- `bin-desktop` — `src/transactions/`, the module's View, ports and entry point, and `src/transaction_import/`, the Import step
