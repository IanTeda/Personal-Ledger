Product Requirements Document
Product Name: Personal Ledger
Version: v0.0.1 (Draft — End-State Requirements Pool; active cycle scope defined in §2.1)
Date: 3 September 2026
Author: Ian Teda

This document owns the product vision, scope, and requirements. Domain vocabulary (precise definitions of terms like "ledger", "category", "account") is defined in `CONTEXT.md`. This PRD references domain terms without formally defining them.

# 1. Introduction

## 1.1 Product Vision

Personal Ledger aims to provide users with an intuitive, user-friendly interface that offers insights into their spending habits and their investment and asset position. By presenting information clearly and concisely, users can effectively track and monitor their overall financial health, make informed decisions, and ultimately achieve their personal financial goals without handing their financial data to a third party.

## 1.2 Problem Statement

Creating a market for paid personal finance tools is challenging because most individuals interested in tracking their finances are unlikely to pay a $100 annual subscription fee. As a result, personal finance tools often pivot toward business finance tools, since businesses are generally willing to spend significantly more than individuals on monitoring and tracking their finances. 

This means personal finance tools often have to be free, which limits active development and features. I have tried many personal finance tools and found them outdated, unappealing, or based on a tracking model I do not necessarily agree with or want to use because it lacks a particular feature or aspect.

Personal Ledger aims to bridge the gap by providing a self-hosted, local-first ledger that lets the owner fully control and own their data. 

## 1.3 Target Audience

