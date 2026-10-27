# Institutions (development)

End-user documentation: [Institutions](../institutions.md).

## Overview

An Institution is the bank, broker or lender an Account is held with: a first-class entity that Accounts name by id. `lib-institutions` is the Institutions VIPER module, toolkit-free and I/O-free: the Entity and its add rule, the Interactor, the page and Dialog Presenters, the ports, the keymap and its Messages. `lib-database` owns the `institutions` table and the Institutions adapter (`adapter/institutions.rs`) the Desktop persists Institutions through. The Desktop's half of the module is `bin-desktop/src/institutions/`. Institutions were the first domain of slice 5 of the VIPER transition ([#847](https://github.com/IanTeda/Personal-Ledger/issues/847)), following the [Migrating a domain](viper-architecture.md#migrating-a-domain) checklist Payees set.

## Data model

The table comes from `migrations/client/20260904110000_create_institutions_table.sql`, created before `accounts`, whose `institution_id` will reference it once Accounts persist (#853).

`institutions`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, an `InstitutionId` (UUIDv7) |
| `name` | TEXT | Not null, unique `COLLATE NOCASE`, stored trimmed |
| `products` | TEXT | Not null, the joined Products label (`GLOSSARY.md`'s Products), stored as given |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_institutions_set_updated_on` |

Invariants and deliberate omissions:

- **Rows read back in rowid order**, the order they were added in. There is no position column, since an Institution can't be moved.
- **`name` is unique ignoring ASCII case.** The add rule refuses a blank or taken name before anything is written (`InstitutionError`), and the collation backs it for a write that skips the rule.
- **`products` is one free-form label**, the Add Dialog's chips joined with ` · `, not a table of Products. It never limits which Accounts may name the Institution. An Institution has no Default unit (#877).
- **No foreign keys point at it yet.** `accounts` isn't persisted, so nothing refuses deleting an Institution an Account names. Delete doesn't exist either (see the gaps below).

## Domain types

`InstitutionId` is in `lib-core`, with the fixed seed ids (`SEED_ANZ` … `SEED_SUPER_FUND`) that `lib-accounts`' stub names, since feature crates never depend on each other. `lib_institutions::Institution` is the domain type (`id`, `name`, `products`), `InstitutionDraft` what the Dialog asks for, and `InstitutionError` the refusal (`NameRequired`, `DuplicateName`), declared in `entity.rs` beside `insert_institution`, `get`, `find_by_name` and `default_institutions`.

The "No institution" placeholder for Cash Accounts is `lib_accounts::NO_INSTITUTION`, `InstitutionId::seed(0)`: in `lib-accounts`, not in `default_institutions()`, and not shown on the page.

## Persistence

`crates/libs/lib-database/src/adapter/institutions.rs` is the Institutions adapter (ADR-0041, ADR-0046): it loads every Institution at start-up and serves `lib-institutions`' `InstitutionRepository` from memory, each write joining the one pending batch the Desktop commits once per input. A write is an upsert (`ON CONFLICT (id) DO UPDATE`), so seeding again leaves the same rows and an edit will need no new query.

`InstitutionsStore` (`adapter/store.rs`) is what `Stores` holds it by, with `lib_institutions::fake::InMemoryInstitutions` as the in-memory store. `Stores::with_units` fills it with an empty store for the TUI's test fixture. `lib-institutions`' contract suite (`contract.rs`, feature `contract-tests`, which `lib-database`'s dev-dependencies enable) runs against both.

A new Ledger, and `mise run db-seed`, write `lib_institutions::fake::InMemoryInstitutions::stub`'s Institutions (`seed::stub_institutions`) after the Units and before the Payees, Transactions and Bills. A Ledger file opened before the table existed starts with no Institutions, since the first-launch seed runs only on a new file.

## UI

The Desktop's Institutions page reads and writes the adapter through `lib-institutions`' Presenters; the Accounts page and its Institution select still read the Ledger's read copy. The TUI doesn't persist Institutions and draws none of the module's Views.

- **Desktop:** the Settings Institutions page, handoff `docs/ux/desktop-mockups/16-settings/README.md`, screens 16h and 16e.
  - `crates/libs/lib-institutions/`, the module. Start here. `interactor/add_institution.rs` is the one Interactor. `presenter/` has the page (`institutions_page.rs`, 16h) and the Add Dialog (`institution_form.rs`, 16e), whose eight `InstitutionProduct` chips (savings, offset, credit card, loan, investment, crypto, retirement, cash) join into `products`. `protocols.rs` holds the ports: `InstitutionRepository`, the `InstitutionsPorts` bundle with its `SharedPorts` alias, and the `InstitutionsRouter`. `keymap.rs` binds the shared flat movement and `new` commands; `edit` and `delete` are not bound yet. `i18n/` holds its Messages and `fake.rs` the in-memory ports and a recording Router.
  - `src/institutions/`, the module's Desktop half: `view/page.rs` the page Entity and `view/page/render.rs` its list, `view/dialog.rs` and `view/dialog/institution_form.rs` the Add Dialog as an Entity on the Dialog host, `router.rs` the `InstitutionsRouter`, and `ports.rs`. `ports.rs`'s `LedgerBackedPorts` owns the boxed `InstitutionsStore` and implements both `InstitutionsPorts` and `InstitutionsStore`, so one `Rc<RefCell<…>>` is the module's ports and `LedgerStore`'s `DesktopInstitutionsPorts`. It also holds the read copy's refresh (`refresh`, `reload`).
  - The page reaches `Root` through `settings::pages::SettingsSetup`'s `institutions`. It takes the default plain-page `?` hints, built from the keymap, so both the status line and the cheat sheet follow `[keybindings]`. It is reached from the Settings index or `:settings institutions`; there is no `:institutions` alias.
  - The Add Dialog raises an `Added <name>` Toast before it closes; the page lands only the outcome and selects the new row. Edit and delete raise a "not yet built" status Notice through `Notify` (ADR-0048).

## Traceability

Desktop locations for the ticked requirements.

| Requirement | Code | Test |
| --- | --- | --- |
| INS-001 | `lib_institutions::presenter::InstitutionsPagePresenter`, `institutions/view/page/render.rs` | `lib_institutions::presenter::institutions_page::tests::rows_run_in_the_order_added_with_the_first_selected`, `settings_institutions_keyboard::rows_move_with_j_k_and_the_ends` |
| INS-002 | `lib_institutions::insert_institution`, `lib_institutions::interactor::AddInstitution`, `institutions/view/dialog/institution_form.rs` | `settings_institutions_keyboard::adding_an_institution_selects_it_and_raises_a_toast` |
| INS-003 | `lib_database::adapter::institutions`, `seed::stub_institutions` | `lib_database::adapter::institutions::tests::an_added_institution_reloads_the_same`, `smoke_institutions_persist::an_added_institution_is_there_after_a_restart` |
| INS-004 | `LedgerStore::refresh_institutions`, `Ledger::refresh_institutions` | `smoke_institutions_persist::an_added_institution_is_there_after_a_restart` |

## Decisions

- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the one-file adapter and the shared writer the Institutions adapter follows.
- [ADR-0048](../adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md) — the Toast and status Notices the page and Dialog raise.
- [VIPER transition slice 5](https://github.com/IanTeda/Personal-Ledger/issues/847) — Institutions as the first domain after the Payees template.
- `GLOSSARY.md` — Institution, Products, Account.

## Open questions and known gaps

- **Halfway states.** The Accounts page, its Institution select and the Documents picker's Account link labels read the Ledger's read copy of the Institutions, which stays until Accounts migrate (#853). It is a row in [During the transition](viper-architecture.md#during-the-transition).
- **No edit or delete.** Backlog: [#869](https://github.com/IanTeda/Personal-Ledger/issues/869), including the refusal of deleting an Institution an Account names. The port has only add; an edit needs an upsert on it, which the adapter's write already is.
- **No Default unit.** The Add Dialog's Default unit select was never saved and went in #873; saving one that prefills the Account form is backlog ([#877](https://github.com/IanTeda/Personal-Ledger/issues/877)).
- **The Cash placeholder has no row.** Once `accounts.institution_id` gets a foreign key, `NO_INSTITUTION` needs a seeded row kept off the page, or Cash Accounts need a nullable column (#853).
- **A file from before #849 has no Institutions.** The first-launch seed runs only on a new file, so #853 must decide between a fresh Ledger and seeding an empty table.
- **The Add Dialog's Toast is new behaviour**, added with the Presenter (#850).
- **The TUI doesn't persist Institutions.** It draws fixtures until its redesign (#805).
- **No Change Set emission**, so Institution writes do not sync.
