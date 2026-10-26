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

`lib-transactions` is a ports-only VIPER module so far (`docs/development/viper-architecture.md`, [#776](https://github.com/IanTeda/Personal-Ledger/issues/776)): the `Transaction` and `Split` types, the repository port, its in-memory twin and the stub seed. It has no Interactors, Presenters or Router trait until the Transactions View migrates; the View and every writer of Transactions are still today's code.

- **`protocols.rs`** — `TransactionRepository`: every Transaction, one by id, `upsert_transaction` (the Transaction with its Splits, whole) and `delete_transaction`.
- **`fake.rs`** — `InMemoryTransactions`, and the stub seed (`fake::stub_transactions`, behind a `SeedNames` lookup, since `lib-categories`, `lib-payees` and `lib-tags` depend on `lib-transactions`). `lib_ledger::default_transactions` joins it to the other stubs.
- **`contract.rs`** (behind `contract-tests`) — the suite the fake and the adapter both pass: Splits and Tags round-trip, delete, a failed write.

## Persistence

- **Schema** — `transactions` (id, date, account, description, status, flagged), `splits` keyed `(transaction_id, position)` with ADR-0019's optional, unique `bill_schedule_entry_id`, and `split_tags` keyed `(transaction_id, position, tag_position)`. None has a foreign key to `accounts`, `categories`, `payees` or `tags`, which nothing persists yet; each table says why.
- **Adapter** — `lib-database`'s `adapter/transactions.rs`, `TransactionsAdapter`: rows in memory, writes through the shared batch, reload on failure (ADR-0046). A Transaction is saved whole, its Splits and Tags written afresh, and a rewrite keeps a Split's Match to a Bill Schedule entry, which is the Bills adapter's to write. It also answers `lib-bills`' `BillTransactions` read port (Match candidates, a Paid entry's figures).
- **Store** — `adapter/store.rs`' `TransactionsStore`, which the bins take, and `SharedTransactionsStore`, the handle `LedgerStore` and `lib-database`'s `BillPaymentsAdapter` both hold, so Pay it directly's Transaction joins the same commit as its entry.
- **Seeds** — the first launch and `mise run db-seed` write the stub Ledger's Transactions (`seed::stub_ledger_transactions`, the Bills' and Documents' history included), then `db-seed` adds generated ones (`seed::fake_transactions`).
- **Balances** — an Account's Balance is its starting balance plus its Splits, computed on read and never stored (`lib-database`'s `accounts/balance.rs`, summed in Rust since `Money` is stored as text).

## Desktop

- `src/view/transactions/` — today's Transactions View, reading the `Ledger` read copy of the Transactions, refreshed from the port after each commit and on rollback, newest first.
- `src/transactions/ports.rs` — the forwarding shim, `forward`: each `LedgerCommand`'s `UpsertTransaction` goes to the port in the same commit as the command. Writers carried: Import commit, Tag delete and merge, Category reassign. Account delete writes through `lib-account-deletion`'s port instead (#854), and `DeleteTransaction` left `LedgerChange` with it.
- `src/transactions/router.rs` — `show(cx, id)`, the entry point other modules' Routers call (the Bills Router's `show_transaction`). It emits today's `Reveal::Transaction(id)` until the View migrates.
- Each of these is a stand-in with its removal condition in the VIPER doc's [During the transition](viper-architecture.md#during-the-transition).

## Status workflow

Transaction Status (Open → Cleared → Reconciled) is a simple field, not a state machine enforced at the database level. The intended rule, that a Reconciled transaction's other fields (amount, category, payee, date, account) cannot change until its Status moves back to Open or Cleared, is not enforced yet: it becomes a Transactions Interactor rule when the View migrates.

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
- **CSV import:** parsing and inserting transactions from a CSV feed (TXN-010).
- **Bank-feed sync:** pulling transactions directly from a bank (future consideration, out of scope for V1).

## Related codebase areas

- `lib-core` — `TransactionId`, `TransactionStatus`, `Money`
- `lib-transactions` — the types, port, fake and contract suite
- `lib-database` — `adapter/transactions.rs` and the shared writer
- `lib-bill-payments` — Pay it directly, which writes a Transaction and the Bill Schedule entry it settles in one commit
- `bin-tui` — the Transactions View, on its fixture
- `bin-desktop` — the Transactions View on the read copy, and `src/transactions/`
