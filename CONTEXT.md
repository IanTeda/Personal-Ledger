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
The fixed thing a balance, principal, or holding quantity is denominated in — either fiat money (e.g. AUD) or a tradeable non-currency instrument (e.g. a stock ticker like AAPL, or bitcoin). Every Account, of any Account Kind (see below), has exactly one Unit, fixed at creation. Personal Ledger does not convert between Units in V1: a Transaction Account or Credit Card Account only ever holds Transactions in its own Unit, and a Transaction may only move to another Transaction Account or Credit Card Account sharing that same Unit (see `docs/product-requirements.md`, Constraints) — Trade (see below) is the deliberate exception, crossing from a Transaction Account into a differently-Unit-denominated Investment Account by design, not by relaxing this rule; a Repayment still crosses from Transaction Account into Loan Account as a different Account Kind, but typically stays within one Unit since a Loan Account and the Transaction Account repaying it are usually the same currency.
_Avoid_: Commodity, Currency, asset class, exchange rate, conversion.

**Ledger**:
The complete set of Accounts, Institutions, Categories, and Transactions owned by
one self-hoster — a logical whole, physically replicated as a full local SQLite
database on each Client and synced between them by the Sync Server.
_Avoid_: Book, journal.

**Client**:
An app instance that holds its own full local copy of the Ledger and can create, read,
update, and delete against it entirely offline — Desktop or TUI.
_Avoid_: Device, backend — "device" conflates the physical machine with the app
instance running on it; "backend" implied a single authoritative server, which no
longer describes the architecture.

**Preference**:
A user-editable runtime setting that changes how a Client behaves, edited from inside a
running Client — distinct from Configuration (see below), which is deployment-time and
edited outside the running app. Each Preference is individually either Ledger-scoped
(stored in `lib-database`, synced via Change Sets like other Ledger data) or
Client-scoped (stored locally, never synced); which kind a given Preference is gets
decided as it's defined, in CC-TUI-001/CC-DESKTOP-001 (`docs/product-requirements.md`).
As of [ADR-0014](docs/adr/0014-preferences-table-and-leaner-sync-server-config.md) and
[ADR-0021](docs/adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md),
the Preferences are the default Unit for new Accounts, colour theme, and date style — all
three Ledger-scoped, so they sync across a user's own Clients rather than being set
separately on each one. The date style is nullable: no value means "use the Locale's
default", and only an explicit choice is stored and synced. The Locale is not a Preference
(see Locale, below). No Client-scoped Preference exists
yet; that half of the split stays available for whenever a genuinely per-device setting
shows up.
_Avoid_: Configuration, setting — "Configuration" is reserved for `lib-config`'s layered,
deployment-time config (files/env vars, see `docs/settings.md`); a Preference is
edited by the user from inside a running Client instead.

**Configuration**:
Deployment-time settings loaded by `lib-config` before or as a Client/the Sync Server
starts — defaults, an explicit path, or environment variables, plus a file-location
search in between (the full system/user/executable-directory/working-directory search
for a Client; a single configured file path for the Sync Server, per
[ADR-0014](docs/adr/0014-preferences-table-and-leaner-sync-server-config.md)) — in that
precedence order (see `docs/settings.md`). Not user-editable from inside a running
app — see Preference, above, for that. Covers only settings needed before the app (or
its database) can run — the database connection/pool settings and the telemetry level —
not display-level settings, which are Preferences instead (see above). The Locale is the one
display-level exception, because it is per-Client, needed before the interface can be drawn, and
only changed by editing the file and restarting (see Locale, below).
_Avoid_: Preference, setting.

