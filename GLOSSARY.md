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
the Preferences are the default Unit for new Accounts, Colour Theme, Colour Appearance and date style — all four Ledger-scoped, so they sync across a user's own Clients rather than being set separately on each one. The Colour Theme and Colour Appearance are nullable like the date style ([ADR-0023](docs/adr/0023-colour-theme-preferences-and-theme-role-overrides.md)). The date style is nullable: no value means "use the Locale's
default", and only an explicit choice is stored and synced. The Locale is not a Preference
(see Locale, below). The first Client-scoped Preference is Toasts on/off, default on
([ADR-0027](docs/adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)),
because whether Toasts are welcome depends on the device, not the Ledger.
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

**Toast**:
A short, non-blocking notice a Client shows to report the outcome of an action or event (an Account saved, an import finished, a sync failed). It is the same concept in both Clients; only how it is drawn differs. Distinct from the status-line message, which answers the key or command just typed (an unbound `g`-jump, "not yet built") and clears on the next keypress. The user can turn Toasts off, and the status line can keep the latest Toast's text.
_Avoid_: notification (reserved for anything operating-system level), snackbar, alert, flash.

**Toast Kind**:
The severity a Toast carries, one of Info, Success, Warning or Error. Each Toast Kind is drawn in its own calculated colours.
_Avoid_: level, type, severity (alone).

**Dialog**:
A blocking panel a Client opens over the current view to add, edit, remove or confirm something. While it is open it takes every key; it ends by being confirmed, which applies it, or cancelled, which discards whatever was typed. `Enter` confirms only once the form is valid. The first `Esc` closes an open select inside it; the next cancels the Dialog. Only one Dialog is open at a time. Distinct from a Toast, which never blocks, and from the command palette, which navigates rather than edits.
_Avoid_: modal (as a noun), popup, window.

**Chrome**:
The always-present frame the Desktop Client draws around the current view: the top bar, the rails, the status line, the command palette, Toasts and the host that opens Dialogs. It stays the same whichever view is showing; the view's own content is never Chrome.
_Avoid_: shell (the whole application, not just its frame), frame, layout.

**View**:
The content the Desktop Client shows inside the Chrome for where the user is: one destination's page, or one Settings page. A destination can have more than one View: Transactions shows either the list or an Import.
_Avoid_: screen, page (alone), tab.

**Colour Theme**:
A named set of colours a Client draws its interface in, holding a value for every Colour Role in each of its two Colour Variants (light and dark). A handful ship built into the Clients. The user's chosen Colour Theme and Colour Appearance are Ledger-scoped Preferences, so they sync across the user's own Clients, but a `[theme]` section in a Client's static Configuration overrides individual Colour Roles on that one Client (it cannot choose the Colour Theme or Colour Appearance itself).
_Avoid_: theme (alone), palette — "palette" already names the command palette; "theme" alone is ambiguous with typography and spacing.

**Colour Role**:
One named slot in a Colour Theme (for example accent, foreground, background, selection foreground) that interface code asks for by its purpose, never by a literal colour. Also the key a `[theme]` Configuration override is written against.
_Avoid_: token, swatch, colour variable.

**Colour Variant**:
The light or the dark half of a Colour Theme: a complete value for every Colour Role, chosen by the Colour Appearance.
_Avoid_: Variant (alone), mode, flavour.

