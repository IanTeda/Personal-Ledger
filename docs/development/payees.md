# Payees (development)

End-user documentation: [Payees](../payees.md).

## Overview

A Payee is who a Split was paid to or received from: a first-class entity, not a free-text column. `lib-payees` is the Payees VIPER module, toolkit-free and I/O-free: the Entity and its rules, one Interactor per intent, the page and Dialog Presenters, the ports, the keymap and its Messages. `lib-database` owns the `payees` and `payee_aliases` tables and the Payees adapter (`adapter/payees.rs`) the Desktop persists Payees through. The Desktop's half of the module is `bin-desktop/src/payees/`. Payees were slice 4 of the VIPER transition ([#829](https://github.com/IanTeda/Personal-Ledger/issues/829)), the worked example for [Migrating a domain](viper-architecture.md#migrating-a-domain).

## Data model

Both tables come from `migrations/client/20260905090000_create_payees_table.sql`, created before `splits` and `bill_plans` because both reference `payees(id)`, with no cascade: the foreign keys backstop the delete refusal.

`payees`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `name` | TEXT | Not null, unique `COLLATE NOCASE` — case-insensitive uniqueness |
| `default_category_id` | UUID | Nullable, the leaf Category that pre-fills a new Split; no foreign key until Categories persist |
| `is_active` | BOOLEAN | Not null, defaults true |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_payees_set_updated_on` |

`payee_aliases`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `payee_id` | UUID | Not null, references `payees(id)`, cascading on delete |
| `pattern` | TEXT | Not null, unique, a literal substring (the column name predates the ADR-0012 amendment) |

Indexed on `payee_aliases(payee_id)`.

Invariants and deliberate omissions:

- **A Payee's aliases are rewritten with it.** The adapter deletes and re-inserts a Payee's aliases on every write, in the order the Payee holds them, and reads them back in rowid order, so typed order survives with no order column and the table carries no timestamp columns. The contract suite sorts them, since order isn't a schema guarantee.
- **`pattern` is a literal substring, matched as a case-insensitive *contains*** (ADR-0012's match-rules amendment). A rename-generated alias and a hand-authored match rule are the same row with no kind flag. Stored trimmed and upper-cased, deduplicated and unique across Payees (the rules refuse a taken one, and the schema backs them); the longest matching alias wins, then the Payee name alphabetically. An inactive Payee's aliases don't match. Nothing compiles a regex.
- **Delete only when unreferenced.** A Payee a Split or Bill Plan names is deactivated via `is_active`, never hard-deleted: `lib_payees::delete_payee` refuses it with `DeletePayeeRefusal::InUse`, reading `PayeeUsage`, and `splits.payee_id` and `bill_plans.payee_id` refuse it in the file. An unreferenced one is hard-deleted, its aliases cascading (ADR-0012's delete amendment).
- **Case-insensitive uniqueness is enforced by the collation**, so "Kmart" and "KMART" cannot coexist.

## Domain types

None in `lib-core` beyond `PayeeId`. `lib_payees::Payee` is the domain type: `id: PayeeId`, `name`, `aliases`, `default_category`, `is_active`. Its rules raise `PayeeError` (`DuplicateName`, `AliasTaken`, `NameRequired`, `NotFound`, `InUse`), declared in `entity.rs` beside them rather than in an `error.rs`. The delete refusal is `DeletePayeeRefusal`, and `PayeeUse { splits, bill_plans }` is what `PayeeUsage` answers; both live in `lib-payees`, not `lib-ledger`. `PayeeCategory` is Payees' own read type for a Default category option, an id and a display label. The older `lib_database::Payees` (`payees/model.rs`) and `lib_database::PayeeAliases` row types are not used by either Client.

## Persistence

`crates/libs/lib-database/src/adapter/payees.rs` is the Payees adapter (ADR-0041, ADR-0046): it loads every Payee with its aliases at start-up and serves `lib-payees`' `PayeeRepository` from memory, each write joining the one pending batch the Desktop commits once per input. It answers `PayeeUsage` by counting `splits` and `bill_plans` in the file, a blocking read through the writer that sees committed rows only. A failed count logs and answers unknown (`None`), and fails closed (#840, reversing #832's fail-open): the Delete Interactor refuses with `DeletePayeeRefusal::Unchecked`, and the Delete Dialog offers what a Payee in use is offered, Deactivate or Reactivate, with a warning that its use couldn't be checked. The person learns before confirming, as #829's stories ask, rather than from the Error Toast after a delete the file refuses. The foreign keys stay the backstop for a Split or Bill Plan that lands between the count and the delete's commit; that refusal reloads the Payees with an Error Toast (`settings_payees_keyboard::a_delete_the_files_foreign_keys_refuse_puts_the_payee_back_with_an_error_toast`). It also answers `lib-bills`' `BillPayees` from its rows: `PayeesStore` carries a `bill_payees()` method, since the orphan rule stops `lib-database` implementing `BillPayees` for `lib-payees`' fake.

`PayeesStore` (`adapter/store.rs`) is what `Stores` holds it by, with `lib_payees::fake::InMemoryPorts` as the in-memory store. `lib-payees`' contract suite (`contract.rs`, feature `contract-tests`, which `lib-database`'s dev-dependencies enable) runs against both, with `PortsFactory::name_payee` writing the Splits and Bill Plans that make a Payee used.

A new Ledger, and `mise run db-seed`, write `lib_payees::fake::InMemoryPorts::stub`'s Payees (`seed::stub_payees`) after the Units and before the Transactions and Bills, so the foreign keys hold. The seed is upsert-idempotent only over stub data: aliases are unique across Payees, so a dev database where another Payee owns one of the stub's patterns fails the Payees seed.

The older per-entity modules, `crates/libs/lib-database/src/payees/`, predate the adapter and no Client calls them:

| File | Contents |
| --- | --- |
| `model.rs` | The `Payees` row struct and mock constructor |
| `builder.rs` | Fluent builder for tests |
| `find.rs` | Queries retrieving Payees |
| `insert.rs` | Insert |
| `update.rs` | Update, including rename |
| `delete.rs` | Delete, its aliases cascading |
| `resolve.rs` | `Payees::resolve_or_create`, below |
| `totals.rs` | `Payees::totals` — per-Payee Transaction totals |

`crates/libs/lib-database/src/payee_aliases/` is smaller: `model.rs`, `find.rs` and `mod.rs` only, because aliases are written by the rename path in `payees/update.rs`. `Payees::resolve_or_create` turns typed Payee text into a `payee_id`: reuse an exact, case-insensitive match on `payees.name`, else an alias match, else create a new Payee. It has no production call site, and it still treats an alias as an exact match rather than ADR-0012's *contains*; a Transaction form would resolve through `lib_payees::match_alias` instead.

## UI

The Desktop's Payees page reads and writes the store through `lib-payees`' Presenters; the other Views read the Ledger's read copy, and Bills read `BillPayees`. The TUI doesn't persist Payees.

- **TUI:** `crates/bins/bin-tui/src/payee/` (`mod.rs`, `fixture.rs`) supplies in-memory Payees; the screen is `view/payees.rs`. Spec: `docs/ux/tui-mockups/04-payees/README.md`. It doesn't compose `lib-payees`' Message layer, since it draws none of the module's Views; its keymap's defaults are still in the TUI's clash test.
- **Desktop:** the Payees surface, handoff `docs/ux/desktop-mockups/20-payees/`, screens 6a–6e.
  - `crates/libs/lib-payees/`, the module. Start here. `entity.rs` holds the model and rules: `normalise_alias`, `alias_owner`, `match_alias` (longest alias wins), `insert_payee`, `edit_payee`, `delete_payee` (written over rows and a `PayeeUsage`, so `lib-ledger` could run it on its own rows while the legacy page did), `set_active`, and the counts. `interactor/` has one Interactor per intent: add, edit, delete, deactivate, reactivate. `presenter/` has the page (`payees_page.rs`, 6a), the Add and Edit form (`payee_form.rs`, 6b and 6c) and the Delete Dialog (`delete_payee.rs`, 6d), whose `DeleteAction` (delete, deactivate or reactivate) is chosen from `PayeeUsage` when it opens. `protocols.rs` holds the ports: `PayeeRepository`, `PayeeUsage`, `PayeeCategories` (the Default category select's leaves, which `lib-ledger`'s `bill_reads.rs` answers from the stub's Categories until they migrate), the `PayeesPorts` bundle and the `PayeesRouter`. `keymap.rs` binds the shared flat `new`, `edit`, `delete` and movement commands, `i18n/` holds its Messages, `fake.rs` the in-memory ports and a recording Router.
  - `src/payees/`, the module's Desktop half: `view/page.rs` the page Entity and `view/page/render.rs` its list, `view/dialog.rs` the Dialogs as Entities on the Dialog host with one View each in `view/dialog/` (the match-rule chips and input are `rules_field.rs`), `router.rs` the `PayeesRouter`, and `ports.rs`. `ports.rs`'s `LedgerBackedPorts` owns the boxed `PayeesStore` and implements both `PayeesPorts` and `PayeesStore`, so one `Rc<RefCell<…>>` is the module's ports, `LedgerStore`'s `SharedPayeesStore` and the Bills reads' `BillPayees`. It also holds the halfway shims: the Categories copied from the Ledger before each event, the forwarding shim (`forward`) and the read copy's refresh (`refresh`). Its `in_memory` counts the stub's Splits and Bill Plans for usage, since `InMemoryPorts` answers "unused" for every Payee.
  - The page reaches `Root` through `settings::pages::SettingsSetup`'s `payees`, and Documents' `Reveal::Payee` reaches it as the page Presenter's `Reveal`. It takes the default plain-page `?` hints; its status-line legend is built from the keymap, so both follow `[keybindings]`.
  - `src/import.rs` and `src/view/import.rs`, the stubbed Import "match payees" step (6e), opened by the `:import` palette verb: an 18-row seeded statement, its Category select's `CategoryOptions` (the Ledger's `PayeeCategories`), `cleaned_name` and `cleaned_token` (the suggested Payee name and remembered alias), `match_row`, `summary` and `commit`, which creates new Payees, remembered aliases and one-Split Transactions, persisted through the forwarding shims. `src/view/transactions/import_step.rs` holds the step's keys, clicks and commit on the `TransactionsView` Entity, which shows it in place of the Transactions page (ADR-0037).
  - Messages in `lib-payees`' `i18n/en-US/payees.ftl`, and the Desktop's `import.ftl` and `shell.ftl` (the two `desktop-toast-write-failed-payees` Toasts).

> **Desktop navigation:** `Payees` is no longer a rail noun and has no `g` binding. The page is mounted by the paged Settings surface (`SettingsPage::Payees`) inside the legacy Settings frame (#815). It is reached from the Settings index, `:settings payees` or the `:payees` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

Desktop locations for the ticked requirements. The TUI's PAY-001 to PAY-004 live in `crates/bins/bin-tui/src/view/payees.rs` and `src/payee/`.

| Requirement | Code | Test |
| --- | --- | --- |
| PAY-001 | `lib_payees::insert_payee`, `lib_payees::interactor::AddPayee`, `payees/view/dialog/payee_form.rs` | `lib_payees::interactor::add_payee::tests::adding_a_payee_writes_it_active_with_its_aliases_normalised`, `settings_payees_keyboard::typing_a_name_and_pressing_enter_adds_and_selects_the_payee` |
| PAY-002 | `lib_payees::presenter::PayeesPagePresenter`, `payees/view/page/render.rs` (inactive rows dimmed and tagged); Transactions payee filter | `lib_payees::presenter::payees_page::tests::the_rows_run_a_to_z_ignoring_case_with_their_category_and_rules` |
| PAY-003 | `lib_payees::edit_payee`, `lib_payees::interactor::EditPayee` | `lib_payees::entity::tests::a_rename_keeps_the_old_name_as_an_alias`, `settings_payees_keyboard::e_edits_the_selected_payee_and_enter_saves_the_new_name` |
| PAY-004 | `lib_payees::interactor::DeactivatePayee`, `ReactivatePayee`, `payees/view/dialog/delete_payee.rs` | `lib_payees::presenter::delete_payee::tests::deactivate_and_reactivate_keep_the_payee_and_raise_their_toasts`, `settings_payees_keyboard::d_on_a_used_payee_deactivates_it_and_keeps_it_listed` |
| PAY-005 | `lib_payees::presenter::PayeesPageViewModel`, `payees/view/page/render.rs` | `lib_payees::presenter::payees_page::tests::the_scope_counts_active_payees_and_those_without_a_default` |
| PAY-007 | `Payee::default_category`, `PayeeCategories` | `lib_payees::presenter::payee_form::tests::the_default_category_offers_none_then_every_leaf_and_ignores_others`, `settings_payees_mouse::a_rule_and_a_default_category_are_added_by_clicks` |
| PAY-008 | `payees/view/dialog/rules_field.rs`, `lib_payees::match_alias` | `lib_payees::presenter::payee_form::tests::rules_are_normalised_deduped_and_another_payees_is_refused_in_the_input`, `settings_payees_keyboard::enter_in_the_rule_input_adds_a_chip_and_backspace_takes_it_back` |
| PAY-009 | `lib_payees::delete_payee`, `lib_payees::interactor::DeletePayee`, `lib_payees::presenter::DeleteAction` | `lib_payees::presenter::delete_payee::tests::the_three_outcomes_are_chosen_from_the_payees_use_when_it_opens`, `settings_payees_keyboard::d_deletes_an_unused_payee_once_its_name_is_typed`, `settings_payees_keyboard::d_on_a_payee_whose_use_cant_be_counted_offers_only_deactivate` |
| PAY-010 | `import.rs`, `view/import.rs` | `import::tests::commit_creates_payees_with_remembered_aliases_and_appends_transactions`, `import_keyboard::a_committed_new_payee_shows_on_the_payees_page_with_its_remembered_rule` |
| PAY-011 | `lib_database::adapter::payees`, `seed::stub_payees` | `lib_database::adapter::payees::tests::written_rows_reload_the_same`, `smoke_payees_persist::an_added_and_a_renamed_payee_are_there_after_a_restart` |

## Decisions

- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — Payee as a first-class entity with rename aliases, reversing the original free-text decision. Covers why renaming changes one row rather than bulk-rewriting Transactions and why aliases are write-once. Its amendments make hand-authored match rules Payee Aliases (literal *contains*, longest wins; #282) and allow hard-deleting an unreferenced Payee (#283).
- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the one-file adapter and the shared writer the Payees adapter follows.
- [VIPER transition slice 4](https://github.com/IanTeda/Personal-Ledger/issues/829) — Payees as the Settings template: the module, persistence, the halfway states and the [Migrating a domain](viper-architecture.md#migrating-a-domain) checklist.
- Desktop Payees Surface map, [#281](https://github.com/IanTeda/Personal-Ledger/issues/281) — the import step's scope and entry (#284).
- `GLOSSARY.md` — Payee, Payee Alias, Split, Transaction.

## Open questions and known gaps

- **Halfway states.** The Transactions View, Documents, Import and Budgets read the Ledger's read copy of the Payees; Import commit and Category reassign write through the forwarding shim (#810, #813); `PayeeCategories` comes from the Ledger until Categories migrate (#808). Each is a row in [During the transition](viper-architecture.md#during-the-transition).
- **The contract's writes don't run the Interactors.** `contract.rs` applies the Entity rules to rows itself, so the Interactors are tested only on the fakes ([#836](https://github.com/IanTeda/Personal-Ledger/issues/836)).
- **The Deactivate Dialog counts transactions only.** A Payee only Bill Plans name reads "used by 0 transactions" ([#837](https://github.com/IanTeda/Personal-Ledger/issues/837)).
- **A delete the foreign keys refuse shows its success Toast first.** Only when a Split or Bill Plan lands between the count and the commit: the Dialog's "Deleted payee" Toast shows, then the Error Toast as the Payees reload. The commit runs after the Dialog closes, so it can't know sooner (ADR-0046).
- **The `Payee` to `BillPayee` mapping exists twice**, in `lib_database::adapter::payees` and `lib-ledger`'s `bill_reads.rs`, which the stub's Bills seed needs; the copy goes with the stub seed.
- **The usage count is taken once in memory.** The in-memory Desktop and the UI harness count the stub's Splits and Bill Plans when they're built, so a Split a test adds later isn't counted.
- **The older `payees/` modules are unused**, and `resolve_or_create` still matches an alias exactly.
- **The TUI doesn't persist Payees.** It draws fixtures until its redesign (#805).
- **Import is a stub.** No CSV upload or parsing, and steps 1 and 3 of the handoff's stepper are not built.
- **Default Category pre-fill is unused.** The desktop has no transaction form, so only 6e reads it.
- **No merge operation.** ADR-0012 describes the Client offering a merge with an existing Payee before insert; there is no merge query in `lib-database`.
- **No Change Set emission**, so Payee writes do not sync.