**Locale**:
The language-and-region choice (e.g. `en-AU`, `en-US`, `en-GB`) a Client presents its interface in. It decides both which translated Messages are shown and how numbers, dates and currency amounts are formatted, so the two never disagree. The source Locale, in which every Message is first written and which is the only complete one, is `en-US`; `en-GB` and `en-AU` hold only the Messages that differ, falling back `en-AU` → `en-GB` → `en-US`. A pseudo-Locale (`en-XA`) is a deliberately distorted stand-in used to spot untranslated text and layout overflow, never offered to the user.
The Locale is Configuration, not a Preference: a Client takes it from the operating system, unless a static `lib-config` setting (file, environment variable or command-line flag) overrides it, falling back to `en-US` when the system value is unsupported or unreadable. It is per-Client, is never synced, and changes only by editing that setting and restarting the Client — there is no in-app switch.
_Avoid_: language, region, culture.

**Message**:
One piece of translatable interface text — a button label, a heading, a status-bar hint — identified by a stable id that is the same in every Locale, with one translation per Locale. Not user data: a Payee name or an Account name is Ledger data and is never a Message. Enumerated domain values (an Account Kind, a Transaction Status) are shown to the user through Messages, never by their stored token, which stays stable. Typed command names are an interface, not Messages.
_Avoid_: string, label, copy, translation (a translation is one Locale's rendering of a Message).

**Catalogue**:
The complete set of Messages translated into one Locale. A Locale whose Catalogue lacks a Message falls back to a wider Locale's Catalogue, ultimately the source Locale's.
_Avoid_: bundle, dictionary, resource.

**Sync Server**:
A separate deployable component — a headless service, not a Client — that syncs each
Client's local Ledger copy with the others by pushing and pulling Change Sets, not
exposing full CRUD. It maintains its own durable store of Change Sets (not a full
Ledger copy) so a Client that has been offline can catch up without both peers being
online simultaneously. It also acts as its own OAuth2 authorization server for its
Clients: a Client authenticates via a one-time browser login (Authorization Code +
PKCE over a loopback redirect), then calls the sync gRPC API with a bearer token,
never holding a persistent username/password itself — see
[ADR-0010](docs/adr/0010-oauth2-pkce-native-app-auth.md).
_Avoid_: Backend, Web App, reconcile/reconciliation — "backend" is avoided across this
glossary now that Clients hold their own local data directly rather than depending on
a central server for reads/writes; "Web App" was an earlier, now-abandoned design
where this role rode along on a browser-facing Client instead of being its own
component; "reconcile" is reserved for Transaction Status's reconciliation workflow
(see below) — the Sync Server *syncs* Ledger copies, a different concept that happens
to share an English word with it.

**SyncUser**:
The Sync Server's own bootstrap authentication credential — a username, an Argon2
password hash, and the current refresh-token hash (if any active session), provisioned
once at first run. Single-account this cycle (see
[ADR-0010](docs/adr/0010-oauth2-pkce-native-app-auth.md)): there is exactly one SyncUser
per Sync Server instance, not a per-human-user table. A SyncUser is not Ledger data — it
never syncs via Change Sets, and it has no relationship to any Client's local Accounts,
Categories, or Transactions.
_Avoid_: Account — reserved for the domain Ledger entity (see below); a SyncUser
authenticates *to* the Sync Server, an Account holds money *inside* the Ledger, and
conflating the two was an earlier implementation mistake this term corrects. Also avoid
User — too generic to convey that this is scoped to one Sync Server instance's own login,
not a general concept of "a person using the software."

**Change Set**:
The unit of data the Sync Server pushes and pulls between Clients to propagate one
Client's local edits to the others, at field granularity: a stable Change Set ID
(`RowID`, UUIDv7-based), the target table/row/field identifiers, the new value, a
Hybrid-Logical-Clock timestamp, the originating Client's stable ID, and a
version/parent-version field held for a future CRDT or manual-merge upgrade path.
Conflicting Change Sets to the same field are resolved last-write-wins by that
timestamp, tie-broken by Client ID — see
[ADR-0009](docs/adr/0009-lww-sqlite-change-set-log.md).
_Avoid_: Diff, patch, delta — those are implementation-neutral synonyms; Change Set is
this glossary's canonical term.

**Institution**:
The financial institution (e.g. a bank or broker) that an Account is held with — a first-class entity as of [ADR-0017](docs/adr/0017-loan-and-investment-as-entities-not-account-types.md), not a free-text field: every Account, of any Account Kind (see below), carries a mandatory link to exactly one Institution. A Cash-type Transaction Account (physical, on-hand cash — see Transaction Account, below) has no real-world institution, so it links to a system-seeded placeholder Institution rather than leaving the field empty; the link itself stays mandatory at the data-model level even where it's meaningless in the real world. See `docs/accounts.md`.
_Avoid_: Company, organization.

**Account**:
A place where value is held or owed. An umbrella term over four Account Kinds (see below) — Transaction Account, Credit Card Account, Loan Account, Investment Account — each with its own record shape, workflow, and view (see [ADR-0017](docs/adr/0017-loan-and-investment-as-entities-not-account-types.md), [ADR-0018](docs/adr/0018-account-kind-umbrella-and-credit-card-account.md)); the app may still group all four under one "Accounts" navigation area for browsing, but underneath they are not rows in one shared table with a type column. Every Account, regardless of Kind, is denominated in exactly one Unit and linked to exactly one Institution (see above). See `docs/accounts.md`.
_Avoid_: Using "Account" alone once a specific Kind matters — say "Transaction Account", "Loan Account", etc., the same way "Category" and "Category Type" aren't used interchangeably.

**Account Kind**:
Which of the four kinds of Account (see above) a given Account is: Transaction Account, Credit Card Account, Loan Account, or Investment Account. Decided when the Account is created. See [ADR-0018](docs/adr/0018-account-kind-umbrella-and-credit-card-account.md).
_Avoid_: Account Type — reserved for Transaction Account Type (see below), a separate, narrower classification that only applies within Transaction Account, so the two axes stay textually distinct.

**Transaction Account**:
An everyday place where spending money is held: Cash or Bank (its Transaction Account Type, see below). Denominated in exactly one Unit, whose Balance is derived from the Transactions posted against it, and linked to exactly one Institution (see above). One of two Account Kinds — Credit Card Account is the other (see below) — that accepts Transactions directly; Loan Account and Investment Account do not.
_Avoid_: Wallet, plain "Account" once the Kind matters. Not to be confused with Category: a Transaction Account is *where* money sits, a Category is *what* it's classified as.

**Transaction Account Type**:
Cash or Bank — the classification within Transaction Account itself (see above), distinct from Account Kind (see above), which is the broader Transaction/Credit Card/Loan/Investment split. Kept as an explicit field rather than inferred from whether the Transaction Account links to the placeholder Institution (Cash) or a real one (Bank), so classification and Institution-linking stay independent concepts.
_Avoid_: Account Type — see Account Kind, above, for why the two axes are named differently.

**Credit Card Account**:
Revolving debt held with a lender — its own Account Kind (see above; [ADR-0018](docs/adr/0018-account-kind-umbrella-and-credit-card-account.md)), not a Transaction Account Type, because it carries data a Transaction Account can't express: a Credit Limit, an interest rate tracked as a history over time (the same shape as Loan Account's — card rates change too), and a statement/due date. Unlike Loan Account and Investment Account, a Credit Card Account *does* accept Transactions directly, the same way a Transaction Account does — spending on the card is a plain Transaction (with Splits), and its Balance is computed from them the same way. Paying down a Credit Card Account is an ordinary same-Unit Transaction moving cash from a Transaction Account into it (see Unit, above), not the Repayment mechanism (see below), which stays specific to Loan Account because a Credit Card Account, unlike a Loan Account, already accepts a Transaction directly.
_Avoid_: Loan Account — both are debt, but a Loan Account accepts no direct Transactions at all, and a Credit Card Account does. Repayment — reserved for Loan Account's own mechanism (see below).

