# Account Kinds

Personal Ledger models everyday spending money, revolving card debt, money owed to a lender, and money held as an investment as four separate entities — Transaction Account, Credit Card Account, Loan Account, and Investment Account — rather than one generic "account" with a type flag. Each has its own record shape, its own workflow, and its own view, even though the app still groups all four together under one "Accounts" area of navigation for browsing. This document explains why they're separate, what each one is, and how the mechanisms that move value between them (Transaction, Split, Trade, Repayment) fit together.

See `CONTEXT.md` for the one-paragraph canonical definition of each term; this document is where the fuller shape and relationships live.

## Why four entities, not one

Before this design, Personal Ledger modelled Cash, Bank, Credit Card, Investment, and Loan as five variants of a single `Account` type, all sharing one table. That works as long as every variant needs the same data — a name, a Unit, a starting balance. It stops working once a variant needs data the others don't:

- A Loan Account needs principal, an interest rate history, a term, and a next-due-date — none of which mean anything for a plain Bank Account.
- An Investment Account needs a quantity that only ever changes through a Trade, never a plain Transaction — a Bank Account's balance changes through Transactions and nothing else.
- A Credit Card Account needs a credit limit and an interest rate history too — revolving debt, not a simple cash balance — even though, unlike a Loan, it still accepts Transactions directly the same way a plain Bank Account does.

Rather than bolting loan-, investment-, and credit-card-specific columns onto one shared table, each becomes its own entity, grouped under the umbrella term **Account** (see below). Loan Account and Investment Account leave the shared-Transactions machinery entirely; Credit Card Account keeps it (it still posts Transactions and computes a Balance the same way) but still gets its own entity because its extra data (credit limit, rate history, statement date) doesn't belong on a plain Transaction Account either.

## Institution

The financial institution — a bank, a broker, a lender — that an Account is held with. A first-class entity, not a free-text field.

Every Account, regardless of Account Kind, carries a **mandatory** link to exactly one Institution. This is deliberately strict, including for Cash: a Cash-type Transaction Account represents physical, on-hand cash, which has no real-world institution — so it links to a system-seeded placeholder Institution (e.g. representing "no institution") rather than leaving the field empty. Two alternatives were considered and rejected:

- Making the link optional (nullable) — rejected because every real Bank, Credit Card, Loan, and Investment record always has a genuine Institution, and an optional field invites inconsistent data for no principled reason.
- Forcing the user to pick a real Institution even for Cash — rejected as meaningless data entry.

Examples: "Commonwealth Bank", "Vanguard", "IG Markets", the system-seeded placeholder for Cash.

## Account and Account Kind

**Account** is the umbrella term: a place where value is held or owed. It is not itself a table with a type column — it's the shared concept behind four separate entities, each with its own shape. **Account Kind** is which of the four an Account is: Transaction Account, Credit Card Account, Loan Account, or Investment Account, decided when the Account is created.

Every Account, of any Kind, shares exactly two things: a Unit (fixed at creation) and a mandatory Institution link (see above). Everything else — whether it accepts Transactions directly, what "the current amount" is called, what extra fields it carries — is specific to its Kind.

There's a second, narrower "type" concept that's easy to confuse with Account Kind: **Transaction Account Type** (Cash or Bank) only classifies *within* Transaction Account — it says nothing about Credit Card, Loan, or Investment Accounts. The two are named differently (Account *Kind* vs. Transaction Account *Type*) specifically so a reader can tell which axis is meant without more context.

| Account Kind        | Accepts Transactions directly? | Has a Balance? | "Current amount" concept |
| -------------------- | ------------------------------- | --------------- | -------------------------- |
| Transaction Account  | Yes                              | Yes              | Balance                    |
| Credit Card Account  | Yes                              | Yes              | Balance                    |
| Loan Account         | No — Repayment only             | No               | Outstanding Principal       |
| Investment Account   | No — Trade only                 | No               | Quantity                    |

## Transaction Account

An everyday place where spending money is held. Denominated in exactly one Unit, fixed at creation. Its Balance is derived from the Transactions posted against it — not stored directly.

Fields:

- Name — a human-readable label (e.g. "Everyday Spending").
- Transaction Account Type — Cash or Bank.
- Unit — fixed at creation.
- Institution — mandatory (see above).
- Starting Balance — in the Transaction Account's Unit, combined with its Transactions to compute the current Balance.
- Active flag — soft-delete; an inactive Transaction Account isn't offered for new Transactions but its history remains valid.