**Colour Appearance**:
The user's choice of Light, Dark or System, deciding which Colour Variant of the Colour Theme is shown; System follows the operating system's (or terminal's) own light/dark setting.
_Avoid_: Appearance (alone), mode, dark mode.

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
A canonical, user-visible name for the business, organisation, or individual money moved to or from on a Split (e.g. "Woolworths", an employer), recorded so spending or income can be totalled by who it went to or came from. Optional on a Split (see below) — a Split, not the Transaction as a whole, is what actually carries a Payee. A first-class entity as of [ADR-0012](docs/adr/0012-payee-entity-with-rename-aliases.md) — a Split links to a Payee, rather than storing its name as free text — with its own lifecycle (`is_active`, no hard delete once referenced; an unreferenced Payee may be hard-deleted with its aliases). An inactive Payee keeps its history and stays findable in filters, but its aliases stop matching on import and it is not offered for new Splits. A Payee not yet seen is auto-created the first time its name is entered on a Split; no separate manual "create a Payee" step is required, though one exists for consistency (see `docs/product-requirements.md`, Constraints). A Payee may carry an optional **default Category** (a leaf Category) that pre-fills the Category of *new* Splits given that Payee; changing it never alters existing Splits, and a deactivated default Category still stands but is shown as inactive.
_Avoid_: Vendor, merchant, contact — those imply money only ever flows outward, whereas a Payee can be the source of a Split (e.g. an employer) as well as its destination.

**Payee Alias**:
A string recorded on a Payee that resolves raw text — a bank-statement description on import, or a name typed on a Split — to that Payee: either a former name preserved automatically when the Payee is renamed, or a **match rule** the user adds by hand (e.g. `WOOLWORTHS`, `WW SUPERMARKET`), one per format the Payee appears in. Both kinds behave identically: stored trimmed and upper-cased, unique across all Payees, matched as a case-insensitive *contains* against the raw text, the longest matching alias winning (then Payee name alphabetically), and removable by hand without changing past Splits, which link to the Payee by identity. Aliases are not a search key for Transactions. See [ADR-0012](docs/adr/0012-payee-entity-with-rename-aliases.md) and its amendment.
_Avoid_: Rename, history — a Payee Alias is the *record* a rename leaves behind (or a hand-authored rule), not the act of renaming itself. "Match rule" is the UI name for a Payee Alias, not a separate concept.

**Tag**:
A user-defined, freeform label attached to any number of Splits (see below), independent of their Category and Payee — for cross-cutting totals a fixed, exactly-one classification can't express (e.g. "Japan Trip 2026" spanning several Categories and Payees). Globally unique ignoring case, spaces and punctuation (`work trip`, `Work-Trip` and `work_trip` are the same name), with an optional user-chosen colour (a hex value that is the user's data, not a Colour Theme role), and retired via `is_active` like Unit/Category/Account/Payee (an inactive Tag keeps its history and stays findable in filters, but is not offered for new Splits) — yet, unlike those, always hard-deletable, used or not: removing a Tag untags its Splits and deletes it, never touching their Amounts, Categories or Payees. Two Tags can be merged: every Split carrying the source carries the target instead, and the source is deleted. It has no rename-alias history (unlike Payee) — nothing auto-creates a Tag from parsed import text that would later need reconciling under a rename, so an in-place rename is enough. A Tag's total sums cleanly only when every Split carrying it shares one Unit; a Tag spanning mixed Units reports per-Unit subtotals rather than a summed cross-Unit figure (the Tags page, with one TOTAL cell, reads "mixed units"), the same rule Accounts' type-grouped subtotals already follow (see `docs/ux/tui-mockups/02-accounts/README.md`). See [ADR-0015](docs/adr/0015-tag-as-independent-transaction-label.md).
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
A named view over the one Ledger that tracks spending against a plan (e.g. "Personal spending", "Kitchen reno"), with a Method (see below), exactly one Unit locked at creation (every figure in it is in that Unit), and a set of on-budget Accounts in that Unit. A Budget never owns or copies Transactions: a Split counts in every Budget whose on-budget Accounts and Categories cover it, so Budgets may overlap. Exactly one Budget is the default (a field of the Budget, synced with the Ledger), read by the Dashboard, the Budgets rail badge and first launch; the Budget last opened is a Client-scoped Preference. A Budget is archived, never deleted: an archived Budget keeps its records and computed figures and can be viewed and restored, but is read-only and feeds nothing outside its own screens (no rail badge, no Dashboard); the default Budget cannot be archived until another is made default. A new Account joins no Budget until it is added. Since the default cannot be archived, there is always at least one active Budget; on first run it is Personal spending. A new Budget starts **Empty**, as a **Copy** of another Budget's current-month effective Budget Amounts and Rollover, or from **Last 3 months' spending** (the Fill 3-month average, below, on its chosen Accounts; Categories averaging 0 stay Unbudgeted), each written as Onward Budget Amounts from the current month. Duplicating a Budget copies its Unit, Method, Accounts and full Budget Amount chains and Rollover, under the name "<name> copy", and never as the default. The Ledger's original per-Category limits migrate to a Category limits Budget named **Personal spending** (on all Accounts in its Unit), which the Categories Monthly budget field (5c) reads and writes by its id, whatever it is later renamed; while it is archived, 5c shows its value read-only. See [ADR-0029](docs/adr/0029-budgets-are-named-overlapping-views-over-one-ledger.md).
_Avoid_: Plan (that is a Bill Plan), account group — a Budget is a view, not a partition of the Ledger.

