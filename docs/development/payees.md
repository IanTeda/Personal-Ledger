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
| `pattern` | TEXT | Not null, stored as a regex |

Indexed on `payee_aliases(payee_id)`.

Invariants and deliberate omissions:

- **`payee_aliases` rows are write-once.** An alias is only ever inserted by a rename and never updated, so the table carries no timestamp columns at all — the `id`'s embedded UUIDv7 timestamp stands in for when the rename happened. This is the same reasoning `transactions` uses to omit `created_on` (FR.21). Do not add `created_on`/`updated_on` here without revisiting that decision.
- **`pattern` is a regex by storage, not yet by use.** V1 only ever auto-generates an exact-match pattern from the literal old name. The column is a regex so a future cycle can hand-broaden a pattern to catch several old spellings without a schema change.
- **Soft delete only.** A Payee referenced by a Transaction is deactivated via `is_active`, never hard-deleted, enforced by the FK-enforcement pragma. `delete.rs` exists and does not itself check for references.
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

- **TUI** — `crates/bins/bin-tui/src/payee/` (`mod.rs`, `fixture.rs`) supplies in-memory Payees; the screen is `view/payees.rs`. Spec: `docs/ux/tui/payees/README.md`.
- **Desktop** — `crates/bins/bin-desktop/src/payees.rs` is a `gpui`-free stub seeded from the TUI's fixture names plus the few the Transactions seed data needs. Its `Payee` struct keys by `id: u32` and carries `aliases: Vec<String>` so search and filters can grow to match former names — **nothing matches on aliases yet.**

## Traceability

Omitted: [payees.md](../payees.md) carries no requirement checklist yet, so there is nothing ticked to trace. Add the table here when `PAY-` requirements are written.

## Decisions

- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — Payee as a first-class entity with rename aliases, reversing the original free-text decision. Covers why renaming changes one row rather than bulk-rewriting Transactions, why aliases are write-once, and why `pattern` is a regex.
- `CONTEXT.md` — Payee, Split, Transaction.

## Open questions and known gaps

- **Neither Client is wired to `lib-database`**, so `resolve_or_create` has no production call site yet — the auto-create and alias-match behaviour is exercised only by its own tests.
- **Alias matching is unused in the UI.** The desktop stub stores aliases but matches nothing against them; ADR-0012's suggestion UI (CC-TUI-007, issue #71) is what motivated the entity and is still unbuilt.
- **`pattern` is only ever an exact-match literal.** Hand-broadened patterns are a future cycle, and nothing validates that a stored pattern is a well-formed regex.
- **No merge operation.** ADR-0012 describes the Client offering a merge with an existing Payee before insert; there is no merge query in `lib-database`.
- **No Change Set emission**, so Payee writes do not sync.
