# Categories (development)

End-user documentation: [Categories](../categories.md).

## Overview

A Category is the required classification on every Split — what the money was for. `lib-core` owns the `CategoryTypes` enum (the five accounting classifications), `lib-database` owns the `categories` table and its CRUD, and both bins currently render Categories from their own in-memory stubs rather than from the database.

The important thing to know before changing anything here: **the persisted schema is flat and the UI models a tree.** `categories` has no `parent_id` column, while both Clients present nested Categories with rollup totals. The tree-shaped schema is deliberately undecided — see [Open questions and known gaps](#open-questions-and-known-gaps).

## Data model

`categories` (`migrations/client/20241209000000_create_categories_table.sql`):

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `code` | TEXT | Unique, not null |
| `name` | TEXT | Unique, not null |
| `description` | TEXT | Nullable |
| `url_slug` | TEXT | Unique, nullable |
| `category_type` | TEXT | Not null, `CHECK` in `('asset', 'equity', 'expense', 'income', 'liability')` |
| `color` | TEXT | Nullable, `CHECK` enforces `#` prefix and length 7 |
| `icon` | TEXT | Nullable |
| `is_active` | BOOLEAN | Not null, defaults true |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_categories_set_updated_on` |

Five indexes cover the frequent lookups: `(category_type, is_active)`, `(is_active, category_type)`, `code`, and `created_on`/`updated_on` descending.

Invariants the schema does not enforce:

- **No hierarchy.** There is no `parent_id`, so nesting exists only in the bins' in-memory models. Nothing in the database prevents two Categories the UI would consider siblings from having incompatible `category_type` values.
- **Soft delete only.** A Category referenced by a Split should be deactivated via `is_active`, not deleted, matching the Unit/Payee/Account lifecycle. `delete.rs` exists and does a hard delete; nothing above it currently blocks deleting a referenced Category.
- **`code` and `name` are both globally unique**, which the flat schema can enforce but a tree schema generally cannot — two different parents will eventually want a child called "Fees".

## Domain types

- `lib_core::CategoryTypes` (`category_types.rs`) — the five accounting classifications: asset, equity, expense, income, liability. Its `as_str()` tokens are what the `category_type` `CHECK` constraint matches, so the two must stay in step.
- `lib_core::UrlSlug` (`url_slug.rs`) — the validated slug behind `url_slug`.
- `lib_core::HexColor` (`hex_color.rs`) — the validated colour behind `color`; the migration's `CHECK` is a second, weaker guard at the storage layer.
- `lib_core::RowID` (`row_id.rs`) — UUIDv7 primary key.

`CategoryTypes` also carries a `Label` implementation in `lib-locale` (`vocabulary.rs`), so a call site renders a variant with `category_type.label()` rather than its stored token. The stored token is never displayed.

## Persistence

`crates/libs/lib-database/src/categories/`, following the workspace one-file-per-operation split:

| File | Contents |
| --- | --- |
| `model.rs` | The `Categories` row struct and its `fake`-based mock constructor |
| `builder.rs` | Fluent builder for constructing rows in tests |
| `find.rs` | Queries retrieving Categories |
| `insert.rs` | Insert |
| `update.rs` | Update |
| `delete.rs` | Delete |
| `totals.rs` | FR.35 per-Category Transaction totals |

`totals.rs` is the one worth reading before touching aggregation. It sums the raw **signed** amount per Category over a date range, scoped to a single Unit or a single Account at a time. Unlike `Budgets::current_progress`, it does not negate Expense amounts, because a Category here may be any of the five types and negating would invert an Income Category's total. Summation happens in Rust over `BigDecimal`, not in SQL via `SUM()`, because `Money` is stored as arbitrary-precision TEXT that SQLite cannot sum correctly.

## UI

Neither Client reads Categories from `lib-database` yet. Both render their own stub.

- **TUI** — `crates/bins/bin-tui/src/category/` holds a `CategoryStore` seam over an in-memory tree, with `fixture.rs` supplying the data. The seam exists precisely so the screen and popups can be moved onto real persistence later by adding a second `impl CategoryStore`, with no UI rewrite. The screen itself is `view/categories.rs`. State that changes (tree shape, fold state, form drafts) lives in the owning `View`/popup and is mutated in its own `handle_key`/`update`, never routed through `view::Action` — `Action` only carries a `RowID` when signalling a transition across the `Shell` boundary. `docs/ux/tui/categories/README.md` is the authority on the model: two fixed, non-deletable roots (income and expenses), kind inherited from the root and never stored per node, unlimited depth, and each node carrying both a direct amount and a computed rollup.
- **Desktop** — `crates/bins/bin-desktop/src/view/categories/` (`mod.rs`, `add_dialog.rs`, `edit_dialog.rs`, `delete_dialog.rs`) is the full Categories surface built against `docs/ux/desktop/Categories/`: the 5a landing page (tree, sections, rollups), 5b Add dialog (name, type, parent, budget, type-locking), 5c Edit dialog (same form, pre-filled, usage notice), and 5d Delete dialog (confirmation, destructive treatment). The stub tree (`crates/bins/bin-desktop/src/categories.rs`) has 12 Categories, three levels deep, keyed by `id: u32` (not `RowID`). Only leaf Categories are assigned to Splits; a parent exists to roll up and to filter by. Status-line hints and full database wiring are future work; see the Acceptance pass in the Categories README for full details.

The two stubs are independent and do not share a model. Desktop differs from TUI in depth (3 vs. unlimited), structure (flat keys vs. semantic roots), and data (12 fixed vs. dynamic fixture).

## Traceability

Omitted: [categories.md](../categories.md) carries no requirement checklist yet, so there is nothing ticked to trace. Add the table here when `CAT-` requirements are written.

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — why Tag is a separate cross-cutting label rather than a grouping layered over Category. Relevant here because `CONTEXT.md`'s Category entry previously listed Tag under `_Avoid_`.
- `CONTEXT.md` — Category, Split, Transaction.
- FR.35 in `docs/product-requirements.md` — per-Category totals.

## Open questions and known gaps

- **The tree schema is not decided.** This is the single biggest gap. `categories` is flat; both Clients model a tree; `category/mod.rs` states the real schema waits on "a future map, once the tree-shaped schema itself is decided". Anyone adding a `parent_id` needs to settle, at minimum: whether `name`/`code` uniqueness becomes per-parent, whether `category_type` is stored per node or inherited from the root (the TUI model inherits), and what prevents cycles.
- **Neither Client is wired to `lib-database`.** Both read stubs, and the two stubs disagree — the TUI's has two fixed roots and unlimited depth, the desktop's has 12 fixed nodes keyed by `u32`.
- **`delete.rs` is a hard delete** with nothing above it enforcing the soft-delete lifecycle the ADRs describe for sibling entities.
- **No Change Set emission.** Like every other entity, Categories writes do not emit Change Sets, so nothing here syncs yet.
