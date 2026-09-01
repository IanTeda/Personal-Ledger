# Personal Ledger

A self-hosted, single-user personal finance ledger: expenses, investments, and assets
tracked locally, without a cloud dependency or subscription.

> This file is the project's domain glossary — a single source of truth for what each
> term means, not a spec. Anyone or anything producing output that names a domain
> concept (an issue title, a refactor proposal, a test name) should use the term as
> defined here rather than drifting to a synonym listed under `_Avoid_`; if a needed
> concept isn't defined yet, that's a signal to either reconsider the invented language
> or flag a real gap (see `docs/agents/domain.md`). Hard-to-reverse decisions that come
> out of sharpening this vocabulary are recorded separately as ADRs in `docs/adr/`
> (e.g. [ADR-0001](docs/adr/0001-single-entry-not-double-entry.md)); product vision,
> scope, and requirements live in `docs/product-requirements.md`, not here.

## Language

**Unit**:
The fixed thing an Account's balance is denominated in and a Transaction's amount is
measured in — either fiat money (e.g. AUD) or a tradeable non-currency instrument (e.g.
a stock ticker like AAPL, or bitcoin). Every Account has exactly one Unit, fixed at
creation. Personal Ledger does not convert between Units in V1: an Account only ever
holds Transactions in its own Unit, and a Transaction may only move to another Account
sharing that same Unit (see `docs/product-requirements.md`, Constraints).
_Avoid_: Commodity, Currency, asset class, exchange rate, conversion.

**Ledger**:
The complete set of Accounts, Categories, and Transactions owned by one self-hoster —
a logical whole, physically replicated as a full local SQLite database on each Client
and reconciled between them by the Sync Server.
_Avoid_: Book, journal.

**Client**:
An app instance that holds its own full local copy of the Ledger and can create, read,
update, and delete against it entirely offline — Desktop, TUI, or Web App. The Web App
instance additionally plays the Sync Server role (see below); that is a role it takes
on, not a fourth, architecturally distinct component.
_Avoid_: Device, backend — "device" conflates the physical machine with the app
instance running on it; "backend" implied a single authoritative server, which no
longer describes the architecture.

**Sync Server**:
The role the Web App instance plays in reconciling each Client's local Ledger copy
with the others — pushing and pulling change sets, not exposing full CRUD. Not a
separate deployable component; it is what the Web App does in addition to being a
Client itself.
_Avoid_: Backend, server — "backend" is avoided across this glossary now that Clients
hold their own local data directly rather than depending on a central server for
reads/writes.

**Institution**:
The financial institution (e.g. a bank) that holds an Account — not modelled as an
entity in V1 (see `docs/product-requirements.md`, Future Considerations); Accounts are
tracked without a linked Institution for now.
_Avoid_: Company, organization.

**Account**:
A place where value is held or owed — Cash, Bank, Credit Card, Investment, or Loan —
denominated in exactly one Unit, whose Balance is derived from the Transactions
posted against it.
_Avoid_: Wallet. Not to be confused with Category: an Account is *where* money sits, a
Category is *what* it's classified as.

**Balance**:
An Account's current accumulated total, computed by summing the Transactions posted
against it, expressed in its Unit.
_Avoid_: Total, running balance.

**Balance Check**:
A point-in-time assertion of what an Account's Balance should be (e.g. from a bank
statement), entered manually or read from a Balance column when importing a CSV file,
checked against the Account's own Balance as computed from its Transactions.
_Avoid_: Balance — an Account's Balance is the computed total from its Transactions; a
Balance Check is a separate assertion compared against that total, not the total
itself.

**Category**:
A user-defined label (e.g. "Groceries", "Salary") used to classify a Transaction,
carrying exactly one Category Type.
_Avoid_: Tag, group.

**Category Type**:
One of the five fixed accounting classifications a Category carries: asset, liability,
equity, income, or expense.
_Avoid_: Account type.

**Payee**:
An optional free-text label on a Transaction naming the business, organisation, or
individual its money moved to or from (e.g. "Woolworths", an employer), recorded so
spending or income can be totalled by who it went to or came from. Not a separate
entity in V1 — matched by exact text only, with no normalisation (see
`docs/product-requirements.md`, Constraints).
_Avoid_: Vendor, merchant, contact — those imply money only ever flows outward, whereas
a Payee can be the source of a Transaction (e.g. an employer) as well as its
destination.

**Transaction**:
A single-entry record of an amount moving against exactly one Account, exactly one
Category, and an optional Payee, on a date, carrying a Transaction Status and,
independently, a Flagged marker. Personal Ledger is deliberately single-entry, not
double-entry (see
[ADR-0001](docs/adr/0001-single-entry-not-double-entry.md)) — a Transaction is one row,
not a balanced pair.
_Avoid_: Posting, entry, ledger entry — these imply the balanced debit/credit pairs of
double-entry bookkeeping, which Personal Ledger does not use.

**Transaction Status**:
Where a Transaction sits in the reconciliation workflow — one of:
- **Open**: recorded but not yet confirmed against any external source. The default
  status for a newly-created Transaction.
- **Cleared**: confirmed as having occurred (e.g. it appears on a bank statement or in
  a bank feed), but not yet checked off during a formal reconciliation.
- **Reconciled**: matched against an account statement and confirmed as part of the
  total it asserts — the most-confirmed status, not expected to change afterward. A
  Reconciled Transaction's other fields cannot be changed until its Transaction Status
  is first moved back to **Open** or **Cleared** (see `docs/product-requirements.md`,
  Constraints).
_Avoid_: State, Flagged — Flagged is a separate, orthogonal marker (see below), not a
fourth Transaction Status.

**Flagged**:
A marker a user sets on a Transaction for follow-up or review (e.g. an amount that
looks wrong), independent of its Transaction Status — a Transaction can be Flagged at
any point in the reconciliation workflow, including after it's Reconciled.
_Avoid_: Transaction Status — Flagged layers on top of whichever status a Transaction
is in; it is not one of Open/Cleared/Reconciled.

**Budget**:
A limit on the total amount of Transactions in one Category over a recurring period
(e.g. "$500 per month for Groceries"), denominated in one Unit, compared against actual
spending in that Category and Unit to show how much of the period's limit remains.
_Avoid_: Limit, allowance, envelope — envelope budgeting allocates every dollar of
income across categories; a Budget here is a cap on one Category, not a full
allocation.
