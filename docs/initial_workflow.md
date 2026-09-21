# Setting up your ledger

This page walks you through the first thing you do with Personal Ledger: turning an empty ledger into one that is ready for real transactions. It explains what to expect at each step, what you can change later and what you can't, and how to know when you're done. Plan on 20 to 30 minutes if you have your bank details to hand.

> **Heads up:** Personal Ledger is still being built, and there is no guided setup wizard yet. This page describes the setup order the app is designed around, using the screens that exist today. Where a step describes something still on its way, it says so.

## What to expect

Setting up comes down to seven steps, in this order:

1. Name your ledger.
2. Choose your units.
3. Add your institutions.
4. Add your accounts, each with an opening balance.
5. Set up your categories.
6. Bring in your recent history.
7. Check your balances against your statements.

The order matters because each step feeds the next. An account needs a unit and an institution before you can create it, and a transaction needs an account and a category before you can enter it. If you follow the list top to bottom, the app never asks you for something you haven't set up yet.

You can stop after any step and pick up later. Nothing is lost, and you can start entering transactions as soon as you have one account and a few categories.

## If you're coming from another program

If you've used KMyMoney, GnuCash or Microsoft Money, most of this will feel familiar. Those programs all open with a short setup: pick a currency, add your accounts with an opening balance, and choose a starting set of categories. Personal Ledger asks for the same things, with a few differences worth knowing:

- **No accounting jargon.** GnuCash starts you with a tree of assets, liabilities, equity, income and expenses, and gives your opening balances an "Opening Balances" equity account to balance against. Personal Ledger uses single-entry bookkeeping, so an account simply starts with a balance and you never deal with a balancing account.
- **Institutions are their own step.** Like Microsoft Money, Personal Ledger knows which bank or broker each account lives with. Unlike Money, it does not link to your bank or download transactions, so your data stays on your own machines.
- **Units instead of a single currency.** KMyMoney and Money ask for one base currency up front. Personal Ledger asks for a default unit, which can be a currency, but also a share ticker or a cryptocurrency, and each account has exactly one.
- **Categories start nearly empty.** Instead of choosing a large template (GnuCash's "Common Accounts", Money's Home or Business lists), you start with just Income and Expenses and grow the list as you go.

## Step 1: Name your ledger

Your ledger is everything you track in one place: your accounts, institutions, categories and transactions. Open **Settings** (press `g` then `s`) and choose **General**. Fill in:

- **Ledger name:** anything that helps you recognise it, such as "Household".
- **Owner:** your name.
- **Financial year starts:** the month your financial year begins. In Australia this is normally July. Reports use it to group your figures.
- **Base unit:** the unit most of your money is held in, such as AUD.

All of these can be changed later.

## Step 2: Choose your units

A unit is what an account's amount is measured in: dollars, shares, bitcoin. Every account has exactly one unit, and **it can't be changed once the account exists**, so it pays to get this right.

Open **Settings**, then **Units**. Check that your everyday currency is in the list and add anything else you need, such as a share ticker for a brokerage holding. Then mark your everyday currency as the default, so new accounts start with it already chosen.

If all your money is in one currency, you'll only ever need this step once.

## Step 3: Add your institutions

An institution is the bank, lender or broker an account is held with, such as "Commonwealth Bank" or "Vanguard". Every account needs one.

Open **Settings**, then **Institutions**, and add the ones you use. You don't need to add them all now. You can always add another when you create an account.

Physical cash has no bank, so you don't need to add anything for it. Personal Ledger includes a built-in "No institution" entry that cash accounts use.

## Step 4: Add your accounts

Open **Accounts** (press `g` then `a`) and choose **Add account**. Each account asks for:

- **Name:** what you'll call it, such as "Everyday Account".
- **Kind:** what sort of account it is (see below).
- **Institution and unit:** picked from the lists you built in steps 2 and 3.
- **Opening balance:** how much the account held the moment before your first transaction.
- **Account number:** optional.

