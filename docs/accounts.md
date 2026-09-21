# Accounts

An account is a place where you hold money or track what you owe. Personal Ledger tracks four kinds: everyday transaction accounts (bank and cash), credit cards, loans, and investments. Each kind has its own fields and workflow because each does something different.

> **Heads up:** investment and loan accounts are in the design phase. For now, this page covers transaction and credit card accounts, which are built. Dedicated screens for loans and investments are still to come.

## Terminology

- **Account Kind:** which type of account it is (Transaction, Credit Card, Loan, or Investment) — decided when you create it and can't be changed later.
- **Balance:** the current total in a transaction or credit card account, computed from all its transactions.
- **Unit:** the currency or security an account is denominated in (AUD, USD, AAPL shares, etc.) — fixed at creation.

## Account kinds

Personal Ledger splits accounts into four kinds, each with its own shape and workflow.

### Transaction Account

A place to hold everyday spending money — bank accounts and cash. Your Balance is computed from all transactions posted against it. You can record transactions, move money between accounts of the same currency, and track your spending against this account. Cash accounts link to a system placeholder for the institution; bank accounts link to a real bank.

### Credit Card Account

Revolving debt. Like a Transaction Account, you can record spending against it and see your Balance. But it also tracks your credit limit and interest rate. Paying down a credit card is a plain transaction from a bank account into it — the same mechanism you'd use for any same-currency transfer.

### Loan Account

An amount owed to a lender — a home loan, a car loan, or a personal loan. Unlike a Transaction Account, you don't record individual transactions against a Loan Account. Instead, you record Repayments (payments toward the loan), which split the amount you paid into principal (reducing what you owe) and interest (the lender's cut). A Loan Account tracks the original principal, an interest rate history (rates change), and your next due date.

### Investment Account

A single holding or position — "10 shares of AAPL" or "0.5 bitcoin". You don't record transactions against it. Instead, you record Trades (buys and sells), which move cash from a transaction account and change your quantity held. Each investment account is one position at one broker; if you hold several securities, that's several investment accounts.

## Approach

The typical workflow depends on which account kind you're working with:

**Transaction and Credit Card Accounts:**
1. Create the account and link it to your financial institution.
2. Set a starting balance (or import one from a bank statement).
3. Record transactions as they happen, or import them.
4. Review and mark them as cleared (seen on a statement) or reconciled (formally checked off).

**Loan Accounts (not yet built):**
1. Create the account and enter the original principal, term, and interest rate history.
2. Record Repayments as you pay them down, splitting them into principal and interest.
3. Review loan progress and any interest-rate changes.

**Investment Accounts (not yet built):**
1. Create the account for each holding (one per security, one per broker if you hold the same security in multiple places).
2. Record Trades as you buy or sell.
3. Track your quantity and cost basis.

## Related

- **Transactions:** move money within transaction and credit card accounts.
- **Categories:** used to classify each part of a transaction.
- **Payees:** optional on each transaction, tracking who the money went to or came from.
- **Tags:** extra labels for cross-cutting themes like trips or tax deductions.
- **Balance Checks:** formal assertions of what an account should total, checked against your recorded balance.

## Getting around

Press `g` then `a` to jump to Accounts from anywhere. For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Accounts. Ticked means built.

- [x] ACC-001: Create a transaction account (bank or cash) with name, unit, and starting balance
- [x] ACC-002: Create a credit card account with name, unit, credit limit, and interest rate
- [x] ACC-003: List accounts, filter by kind and active status, sort
- [x] ACC-004: Edit an account's name, type, or active status
- [x] ACC-005: Delete an account
- [x] ACC-006: Move transactions between accounts (same unit only)
- [x] ACC-007: View account balance computed from transactions
- [ ] ACC-008: Create a loan account with principal, term, rate history, and due date
- [ ] ACC-009: Create an investment account for a single holding
- [ ] ACC-010: Record loan repayments (principal and interest splits)
- [ ] ACC-011: Record investment trades (buys and sells)
- [ ] ACC-012: Dedicated loan account screens and views
- [ ] ACC-013: Dedicated investment account screens and views

## For developers

Curious how accounts are structured in the codebase, or planning to change them? See the [Accounts development documentation](development/accounts.md).
