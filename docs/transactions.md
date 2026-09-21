# Transactions

A transaction is a record of money moving in or out of an account on a date. Personal Ledger uses them to track spending, income, and transfers. Every transaction can be split across several categories, so a shopping trip that mixed groceries with household goods stays split between those categories, just like real life.

> **Heads up:** reconciliation (matching transactions against statements) is still being built. For now, you can record transactions and mark their status as Open, Cleared, or Reconciled, but the formal reconciliation screen is coming later.

## Terminology

- **Split:** one line of a transaction. A transaction can have one or many splits, each with its own category, payee, and tags.
- **Status:** where a transaction sits in the reconciliation workflow — Open (just recorded), Cleared (seen on a statement), or Reconciled (formally checked off).
- **Flagged:** a marker you set on a transaction to flag it for follow-up or review, independent of its status.

## Transaction concept

A transaction moves money on a date against exactly one account. It holds one or more splits — if you split a $100 shopping trip between $60 groceries and $40 household goods, that's one transaction with two splits. Date and the account live on the transaction itself, shared by all its splits.

### The split — category, payee, tags

Every split has a category and an amount. It can also have an optional payee (who the money went to or came from) and any number of tags (extra labels like "tax deductible" or "japan trip").

### Status — Open, Cleared, Reconciled

A transaction starts as **Open** when you first record it. As you check it against bank statements or feeds, you can move it to **Cleared** (you've seen it on a statement but haven't formally reconciled), then **Reconciled** (checked off during a formal reconciliation — the most confirmed status). A Reconciled transaction's fields are locked until you move it back to Open or Cleared.

### Flagged — a marker for anything

The **Flagged** marker is independent of status. You can flag a transaction at any point in the reconciliation workflow to mark it for follow-up — an amount that looks wrong, a duplicate, anything that needs a second look. Flagging a transaction doesn't change its status or lock it.

## Approach

The typical workflow is:

1. Record transactions as they happen, or import them from a bank feed or CSV.
2. Spot-check them against statements as they arrive, marking them Cleared.
3. Reconcile formally during a review cycle — mark the transactions you've checked off as Reconciled.
4. Flag anything that needs follow-up.

## Related

- **Accounts:** transactions are posted against exactly one account, which they update.
- **Categories:** each split has one category.
- **Payees:** optionally on each split — who the money went to or came from.
- **Tags:** optional labels on each split — cross-cutting themes like trips or tax deductions.
- **Bills:** a recurring bill schedule entry can link to a transaction when paid.

## Getting around

Press `g` then `t` to jump to Transactions from anywhere. For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Transactions. Ticked means built.

- [x] TXN-001: Record a single-entry transaction with date, amount, category, account, and status
- [x] TXN-002: Split a transaction across multiple categories
- [x] TXN-003: Assign an optional payee to each split
- [x] TXN-004: Assign multiple tags to a transaction
- [x] TXN-005: Move a transaction between accounts (same currency only)
- [x] TXN-006: Filter and search transactions by date, account, category, payee, status, or tags
- [x] TXN-007: Set a Flagged marker, independent of status
- [x] TXN-008: Move a transaction through Open/Cleared/Reconciled workflow
- [ ] TXN-009: Reconciliation screen for formal reconciliation against statements
- [ ] TXN-010: Import transactions from CSV or bank feeds

## For developers

Curious how transactions are structured in the codebase, or planning to change them? See the [Transactions development documentation](development/transactions.md).