**Loan Account**:
An amount owed to a lender, tracked as its own Account Kind rather than a kind of Transaction Account (see [ADR-0017](docs/adr/0017-loan-and-investment-as-entities-not-account-types.md)) because it carries data a Transaction can't express: principal, an interest rate tracked as a history over time (a rate plus the date it took effect, not a single fixed value — Australian loans are commonly variable-rate), a term, and a next-due-date. Denominated in exactly one Unit and linked to exactly one Institution (see above). Accepts no Transactions directly — its outstanding amount owing is reduced only by a Repayment (see below) — and is not called a Balance: Balance is specific to Transaction Account and Credit Card Account, computed from Transactions posted against them, and a Loan Account has none.
_Avoid_: Transaction Account, plain "Account" — a Loan Account is not a kind of Transaction Account. Balance — a Loan Account's amount owing is its Outstanding Principal. Credit Card Account — both are debt, but a Credit Card Account accepts direct Transactions and a Loan Account does not.

**Investment Account**:
A single holding or position (e.g. "10 shares of AAPL"), tracked as its own Account Kind rather than a kind of Transaction Account (see [ADR-0017](docs/adr/0017-loan-and-investment-as-entities-not-account-types.md)) because it carries data a Transaction can't express: a quantity that changes only through a Trade (see below), never a plain Transaction. Denominated in exactly one Unit — the security or instrument itself (e.g. AAPL, VAS, BTC) — and linked to exactly one Institution (see above), typically the broker or exchange it's held through. Accepts no Transactions directly, the same way Loan Account doesn't.
_Avoid_: Transaction Account, plain "Account" — an Investment Account is not a kind of Transaction Account. Holding — considered as a separate sub-entity (one Investment Account containing several holdings) and rejected: an Investment Account *is* the holding, one entity per position, not a container for several.