Personal Ledger v0.0.1 targets a single usage persona, but future development cycles will expand this to couples, families, and groups.
Individuals: Individuals who are looking to track and understand their personal financial position
Family, Couples & Groups: People with shared finances looking to understand together and set goals and targets. This is out of scope for v0.0.1 (see [Future Considerations](#7-future-considerations--roadmap)).

## 1.4 Goals & Objectives

The overarching goals and objectives describe the product's ultimate end state. These will be targeted in various degrees across the development cycle or phase. These goals and objectives include the following:

- __Desktop App:__ A desktop native app that works across Linux, macOS, and Windows.
- __TUI App:__ A text-based TUI for a keyboard-focused native app that works across Linux, macOS and Windows.
- __Sync Server App:__ A headless service, deployable via Docker on a homelab, that acts as the always-on sync hub for the native (Desktop/TUI) apps.
- __Local Offline First:__ Provide a self-hosted, offline, local-first ledger for tracking expenses, investments, and assets without a cloud dependency or subscription.
- __Multiple Operating Systems and Devices:__ Support multiple operating systems and devices, each holding its own local-first data store, synced via a sync server when devices come online.
- __Transactions:__ Model transactions against accounts, payees and categories using the five standard accounting categories (asset, liability, equity, income, expense) to classify transactions, without requiring double-entry bookkeeping.
- __Understand Spending:__ Users interested in gaining insight and understanding their spending habits now and over time to make informed decisions.
- __Track Against Budget:__ Users interested in tracking spending against a target budget to achieve financial goals.
- __Reconcile Accounts:__ Users can verify their accounts and transactions against reality through reconciliation of account ledgers.
- __Personal Investors:__ Users who need to track personal investments, understand buy-in costs, capital gains, returns, and tax implications.
- __Personal Loan:__ Users who want to track the progress of paying down a loan.
- __Personal Inventory:__ Users who wish to track their assets and inventory, such as household goods.

## 2. Scope

Four development cycles are planned for this product, each with its own Git branch within the repository; a later cycle's branch supersedes the requirements captured in an earlier one as work is folded forward. Each cycle has overarching aims, limitations and objectives that are described below:

- __Feasibility (v0.0.1):__ This phase will focus on demonstrating the underlying approach and technologies and their suitability in achieving the desired outcomes.
- __Concept (v0.1.0):__ This phase will aim to develop a minimum viable product that demonstrates use cases and functionality.
- __Development (v1.0.0):__ This phase will expand functionality and improve the user experience. At the same time, it will refine the code and security aspects.
- __Fixes & Features (v1.0.1):__ This phase will address any lessons learnt and pain points in using the tool. It will also add features and functionality to enhance the tool's usefulness.

Sections 3–6 hold the product's ultimate end-state requirements — the full functional and non-functional scope across all four cycles, not what any single cycle delivers. As each cycle is scoped, the Functional Requirements (§3) relevant to it are moved out of that pool and into that cycle's own subsection below (e.g. §2.2), keeping their original FR ID for traceability. Non-Functional Requirements, Dependencies, and Assumptions & Constraints (§4–6) remain global across all cycles rather than being moved. Items still listed in §3 have not yet been assigned to a cycle. Each cycle subsection can therefore hold two kinds of items: FR.* requirements moved in from §3, and items native to that cycle's own investigation/delivery work (e.g. FC-* in §2.1) — the two ID namespaces coexist rather than one replacing the other.

## 2.1. Feasibility Cycle (v0.0.1)

The feasibility cycle demonstrates the underlying technologies and approach, specifically:

### Desktop App:

- __FC-DESKTOP-001:__ Investigate and research Rust desktop GUI libraries.
- __FC-DESKTOP-002:__ Demonstrate that line, doughnut, candle stick and divergent graphs work in the desktop app across platforms, as they are a key requirement for visually representing spending, etc.
- __FC-DESKTOP-003:__ Demonstrate that tables work in the desktop app across platforms.
- __FC-DESKTOP-004:__ Demonstrate compiling, installing and running as non-root across Windows, macOS and Linux

### TUI App:

- __FC-TUI-001:__ Investigate and research Rust TUI libraries, and lock the choice in an ADR.
- __FC-TUI-002:__ Demonstrate that line, doughnut, candle stick and divergent graphs each work in the TUI app across platforms, using dummy data — one demonstration per chart type.
- __FC-TUI-003:__ Demonstrate that tables work in the TUI app across platforms.
- __FC-TUI-004:__ Demonstrate compiling, installing and running as non-root across Windows, macOS and Linux — a portable single executable on Windows (no installer, no elevation), and a real user-scope installer/package on macOS (`.dmg`/`.app`) and Linux (`.deb` and/or AppImage). Research and lock the packaging tool choice (e.g. `cargo-dist`, `cargo-packager`) in an ADR before building the per-OS packages.
- __FC-TUI-005:__ Demonstrate the TUI app operating end-to-end against the embedded SQLite persistence layer (`lib-database`/`lib-domain`, see FC-DATA-001) with real (not dummy) data, proving the local-first architecture works through a real client.

### Sync Server App:

- __FC-SYNC-001:__ Investigate and research approaches for syncing independent local SQLite copies across devices (e.g. last-write-wins vs. CRDT vs. manual merge).
- __FC-SYNC-002:__ Investigate and research authentication mechanisms (e.g. OAuth2, JWT, etc.) and implement a secure auth flow for the sync server and client.
- __FC-SYNC-003:__ Demonstrate basic push/pull sync of ledger changes between two local SQLite instances via the sync server, including a Client that was offline catching up on changes queued while it was down.
- __FC-SYNC-004:__ Demonstrate the Sync Server instance acting as the always-on sync hub that Desktop/TUI clients sync through.
- __FC-SYNC-005:__ Demonstrate the Sync Server's Dockerfile builds a multi-arch image (linux/amd64, linux/arm64) that runs correctly on each architecture.
- __FC-SYNC-006:__ Demonstrate docker Compose deployment with a persistent volume for the Sync Server's own durable store, surviving a container restart, and confirm the container runs as a non-root user.
- __FC-SYNC-007:__ Demonstrate auth functionality.

### Local Data:

- __FC-DATA-001:__ Demonstrate an embedded SQLite persistence layer (`lib-database`, `lib-domain`) that each client (Desktop, TUI) can hold and operate against independently, without requiring a network connection.
- __FC-DATA-002:__ Research the best and most efficient way to store and calculate running Balances.

### 2.2. Concept Cycle (v0.1.0)

#### 2.2.1 All

Implement these requirements to bring the TUI, Desktop and Sync up to functionality

- __CC-ALL-001:__ Ensure all dependencies are up-to-date and cross-compilation is enabled and compiles on all platforms.
- __CC-ALL-002:__ Refactor lib_domain into lib_core (mechanical rename — no new shared responsibilities identified beyond today's lib_domain scope; revisit if real shared logic emerges once CC-TUI-005+/CC-DESKTOP-005+ build real CRUD).
- __CC-ALL-003:__ ~~Research depreciated Chrono crate and replace it~~ — resolved: chrono is not deprecated (`0.4.45`, actively maintained, one old advisory fixed years ago); `jiff` was ruled out because `sqlx-sqlite` has no `jiff` feature. Keeping chrono, no ADR needed (status quo confirmed, not a real trade-off). See `docs/research/chrono-alternatives.md` (branch `research/chrono-alternatives`).
- __CC-ALL-004:__ Review, research and grill the workspace code architecture and structure for best practice, readability and maintainability.
- __CC-ALL-006:__ Wire `lib_telemetry::init` into `bin-tui` and `bin-desktop` (already done for `bin-sync-server`); confirm telemetry implementation is compatible across binaries and with cross-compilation.
- __CC-ALL-007:__ Research and decide on SQLite encryption-at-rest (see `docs/research/sqlite-encryption.md`); the choice of SQLite itself is already settled (§5 Dependencies, `lib-database`).

#### 2.2.2 TUI App:

- __CC-TUI-001:__ Research and decide on screens and user flows for the TUI app. How are we going to do applicatoin settings?
- __CC-TUI-002:__ Build out a TUI app using the TUI framework.
- __CC-TUI-003:__ Research and decide on keybinding and navigation/workflow for the TUI app.
- __CC-TUI-004:__ Research and decide on the TUI app's data model and persistence layer.
- __CC-TUI-005:__ Build out the units funcationality.
- __CC-TUI-007:__ Build out the payee functionality.
- __CC-TUI-006:__ Build out the categories functionality.
- __CC-TUI-008:__ Build out the accounts functionality.
- __CC-TUI-009:__ Build out the transaction functionality.
- __CC-TUI-010:__ Build out the budgeting functionality using line-item budgeting only (matching `CONTEXT.md`'s existing Budget definition — a cap on one Category, not a full allocation); envelope and reverse budgeting are deferred to Future Considerations (§7).
- __CC-TUI-011:__ Build out the reporting functionality.
- __CC-TUI-012:__ Build out the CSV import/export functionality.

#### 2.2.3 Desktop App:

Mirrors §2.2.2, built in parallel with the TUI cycle rather than sequenced after it — both clients share `lib-database`/`lib-domain`, and `bin-desktop` already has a feasibility-cycle scaffold (GPUI, ADR-0007/0008) as mature as `bin-tui`'s.

- __CC-DESKTOP-001:__ Research and decide on screens and user flows for the Desktop app, including how the Preference model from CC-TUI-001 is shared or diverges for Desktop (see `CONTEXT.md` — Preference).
- __CC-DESKTOP-002:__ Replace the feasibility cycle's dummy-data chart/table demo (`bin-desktop/src/main.rs`) with live screens backed by `lib-database`.
- __CC-DESKTOP-003:__ Research and decide on navigation/workflow (menus, keyboard shortcuts) for the Desktop app.
- __CC-DESKTOP-004:__ Reuse the TUI's data model and persistence layer decision (CC-TUI-004) — no separate research needed unless Desktop surfaces a real gap.
- __CC-DESKTOP-005:__ Build out the units functionality.
- __CC-DESKTOP-006:__ Build out the categories functionality.
- __CC-DESKTOP-007:__ Build out the payee functionality.
- __CC-DESKTOP-008:__ Build out the accounts functionality.
- __CC-DESKTOP-009:__ Build out the transaction functionality.
- __CC-DESKTOP-010:__ Build out the budgeting functionality (line-item only — see CC-TUI-010).
- __CC-DESKTOP-011:__ Build out the reporting functionality.
- __CC-DESKTOP-012:__ Build out the CSV import/export functionality.


### 2.3. Development Cycle (v1.0.0)

This will be defined before starting the development cycle.

- __DC-ALL-001:__ Password string in the configuration file should be an encrypted string. On loading the app, if the string is plain text, it should be encrypted and saved to the configuration file before starting the app.

### 2.4. Fixes & Features Cycle (v1.0.1)

This will be defined before starting the concept development cycle.

## 3. Functional Requirements

The requirements below describe the product's ultimate end state across all development cycles (§2). They are not all in scope for any single cycle; as each cycle (§2.2–2.4) is scoped, its relevant items are moved out of this section and into that cycle's own requirements list above, keeping their original FR ID. Items still listed here have not yet been assigned to a cycle.

### Units

- __FR.1:__ The system shall allow for multiple different Units like currencies, crypto, stocks, equities, precious metals, etc.
- __FR.2:__ When a transaction is between different Units, it should include an exchange rate.
- __FR.3:__ Unit prices will be stored locally as part of a transaction exchange and every week to avoid excessive granularity. If more granularity is needed, it can be queried from the internet dynamically.

### Categories

- __FR.4:__ The system shall allow creating a category with a code, name, optional description, optional URL slug, one of the five accounting types, optional colour, and optional icon.
- __FR.5:__ The system shall allow retrieving a category by ID, by code, or by slug.
- __FR.6:__ The system shall allow listing categories with pagination, filtering by type and/or active status, and sorting.
- __FR.7:__ The system shall allow partially updating a category via a field mask.

### Accounts

- __FR.8:__ The system shall allow deleting a category, individually or in batch.
- __FR.9:__ The system shall allow activating and deactivating a category.
- __FR.10:__ The system shall allow creating an account with a name, a type (Cash, Bank, Credit Card, Investment, or Loan), a Unit it's denominated in (fixed at creation), and a starting balance in that Unit.
- __FR.11:__ The system shall allow retrieving an account by id.
- __FR.12:__ The system shall allow listing accounts with pagination and filtering by type and/or active status.
- __FR.13:__ The system shall allow updating an account's name, type, or active status.
- __FR.14:__ The system shall allow deleting an account. On deleting an account, there will be an option to transfer all the transactions under an account to another account.
- __FR.15:__ The system shall allow the merging of two accounts into one.
- __FR.16:__ The system shall allow creating a single-entry transaction with a UUIDv7, date, an amount, exactly one Category, exactly one account, an optional payee (free text), an optional description, optional ID, a Transaction Status (Open, Cleared, or Reconciled — defaulting to Open), and an independent Flagged marker (defaulting to unset), and shall update the linked account's running Balance accordingly.
- __FR.17:__ The system shall allow retrieving a transaction by id.
- __FR.18:__ The system shall allow listing transactions with pagination, filtering by account, Category, payee (exact match), Transaction Status, Flagged state, and/or date range, and sorting.
- __FR.19:__ The system shall allow updating a transaction — including its Transaction Status and Flagged marker, each settable independently of the other — and shall adjust the affected account balance(s) accordingly, including moving a transaction between accounts that share the same Unit (moving it to an account with a different Unit is rejected — see Constraints). Status and Flagged changes are always explicit user actions; the system never infers or auto-applies them. A Reconciled transaction's other fields cannot be updated until its Transaction Status is first moved back to Open or Cleared (see Constraints).
- __FR.20:__ The system shall allow deleting a transaction, and shall reverse its effect on the linked account's Balance.
- __FR.21:__ The UUIDv7 will be used to determine the transaction creation date.

### Budgets

- __FR.22:__ The system shall support line item budgeting (see CC-TUI-010/CC-DESKTOP-010 for the concept cycle's scoping of this to line-item only). Envelope and reverse budgeting are future considerations (§7).
- __FR.23:__ The system shall allow creating a budget with exactly one Category, a limit amount, a Unit it's denominated in, and a recurring period (weekly, monthly, quarterly, or yearly).
- __FR.24:__ The system shall allow retrieving a budget by ID.
- __FR.25:__ The system shall allow listing budgets with pagination and filtering by Category and/or active status.
- __FR.26:__ The system shall allow updating a budget's limit amount, period, or active status.
- __FR.27:__ The system shall allow deleting a budget.

### Balance Checks

- __FR.28:__ The system shall allow creating a Balance Check with exactly one account, a date, and an asserted balance amount in that account's Unit, entered manually.
- __FR.29:__ The system shall allow retrieving a Balance Check by id.
- __FR.30:__ The system shall allow listing Balance Checks with pagination and filtering by account and/or date range.
- __FR.31:__ The system shall allow updating a Balance Check's date or asserted balance amount.
- __FR.32:__ The system shall allow deleting a Balance Check.
- __FR.33:__ The system shall allow importing Balance Checks for one account from a CSV file containing a date column and a balance column, creating one Balance Check per row; a malformed row shall abort the entire import rather than partially applying it.

### Reporting

- __FR.34:__ The system shall report the current Balance of a given account, or of all accounts, each expressed in its own Unit (no cross-Unit aggregation).
- __FR.35:__ The system shall report the sum of transaction amounts per Category over a given date range, scoped to a single Unit (or account) at a time.
- __FR.36:__ The system shall report the sum of transaction amounts per payee over a given date range, scoped to a single Unit (or account) at a time, matching payees by exact text (see Constraints — no payee normalisation in V1).
- __FR.37:__ The system shall report, for a given budget, the limit amount, the actual spending in its Category and Unit for the current period, and the amount remaining (or the amount over, if exceeded).
- __FR.38:__ The system shall report, for a given Balance Check, the difference between its asserted amount and its account's computed Balance as of that date.

### Platform

- __FR.39:__ The system shall expose Category, Account, Transaction, Budget, and Balance Check operations (FR.4–FR.38) via a local embedded library API (`lib-database`/`lib-domain`), called in-process by each client (Desktop, TUI) against its own local SQLite store.
- __FR.39a:__ The sync server shall expose a versioned gRPC sync protocol for syncing a client's local ledger data with other devices — pushing and pulling change sets rather than exposing full CRUD — and each client shall be able to operate entirely offline against its local store between syncs. The Sync Server persists a durable log of change sets so a Client that has been offline can catch up without both peers being online simultaneously.
- __FR.40:__ The system shall support layered configuration (defaults, system, user, executable-directory, working-directory, explicit path, environment variables).
- __FR.41:__ The system shall emit structured, level-configurable tracing output.

## 4. Non-Functional Requirements

- __NFR.1 Reliability (priority):__ Transaction creation, update, and deletion must keep an account's running Balance consistent with its recorded transactions, even on partial failure (no orphaned balance updates). A Balance Check CSV import is atomic: a malformed row aborts the whole import rather than partially applying it. Database migrations must be safe to re-run.
- __NFR.2 Security & Privacy (priority):__ All data is stored locally in SQLite; no data leaves the device by default. The app never requires root/administrator privileges to run. No `unsafe` code anywhere in the workspace (lint-enforced). Any secrets are wrapped in `secrecy::Secret` so they cannot leak into logs or traces.
- __NFR.3 Performance:__ Category, account, transaction, budget, and Balance Check CRUD, and the balance, category-total, payee-total, budget-vs-actual, and balance-check-variance reports, should respond quickly for typical personal-ledger data volumes (thousands, not millions, of transactions).
- __NFR.4 Usability (API-level):__ Since V1 has no UI, the local library API (`lib-database`) is the primary usability surface: errors are structured (via `thiserror`) rather than raw SQL or library errors, so a future client can present them meaningfully. The sync server's gRPC protocol (FR.39a) is a separate, narrower surface for device sync, not the general-purpose API.
- __NFR.5 Maintainability:__ Each entity's persistence logic is split into separate `find`/`insert`/`update`/`delete`/`builder`/`model` files, following the existing `categories/` convention in `lib-database`. SQL queries list explicit columns; no `SELECT *`.
- __NFR.6a Compatibility (Desktop/TUI):__ The Desktop and TUI apps build and run natively on Linux, macOS, and Windows — anywhere the pinned Rust toolchain, `protoc`, and SQLite are available.
- __NFR.6b Compatibility (Sync Server):__ The Sync Server ships as a multi-arch (amd64/arm64) Docker image and runs on any Docker host — Linux, macOS, or Windows — without requiring a native build on the host OS.
- __NFR.7 (placeholder):__ GUI-framework dependencies and any UI-specific NFRs (e.g. rendering performance, offline/sync UX consistency) are TBD, to be added when the Desktop and TUI cycles are scoped.
- __NFR.8 (placeholder):__ Sync Server-specific NFRs (change-set log durability, container security posture, backup/restore) are TBD, to be added when the Sync feasibility cycle work (§2.1 FC-SYNC-*) concludes.

## 5. Dependencies

- Rust toolchain, `protoc`, and other dev tools pinned in `mise.toml`.
- Core crates: `tonic` (gRPC), `sqlx` (SQLite persistence), `serde`, `tracing`, `thiserror`, `uuid` (v7), `chrono`, `secrecy`, and a CSV-parsing crate (e.g. `csv`) for Balance Check import.
- SQLite as the embedded database engine for each Client (no external database server required for V1); the Sync Server's own Change Set log also uses SQLite via `sqlx`, reusing `lib-database`'s conventions (see [ADR-0009](docs/adr/0009-lww-sqlite-change-set-log.md)).
- A decision will need to be made on the best time crate, as chrono is deprecated.
- `cargo-make`, mdBook, and rustdoc for documentation builds.
- Docker (or another OCI-compatible container runtime) to build and run the Sync Server.
- GUI-framework dependencies (Desktop/TUI) and any additional sync-protocol client dependencies are TBD, to be added once their respective cycles are scoped.

## 6. Assumptions and Constraints

### 6.1 Assumptions

- The user trusts their own machine/network to hold their financial data; V1 relies on OS/filesystem-level protection rather than application-level encryption-at-rest.
- The Sync Server is assumed to run within the user's own trusted network (e.g. a homelab), not exposed directly to the public internet; authentication (FC-SYNC-002/007) protects against other devices on that trusted network, not against a hostile network.

### 6.2 Constraints

- No `unsafe` code anywhere in the workspace (lint-enforced, see root `Cargo.toml`).
- Each Account's Unit is fixed at creation; a Transaction's amount is always measured in its Account's Unit, and a Transaction may only be moved to another Account sharing that same Unit.
- A Balance Check's CSV import reads exactly a date column and a balance column for one Account; it does not import Transactions, categories, or payees.
- Transaction Status (Open, Cleared, Reconciled) and Flagged are both set manually by the user; the system never automatically matches a Transaction to a Balance Check or infers a status change. Flagged is independent of Transaction Status — a Transaction may be Flagged in any status, including Reconciled.
- A Reconciled Transaction's other fields (amount, Category, account, payee, description, date) cannot be updated until its Transaction Status is first moved back to Open or Cleared; the Status and Flagged fields themselves remain independently settable at any time.
- SQL queries must list explicit columns; no `SELECT *`.
- Secrets must be wrapped in `secrecy::Secret`.

## 7. Future Considerations & Roadmap

- __Shared ledgers:__ Support for Family, Couples & Groups sharing a single ledger (see §1.3) — out of scope for v0.0.1, which targets a single individual.
- __Institution:__ Associating an Account with the financial Institution that holds it (e.g. a bank) is deferred past V1; no Institution entity, field, or CRUD exists in this cycle.
- __Cross-Unit conversion:__ V1 keeps every Account and Transaction in one fixed Unit with no conversion between Units (see `CONTEXT.md` — Unit); multi-Unit rollups/conversion are a future consideration.
- __Payee normalisation:__ V1 matches payees by exact free text only (see Constraints); fuzzy/normalised payee matching is a future consideration.
- __Double-entry accounting:__ Personal Ledger deliberately uses single-entry Transactions in V1 (see [ADR-0001](docs/adr/0001-single-entry-not-double-entry.md)); revisiting this for audit-grade, structurally-balanced accounting is a future consideration should the need arise.
- __Desktop & TUI UI:__ The concrete UI/UX for each client app (§1.4) is not yet detailed as Functional Requirements; deferred until each respective cycle is scoped.
- __Multi-device sync protocol:__ End-state Functional Requirements for the sync server's protocol beyond FR.39a's placeholder are deferred until a sync-focused cycle is scoped (see FR.39a, §2.1 Sync).
- __Change Set log retention:__ Every Change Set is kept indefinitely for this cycle — deliberately not decided, since a feasibility demo on one self-hoster's own small device fleet doesn't yet produce the usage/volume data a pruning policy would need to design against (see [ADR-0009](docs/adr/0009-lww-sqlite-change-set-log.md)); revisit once a later cycle has that data.
- __Envelope & reverse budgeting:__ Personal Ledger's concept cycle (§2.2) scopes Budgets to line-item only (see CC-TUI-010, CC-DESKTOP-010, FR.22); envelope budgeting (allocating every dollar of income across categories) and reverse budgeting are deferred past V1.
- __Personal Investors:__ Tracking buy-in costs, capital gains, returns, and tax implications for investments (see §1.4) is a future consideration with no Functional Requirements defined yet.
- __Personal Loan:__ Tracking progress paying down a loan (see §1.4) is a future consideration with no Functional Requirements defined yet.
- __Personal Inventory:__ Tracking assets and household inventory (see §1.4) is a future consideration with no Functional Requirements defined yet.
