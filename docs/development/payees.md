# Payees (development)

End-user documentation: [Payees](../payees.md).

## Overview

A Payee is who a Split was paid to or received from — a first-class entity, not a free-text column. `lib-payees` holds the Entity and its rules; `lib-database` owns the `payees` and `payee_aliases` tables and the Payees adapter (`adapter/payees.rs`) the Desktop persists Payees through (#831).

The older per-entity modules (`payees/`, `payee_aliases/`, with the `resolve_or_create` auto-create path) predate the adapter and no Client calls them.

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

- **A Payee's aliases are rewritten with it.** The adapter deletes and re-inserts a Payee's aliases on every write, in the order the Payee holds them, and reads them back in rowid order, so the table carries no timestamp columns.
- **`pattern` is a literal substring, matched as a case-insensitive *contains*** (ADR-0012's match-rules amendment). A rename-generated alias and a hand-authored match rule are the same row with no kind flag. Stored trimmed and upper-cased, deduplicated and unique across Payees (the rules refuse a taken one, and the schema backs them); the longest matching alias wins, then the Payee name alphabetically. Nothing compiles a regex.
- **Delete only when unreferenced.** A Payee a Split or Bill Plan names is deactivated via `is_active`, never hard-deleted: `lib_payees::delete_payee` refuses it, reading `PayeeUsage`, and `splits.payee_id` and `bill_plans.payee_id` refuse it in the file. An unreferenced one is hard-deleted, its aliases cascading (ADR-0012's delete amendment).
- **Case-insensitive uniqueness is enforced by the collation**, so "Kmart" and "KMART" cannot coexist.

## Domain types

None in `lib-core`. `lib_payees::Payee` is the domain type: `id: PayeeId`, `name`, `aliases`, `default_category`, `is_active`. The older `lib_database::Payees` (`payees/model.rs`) and `lib_database::PayeeAliases` row types are not used by either Client.

## Persistence

`crates/libs/lib-database/src/adapter/payees.rs` is the Payees adapter (ADR-0041, ADR-0046): it loads every Payee with its aliases at start-up and serves `lib-payees`' `PayeeRepository` from memory, each write joining the one pending batch the Desktop commits once per input. It answers `PayeeUsage` by counting `splits` and `bill_plans` in the file, and `lib-bills`' `BillPayees` from its rows. `PayeesStore` (`adapter/store.rs`) is what `Stores` holds it by, with `lib_payees::fake::InMemoryPorts` as the in-memory store. `lib-payees`' contract suite runs against both. A new Ledger, and `mise run db-seed`, write `lib_payees::fake::InMemoryPorts::stub`'s Payees before the Transactions and Bills, so the foreign keys hold.

On the Desktop the Payees page writes the store through `lib-payees`' Interactors, each input committing what it wrote (#832). Import commit and Category reassign are still `LedgerCommand`s: `LedgerStore::apply` forwards each `UpsertPayee` to the store (`bin-desktop/src/payees/ports.rs`), committed with the input the command ran in. The Ledger's Payees are a read copy refreshed from the store after either (see the "During the transition" table in `viper-architecture.md`). A failed Payees write reloads the store and the copy with an Error Toast.

The older per-entity modules, `crates/libs/lib-database/src/payees/`:

| File | Contents |
| --- | --- |
| `model.rs` | The `Payees` row struct and mock constructor |
| `builder.rs` | Fluent builder for tests |
| `find.rs` | Queries retrieving Payees |
| `insert.rs` | Insert |
| `update.rs` | Update, including rename |
| `delete.rs` | Delete |
| `resolve.rs` | `Payees::resolve_or_create` — the entry point described below |
| `totals.rs` | `Payees::totals` — per-Payee Transaction totals |

`crates/libs/lib-database/src/payee_aliases/` is deliberately smaller — `model.rs`, `find.rs`, `mod.rs` only. There is no `insert.rs`, `update.rs` or `delete.rs`, because aliases are written by the rename path in `payees/update.rs`, not managed directly.

**`resolve.rs` is the file to read first.** `Payees::resolve_or_create` is the single entry point Transaction entry goes through to turn typed Payee text into a `payee_id`, in this order:

1. Reuse an exact, case-insensitive match on `payees.name`.
2. Fall back to a Payee Alias match, resolving an old spelling to the current Payee.
3. Auto-create a new canonical Payee.

Anything that accepts a typed Payee name should call this rather than inserting directly, or step 2 is silently lost and renames stop reconciling.

## UI

The Desktop's Payees page reads `lib-database`'s store through `lib-payees`' Presenters, the other Views through the Ledger's read copy, and Bills through `BillPayees`; the TUI doesn't.

- **TUI** — `crates/bins/bin-tui/src/payee/` (`mod.rs`, `fixture.rs`) supplies in-memory Payees; the screen is `view/payees.rs`. Spec: `docs/ux/tui-mockups/04-payees/README.md`.
- **Desktop** — the Payees surface (handoff `docs/ux/desktop-mockups/20-payees/`, screens 6a–6e), persisted through the Payees adapter:
  - `crates/libs/lib-payees/` — the Payees VIPER module, `gpui`-free and unit-tested without a window. Start here. `entity.rs` holds the model and rules: the seeded `Payee` (`id: PayeeId`, `name`, `aliases`, `default_category`, `is_active`), `normalise_alias`, `alias_owner`, `match_alias` (longest alias wins), `insert_payee`/`edit_payee`/`delete_payee` (refused while a Split or Bill Plan names the Payee)/`set_active`, and the counts. `interactor/` has one Interactor per intent (add, edit, delete, deactivate, reactivate). `presenter/` has the page (`payees_page.rs`, 6a), the Add and Edit form (`payee_form.rs`, 6b and 6c) and the Delete Dialog (`delete_payee.rs`, 6d), whose `DeleteAction` (delete, deactivate or reactivate) is chosen from `PayeeUsage` when it opens. `protocols.rs` holds the ports: `PayeeRepository`, `PayeeUsage` (the Splits and Bill Plans naming a Payee, which the adapter counts in the file), `PayeeCategories` (the Default category select's leaves, which `lib-ledger` answers from the stub's Categories until they migrate), and the `PayeesRouter`. `keymap.rs` has the page's keys, `i18n/` its Messages, `fake.rs` the in-memory ports and a recording Router, and `contract.rs` (feature `contract-tests`) the checks the fake and the adapter both keep.
  - `src/payees/` — the module's Desktop half: `view/page.rs` the page Entity and `view/page/render.rs` its list, `view/dialog.rs` the Dialogs as Entities on the Dialog host with one View each in `view/dialog/` (the match-rule chips and input are `rules_field.rs`), `router.rs` the `PayeesRouter`, and `ports.rs` the halfway shims: the Categories copied from the Ledger, the forwarding shim and the read copy's refresh.
  - `src/import.rs` and `src/view/import.rs` — the stubbed Import "match payees" step (6e), opened by the `:import` palette verb: an 18-row seeded statement, its Category select's `CategoryOptions` (the Ledger's `PayeeCategories`), `cleaned_name`/`cleaned_token` (the suggested Payee name and remembered alias), `match_row`, `summary` and `commit`, which creates new Payees, remembered aliases and one-Split Transactions, persisted through the forwarding shims.
  - `src/view/transactions/import_step.rs` — the step's keys, clicks and commit on the `TransactionsView` Entity, which shows it in place of the Transactions page (ADR-0037).
  - Messages in `lib-payees`' `i18n/en-US/payees.ftl` and the Desktop's `import.ftl`.

> **Desktop navigation:** `Payees` is no longer a rail noun and has no `g` binding. The page is `payees/view/page.rs`, mounted by the paged Settings surface (`SettingsPage::Payees`) inside the legacy Settings frame (#815); its Dialogs sit beside it in `payees/view/dialog/`. It is reached from the Settings index, `:settings payees` or the `:payees` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

Desktop locations for the ticked requirements. The TUI's PAY-001 to PAY-004 live in `crates/bins/bin-tui/src/view/payees.rs` and `src/payee/`.

| Requirement | Desktop location |
| --- | --- |
| PAY-001 | `lib_payees::insert_payee`, `lib_payees::interactor::AddPayee`, `payees/view/dialog/payee_form.rs` |
| PAY-002 | `payees/view/page/render.rs` (inactive rows dimmed); Transactions payee filter |
| PAY-003 | `lib_payees::edit_payee` |
| PAY-004 | `lib_payees::interactor::DeactivatePayee`, `payees/view/dialog/delete_payee.rs` |
| PAY-005 | `payees/view/page/render.rs` |
| PAY-007 | `Payee::default_category`, `PayeeCategories` |
| PAY-008 | `payees/view/dialog/rules_field.rs`, `lib_payees::match_alias` |
| PAY-009 | `lib_payees::delete_payee`, `lib_payees::presenter::DeleteAction` |
| PAY-010 | `import.rs`, `view/import.rs` |

## Decisions

- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — Payee as a first-class entity with rename aliases, reversing the original free-text decision. Covers why renaming changes one row rather than bulk-rewriting Transactions and why aliases are write-once. Its amendments make hand-authored match rules Payee Aliases (literal *contains*, longest wins; #282) and allow hard-deleting an unreferenced Payee (#283).
- Desktop Payees Surface map, [#281](https://github.com/IanTeda/Personal-Ledger/issues/281) — the import step's scope and entry (#284).
- `GLOSSARY.md` — Payee, Split, Transaction.

## Open questions and known gaps

- **The older `payees/` modules are unused.** `resolve_or_create` has no production call site, and `resolve.rs` and its rename path still treat an alias as an exact match; the adapter and `lib-payees` replace them.
- **The TUI doesn't persist Payees.** It draws fixtures until its redesign (#805).
- **Import is a stub.** No CSV upload or parsing, and steps 1 and 3 of the handoff's stepper are not built.
- **Default Category pre-fill is unused.** The desktop has no transaction form, so only 6e reads it.
- **No merge operation.** ADR-0012 describes the Client offering a merge with an existing Payee before insert; there is no merge query in `lib-database`.
- **No Change Set emission**, so Payee writes do not sync.