### The four kinds of account

- **Transaction account:** where everyday money sits, either a bank account or cash. You enter transactions directly against it.
- **Credit card account:** revolving debt with a credit limit. You enter spending against it like any other account, and pay it off by moving money in from a transaction account.
- **Loan account:** money you owe a lender, such as a mortgage. You don't enter transactions against it. You record repayments, and the amount owing goes down.
- **Investment account:** a single holding, such as 10 shares of a fund. Its quantity changes only when you record a trade.

See [Account Kinds](accounts.md) for the full picture.

### Choosing an opening balance

Pick one start date for your ledger and use it for every account. The first day of a month or of your financial year works well, because statements line up with it. Then, for each account, take the balance from your statement on that date. Money owed on a credit card or loan counts as a negative amount.

Two things are fixed once the account exists: its **unit** and its **opening balance**. You can rename an account at any time, but if you get an opening balance wrong, the fix is to delete the account and add it again. It's much easier to check it before you save.

## Step 5: Set up your categories

Categories answer "where does my money go?". Open **Categories** (press `g` then `c`). You'll find two starting points already there, **Income** and **Expenses**, which can't be removed. Everything else you add sits under one of them.

Don't try to design the perfect list. A dozen categories is enough to begin with, for example Salary and Interest under Income, and Groceries, Rent, Utilities, Transport, Dining out and Health under Expenses. You can add more the first time a transaction doesn't fit, and you can nest them, so Utilities can later hold Electricity and Water.

See [Categories](categories.md) for how nesting and types work.

## Step 6: Bring in your recent history

Decide how much history you want, then enter it.

- **Start fresh (recommended).** Begin at the opening balance date from step 4 and enter transactions from there. This is the quickest way to a ledger you trust.
- **Back-fill a few months.** Set your opening balance date earlier, then enter the transactions since. Only do this if you want the older figures in your reports.

Open **Transactions** and add each one with its date, amount, account and category. You can add a payee (who you paid, or who paid you) and tags too. Neither needs setting up beforehand: a payee you haven't used before is created the first time you type it. See [Payees](payees.md) and [Tags](tags.md).

> **Heads up:** importing transactions from a bank file, such as CSV, QIF or OFX, is not available yet. When CSV import arrives, it will bring in only balance checks (a date and a balance). Transactions are entered by hand for now.

## Step 7: Check your balances

Once your transactions are in, compare each account with its latest bank or card statement. If the balance in Personal Ledger matches, the account is set up correctly.

If it doesn't, the difference is usually one of these:

- A wrong opening balance.
- A transaction that's missing, entered twice or dated on the wrong side of the cut-off.
- A transaction that hasn't cleared the bank yet.

Once an account matches, you can record a **balance check**: a note of what the balance should be on a given date, taken from your statement. Personal Ledger compares it with the balance it works out from your transactions and tells you if they disagree. It's a good habit to repeat each time a statement arrives.

## Optional: sync and back up

If you use Personal Ledger on more than one computer, you can connect them through your own sync server. See [Deployment](deployment.md) for how to run one, and **Settings**, then **Sync server**, to connect. Setting up sync is best done after your ledger is in good shape, so you copy a tidy ledger rather than a half-finished one.

**Settings**, then **Data & backup** is where your backup options live. Take a backup once your opening balances are in, so you always have a clean starting point to return to.

## Setup checklist

You're ready for everyday use when:

- [ ] Your ledger has a name, owner, financial year and base unit.
- [ ] Your units are set, with a default chosen.
- [ ] Your institutions are added.
- [ ] Every account you use is added, with the right kind, unit and opening balance.
- [ ] You have a starting set of categories.
- [ ] Your transactions are entered from your opening balance date.
- [ ] Each account matches its latest statement.

From here, see [Getting around](navigation.md) for the keyboard shortcuts that make daily entry quick.