Transaction Account Type stays a two-value field (Cash, Bank) rather than being inferred from whether the Institution is the Cash placeholder or a real one — keeping classification and Institution-linking independent, so one can change without silently implying the other.

## Credit Card Account

Revolving debt held with a lender. Unlike Loan Account, a Credit Card Account behaves like a Transaction Account for spending: it accepts Transactions directly (with Splits), and its Balance is computed from them the same way.

Fields:

- Name — a human-readable label (e.g. "Rewards Visa").
- Unit — fixed at creation.
- Institution — mandatory (see above), the card issuer.
- Starting Balance — same mechanics as Transaction Account.
- Credit Limit — the maximum balance the card allows.
- Interest Rate History — a list of (rate, effective-from date) pairs, the same shape as Loan Account's (see below) — card rates change too, so a single scalar rate field would be wrong from day one.
- Statement/Due Date — when the current statement is due.
- Active flag — soft-delete, same pattern as Transaction Account.

Paying down a Credit Card Account is an **ordinary Transaction**, not a Repayment: it's a same-Unit Transaction moving cash from a Transaction Account into the Credit Card Account, using the same "a Transaction may move to another Account sharing that same Unit" rule that already covers any same-Unit transfer (see `CONTEXT.md`, Unit). Repayment (see below) stays specific to Loan Account, precisely because a Loan Account — unlike Credit Card Account — can't accept a Transaction directly at all.

## Loan Account

An amount owed to a lender. A Loan Account is not a Transaction Account: it carries data a Transaction can't express, and its amount owing changes through a dedicated mechanism (Repayment), not through plain Transactions.

Fields:

- Name — a human-readable label (e.g. "Home Loan").
- Unit — fixed at creation, the currency the Loan is denominated in.
- Institution — mandatory (see above), the lender.
- Principal — the original amount borrowed.
- Interest Rate History — a list of (rate, effective-from date) pairs, not a single fixed rate. Many Australian loans (mortgages in particular) are variable-rate. The Loan's *current* rate is whichever entry has the latest effective-from date on or before today.
- Term — the Loan's original duration.
- Next Due Date — when the next Repayment is expected.
- Active flag — soft-delete, same pattern as Transaction Account.

A Loan Account's amount owing is its **Outstanding Principal**, not a Balance — Balance is specific to Transaction Account and Credit Card Account (computed from Transactions), and a Loan Account has neither Transactions posted against it nor a stored balance field. Outstanding Principal is derived from Principal minus the sum of all Repayments' principal Splits against this Loan.

Reporting concepts like an amortisation schedule or a payoff projection are future design work, out of scope for this document — they read from the Loan's Principal, Interest Rate History, and Repayment history, once that reporting is actually designed.

## Investment Account

A single holding or position — e.g. "10 shares of AAPL" — not a container of several holdings. An Investment Account is not a Transaction Account: its quantity changes only through a Trade, never through a plain Transaction.

Fields:

- Name — a human-readable label (e.g. "Apple Inc. — CommSec").
- Unit — fixed at creation, the security or instrument itself (e.g. AAPL, VAS, BTC), not the currency it's priced in.
- Institution — mandatory (see above), the broker or exchange it's held through.
- Quantity — derived from the sum of all Trades against this Investment Account (buys add, sells subtract), not stored directly.
- Active flag — soft-delete, same pattern as Transaction Account.

A "Holding" concept distinct from Investment Account — one Investment Account containing several tickers, the way a real brokerage account often works — was considered and rejected: an Investment Account *is* the holding, one entity per position. If a real brokerage account holding several securities needs modelling later, that's several Investment Accounts sharing one Institution, not one entity containing several holdings.

Market value, unrealised/realised capital gains, cost basis, and dividends are deliberately **not** modelled here. `docs/product-requirements.md` already marks capital-gains tracking as a future consideration with no requirements defined, and a prior requirement to store Unit prices (FR.3) was struck out because "no cross-Unit exchange exists to price" — valuation needs a pricing mechanism Personal Ledger doesn't have yet. Cost Basis and Dividend are less blocked (they don't need live market pricing) but are still reporting-feature detail, parked for a future design pass rather than guessed at here.