**Balance**:
A Transaction Account's or Credit Card Account's current accumulated total, computed
by summing the Transactions posted against it, expressed in its Unit. Loan Account and
Investment Account have no Balance — see their own entries, above, for the analogous
concepts (Outstanding Principal, Quantity).
_Avoid_: Total, running balance.

**Balance Check**:
A point-in-time assertion of what a Transaction Account's or Credit Card Account's
Balance should be (e.g. from a bank or card statement), entered manually or read from
a Balance column when importing a CSV file, checked against the Account's own Balance
as computed from its Transactions.
_Avoid_: Balance — a Transaction Account's or Credit Card Account's Balance is the
computed total from its Transactions; a Balance Check is a separate assertion compared
against that total, not the total itself.

**Category**:
A user-defined label (e.g. "Groceries", "Salary") used to classify a Split (see below) — the unit a Transaction is actually broken into — carrying exactly one Category Type. Categories nest up to three levels deep (e.g. Housing › Utilities › Electricity): a sub-category always carries its parent's Category Type, only a leaf (a Category with nothing nested under it) is assigned to a Split, and a parent's totals roll up its descendants'.
_Avoid_: Group. Tag was once considered a synonym for Category and rejected on those
grounds; Tag is now its own independent entity (see below), not a revival of that
rejected idea.

**Category Type**:
One of the two fixed classifications a Category carries: Expense (money going out) or Income (money coming in). Assets and liabilities are held by Accounts, not classified by Category, so the asset, liability and equity types once listed here were dropped.
_Avoid_: Account type.

**Uncategorised**:
The system-provided leaf Category, one per Category Type, that a deleted Category's Splits are re-pointed to, so deleting a Category never deletes a Split. Created the first time it is needed.
_Avoid_: Unknown, Other — those read as user-made Categories.

**Payee**:
A canonical, user-visible name for the business, organisation, or individual money moved to or from on a Split (e.g. "Woolworths", an employer), recorded so spending or income can be totalled by who it went to or came from. Optional on a Split (see below) — a Split, not the Transaction as a whole, is what actually carries a Payee. A first-class entity as of [ADR-0012](docs/adr/0012-payee-entity-with-rename-aliases.md) — a Split links to a Payee, rather than storing its name as free text — with its own lifecycle (`is_active`, no hard delete once referenced). A Payee not yet seen is auto-created the first time its name is entered on a Split; no separate manual "create a Payee" step is required, though one exists for consistency (see `docs/product-requirements.md`, Constraints).
_Avoid_: Vendor, merchant, contact — those imply money only ever flows outward, whereas a Payee can be the source of a Split (e.g. an employer) as well as its destination.

