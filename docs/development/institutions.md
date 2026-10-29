# Institutions (development)

End-user documentation: [Institutions](../institutions.md).

## Overview

An Institution is the bank, broker or lender an Account is held with: a first-class entity that Accounts name by id. `lib-institutions` is the Institutions VIPER module, toolkit-free and I/O-free: the Entity and its add and edit rules, the Interactors, the page and Dialog Presenters, the ports, the keymap and its Messages. `lib-database` owns the `institutions` table and the Institutions adapter (`adapter/institutions.rs`) the Desktop persists Institutions through. The Desktop's half of the module is `bin-desktop/src/institutions/`. Institutions were the first domain of slice 5 of the VIPER transition ([#847](https://github.com/IanTeda/Personal-Ledger/issues/847)), following the [Migrating a domain](viper-architecture.md#migrating-a-domain) checklist Payees set.

## Data model

The table comes from `migrations/client/20260904110000_create_institutions_table.sql`, created before `accounts`, whose `institution_id` will reference it once Accounts persist (#853).

`institutions`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, an `InstitutionId` (UUIDv7) |
| `name` | TEXT | Not null, unique `COLLATE NOCASE`, stored trimmed |
| `products` | TEXT | Not null, the joined Products label (`GLOSSARY.md`'s Products), stored as given |
| `usual_unit_id` | UUID | Nullable foreign key to `units(id)`, no cascade: the Usual unit (`GLOSSARY.md`) |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_institutions_set_updated_on` |

Invariants and deliberate omissions:

- **Rows read back in rowid order**, the order they were added in. There is no position column, since an Institution can't be moved.
- **`name` is unique ignoring ASCII case.** The add rule refuses a blank or taken name before anything is written (`InstitutionError`), and the collation backs it for a write that skips the rule.
- **`products` is one free-form label**, the Add Dialog's chips joined with ` · `, not a table of Products. It never limits which Accounts may name the Institution. An Institution may name a Usual unit (`GLOSSARY.md`, #877): a prefill hint, never a second default Unit.
- **`accounts.institution_id` references it**, `NOT NULL` with no cascade, so the file refuses deleting an Institution an Account names. The `DeleteInstitution` Interactor refuses it first, with the Accounts' names; the key is the backstop.

## Domain types

`InstitutionId` is in `lib-core`, with the fixed seed ids (`SEED_PERSONAL`, then `SEED_ANZ` … `SEED_SUPER_FUND`) that `lib-accounts`' stub names, since feature crates never depend on each other. `lib_institutions::Institution` is the domain type (`id`, `name`, `products`, `usual_unit`), `InstitutionDraft` what the Dialog asks for, and `InstitutionError` the add and edit refusal (`NameRequired`, `DuplicateName`, `NotFound`), declared in `entity.rs` beside `insert_institution`, `edit_institution` (the same name rule, skipping the Institution's own id, so a change of capitals is accepted), `get`, `find_by_name`, `personal_institution` and `default_institutions`.

The Personal Institution (`GLOSSARY.md`, [ADR-0049](../adr/0049-a-cash-account-names-the-personal-institution.md)) is `personal_institution(name)`: `SEED_PERSONAL`, the name given and Products `cash`. It is an ordinary row, listed first by `default_institutions()` and shown, edited and deleted on the page like any other (refused while an Account names it, as any Institution is). The Account form defaults a new Cash Account to it when it is on the Ledger, by its id.

## Persistence

`crates/libs/lib-database/src/adapter/institutions.rs` is the Institutions adapter (ADR-0041, ADR-0046): it loads every Institution at start-up and serves `lib-institutions`' `InstitutionRepository` from memory, each write joining the one pending batch the Desktop commits once per input. The port's writes are `upsert_institution`, which adds a new id after the rows there and replaces a known id where it stands (the Interactors `AddInstitution` and `EditInstitution` both end in it; the SQL is an upsert, `ON CONFLICT (id) DO UPDATE`, so seeding again leaves the same rows), and `delete_institution`, which `DeleteInstitution` ends in. A delete the foreign key refuses fails in the file, is reported against the Institutions domain and reloaded like any other failed write.

What names an Institution is the Accounts', so `lib-institutions` declares a read port, `InstitutionUsage` (`institution_usage(id) -> InstitutionUse { accounts: Vec<String> }`, the Accounts' names in their order), and each Client answers it. `AccountsAdapter` answers it from its in-memory rows as it answers `UnitUsage`; `AccountsStore::institution_usage` is the default every store gets from its Accounts, so the in-memory stores and the UI harness answer it too. `InstitutionsPorts::usage` hands it to the Interactor and the Presenter beside the repository.

`InstitutionsStore` (`adapter/store.rs`) is what `Stores` holds it by, with `lib_institutions::fake::InMemoryInstitutions` as the in-memory store. `FakeStores::stub` fills it with the stub seed for the TUI's in-memory Shell and test fixture. `lib-institutions`' contract suite (`contract.rs`, feature `contract-tests`, which `lib-database`'s dev-dependencies enable) runs against both.

`open_storage(path, today, personal)` writes the Personal Institution, named `personal`, into a file it creates, before and apart from the first-launch stub seed; the Desktop and the TUI pass `lib_locale::msg::institution_personal()`. Opening an existing file writes nothing of it. A new Ledger, and `mise run db-seed`, then write `lib_institutions::fake::InMemoryInstitutions::stub`'s seven Institutions (`seed::stub_institutions`) after the Units and before the Accounts, Payees, Transactions and Bills; `open_storage` gives the stub's Personal Institution the name passed. A Ledger file opened before the table existed starts with no Institutions, since the first-launch seed runs only on a new file.

## UI

The Desktop's Institutions page reads and writes the adapter through `lib-institutions`' Presenters; the Accounts page and its Institution select still read the Ledger's read copy. The TUI doesn't persist Institutions and draws none of the module's Views.

- **Desktop:** the Settings Institutions page, handoff `docs/ux/desktop-mockups/16-settings/README.md`, screens 16h and 16e.
  - `crates/libs/lib-institutions/`, the module. Start here. `interactor/` has `AddInstitution`, `EditInstitution` and `DeleteInstitution`, which re-checks usage and refuses an Institution in use with a `DeleteInstitutionRefusal::InUse` carrying the `InstitutionUse`. `presenter/` has the page (`institutions_page.rs`, 16h) and the Add and Edit Dialog (`institution_form.rs`, 16e; `add` and `edit(ports, id)` open its two modes, Edit pre-filled with the Products split back into chips), whose eight `InstitutionProduct` chips (savings, offset, credit card, loan, investment, crypto, retirement, cash) join into `products`; and the Delete Dialog (`delete_institution.rs`), which opens unused (the name typed back, exact and case-sensitive, enables Delete institution) or refused (up to five Account names then "and N more", only Close) from the usage read when it opens, and turns refused when its confirm finds usage changed. `protocols.rs` holds the ports: `InstitutionRepository`, the `InstitutionsPorts` bundle with its `SharedPorts` alias, and the `InstitutionsRouter`. `keymap.rs` binds the shared flat movement, `new`, `edit` and `delete` commands. The page Presenter also holds the row menu (`OpenMenu`, `CloseMenu`, `Menu(MenuAction)`), as Units' does. `i18n/` holds its Messages and `fake.rs` the in-memory ports and a recording Router.
  - `src/institutions/`, the module's Desktop half: `view/page.rs` the page Entity and `view/page/render.rs` its list, `view/dialog.rs` and `view/dialog/institution_form.rs` the Add and Edit Dialog as an Entity on the Dialog host, `view/dialog/delete_institution.rs` the Delete Dialog as another, `router.rs` the `InstitutionsRouter`, and `ports.rs`. `ports.rs`'s `LedgerBackedPorts` owns the boxed `InstitutionsStore` and implements both `InstitutionsPorts` and `InstitutionsStore`, so one `Rc<RefCell<…>>` is the module's ports and `LedgerStore`'s `DesktopInstitutionsPorts`. It also holds the read copy's refresh (`refresh`, `reload`) and the Accounts store, shared with the Units and Bills reads, which it answers `InstitutionUsage` from on each ask.
  - The page reaches `Root` through `settings::pages::SettingsSetup`'s `institutions`. It takes the default plain-page `?` hints, built from the keymap, so both the status line and the cheat sheet follow `[keybindings]`. It is reached from the Settings index or `:settings institutions`; there is no `:institutions` alias.
  - The Dialogs raise an `Added <name>`, `Saved <name>` or `Deleted <name>` Toast before they close; the page lands only the outcome and selects the row, or for a delete the neighbour it noted when the Dialog opened (the next row, else the previous). A right-click on a row selects it and opens the menu at the pointer (`Esc` or a press outside closes it). Delete, from `d`, the row link or the menu, opens the Delete Dialog. After a save or a delete, `ports::refresh` copies the rows into the `Ledger` read copy and a failed write is rolled back through `reload_institutions`, so the Accounts page and the Account form show the new name.

## Traceability

Desktop locations for the ticked requirements.

| Requirement | Code | Test |
| --- | --- | --- |
| INS-001 | `lib_institutions::presenter::InstitutionsPagePresenter`, `institutions/view/page/render.rs` | `lib_institutions::presenter::institutions_page::tests::rows_run_in_the_order_added_with_the_first_selected`, `settings_institutions_keyboard::rows_move_with_j_k_and_the_ends` |
| INS-002 | `lib_institutions::insert_institution`, `lib_institutions::interactor::AddInstitution`, `institutions/view/dialog/institution_form.rs` | `settings_institutions_keyboard::adding_an_institution_selects_it_and_raises_a_toast` |
| INS-003 | `lib_database::adapter::institutions`, `seed::stub_institutions` | `lib_database::adapter::institutions::tests::an_added_institution_reloads_the_same`, `smoke_institutions_persist::an_added_institution_is_there_after_a_restart` |
| INS-004 | `LedgerStore::refresh_institutions`, `Ledger::refresh_institutions` | `smoke_institutions_persist::an_added_institution_is_there_after_a_restart` |
| INS-004 (Cash) | `lib_institutions::personal_institution`, `lib_database::open_storage`, `AccountFormPresenter`'s Cash default | `lib_database::storage::tests::a_new_ledger_is_written_with_the_personal_institution_under_the_name_passed`, `settings_accounts_keyboard::a_cash_account_added_from_the_keyboard_names_the_personal_institution` |
| INS-005 | `lib_institutions::edit_institution`, `lib_institutions::interactor::EditInstitution`, `InstitutionFormPresenter::edit` | `lib_institutions::contract::an_edited_institution_reloads_the_same`, `settings_institutions_keyboard::enter_saves_an_edit_selects_the_row_and_raises_a_toast`, `smoke_institutions_persist::an_edited_institution_is_there_after_a_restart` |
| INS-006 | `lib_institutions::interactor::DeleteInstitution`, `lib_institutions::presenter::DeleteInstitutionPresenter`, `InstitutionUsage` (`AccountsAdapter`, `AccountsStore::institution_usage`), `institutions/view/dialog/delete_institution.rs` | `lib_institutions::contract::a_deleted_institution_is_gone_after_a_reload`, `lib_institutions::contract::deleting_one_in_use_is_refused_and_writes_nothing`, `lib_institutions::presenter::delete_institution::tests`, `lib_database::adapter::accounts::tests::institution_usage_counts_the_accounts_that_name_it`, `settings_institutions_keyboard::delete_waits_for_the_name_typed_back_exactly_then_removes_the_row`, `settings_institutions_keyboard::d_opens_the_refused_dialog_for_an_institution_an_account_names`, `smoke_institutions_persist::a_deleted_institution_is_gone_after_a_restart` |

## Decisions

- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the one-file adapter and the shared writer the Institutions adapter follows.
- [ADR-0048](../adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md) — the Toast and status Notices the page and Dialog raise.
- [VIPER transition slice 5](https://github.com/IanTeda/Personal-Ledger/issues/847) — Institutions as the first domain after the Payees template.
- [ADR-0049](../adr/0049-a-cash-account-names-the-personal-institution.md) — a Cash Account names the Personal Institution, not a placeholder.
- `GLOSSARY.md` — Institution, Personal Institution, Products, Account.

## Open questions and known gaps

- **Halfway states.** The Accounts page, its Institution select and the Documents picker's Account link labels read the Ledger's read copy of the Institutions, which stays until Accounts migrate (#853). It is a row in [During the transition](viper-architecture.md#during-the-transition).
- **Delete reads usage, not the foreign key.** The Dialog and Interactor refuse from `InstitutionUsage`; a delete that gets past them (an Account added to a file the Desktop's Accounts copy doesn't yet hold) fails against the key and is rolled back with the Error Toast.
- **The Usual unit is saved, and a Unit's delete reassigns it.** `institutions.usual_unit_id` is a nullable foreign key to `units` with no cascade, so the Institutions must stop naming a Unit before it goes. `UnitUse` counts the Institutions (from the Ledger read copy) and `lib-units`' `DeleteUnit` refuses with the count, which the TUI keeps; the Desktop's Delete unit Dialog is `lib-unit-deletion`'s, which asks for each Institution's new Usual unit (None by default) and writes them with the delete in one commit (`docs/development/units.md`). The Add and Edit Dialog's select (None, then the active Units; Edit keeps listing a since-deactivated Unit it already names) reads the Units through the `InstitutionUnits` port, which the Desktop copies from the Ledger on each Institutions refresh. The Account form's prefill came in [#901](https://github.com/IanTeda/Personal-Ledger/issues/901) and the Unit delete Dialog that reassigns in [#902](https://github.com/IanTeda/Personal-Ledger/issues/902) and [#903](https://github.com/IanTeda/Personal-Ledger/issues/903) (parent [#877](https://github.com/IanTeda/Personal-Ledger/issues/877)). The stub gives the Cryptocurrency Exchange the BTC Unit.
- **A file from before #849 has no Institutions.** The first-launch seed runs only on a new file, so #853 must decide between a fresh Ledger and seeding an empty table.
- **The Add Dialog's Toast is new behaviour**, added with the Presenter (#850).
- **The TUI doesn't persist Institutions.** It draws fixtures until its redesign (#805).
- **No Change Set emission**, so Institution writes do not sync.