## Transaction and Split

A Transaction is a single-entry record of an amount moving against exactly one Transaction Account or Credit Card Account — the two Account Kinds that accept Transactions directly — on a date, carrying a Transaction Status and, independently, a Flagged marker. Personal Ledger is deliberately single-entry, not double-entry (see [ADR-0001](adr/0001-single-entry-not-double-entry.md)).

A Transaction is composed of one or more **Splits**. A Split is one line of a Transaction: an Amount and a Category, plus an optional Payee and any number of Tags. A Transaction's Splits' Amounts sum to its total. Date and the Transaction Account or Credit Card Account it's posted against live on the Transaction itself, shared by every Split within it.

This means a plain, single-Category Transaction — the common case — is just the one-Split case of the same structure, not a separate type from a multi-Category one. Nothing in the app needs to handle "a plain Transaction" and "a split Transaction" as two different shapes.

Example — a $2,000 home Loan Account repayment recorded through the Repayment mechanism below still uses Splits for its principal/interest breakdown:

| Split | Category       | Amount    |
| ----- | -------------- | --------- |
| 1     | Loan Interest  | $800.00   |
| 2     | Loan Principal | $1,200.00 |

## Trade

The mechanism for buying or selling an Investment Account. A Trade moves cash out of (or into) a Transaction Account, in its Unit, and changes an Investment Account's quantity, in its Unit.

This is structurally different from a Transaction/Split for two reasons:

1. **It crosses Account Kinds.** A Split moves value within one Account. A Trade moves value from a Transaction Account into an Investment Account (or back) — two different Account Kinds entirely.
2. **It's quantity- and price-aware.** A Split is just a Category and an Amount. A Trade needs a quantity of the security bought or sold and a price, which together determine the cash Amount on the Transaction Account side.

No exchange rate is involved, and this doesn't reopen the "Personal Ledger doesn't convert between Units in V1" rule (see `CONTEXT.md`, Unit): each side of a Trade is simply denominated in its own Unit already — the Transaction Account side in currency, the Investment Account side in the security itself. The Trade's price is what ties the two amounts together, not a currency-conversion exchange rate.

Example — buying 10 shares of AAPL at $150.00 each from a cash Transaction Account:

| Side        | Entity                            | Unit | Amount     |
| ----------- | ---------------------------------- | ---- | ---------- |
| Cash out    | Transaction Account (cash holding) | AUD  | -$1,500.00 |
| Quantity in | Investment Account (AAPL)          | AAPL | +10        |

Further Trade detail — fees, dividend reinvestment, partial fills — is future design work once Trade is actually built; this document covers only the shape agreed so far: a Transaction-Account-to-Investment-Account mechanism, distinct from Transaction/Split, quantity- and price-aware.

## Repayment

The mechanism for paying down a Loan Account — the symmetric counterpart to Trade, for Loan Account instead of Investment Account. A Repayment moves cash out of a Transaction Account and reduces the Loan Account's Outstanding Principal.

Like Trade, a Repayment crosses Account Kinds — Transaction Account into Loan Account — which a same-Account Transaction structurally can't do. Unlike Trade, a Repayment reuses the Split concept for its own breakdown: a Repayment carries Splits the same way a Transaction does, typically one Split categorised as principal and one as interest, summing to the repayment total (see the example table above).

Repayment is specific to Loan Account. Paying down a Credit Card Account does **not** use Repayment — it uses an ordinary Transaction instead, because Credit Card Account, unlike Loan Account, already accepts Transactions directly (see Credit Card Account, above).

## What's deliberately out of scope here

This document captures the entity and mechanism *shape* agreed so far — it is not a functional spec, and `docs/product-requirements.md` still owns actual requirements (Loan and Investment tracking remain "future consideration" there; nothing here moves them into an active cycle). Left for later, once that work is actually scoped:

- Amortisation schedules and payoff projections for Loan Account, and the equivalent for Credit Card Account.
- Cost basis, realised/unrealised capital gains, market value, and dividends for Investment Account — all blocked on a pricing mechanism Personal Ledger doesn't have yet.
- Trade's exact fields (fees, partial fills, dividend reinvestment).
- Any dedicated screens/views for Loan Account, Investment Account, or Credit Card Account in the TUI or desktop app.