**Payee Alias**:
A former name of a Payee, preserved automatically when the Payee is renamed so that typing the old name later still resolves to (and suggests) the current Payee — a Split already linked to that Payee shows its current name immediately, with nothing to bulk-update, since the link is by identity, not by stored text. Stored as a regex pattern to leave room for a future hand-authored pattern (e.g. matching several old variants at once), but in V1 a Payee Alias is only ever auto-generated, as an exact match on the prior name, by the rename that creates it — there is no manual alias-authoring UI yet. See [ADR-0012](docs/adr/0012-payee-entity-with-rename-aliases.md).
_Avoid_: Rename, history — a Payee Alias is the *record* a rename leaves behind, not the act of renaming itself.

**Tag**:
A user-defined, freeform label attached to any number of Splits (see below), independent of their Category and Payee — for cross-cutting totals a fixed, exactly-one classification can't express (e.g. "Japan Trip 2026" spanning several Categories and Payees). Globally unique (case-insensitive) and soft-deleted via `is_active` like Unit/Category/Account/Payee, but with no rename-alias history (unlike Payee) — nothing auto-creates a Tag from parsed import text that would later need reconciling under a rename, so an in-place rename is enough. A Tag's total sums cleanly only when every Split carrying it shares one Unit; a Tag spanning mixed Units reports per-Unit subtotals rather than a summed cross-Unit figure, the same rule Accounts' type-grouped subtotals already follow (see `docs/ux/tui/accounts/README.md`). See [ADR-0015](docs/adr/0015-tag-as-independent-transaction-label.md).
_Avoid_: Label — too generic, collides with a UI form field's own label. Category — Category is exactly-one and required per Split; Tag is many-to-many and optional.

**Split**:
One line of a Transaction: an Amount and a Category, plus an optional Payee and any number of Tags. Every Transaction is composed of one or more Splits whose Amounts sum to the Transaction's total — a plain, single-Category Transaction is just the one-Split case of the same structure, not a separate kind of Transaction. Date and the Transaction Account or Credit Card Account it's posted against live on the Transaction itself, shared by all its Splits.
_Avoid_: Transaction Line — Split is this glossary's canonical term (shorter, matches the one-word style of Payee/Tag/Category). Posting, entry — reserved by Transaction's own _Avoid_ note, for the same double-entry-bookkeeping reason.

**Transaction**:
A single-entry record of an amount moving against exactly one Transaction Account or Credit Card Account — the two Account Kinds that accept Transactions directly (see Account Kind, above) — on a date, carrying a Transaction Status and, independently, a Flagged marker. Composed of one or more Splits (see above), which carry the Category, optional Payee, and Tags — a plain Transaction (one Category, one Payee) is just the one-Split case, not a distinct shape. Personal Ledger is deliberately single-entry, not double-entry (see [ADR-0001](docs/adr/0001-single-entry-not-double-entry.md)) — a Transaction, and each of its Splits, is one row, not a balanced pair; splitting a Transaction's amount across several Categories is not the same thing as double-entry bookkeeping's balanced debit/credit pairs across Accounts.
_Avoid_: Posting, entry, ledger entry — these imply the balanced debit/credit pairs of double-entry bookkeeping, which Personal Ledger does not use.

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
is in; it is not one of Open/Cleared/Reconciled. Also distinct from a Transaction
"carrying a Bill" (see Bill Schedule, below) — that is a derived fact of a Bill Schedule
link, not a variant of this marker.

