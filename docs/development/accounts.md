# Accounts (development)

End-user documentation: [Accounts](../accounts.md).

## Overview

An Account is a place where value is held or owed, at one Institution and in one Unit. `lib-accounts` is the Accounts VIPER module, toolkit-free and I/O-free: the Entity with its rules and stub seed, the Add and Edit Interactors, the page and form Presenters, the ports, the keymap and its Messages. `lib-account-deletion` is the cross-domain module for Account delete, since deleting writes the Accounts, Transactions and Bills. `lib-database` owns the `accounts` table, the Accounts adapter (`adapter/accounts.rs`) and the deletion adapter (`adapter/account_deletion.rs`). The Desktop's halves are `bin-desktop/src/accounts/` and `bin-desktop/src/account_deletion/`. Accounts were the second domain of slice 5 of the VIPER transition ([#847](https://github.com/IanTeda/Personal-Ledger/issues/847)), following the [Migrating a domain](viper-architecture.md#migrating-a-domain) checklist Payees set. The design of the four Account Kinds is at the end of this page.

## Data model

The table comes from `crates/libs/lib-database/migrations/client/20260905000000_create_accounts_table.sql`, created after `institutions` and `units`.

`accounts`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, an `AccountId` (UUIDv7) |
| `name` | TEXT | Not null |
| `institution_id` | UUID | Not null, references `institutions(id)` with no cascade |
| `account_type` | TEXT | Not null, one of `cash`, `bank`, `credit_card`, `investment`, `loan` |
| `unit_id` | UUID | Not null, references `units(id)`, fixed at creation |
| `balance` | TEXT | Not null, the opening balance as a decimal string in the Unit's precision |
| `opened_at` | TEXT | Not null, a calendar date |
| `account_number` | TEXT | Nullable, the form's optional field |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_accounts_domain_set_updated_on` |

Who names an Account, and what the file does when one is deleted:

| Column | Behaviour on deleting the Account |
| --- | --- |
| `transactions.account_id` | Refuses; the deletion adapter removes the Transactions first |
| `bill_plans.account_id` | Refuses; `DeleteAccount` refuses first, so the key is a backstop |
| `balance_checks.account_id` | `ON DELETE CASCADE` |
| `budgets` | Not persisted; a Budget's `account_ids` keep the id (see the gaps) |

Invariants and deliberate omissions:

- **`institution_id` is never NULL.** Every Account names a stored Institution, a Cash Account usually the Personal Institution (`InstitutionId::SEED_PERSONAL`), which every new Ledger file is written with ([ADR-0049](../adr/0049-a-cash-account-names-the-personal-institution.md), #875). The foreign key refuses an id with no row.
- **Rows read back in rowid order**, the order they were added in. There is no position column; the page groups by type in a fixed order (`GROUP_ORDER`).
- **Unit, opening balance and opened date never change** after the Account is added. `edit_account` writes only the name, Institution, type and account number.
- **There is no transaction count column.** The Dialogs' count is `COUNT(*) FROM transactions WHERE account_id = ?`, so an Import can't leave it wrong (#852, #853).
- **A file made before #853 can't migrate.** Rewriting `create_accounts_table.sql` in place changed its checksum, and nothing seeds an empty table on an existing file, so such a Ledger needs to be started fresh.

## Domain types

`AccountId` is in `lib-core`. `lib_accounts::Account` is the domain type (`id`, `name`, `institution`, `account_type`, `unit`, `balance`, `account_number`, `opened_at`); `AccountDraft` is what Edit can change and `NewAccount` is the draft plus what is fixed at creation. `AccountError` (`NotFound`, `Incomplete`) is the refusal. Beside them in `entity.rs` are `insert_account`, `edit_account`, `remove_account`, `get`, `group_accounts` and `display_order` (the page's grouping), `net_worth`, `find_by_name` (the `:accounts <name>` lookup) and `default_accounts`, the stub seed.

The Account's `institution` and `unit` are ids, so it stays on them whatever they are renamed. Names are looked up when shown.

`lib-account-deletion` holds `DeleteAccount`, the Interactor; `AccountDeletionError` (including `PaidFromByBillPlans`); `AccountRemoval { account, transactions, unmatched }`, the one value its write takes; and the Delete Dialog's Presenter.

## Persistence

`crates/libs/lib-database/src/adapter/accounts.rs` is the Accounts adapter (ADR-0041, ADR-0046): it loads every Account at start-up and serves `lib-accounts`' `AccountRepository` from memory, each write joining the one pending batch the Desktop commits once per input. A write is an upsert (`ON CONFLICT (id) DO UPDATE`). `AccountsStore` (`adapter/store.rs`) is what `Stores` holds it by, with `lib_accounts::fake::InMemoryAccounts` as the in-memory store, and `lib-accounts`' contract suite runs against both. `Stores::with_units` fills it with an empty store for the TUI's test fixture.

- **`AccountUsage` counts from the file.** `account_usage(id)` is one `COUNT(*)` over `transactions`, and `UnitHoldings::holdings(unit)` is one `GROUP BY account_id`, so a Units event costs one query per Unit. The count is what the last commit left, never a pending write. It returns a plain `AccountUse`, not an `Option`: it only feeds a figure, never a refusal, so a failed count is logged and answered as 0. The Budgets count is not here; Budgets aren't persisted, and the Ledger answers it until [#811](https://github.com/IanTeda/Personal-Ledger/issues/811).
- **The adapter also answers other modules' reads.** Bills reads `BillAccounts` and the Units page reads its holdings from the Accounts store, through a small wrapper in the bin (the orphan rule, checklist step 4).
- **Account delete is `AccountDeletionAdapter`** (`adapter/account_deletion.rs`). It holds the Accounts, Transactions and Bills stores, which the Desktop shares with their own adapters (`SharedAccountsStore`, `SharedTransactionsStore`, `SharedBillsStore`). `remove_account(AccountRemoval)` writes the Bill Schedule entries, then the Transactions, then the Account, in that order into the one pending batch, so they land in one database transaction. The order is the foreign keys': the Transactions must go before the Account, and the entries are Unmatched first. A failed write is reported on the shared channel like any other, naming the domains of the writes it dropped.
- **Seed order.** `seed::stub_accounts` runs after the Institutions and Units and before the Payees, Transactions and Bills. A file made before the table existed is not reseeded.

## UI

The Desktop's Accounts page reads and writes the adapter through `lib-accounts`' Presenters. The Transactions View, Budgets, Documents, Import and the Dashboard still read the Ledger's read copy of the Accounts. The TUI doesn't persist Accounts and draws only fixtures.

- **Desktop:** the Settings Accounts page, handoff `docs/ux/desktop-mockups/16-settings/README.md`.
  - `crates/libs/lib-accounts/`, the module. Start here. `interactor/add_account.rs` and `interactor/edit_account.rs` are the Interactors. `presenter/accounts_page.rs` is the page (rows grouped by type, the scope reads and net worth) and `presenter/account_form.rs` the Add and Edit form. `protocols.rs` holds the ports: `AccountRepository`, `AccountUsage`, the read ports `AccountInstitutions` and `AccountUnits` that feed the form's selects, the `AccountsPorts` bundle and the `AccountsRouter`. There is no delete Interactor in it; `AccountRepository::delete_account` is `lib-account-deletion`'s to call.
  - `src/accounts/`, the module's Desktop half: `view/page.rs` and `view/page/render.rs` the page Entity and its rows, `view/dialog.rs` and `view/dialog/account_form.rs` the Add and Edit Dialogs as Entities on the Dialog host, `router.rs` the `AccountsRouter`, and `ports.rs`. `ports.rs`'s `LedgerBackedPorts` owns the boxed `AccountsStore` and implements both the module's ports and the store trait, so one `Rc<RefCell<…>>` is the module's ports, `LedgerStore`'s read copy and the Units, Bills and deletion adapters' store. It copies the Institutions, Units, base Unit and Budgets counts from the Ledger before each event.
  - `enter` is the `select` command and a row click is `Open(id)`; both go through `AccountsRouter::open_ledger`, which emits `Reveal::AccountTransactions`. `AccountsDialog::AddAccount { name }` carries the `:accounts new <name>` argument.
  - `crates/libs/lib-account-deletion/`, the cross-domain module, with `src/account_deletion/view/dialog/delete_account.rs` its Dialog. The Presenter is lent a `DeletionReads` once, at open: `AccountUsage` plus the Unit code the balance is shown in. The adapter holds only three stores and can't reach the Budgets count or the Unit, so the Desktop answers the reads from `LedgerStore` (`account_deletion::LedgerReads`). This is the Pay Dialog's split: the atomic port for checks, separate reads for wording.
  - The Delete Dialog reads the Bill Plans that pay from the Account when it opens, says "N bills are paid from this account. Move them to another account before deleting this one." and keeps confirm off. `DeleteAccount` refuses the same case. This is new behaviour: before #854 the file rolled the delete back with an Error Toast.
  - After a delete, `AccountsPagePresenter` selects the row that slid into the deleted one's place (`AccountsPageEvent::Deleted`), since an id selection would fall to the first row. A confirm on an Account deleted under an open Edit shows "This account has since been deleted." under the form, which the legacy Dialog did silently. Add and Edit raise no Toast.
  - The page reaches `Root` through `settings::pages::SettingsSetup`'s `accounts`, takes `Page::hints` from the Presenter so the status line and `?` follow `[keybindings]`, and is reached from the Settings index, `:settings accounts` or the `:accounts` alias.

## Traceability

Desktop locations for the ticked requirements. ACC-001 to ACC-003, ACC-006 and ACC-007 predate the module and are not traced here.

| Requirement | Code | Test |
| --- | --- | --- |
| ACC-004 | `lib_accounts::edit_account`, `lib_accounts::interactor::EditAccount`, `accounts/view/dialog/account_form.rs` | `settings_accounts_keyboard::e_edits_the_selected_account_and_enter_saves_the_new_name` |
| ACC-005 | `lib_account_deletion::interactor::DeleteAccount`, `account_deletion/view/dialog/delete_account.rs` | `settings_accounts_keyboard::d_deletes_an_account_only_once_its_name_is_typed_back` |
| ACC-014 | `lib_database::adapter::accounts`, `seed::stub_accounts` | `lib_database::adapter::accounts::tests::an_added_account_reloads_the_same`, `smoke_accounts_persist::an_added_account_is_there_after_a_restart` |
| ACC-015 | `lib_database::adapter::account_deletion`, `lib_account_deletion::presenter` | `lib_database::adapter::account_deletion::tests::a_delete_lands_the_account_its_transactions_and_their_unmatch`, `…::an_account_a_bill_plan_pays_from_is_refused`, `…::a_deleted_accounts_balance_checks_go_with_it`, `settings_accounts_keyboard::an_account_a_bill_plan_pays_from_cannot_be_deleted` |

## Decisions

- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the one-file adapters and the shared writer.
- [ADR-0048](../adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md) — the Toast and status Notices the page raises.
- [VIPER transition slice 5](https://github.com/IanTeda/Personal-Ledger/issues/847) — Accounts as the second domain, and [#854](https://github.com/IanTeda/Personal-Ledger/issues/854), where Account delete became `lib-account-deletion`, the second cross-domain module after `lib-bill-payments`.
- `GLOSSARY.md` — Account, Account Kind, Institution, Unit.

## Open questions and known gaps

- **Halfway states.** Every other reader of Accounts uses the Ledger's read copy, `LedgerBackedPorts` copies several things from the Ledger before each event, and `impl AccountUsage for Ledger` serves the TUI and the Budgets count. All are rows in [During the transition](viper-architecture.md#during-the-transition).
- **Budgets may hold a deleted Account's id.** Account delete leaves a Budget's `account_ids` alone, and `lib-budgets` ignores an id with no Account (`Budget::counts`), so a Budget left with none compares nothing and its edit drops the id. Removed when Budgets persist with a cascading `budget_accounts` link ([#811](https://github.com/IanTeda/Personal-Ledger/issues/811)).
- **No Account can be deleted while a Bill Plan pays from it.** In the stub, that is ANZ Everyday (6 Plans) and Amex (4). The person moves the Plans first; there is no "move them for me" step.
- **A file from before #853 can't migrate.** Start a fresh Ledger.
- **The TUI doesn't persist Accounts.** It draws fixtures until its redesign ([#805](https://github.com/IanTeda/Personal-Ledger/issues/805)).
- **No Change Set emission**, so Account writes do not sync.
- **The shipped `Account` has five types, not four Kinds.** `account_type` is `cash`, `bank`, `credit_card`, `investment` or `loan` in one table, and none of the Kind-specific fields below (credit limit, rate history, principal) exist yet. The design below is the target, not the code.

## Design: Account Kinds

Personal Ledger accounts represent money spent, money owed or money held as an investment. Every account is initially nominated as one of these four types:

1. Transaction Account,
2. Credit Card Account
3. Loan Account 
4. Investment Account 

Each account type has its own record shape, workflow, and view, even though the app still groups all four under one “Accounts” navigation area for browsing. This document explains why they’re separate, what each one is, and how the mechanisms that move value between them (Transaction, Split, Trade, Repayment) fit together.

See GLOSSARY.md for the one-paragraph canonical definition of each term; this document covers the fuller shape and relationships.

### Why four entities, not one

Before this design, Personal Ledger modelled Cash, Bank, Credit Card, Investment, and Loan as five variants of a single `Account` type, all sharing one table. That works as long as every variant needs the same data — a name, a Unit, a starting balance. It stops working once a variant needs data the others don't:

- A Loan Account needs principal, an interest rate history, a term, and a next-due-date — none of which mean anything for a plain Bank Account.
- An Investment Account needs a quantity that only ever changes through a Trade, never a plain Transaction — a Bank Account's balance changes through Transactions and nothing else.
- A Credit Card Account needs a credit limit and an interest rate history too — revolving debt, not a simple cash balance — even though, unlike a Loan, it still accepts Transactions directly the same way a plain Bank Account does.

Rather than bolting loan-, investment-, and credit-card-specific columns onto one shared table, each becomes its own entity, grouped under the umbrella term **Account** (see below). Loan Account and Investment Account leave the shared-Transactions machinery entirely; Credit Card Account keeps it (it still posts Transactions and computes a Balance the same way) but still gets its own entity because its extra data (credit limit, rate history, statement date) doesn't belong on a plain Transaction Account either.

### Institution

The financial institution — a bank, a broker, a lender — that an Account is held with. A first-class entity, not a free-text field.

Every Account, regardless of Account Kind, carries a **mandatory** link to exactly one Institution. This is deliberately strict, including for Cash: a Cash-type Transaction Account represents physical, on-hand cash, which has no bank but does have a holder, the person keeping the Ledger. So it names the Personal Institution, an ordinary Institution every new Ledger starts with, rather than leaving the field empty ([ADR-0049](../adr/0049-a-cash-account-names-the-personal-institution.md), which replaced an earlier placeholder with no row). Two alternatives were considered and rejected:

- Making the link optional (nullable) — rejected because every real Bank, Credit Card, Loan, and Investment record always has a genuine Institution, and an optional field invites inconsistent data for no principled reason.
- Forcing the user to pick a bank even for Cash — rejected as meaningless data entry; the Account form defaults a new Cash Account to the Personal Institution instead, and any other may be picked.

Examples: "Commonwealth Bank", "Vanguard", "IG Markets", the Personal Institution for Cash.

### Account and Account Kind

**Account** is the umbrella term: a place where value is held or owed. It is not itself a table with a type column — it's the shared concept behind four separate entities, each with its own shape. **Account Kind** is which of the four an Account is: Transaction Account, Credit Card Account, Loan Account, or Investment Account, decided when the Account is created.

Every Account, of any Kind, shares exactly two things: a Unit (fixed at creation) and a mandatory Institution link (see above). Everything else — whether it accepts Transactions directly, what "the current amount" is called, what extra fields it carries — is specific to its Kind.

There's a second, narrower "type" concept that's easy to confuse with Account Kind: **Transaction Account Type** (Cash or Bank) only classifies *within* Transaction Account — it says nothing about Credit Card, Loan, or Investment Accounts. The two are named differently (Account *Kind* vs. Transaction Account *Type*) specifically so a reader can tell which axis is meant without more context.

| Account Kind        | Accepts Transactions directly? | Has a Balance? | "Current amount" concept |
| -------------------- | ------------------------------- | --------------- | -------------------------- |
| Transaction Account  | Yes                              | Yes              | Balance                    |
| Credit Card Account  | Yes                              | Yes              | Balance                    |
| Loan Account         | No — Repayment only             | No               | Outstanding Principal       |
| Investment Account   | No — Trade only                 | No               | Quantity                    |

### Transaction Account

An everyday place where spending money is held. Denominated in exactly one Unit, fixed at creation. Its Balance is derived from the Transactions posted against it — not stored directly.

Fields:

- Name — a human-readable label (e.g. "Everyday Spending").
- Transaction Account Type — Cash or Bank.
- Unit — fixed at creation.
- Institution — mandatory (see above).
- Starting Balance — in the Transaction Account's Unit, combined with its Transactions to compute the current Balance.
- Active flag — soft-delete; an inactive Transaction Account isn't offered for new Transactions but its history remains valid.

Transaction Account Type stays a two-value field (Cash, Bank) rather than being inferred from whether the Institution is the Personal Institution or a bank — keeping classification and Institution-linking independent, so one can change without silently implying the other.

### Credit Card Account

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

Paying down a Credit Card Account is an **ordinary Transaction**, not a Repayment: it's a same-Unit Transaction moving cash from a Transaction Account into the Credit Card Account, using the same "a Transaction may move to another Account sharing that same Unit" rule that already covers any same-Unit transfer (see `GLOSSARY.md`, Unit). Repayment (see below) stays specific to Loan Account, precisely because a Loan Account — unlike Credit Card Account — can't accept a Transaction directly at all.

### Loan Account

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

### Investment Account

A single holding or position — e.g. "10 shares of AAPL" — not a container of several holdings. An Investment Account is not a Transaction Account: its quantity changes only through a Trade, never through a plain Transaction.

Fields:

- Name — a human-readable label (e.g. "Apple Inc. — CommSec").
- Unit — fixed at creation, the security or instrument itself (e.g. AAPL, VAS, BTC), not the currency it's priced in.
- Institution — mandatory (see above), the broker or exchange it's held through.
- Quantity — derived from the sum of all Trades against this Investment Account (buys add, sells subtract), not stored directly.
- Active flag — soft-delete, same pattern as Transaction Account.

A "Holding" concept distinct from Investment Account — one Investment Account containing several tickers, the way a real brokerage account often works — was considered and rejected: an Investment Account *is* the holding, one entity per position. If a real brokerage account holding several securities needs modelling later, that's several Investment Accounts sharing one Institution, not one entity containing several holdings.

Market value, unrealised/realised capital gains, cost basis, and dividends are deliberately **not** modelled here. `docs/product-requirements.md` already marks capital-gains tracking as a future consideration with no requirements defined, and a prior requirement to store Unit prices (FR.3) was struck out because "no cross-Unit exchange exists to price" — valuation needs a pricing mechanism Personal Ledger doesn't have yet. Cost Basis and Dividend are less blocked (they don't need live market pricing) but are still reporting-feature detail, parked for a future design pass rather than guessed at here.

### Transaction and Split

A Transaction is a single-entry record of an amount moving against exactly one Transaction Account or Credit Card Account — the two Account Kinds that accept Transactions directly — on a date, carrying a Transaction Status and, independently, a Flagged marker. Personal Ledger is deliberately single-entry, not double-entry (see [ADR-0001](adr/0001-single-entry-not-double-entry.md)).

A Transaction is composed of one or more **Splits**. A Split is one line of a Transaction: an Amount and a Category, plus an optional Payee and any number of Tags. A Transaction's Splits' Amounts sum to its total. Date and the Transaction Account or Credit Card Account it's posted against live on the Transaction itself, shared by every Split within it.

This means a plain, single-Category Transaction — the common case — is just the one-Split case of the same structure, not a separate type from a multi-Category one. Nothing in the app needs to handle "a plain Transaction" and "a split Transaction" as two different shapes.

Example — a $2,000 home Loan Account repayment recorded through the Repayment mechanism below still uses Splits for its principal/interest breakdown:

| Split | Category       | Amount    |
| ----- | -------------- | --------- |
| 1     | Loan Interest  | $800.00   |
| 2     | Loan Principal | $1,200.00 |

### Trade

The mechanism for buying or selling an Investment Account. A Trade moves cash out of (or into) a Transaction Account, in its Unit, and changes an Investment Account's quantity, in its Unit.

This is structurally different from a Transaction/Split for two reasons:

1. **It crosses Account Kinds.** A Split moves value within one Account. A Trade moves value from a Transaction Account into an Investment Account (or back) — two different Account Kinds entirely.
2. **It's quantity- and price-aware.** A Split is just a Category and an Amount. A Trade needs a quantity of the security bought or sold and a price, which together determine the cash Amount on the Transaction Account side.

No exchange rate is involved, and this doesn't reopen the "Personal Ledger doesn't convert between Units in V1" rule (see `GLOSSARY.md`, Unit): each side of a Trade is simply denominated in its own Unit already — the Transaction Account side in currency, the Investment Account side in the security itself. The Trade's price is what ties the two amounts together, not a currency-conversion exchange rate.

Example — buying 10 shares of AAPL at $150.00 each from a cash Transaction Account:

| Side        | Entity                            | Unit | Amount     |
| ----------- | ---------------------------------- | ---- | ---------- |
| Cash out    | Transaction Account (cash holding) | AUD  | -$1,500.00 |
| Quantity in | Investment Account (AAPL)          | AAPL | +10        |

Further Trade detail — fees, dividend reinvestment, partial fills — is future design work once Trade is actually built; this document covers only the shape agreed so far: a Transaction-Account-to-Investment-Account mechanism, distinct from Transaction/Split, quantity- and price-aware.

### Repayment

The mechanism for paying down a Loan Account — the symmetric counterpart to Trade, for Loan Account instead of Investment Account. A Repayment moves cash out of a Transaction Account and reduces the Loan Account's Outstanding Principal.

Like Trade, a Repayment crosses Account Kinds — Transaction Account into Loan Account — which a same-Account Transaction structurally can't do. Unlike Trade, a Repayment reuses the Split concept for its own breakdown: a Repayment carries Splits the same way a Transaction does, typically one Split categorised as principal and one as interest, summing to the repayment total (see the example table above).

Repayment is specific to Loan Account. Paying down a Credit Card Account does **not** use Repayment — it uses an ordinary Transaction instead, because Credit Card Account, unlike Loan Account, already accepts Transactions directly (see Credit Card Account, above).

### What's deliberately out of scope here

This document captures the entity and mechanism *shape* agreed so far — it is not a functional spec, and `docs/product-requirements.md` still owns actual requirements (Loan and Investment tracking remain "future consideration" there; nothing here moves them into an active cycle). Left for later, once that work is actually scoped:

- Amortisation schedules and payoff projections for Loan Account, and the equivalent for Credit Card Account.
- Cost basis, realised/unrealised capital gains, market value, and dividends for Investment Account — all blocked on a pricing mechanism Personal Ledger doesn't have yet.
- Trade's exact fields (fees, partial fills, dividend reinvestment).
- Any dedicated screens/views for Loan Account, Investment Account, or Credit Card Account in the TUI or desktop app.