**Method**:
How a Budget (see above) measures spending. **Category limits** — a monthly Category Limit (see below) per Expense Category — is the only Method built so far. **Envelope** (zero-based: every dollar received is assigned to a Category), **Percentage split** (buckets measured against income received) and **Project** (one total over a date range, split into lines) are named Methods deferred to later work.
_Avoid_: Type, kind, strategy.

**Category Limit**:
A cap on the total amount of Splits in one leaf Expense Category each month within one Category limits Budget (see above) (e.g. "$500 per month for Groceries" in Personal spending), counting only Splits in Transactions on that Budget's on-budget Accounts, and compared against that actual total to show how much of the month's amount remains. A Category holds at most one Category Limit per Budget, but may hold one in each of several Budgets. How much it is in any given month is its Budget Amount (see below), which can change over time without rewriting closed months. Only a leaf Category holds a Category Limit; a parent Category shows the sum of its descendants' Category Limits rather than one of its own. Its progress view also carries a Known Costs (Bills) figure (see below) alongside actual spend, but a Category Limit has no direct link to any Bill Plan — the two remain independent entities. See [ADR-0028](docs/adr/0028-budget-amounts-are-effective-dated-and-never-rewrite-closed-months.md).
_Avoid_: Limit alone, allowance, Budget (that is the container) — and Income targets, which are not built.

**Budget Amount**:
The figure a Category Limit (see above) holds for one month, set by effective-dated records that each start in a month, at most one per Category per Budget per month. An **Onward** Budget Amount applies from its month until the next record; a **Month-only** Budget Amount applies to its month alone, after which the Onward value carried forward resumes (or no amount, if none is in effect). An explicit 0.00 is a Budget Amount (any spending is over budget), distinct from having none (Unbudgeted, see below). The current and future months can be set; a closed month's Budget Amount is never rewritten.
_Avoid_: Override, category default, limit — a month's figure is simply its Budget Amount, whether set in that month or carried forward.

**Stop**:
A record that ends a Category Limit's Budget Amounts, in one Budget, from a chosen month (the current month or later): from then on the Category has no Budget Amount and its spending is Unbudgeted, until a later Onward Budget Amount resumes it. Nothing is deleted, and closed months keep their Budget Amounts. The act is "stop budgeting".
_Avoid_: Deactivate, inactive, delete — a Stop is dated and non-destructive.

**Unbudgeted**:
Spending, within a Category limits Budget, in a Category that has no Budget Amount (see above) in that Budget in the month — never budgeted, or Stopped (see above). Always listed alongside the budgeted Categories, never hidden: an unbudgeted Category has a row when it has Spent or Known Costs (Bills) in the month. Its Spent is not rolled up into a parent's Spent (a parent compares its budget against budgeted spend only); the Budget's unbudgeted total is the sum of unbudgeted leaves' Spent, Known Costs excluded.
_Avoid_: Uncategorised — that is a system-provided Category, not a Category with no Budget Amount.