**Trade**:
The mechanism for buying or selling an Investment Account (see above): moves cash out of (or into) a Transaction Account, in its Unit, and changes an Investment Account's quantity, in its Unit — two different Account Kinds, two different Units, no exchange rate involved (each side is simply denominated in its own Unit; the Trade's price is what ties the two amounts together). A distinct mechanism from Transaction/Split, not a special case of either, because it is quantity- and price-aware in a way a Split isn't.
_Avoid_: Transaction, Split, Transfer — a Trade crosses from a Transaction Account into an Investment Account, which a same-Account, same-Unit Transaction/Split structurally cannot do; "Transfer" is avoided because it implies moving like-for-like value between two similar places, whereas a Trade converts cash into a holding (or back).

**Repayment**:
The mechanism for paying down a Loan Account (see above): moves cash out of a Transaction Account and reduces the Loan Account's outstanding principal, carrying the principal/interest breakdown as Splits (see above) — the symmetric counterpart to Trade, for Loan Account instead of Investment Account. Specific to Loan Account: paying down a Credit Card Account (see above) uses an ordinary Transaction instead, since a Credit Card Account, unlike a Loan Account, already accepts Transactions directly.
_Avoid_: Transaction — a Repayment crosses from a Transaction Account into a Loan Account, which a same-Account Transaction structurally cannot do; it uses Splits internally for its principal/interest breakdown but is its own mechanism, not a plain Transaction. Credit Card repayment — deliberately not called a Repayment; see Credit Card Account, above.

**Budget**:
A limit (on an Expense Category) or target (on an Income Category) on the total amount of Splits in one Category over a recurring period (e.g. "$500 per month for Groceries", "$4,200 per month from Salary"), denominated in one Unit, compared against the actual total in that Category and Unit to show how much of the period's amount remains. Only an Expense Budget can be over budget; an Income Budget at or past its amount has simply met its target. A parent Category shows the sum of its descendants' Budgets rather than a Budget of its own. Its progress view also carries a Known Costs (Bills) figure (see below) alongside actual spend, but Budget itself has no direct link to any Bill — the two remain independent entities.
_Avoid_: Limit, allowance, envelope — envelope budgeting allocates every dollar of
income across categories; a Budget here is a cap on one Category, not a full
allocation.

**Bill**:
The recurring definition of an expected payment obligation (e.g. "Telstra internet, $89/month") — a schedule and a set of defaults, not itself a record of money moving. Carries exactly one (Expense-type) Category, one Unit, one Account it's expected to be paid from, an optional Payee, an estimated Amount, a flag marking that Amount as Fixed or Estimated, a Recurrence (Weekly, Fortnightly, Monthly on a fixed day, Quarterly, Annually, or One-shoot for a single non-repeating due date), an optional end date after which no further Bill Schedule entries (see below) are generated, and an optional Attention Lead (a day count, unset by default) that widens which not-yet-Overdue Bill Schedule entries also count toward Needs Attention (see below) — set per Bill rather than as a single global setting, since how early a Bill deserves attention varies by Bill (rent versus a small subscription). Independent of Budget (see above) — a Bill may happen to share a Category with a Budget, but there is no direct link between the two entities. See `docs/bills.md` and [ADR-0019](docs/adr/0019-materialized-bill-schedule-transaction-linked.md).
_Avoid_: Bill Planner — that names the feature/screen for managing Bills, not the entity itself. Recurring Bill, Scheduled Bill — Bill is this glossary's canonical term.

