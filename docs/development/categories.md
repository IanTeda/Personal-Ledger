# Categories (development)

End-user documentation: [Categories](../categories.md).

## Overview

A Category is the required classification on every Split — what the money was for. `lib-categories` owns the Entity, its tree rules (a leaf takes Splits, three levels at most, a child takes its parent's type, no cycles), the add, rename and move Interactors and the `CategoryRepository` port; `lib-core` owns `CategoryId` and the `CategoryTypes` enum; `lib-database`'s Categories adapter persists the tree (#858). The Desktop's Categories page still writes through `lib-ledger`'s `LedgerCommand`s, which `LedgerStore` forwards to the adapter, until the page moves onto `lib-categories`' Presenters (#859). Category delete re-points other domains' rows, so it stays a `lib-ledger` command until `lib-reassign` (#813). The TUI renders its own stub.

## Data model

`categories` (`migrations/client/20241209000000_create_categories_table.sql`), rewritten in place by #858, so a Ledger file made before it needs a fresh Ledger (its checksum changed):

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `CategoryId` (UUIDv7, ADR-0019) |
| `name` | TEXT | Not null, not unique: each type has its own top-level Uncategorised |
| `parent_id` | UUID | Null for a top-level Category, else `REFERENCES categories(id)`, no cascade |
| `category_type` | TEXT | Not null, `CHECK` in `('expense', 'income')` |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_categories_set_updated_on` |

Rows read back in rowid order, the order they were added in, which an upsert keeps; the port lists them in tree order, siblings in that order. `splits.category_id`, `bill_plans.category_id` and `payees.default_category_id` reference `categories(id)` with no cascade, and `budgets.category_id` already did.

Invariants the schema leaves to `lib-categories`: the three-level cap, a child taking its parent's type, and no cycles. The foreign keys are backstops: `parent_id`'s refuses deleting a parent (the delete rule takes leaves only), and the Splits', Bill Plans' and Payees' keys refuse deleting a Category they still name, which Category reassign re-points or clears first.

## Domain types

- `lib_categories::Category` — `id`, `name`, `parent` and `category_type`; `default_categories()` is the stub seed, 12 Categories three levels deep.
- `lib_core::CategoryId` — the `RowID` newtype a Category is keyed by (ADR-0040).
- `lib_core::CategoryTypes` (`category_types.rs`) — Expense or Income as a Category stores it. Its `as_str()` tokens are what the `category_type` `CHECK` constraint matches, so the two must stay in step.

`CategoryTypes` also carries a `Label` implementation in `lib-locale` (`vocabulary.rs`), so a call site renders a variant with `category_type.label()` rather than its stored token. The stored token is never displayed.

## Persistence

`lib-database`'s `adapter/categories.rs` (ADR-0041, ADR-0046): it loads every Category into memory and serves `CategoryRepository` from there, each write joining the one pending batch the input commits. `lib-categories`' contract suite runs on it under `sqlx::test`, failing adapter included, beside adapter tests for the foreign-key refusals. It also answers `lib-bills`' `BillCategories` and `lib-payees`' `PayeeCategories` from its rows by `lib-categories`' leaf rule. `CategoriesStore` in `adapter/store.rs` holds the adapter or the in-memory fake, with `delete_category`, the write Category reassign sends through the Desktop's forwarding shim until #813.

The stub seed is written after the Accounts and before the Payees, Transactions and Bills that name it, in the first-launch seed and `db-seed`. The old `src/categories/` module, and its FR.35 per-Category totals reader, went with the old table; nothing read them.

## UI

The Desktop reads Categories from `lib-database`; the TUI renders its own stub.

- **TUI** — `crates/bins/bin-tui/src/category/` holds a `CategoryStore` seam over an in-memory tree, with `fixture.rs` supplying the data. The seam exists precisely so the screen and popups can be moved onto real persistence later by adding a second `impl CategoryStore`, with no UI rewrite. The screen itself is `view/categories.rs`. State that changes (tree shape, fold state, form drafts) lives in the owning `View`/popup and is mutated in its own `handle_key`/`update`, never routed through `view::Action` — `Action` only carries a `RowID` when signalling a transition across the `Shell` boundary. `docs/ux/tui-mockups/03-categories/README.md` is the authority on the model: two fixed, non-deletable roots (income and expenses), kind inherited from the root and never stored per node, unlimited depth, and each node carrying both a direct amount and a computed rollup.
- **Desktop** — `crates/bins/bin-desktop/src/settings/categories/` (`mod.rs` the page Entity, `render.rs`, `form.rs`, `apply.rs`, and `dialogs/` with `add.rs`, `edit.rs`, `delete.rs`) is the full Categories surface built against `docs/ux/desktop-mockups/18-categories/`: the 5a landing page (tree, sections, rollups), 5b Add dialog (name, type, parent, budget, type-locking), 5c Edit dialog (same form, pre-filled, usage notice), and 5d Delete dialog (confirmation, destructive treatment). The page reads the tree from `LedgerStore`'s read copy, refreshed from the Categories store after each write and on rollback, and its Dialogs' `LedgerCommand`s reach the store through the forwarding shim (`settings/categories/ports.rs`), committed with the input. Only leaf Categories are assigned to Splits; a parent exists to roll up and to filter by.

The two stubs are independent and do not share a model. Desktop differs from TUI in depth (3 vs. unlimited), structure (flat keys vs. semantic roots), and data (12 fixed vs. dynamic fixture).

> **Desktop navigation:** `Categories` is no longer a rail noun and has no `g` binding. The page is `settings/categories/mod.rs`, mounted by the paged Settings surface (`SettingsPage::Categories`); its dialogs stay in `settings/categories/dialogs/`. It is reached from the Settings index, `:settings categories` or the `:categories` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

Omitted: [categories.md](../categories.md) carries no requirement checklist yet, so there is nothing ticked to trace. Add the table here when `CAT-` requirements are written.

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — why Tag is a separate cross-cutting label rather than a grouping layered over Category. Relevant here because `GLOSSARY.md`'s Category entry previously listed Tag under `_Avoid_`.
- `GLOSSARY.md` — Category, Split, Transaction.
- FR.35 in `docs/product-requirements.md` — per-Category totals.

## Open questions and known gaps

- **The TUI isn't wired to `lib-database`.** Its stub disagrees with the Desktop's tree — two fixed roots and unlimited depth against 12 nodes three levels deep — until its redesign (#805).
- **Per-Category totals (FR.35) have no reader.** The old module's `totals.rs` went with the old table.
- **No Change Set emission.** Like every other entity, Categories writes do not emit Change Sets, so nothing here syncs yet.