**Spent**:
Within a Budget, the signed sum of Splits in an Expense Category (a leaf, or a parent's budgeted descendants) whose Transaction is on one of the Budget's on-budget Accounts and dated in the month, whatever its Transaction Status (Open, Cleared or Reconciled). Refunds net off, so Spent can be negative; non-Expense Splits (including transfers) never count. **Left** is the effective budget (Budget Amount plus Rollover carry) minus Spent minus Known Costs (Bills); the current month shows Left per day over the days remaining including today, a future month over all its days, a past month none. The **elapsed** share is days elapsed including today over days in the month (past 100%, future 0%). All of a Budget's period figures come from one shared query that Progress, the Category detail, the Budgets rail badge, the Dashboard and Budget vs Actual (which ignores Known Costs) all read.
_Avoid_: Actual, used.

**Over budget**:
A budgeted leaf Category whose Spent exceeds its effective budget in the month (an explicit 0.00 with any spend is over); the over amount is Spent minus effective budget, Known Costs excluded. Only leaves are counted for the Budgets rail badge and the "N over budget" header; a parent is shown over when its rollup is, but never counted. A Category that is not over but whose Left is negative only because of Known Costs (Bills) is **at risk**: shown, never counted.
_Avoid_: Overspent when meaning at risk.

**Rollover**:
Whether a Category Limit's (see above) closed month carries into the next month's effective budget: **None**, **Carry unspent**, or **Carry unspent & overspend**. The carry is the month's effective budget (Budget Amount plus carry in) minus Spent — Known Costs excluded — so it compounds without a cap, and with overspend carried an effective budget can go negative. Only a closed month carries; the current month never does. The setting is held on each Budget Amount record (a new record inherits the Rollover of the record it follows, else None), so changing it never rewrites closed months. A Stop or a month with no Budget Amount resets the carry to zero. Carries are applied on read, never written into a Budget Amount, recomputed from the Budget's current on-budget Accounts and Category tree, and roll up into parent rows.
_Avoid_: Carryover, envelope balance — Rollover is this glossary's canonical term.

**Fill**:
Setting one open month's Budget Amounts (see above) in a Category limits Budget in one step from a source: **copy the previous month as budgeted**, or the **3-month average spent** — the Spent (see above) of the three closed months before the target, on the Budget's current on-budget Accounts, Known Costs excluded, rounded to the nearest 10 half away from zero, negatives floored at 0.00. Fill writes Month-only Budget Amounts for the target month only, one per leaf Category budgeted in the month before it (an unbudgeted Category is never made budgeted, and a budgeted one with no spend gets 0.00), and never overwrites a Category that already has its own record (Onward, Month-only or Stop) in the target month. The plan (Budget Amounts carried forward) is the baseline its preview diffs against, not a source. A new Budget's "Last 3 months' spending" start uses the same average.
_Avoid_: Auto-budget, copy forward — Fill is this glossary's canonical term.

**Budget History**:
A Category limits Budget's month-by-month record of each budgeted leaf Category's effective budget (Budget Amount plus Rollover carry, as recorded) against its Spent, Known Costs (Bills) excluded, both recomputed from the Budget's current on-budget Accounts and Category tree. Its average and over count use closed months in which the Category was budgeted, never the current month. It reaches back to the Budget's first Budget Amount and forward to the current month; future months belong to the plan.
_Avoid_: Budget vs Actual — that is Reports 10d.

**Bill Plan**:
The recurring definition of an expected payment obligation (e.g. "Telstra internet, $89/month") — a schedule and a set of defaults, not itself a record of money moving. Carries exactly one (Expense-type) Category, one Unit, one Account it's expected to be paid from, an optional Payee, an estimated Amount, a flag marking that Amount as Fixed or Estimated, a Recurrence (Weekly, Fortnightly, Monthly, Quarterly, Annually, or One-shot for a single non-repeating due date), a First Due date that anchors the Recurrence (every due date is stepped from it: 7 or 14 days for Weekly and Fortnightly, or its day of the month for Monthly, Quarterly and Annually, clamped to the month's last day when the month is shorter but always computed from the anchor, so 31 Jan gives 28/29 Feb then 31 Mar; a One-shot's First Due is its only due date), an optional inclusive Ends On date after which no further Bill Schedule entries (see below) are generated (never earlier than First Due, and not offered for a One-shot), and an optional Attention Lead (a day count, unset by default and then treated as zero) that brings an unresolved Bill Schedule entry into Needs Attention (see below) that many days before its due date, whatever its status and even across a month boundary — set per Bill Plan rather than as a single global setting, since how early a Bill deserves attention varies by Bill Plan (rent versus a small subscription). Independent of Budget (see above) — a Bill Plan may happen to share a Category with a Budget, but there is no direct link between the two entities. See `docs/bills.md` and [ADR-0019](docs/adr/0019-materialized-bill-schedule-transaction-linked.md).
_Avoid_: Bill — use it only loosely for the feature as a whole (the Bills surface), never for this entity. Bill Planner — the Planner tab of the Bills surface, not the entity itself. Recurring Bill, Scheduled Bill — Bill Plan is this glossary's canonical term.

**Bill Schedule**:
One dated instance of a Bill Plan (see above), generated ahead of time from the Bill Plan's Recurrence so it exists as a real row with its own stable identity (`RowID`) for a Transaction to reference — a Monthly Bill has many Bill Schedule entries, one per due date, the same way a Loan Account's interest rate is tracked as a history of dated entries rather than one row rewritten in place. Carries its own status — Upcoming (due date in a future calendar month), Due (due date within the current calendar month, up to and including the due day), Overdue (due day passed, still unpaid — an Overdue entry from an earlier month is also carried into the current month's Bill Schedule view until resolved, while still showing in its own month), Paid (Matched to a Split), or Skipped (deliberately not paid, whether a recurring cycle skipped or a One-shot Bill Plan cancelled) — and, once Paid, a link to the one Split that settled it, with that Split's amount and its Transaction's date always taking precedence over the Bill Plan's estimate. To **Match** is the act of tying an entry to a Split, one-to-one — by Pay (creating a Transaction from the Bill Plan's defaults) or by Match (choosing an existing unmatched Split); Paid is undone by Unmatch and Skipped by Unskip, and deleting a Matched Split Unmatches its entry. A Transaction with a Matched Split is understood to "carry a Bill" wherever it's displayed — a derived fact of the link existing, not a stored field, and unrelated to a Transaction's separate Flagged marker (see below). "Last paid," average, and same-time-last-year figures are always derived live from Paid entries' Matched-Split amounts; Skipped entries are excluded entirely rather than counted as zero. Entries are generated through the end of next calendar month (always at least one future entry per active Bill Plan), each with an id derived from its Bill Plan and due date so every Client generates the same row. An unresolved entry reads its amount, Account, Payee and Category live from its Bill Plan. Every Bill Schedule entry is retained permanently — an edit to the Bill Plan's Recurrence, First Due or Ends On, or deactivating it, marks unresolved, unlinked entries due today or later as superseded (hidden, never deleted); reaching its end date stops new entries from being generated but never removes past ones — so Bill Schedule doubles as the filterable historical record of what was due, paid, skipped, or missed. See `docs/bills.md` and [ADR-0019](docs/adr/0019-materialized-bill-schedule-transaction-linked.md).
_Avoid_: Bill Occurrence — an earlier working name for this same concept, superseded by Bill Schedule. Link, Merge — Match is the canonical term for settling an entry; Merge belongs to Tags.

**Anticipated Bill**:
A shaded, non-persisted preview row shown inline in a Bill Plan's target Account's transaction list, at the due date of its next unresolved Bill Schedule entry (see above) — a placeholder standing in for money that hasn't moved yet. Replaced entirely by the real Transaction once that Bill Schedule entry resolves to Paid (by creating a new Transaction or linking an existing one); the two never coexist as separate rows for the same Bill Schedule entry.
_Avoid_: Anticipated Transaction, Ghost Transaction — Anticipated Bill is this glossary's canonical term, and it is explicitly not a Transaction (see above) until it resolves into one.

**Known Costs (Bills)**:
A section of a Category Limit's (see above) progress view totalling the estimated Amount of every unpaid Bill Schedule entry (see above) matching its Category and paid from one of the Budget's on-budget Accounts, in the month shown: for the current month, its Due entries plus Overdue entries carried in from earlier months (counted in the current month only, never again in their own month); for a past month, zero (its unpaid entries are now Overdue and carried forward); for a future month, the unpaid entries due that month, as far as entries have been generated. A forecast of committed spending shown alongside its existing actual-spend figure. An Overdue entry stays counted in this total (it is still unpaid, committed spending) but is visually distinguished from merely-Due entries. Once a Bill Schedule entry is Paid, its linked Transaction's actual amount flows into the ordinary spend total instead, so it is never counted in both places. An unbudgeted Category still carries its Known Costs. Every Budget surface reads these figures through one shared period-figures query (see Spent).
_Avoid_: Committed spend, forecast — Known Costs (Bills) is this glossary's canonical term for this figure.

**Task**:
A user-created, free-text note for a general money-related to-do (e.g. "call the bank about this charge") — the one kind of Needs Attention entry (see below) a user creates directly, rather than one the Ledger surfaces on its own. Optionally links to exactly one Transaction for context, independent of that Transaction's Flagged marker (see above) — linking a Task doesn't require the Transaction to be Flagged, and Flagging a Transaction doesn't create a Task. Moves through exactly two states, Open and Done; a Done Task's row is kept, not deleted, the same soft-delete convention as Account/Category/Payee/Tag/Bill Plan's `is_active`. See `docs/needs-attention.md`.
_Avoid_: To-Do, Reminder — Task is this glossary's canonical term. Attention Item — considered and rejected as the entity name; see Needs Attention, below, for why the aggregate list and the persisted entity are named separately.

**Needs Attention**:
The always-current list of everything wanting a user's action right now, computed fresh on every view rather than stored anywhere — every open Task (see above), every currently Flagged Transaction, every Transaction that's Cleared but not yet Reconciled, and every unresolved (not Paid or Skipped) Bill Schedule entry whose due date is on or before today plus that Bill Plan's own Attention Lead (see Bill Plan, above) — a date rule, not a status one, so an Upcoming entry due early next month still counts once inside its lead, and one with no lead counts from its due day. Beyond Task, nothing here is a separately stored fact: each is read directly off the Transaction Status, Flagged marker, or Bill Schedule status a row already carries, so an entry disappears the moment its underlying state resolves (a Transaction gets Reconciled, a Bill Schedule entry gets Paid) with nothing to separately dismiss. See `docs/needs-attention.md` and [ADR-0020](docs/adr/0020-needs-attention-as-derived-view.md).
_Avoid_: Attention Item, To-Do List — Needs Attention names the computed list as a whole; Task (see above) is the only thing actually stored inside it.

**Property**:
A place whose contents are kept as one Inventory register (e.g. a house, a storage unit), holding any number of Rooms. Property names are unique. A user may keep several Properties, each managed under Settings › Inventory and switched between on the Inventory surface. Every Property has exactly one fiat Unit, fixed when it is added, in which its insurance cover and every value of its Inventory Items are denominated. A Property may carry the details of its own insurance cover (insurer, policy number, renewal date, sum insured, single-item limit) or none; the cover is a detail of the Property, not a record of its own, so two Properties under one policy each record it separately. Removing a Property deletes its Rooms and Inventory Items and drops their Document Links, but never a Document or a Transaction; the last Property may be removed. In this glossary Property always means a contents location, never real estate as an Asset.
_Avoid_: Inventory (for the place rather than the feature), Register — the Inventory surface's view of one Property, not a separate thing. Premises, Location.

**Room**:
A named area within exactly one Property (e.g. Kitchen, Garage, Outdoor & shed), in a user-set order that is the order of that Property's room tabs on the Inventory surface: one tab per Room, never grouped. Every Inventory Item sits in one Room. A Room never changes Property. Names are unique within a Property ignoring case; two Properties may each have a Room of the same name. A Property starts with no Rooms and may have none, but then holds no Items. There is no fallback Room: removing a Room that holds Items moves them to another Room of the same Property, so a Property's last Room cannot be removed while it holds Items.
_Avoid_: Unassigned, Other — there is no catch-all Room.

**Inventory Item**:
A physical possession recorded for its value and as something a Document can prove ownership of (e.g. a camera, a ring, a household's contents), belonging to one Property and sitting in one of its Rooms.
_Avoid_: Asset — too broad, and it collides with the Assets Category Type.

**Document**:
A reference to one file kept beside the Ledger, plus its metadata: a Title (defaulting to the file name without its extension, editable), exactly one Document Type, a document date, an optional Key Date and any number of Document Links. The file itself is never renamed or moved; the Ledger holds only the reference. A Document is either **Unfiled** (in the Inbox, carrying the Extracted Facts read from it and the source it arrived from) or **Filed**; it becomes Filed only by the user filing it, never by gaining a Link, so a Filed Document with no Links ("not linked") is valid.
_Avoid_: Attachment, File (for the Ledger record rather than the bytes on disk), Inbox Item — an Inbox entry is just an Unfiled Document, not a separate thing.

**Document Type**:
A user-managed kind of Document, kept as Ledger data in an order the user sets; every Filed Document has exactly one. A new Ledger is seeded with Receipts, Statements, Tax, Insurance, Warranties & manuals, Contracts, Identity, Bills and **Other**; only Other is special afterwards, as the **Default Document Type**: exactly one type, it can never be removed, it is the fallback wherever a type is needed (Accept with no extracted type, the Add Document form) and the pre-selected destination when another type is removed. Names are unique ignoring case; a type has a stable id, so renaming never changes saved filters. There are no subtypes; any finer distinction ("policy", "PDS") belongs in the Title.
_Avoid_: Category — Category classifies Splits, not Documents. Unsorted, Uncategorised — clash with Unfiled and Category. Tag — a Document Type is exactly one per Document, not a free label.

**Financial Year**:
The twelve-month year a date falls in, beginning on the month set as the Financial year start (1 July by default, the Australian financial year), labelled by both calendar years it spans (e.g. FY 2025–26). A Document's Financial Year is always derived from its document date, never stored, so every Document has one, whatever its Document Type. A Document Type's **Financial year** flag decides whether the type is treated as year-bound in the UI: the Financial year filter matches only flag-on types, the detail pane shows the Financial Year fact only for them, and the Inbox names a year only when the extracted type has the flag. Toggling the flag changes what is shown, never the derived value.
_Avoid_: Tax year — the same period under another name; Financial Year is canonical, including for the Document Type flag, the Documents filter and the Settings column.

**Key Date**:
The one forward-dated obligation a Document may carry (zero or one per Document): a date. Its kind — Renews, Ends (also used for a warranty's end), Expires or Revalue — is set by the Document's Document Type (its **Tracks date**), never by the Document; a type that tracks no date gives its Documents no Key Date. The date is optional even where the type tracks one. If the type later changes kind or stops tracking a date, existing dates are kept: the kind relabels, or the date goes inert (no Expiring sort, no Need Review) until the type tracks a date again. Whether it is reminded is also set by the type, not the Document: the type's **Remind** lead time (none, 7, 14, 30 or 60 days, 3 or 6 months) gives a derived, never-stored **Reminder due on** date (Key Date minus lead time) that a future Notifications model reads. Remind is inert when the type tracks no date, and a lead time changed later applies to every Document of the type; a reminder date already passed is treated as missed, not fired. Remind does not change Need Review. Drives the Documents surface's Expiring sort and Need Review count; it is not part of Needs Attention.
_Avoid_: Due date — that belongs to a Bill Schedule entry. Reminder flag on the Document — the lead time belongs to the Document Type.

**Need Review**:
A Filed Document whose Key Date falls within 60 days either side of today, inclusive: soon due, or recently lapsed. A Key Date further ahead is **Upcoming**; one lapsed for longer is **Stale**, and neither needs review. Derived each time it is shown, never stored or dismissed, and local to the Documents surface.
_Avoid_: Needs Attention — that is the Ledger-wide list, which Documents do not join. Expiring — the name of the sort, not of the state.

**Document Link**:
A Document's reference to one record it proves: a Transaction (the whole Transaction, never a single Split), an Inventory Item, an Account, a Payee or a Bill Plan (never a Bill Schedule entry; proof for one paid instance links to the Transaction that paid it). Many-to-many: a Document has any number of Document Links and a record may be linked from any number of Documents.
_Avoid_: Attachment, Match — Match is the Bills term for settling a Bill Schedule entry against a Split.

**Inbox**:
The list of every Unfiled Document, waiting to be filed. Its count is derived, never stored.
_Avoid_: Queue, Unsorted.

**Extracted Facts**:
What was read from an Unfiled Document's file — merchant, date, total and Document Type (only ever one of the Ledger's own Document Types, matched by name), any of which may be absent — and which the user may correct before filing. A Document from which no amount could be read is **Unreadable** and is never given a Suggested Link.
_Avoid_: Metadata — a Document's metadata is what it carries once Filed; Extracted Facts are the raw reading.

**Suggested Link**:
The best candidate Document Link for an Unfiled Document, derived from its Extracted Facts each time it is shown and never stored. Only a Transaction can be a candidate, and only one that agrees on amount or payee. Its **Signals** are which of amount, date and payee agree with the candidate, and its **Signal Strength** is how many agree: Strong (3 of 3), Likely (2) or Weak (1). A readable Document with no candidate has no Suggested Link and is filed by hand.
_Avoid_: Match, Match Strength — Match belongs to Bills.

**File** (verb):
To move a Document from Unfiled to Filed, setting its Document Type and any Document Links. **Accept** is filing with the Suggested Link as offered; filing by hand or linking elsewhere is still filing. **Skip** leaves a Document Unfiled, set aside to the bottom of the Inbox for the session.
_Avoid_: Import — Import brings a file into the Inbox; filing is what happens after.

**Git Backup**:
A Client's scheduled commit of the Ledger to a remote Git repository: a deterministic text dump of the Ledger data plus its Document files, pushed only to a private repository. It is Configuration on one Client, never synced, and independent of the Sync Server. See [ADR-0033](docs/adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md).
_Avoid_: Snapshot (the local Backup's term), Sync — a Git Backup is a one-way copy, never read back except to restore.

**Model Provider**:
A language-model service a Client sends a request to, such as Claude, OpenAI or a local Ollama, set up on that one Client with its key held in the OS keychain. One Model Provider per Client is the default. See [ADR-0034](docs/adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md).
_Avoid_: AI, LLM, model — a model is the one a Model Provider serves, chosen per Model Provider.

**Assistant**:
An external AI agent, such as Claude Desktop or Claude Code, connected to a Client's local MCP server with its own revocable token. It has read-only access to the Ledger. Per Client, never synced. See [ADR-0034](docs/adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md).
_Avoid_: Agent, bot, Model Provider — the Client calls out to a Model Provider, while an Assistant calls in to the Client.

**Search**:
Narrowing a list by **free text** the user types (a query matched against names or labels). Distinct from a Filter: Search takes one typed string and has no structured fields.
_Avoid_: Filter (for free-text matching), Find.

**Filter**:
Narrowing a list by **structured criteria** chosen from known values (e.g. a Transactions view limited to an Account, Payee, Tag or date range), not by a typed string. Distinct from Search.
_Avoid_: Search (for structured criteria).
