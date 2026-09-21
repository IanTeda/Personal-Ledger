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

## Database schema

Transactions live in `lib-database` under a (placeholder names until schema is finalised):

- `transactions` table: id (RowID), account_id, date, status, flagged, created_at, updated_at
- `transaction_splits` table: id (RowID), transaction_id, category_id, amount, payee_id (nullable), created_at, updated_at
- `split_tags` table: id (RowID), split_id, tag_id, created_at, updated_at

The Balance of a Transaction Account or Credit Card Account is derived from all its Transactions' Splits' amounts summing to the account total, not stored directly — see `lib-database`'s `find.rs` / balance-calculation modules for how that's computed.

## CRUD structure

Following the workspace convention, transaction CRUD is split per-file in `lib-database/src/transactions/`:

- `model.rs` — Transaction and Split domain types
- `find.rs` — queries to retrieve transactions
- `insert.rs` — queries to create transactions and splits
- `update.rs` — queries to modify transactions (status, flagged, move to another account)
- `delete.rs` — queries to remove transactions
- `builder.rs` — helper for constructing transactions programmatically in tests

Splits are treated as part of the Transaction's internal shape, not exposed as a separate CRUD surface — a Transaction and its Splits are a logical unit, and updates to one Imply updates to the related Splits.

## Status workflow

Transaction Status (Open → Cleared → Reconciled) is a simple field, not a state machine enforced at the database level. The key constraint is in `update.rs`: a Reconciled transaction's other fields (amount, category, payee, date, account) cannot be modified until its Status is moved back to Open or Cleared first. This is enforced at the query level, not at the application boundary.

The Status change is always an explicit user action — the system never infers or auto-applies status changes (e.g. no automatic Cleared just because a Balance Check matched it).

## Flagged marker

Flagged is an independent boolean field on Transaction, separate from Status. A Transaction can be Flagged at any point in the reconciliation workflow. This is distinct from a Transaction "carrying a Bill" (a derived fact of a Bill Schedule link), which is a separate concept recorded in Bill Schedule, not on Transaction itself.

## Payee auto-creation

When a Split is created with a payee name that doesn't exist, the Payee is auto-created from the name (see [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md)). The insert flow handles this transparently — the name is resolved to or creates a Payee, and the Split links to its id. The TUI/Desktop app typically handles prompting the user for confirmation or offering a merge with an existing Payee before the insert happens.

## Splitting across accounts (same-Unit transfers)

A Transaction cannot move value between two accounts directly — a Split is always against the Transaction's Account. Transferring between two Accounts of the same Unit is an ordinary pair of Transactions: one posting a negative amount to the source account and a positive amount to the destination, with a pair of tags or a shared description so the user can see they're related.

Cross-Unit transfers are not possible in V1 (see `CONTEXT.md` — Unit, and `product-requirements.md` — Constraints).

## Moving a transaction to another account

The `update.rs` module includes logic to move a Transaction from one Account to another, but only if both Accounts share the same Unit. The move reverses the transaction's effect on the old Account's balance and reapplies it to the new one — both Balance calculations flow through the account's aggregate `find.rs` function, so no explicit balance-update step is needed.

## What's not yet built

- **Reconciliation screen:** the workflow for formally matching transactions against a statement and marking them Reconciled is future design work (TXN-009).
- **CSV import:** parsing and inserting transactions from a CSV feed (TXN-010).
- **Bank-feed sync:** pulling transactions directly from a bank (future consideration, out of scope for V1).

## Related codebase areas

- `lib-core` — `RowID`, domain types like `CategoryTypes`
- `lib-database` — Transaction and Split CRUD, Balance calculation
- `bin-tui` — TUI screens for recording and reviewing transactions
- `bin-desktop` — desktop app screens (under development)
