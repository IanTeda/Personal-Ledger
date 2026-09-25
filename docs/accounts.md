# Accounts

An account is a place where you hold money or track what you owe. Personal Ledger tracks four kinds: everyday transaction accounts (bank and cash), credit cards, loans, and investments. Each kind has its own fields and workflow because each does something different.

> **Heads up:** investment and loan accounts are in the design phase. For now, this page covers transaction and credit card accounts, which are built. Dedicated screens for loans and investments are still to come.

## What you can do

- Create transaction accounts (bank or cash) with starting balances
- Create credit card accounts with credit limits and interest rates
- List and filter accounts by kind and status
- Edit account details or deactivate accounts
- View running balances computed from transactions
- Move money between accounts

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

## Worked example

You open Personal Ledger on the 15th of September and decide to set up your accounts. You create a bank account named "Everyday" denominated in AUD with a starting balance of $2,500 (from your last bank statement). You create a credit card account named "Visa" with a $5,000 limit and 19.99% interest rate. Over the next week you record two transactions: a $80 coffee shop purchase against Visa, and a $120 paycheck deposit into Everyday. The Everyday balance now reads $2,620. When your credit card statement arrives on the 25th, you verify it against Personal Ledger by marking those transactions as reconciled, confirming everything matches reality.

## Screens

Personal Ledger shows accounts on a list with their balances, filterable by kind (Transaction, Credit Card, Loan, Investment) and active status. In the terminal app, press `g` then `a` to jump there. In the desktop app, use the menu or press `g` then `a`. The two apps show the same data but may have different layouts for editing and viewing details.

## Rules to know

- An account's Unit (currency) is set at creation and cannot be changed later.
- A transaction can only move between accounts sharing the same Unit.
- Deleting an account affects its transactions — see Transactions for how that works.

## Scope

- Cross-Unit transfers are not supported; see Transactions for details.
- Reports that aggregate accounts across different Units are future considerations — balance queries report each unit separately.
- Account reconciliation workflows (matching statements) are built into transaction status; see Transactions for full details.

## Related

- **Transactions:** move money within transaction and credit card accounts.
- **Categories:** used to classify each part of a transaction.
- **Payees:** optional on each transaction, tracking who the money went to or came from.
- **Tags:** extra labels for cross-cutting themes like trips or tax deductions.
- **Balance Checks:** formal assertions of what an account should total, checked against your recorded balance.

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Accounts | `g` `a` | `g` `a` |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Accounts. Ticked means built in at least one app; the tag says which.

- [x] ACC-001 (TUI, desktop): Create a transaction account (bank or cash) with name, unit, and starting balance
- [x] ACC-002 (TUI, desktop): Create a credit card account with name, unit, credit limit, and interest rate
- [x] ACC-003 (TUI, desktop): List accounts, filter by kind and active status, sort
- [x] ACC-004 (TUI, desktop): Edit an account's name, type, or active status
- [x] ACC-005 (TUI, desktop): Delete an account
- [x] ACC-006 (TUI, desktop): Move transactions between accounts (same unit only)
- [x] ACC-007 (TUI, desktop): View account balance computed from transactions
- [ ] ACC-008: Create a loan account with principal, term, rate history, and due date
- [ ] ACC-009: Create an investment account for a single holding
- [ ] ACC-010: Record loan repayments (principal and interest splits)
- [ ] ACC-011: Record investment trades (buys and sells)
- [ ] ACC-012: Dedicated loan account screens and views
- [ ] ACC-013: Dedicated investment account screens and views

## For developers

Curious how accounts are structured in the codebase, or planning to change them? See the [Accounts development documentation](development/accounts.md).