**Bill Schedule**:
One dated instance of a Bill (see above), generated ahead of time from the Bill's Recurrence so it exists as a real row with its own stable identity (`RowID`) for a Transaction to reference — a Monthly Bill has many Bill Schedule entries, one per due date, the same way a Loan Account's interest rate is tracked as a history of dated entries rather than one row rewritten in place. Carries its own status — Upcoming (due date in a future calendar month), Due (due date within the current calendar month, up to and including the due day), Overdue (due day passed, still unpaid), Paid (linked to a Transaction), or Skipped (deliberately not paid, whether a recurring cycle skipped or a One-shoot Bill cancelled) — and, once Paid, a link to the Transaction that settled it, with the Transaction's own date and amount always taking precedence over the Bill's estimate. A Transaction linked to a Bill Schedule entry is understood to "carry a Bill" wherever it's displayed — a derived fact of the link existing, not a stored field, and unrelated to a Transaction's separate Flagged marker (see below). "Last paid," average, and same-time-last-year figures are always derived live from Paid entries' linked-Transaction amounts; Skipped entries are excluded entirely rather than counted as zero. Every Bill Schedule entry is retained permanently — deactivating its Bill, or the Bill reaching its end date, stops new entries from being generated but never removes past ones — so Bill Schedule doubles as the filterable historical record of what was due, paid, skipped, or missed. See `docs/bills.md` and [ADR-0019](docs/adr/0019-materialized-bill-schedule-transaction-linked.md).
_Avoid_: Bill Occurrence — an earlier working name for this same concept, superseded by Bill Schedule.

**Anticipated Bill**:
A shaded, non-persisted preview row shown inline in a Bill's target Account's transaction list, at the due date of its next unresolved Bill Schedule entry (see above) — a placeholder standing in for money that hasn't moved yet. Replaced entirely by the real Transaction once that Bill Schedule entry resolves to Paid (by creating a new Transaction or linking an existing one); the two never coexist as separate rows for the same Bill Schedule entry.
_Avoid_: Anticipated Transaction, Ghost Transaction — Anticipated Bill is this glossary's canonical term, and it is explicitly not a Transaction (see above) until it resolves into one.

**Known Costs (Bills)**:
A section of a Budget's (see above) progress view totalling the estimated Amount of every unpaid (Due or Overdue) Bill Schedule entry (see above) within the Budget's current period, matching its Category and Unit — a forecast of committed spending shown alongside the Budget's existing actual-spend figure. An Overdue entry stays counted in this total (it is still unpaid, committed spending) but is visually distinguished from merely-Due entries. Once a Bill Schedule entry is Paid, its linked Transaction's actual amount flows into the Budget's ordinary spend total instead, so it is never counted in both places.
_Avoid_: Committed spend, forecast — Known Costs (Bills) is this glossary's canonical term for this figure.

**Task**:
A user-created, free-text note for a general money-related to-do (e.g. "call the bank about this charge") — the one kind of Needs Attention entry (see below) a user creates directly, rather than one the Ledger surfaces on its own. Optionally links to exactly one Transaction for context, independent of that Transaction's Flagged marker (see above) — linking a Task doesn't require the Transaction to be Flagged, and Flagging a Transaction doesn't create a Task. Moves through exactly two states, Open and Done; a Done Task's row is kept, not deleted, the same soft-delete convention as Account/Category/Payee/Tag/Bill's `is_active`. See `docs/needs-attention.md`.
_Avoid_: To-Do, Reminder — Task is this glossary's canonical term. Attention Item — considered and rejected as the entity name; see Needs Attention, below, for why the aggregate list and the persisted entity are named separately.

**Needs Attention**:
The always-current list of everything wanting a user's action right now, computed fresh on every view rather than stored anywhere — every open Task (see above), every currently Flagged Transaction, every Transaction that's Cleared but not yet Reconciled, and every Bill Schedule entry that's Overdue, or Due close enough to its due date to fall inside that Bill's own Attention Lead (see Bill, above). Beyond Task, nothing here is a separately stored fact: each is read directly off the Transaction Status, Flagged marker, or Bill Schedule status a row already carries, so an entry disappears the moment its underlying state resolves (a Transaction gets Reconciled, a Bill Schedule entry gets Paid) with nothing to separately dismiss. See `docs/needs-attention.md` and [ADR-0020](docs/adr/0020-needs-attention-as-derived-view.md).
_Avoid_: Attention Item, To-Do List — Needs Attention names the computed list as a whole; Task (see above) is the only thing actually stored inside it.
