# Payees (development)

End-user documentation: [Payees](../payees.md).

## Overview

A Payee is who a Split was paid to or received from — a first-class entity, not a free-text column. `lib-database` owns the `payees` and `payee_aliases` tables and the resolution path that turns typed text into a `payee_id`. There is no Payee type in `lib-core`: a Payee carries no validation rules beyond name uniqueness, so the row model in `lib-database` is the whole domain type.

Payees is the domain with the most complete persistence layer of the three catalogue entities, including the auto-create-on-entry path that makes Payee creation invisible in the common case. Neither Client is wired to it yet.

## Data model

Both tables come from `migrations/client/20260905090000_create_payees_table.sql`, created before `transactions` because that table references `payees(id)`.

`payees`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `name` | TEXT | Not null, unique `COLLATE NOCASE` — case-insensitive uniqueness |
| `is_active` | BOOLEAN | Not null, defaults true |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_payees_set_updated_on` |

`payee_aliases`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `payee_id` | UUID | Not null, references `payees(id)` |
| `pattern` | TEXT | Not null, a literal substring (the column name predates the ADR-0012 amendment) |

Indexed on `payee_aliases(payee_id)`.

Invariants and deliberate omissions:

- **`payee_aliases` rows are write-once.** An alias is only ever inserted by a rename and never updated, so the table carries no timestamp columns at all — the `id`'s embedded UUIDv7 timestamp stands in for when the rename happened. This is the same reasoning `transactions` uses to omit `created_on` (FR.21). Do not add `created_on`/`updated_on` here without revisiting that decision.
- **`pattern` is a literal substring, matched as a case-insensitive *contains*** (ADR-0012's match-rules amendment). A rename-generated alias and a hand-authored match rule are the same row with no kind flag. Stored trimmed and upper-cased, deduplicated, unique across Payees (not enforced by the schema); the longest matching alias wins, then the Payee name alphabetically. Nothing compiles a regex.
- **Delete only when unreferenced.** A Payee referenced by a Split is deactivated via `is_active`, never hard-deleted, enforced by the FK-enforcement pragma; an unreferenced one may be hard-deleted with its aliases (ADR-0012's delete amendment). `delete.rs` exists and does not itself check for references or remove aliases.
- **No default Category column yet.** `CONTEXT.md`'s optional default Category (a leaf) exists only in the desktop stub; the migration gains a nullable `default_category_id` when persistence is wired.
- **Case-insensitive uniqueness is enforced by the collation**, so "Kmart" and "KMART" cannot coexist.

## Domain types

None in `lib-core`. `lib_database::Payees` (`payees/model.rs`) is the domain type: `id: RowID`, `name: String`, `is_active: bool`, `created_on`, `updated_on`. `lib_database::PayeeAliases` (`payee_aliases/model.rs`) is its counterpart.

## Persistence

`crates/libs/lib-database/src/payees/`:

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

Neither Client reads Payees from `lib-database`.

- **TUI** — `crates/bins/bin-tui/src/payee/` (`mod.rs`, `fixture.rs`) supplies in-memory Payees; the screen is `view/payees.rs`. Spec: `docs/ux/mockups/payees/README.md`.
- **Desktop** — the Payees surface (handoff `docs/ux/desktop/20-payees/`, screens 6a–6e), all on in-memory stubs:
  - `src/payees/mod.rs` — `gpui`-free model and rules: the seeded `Payee` (`id: u32`, `name`, `aliases`, `default_category`, `is_active`), `normalise_alias`, `alias_owner`, `match_alias` (longest alias wins), `insert_payee`/`edit_payee`/`delete_payee`/`set_active`, `usage` (Split count and base-Unit total, computed live from the Transactions stub), and the counts. The dialogs' form state (`PayeeForm`/`PayeeOptions` and `DeleteAction`, which is delete, deactivate or reactivate) is in `src/payees/form.rs`. Start here; it is unit-tested without a window.
  - `src/view/payees/` — `mod.rs` the list (6a), `add_dialog.rs` the Add and Edit dialogs (6b, 6c), `rules_field.rs` the match-rule chips and input, `delete_dialog.rs` the Delete/Deactivate/Reactivate dialog (6d).
  - `src/import.rs` and `src/view/import.rs` — the stubbed Import "match payees" step (6e), opened by the `:import` palette verb: an 18-row seeded statement, `cleaned_name`/`cleaned_token` (the suggested Payee name and remembered alias), `match_row`, `summary` and `commit`, which creates new Payees, remembered aliases and one-Split Transactions in the stubs.
  - `src/shell.rs` — `handle_payees_key` (`n`/`e`/`d`), `handle_import_key`, and `enter` on a Payee row opening Transactions filtered by Payee id (mirrors `open_category_transactions`).
  - Messages in `i18n/en-US/payees.ftl` and `import.ftl`.

> **Desktop navigation:** `Payees` is no longer a rail noun and has no `g` binding. The page is `view/settings/payees.rs`, mounted by the paged Settings surface (`SettingsSection::Payees`); the dialogs and models stay in `view/payees/`. It is reached from the Settings index, `:settings payees` or the `:payees` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

Desktop locations for the ticked requirements. The TUI's PAY-001 to PAY-004 live in `crates/bins/bin-tui/src/view/payees.rs` and `src/payee/`.

| Requirement | Desktop location |
| --- | --- |
| PAY-001 | `payees::insert_payee`, `view/payees/add_dialog.rs` |
| PAY-002 | `view/payees/mod.rs` (inactive rows dimmed); Transactions payee filter |
| PAY-003 | `payees::edit_payee` |
| PAY-004 | `payees::set_active`, `view/payees/delete_dialog.rs` |
| PAY-005 | `view/payees/mod.rs`, `payees::usage` |
| PAY-007 | `Payee::default_category`, `PayeeOptions` |
| PAY-008 | `view/payees/rules_field.rs`, `payees::match_alias` |
| PAY-009 | `payees::delete_payee`, `DeleteAction` |
| PAY-010 | `import.rs`, `view/import.rs` |

## Decisions

- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — Payee as a first-class entity with rename aliases, reversing the original free-text decision. Covers why renaming changes one row rather than bulk-rewriting Transactions and why aliases are write-once. Its amendments make hand-authored match rules Payee Aliases (literal *contains*, longest wins; #282) and allow hard-deleting an unreferenced Payee (#283).
- Desktop Payees Surface map, [#281](https://github.com/IanTeda/Personal-Ledger/issues/281) — the import step's scope and entry (#284).
- `CONTEXT.md` — Payee, Split, Transaction.

## Open questions and known gaps

- **Neither Client is wired to `lib-database`**, so `resolve_or_create` has no production call site yet — the auto-create and alias-match behaviour is exercised only by its own tests.
- **`lib-database` lags the desktop rules.** `resolve.rs` and the rename path still treat an alias as an exact match; the *contains*/longest-wins matching, cross-Payee alias uniqueness, alias upper-casing, hand-added aliases (there is no `payee_aliases/insert.rs`), deleting aliases with their Payee and the default Category column exist only in the desktop stub.
- **Import is a stub.** No CSV upload or parsing, steps 1 and 3 of the handoff's stepper are not built, and the committed rows land only in the in-memory Transactions stub.
- **Default Category pre-fill is unused.** The desktop has no transaction form, so only 6e reads it.
- **No merge operation.** ADR-0012 describes the Client offering a merge with an existing Payee before insert; there is no merge query in `lib-database`.
- **No Change Set emission**, so Payee writes do not sync.
