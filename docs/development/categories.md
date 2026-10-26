# Categories (development)

End-user documentation: [Categories](../categories.md).

## Overview

A Category is the required classification on every Split — what the money was for. `lib-categories` is a VIPER module (slice 5, #847): it owns the Entity, its tree rules (a leaf takes Splits, three levels at most, a child takes its parent's type, no cycles), the add, rename, move and edit Interactors, the `CategoryRepository` port and the Page, Add/Edit and Delete Presenters. It depends on no other feature crate. `lib-core` owns `CategoryId` and the `CategoryTypes` enum; `lib-database`'s Categories adapter persists the tree (#858). The Desktop's Categories page and Dialogs run on the Presenters (#859). Category delete re-points other domains' rows, so it goes through a reassign port the Desktop answers with a `lib-ledger` command until `lib-reassign` (#813), and the Monthly budget goes through a port answered from the Ledger's Budgets until they persist (#811). The TUI renders its own stub.

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

## Interactors and ports

`lib-categories` declares, and the bins or `lib-database` answer:

- **`CategoryRepository`** — the persisted port, listed in tree order. Its contract suite (`contract-tests`) runs against `fake.rs` and the adapter.
- **Interactors** — `AddCategory`, `RenameCategory`, `MoveCategory` and `EditCategory`. Edit is the one the Dialog uses: it runs the rename, the move and the top-level type cascade over the rows, checks every rule, then upserts each changed row, so a refused move leaves the rename unwritten. A move is refused when its subtree would pass three levels (#870).
- **`CategoryBudget`** — the Monthly budget: read the field, `check_monthly_budget` on the tree as the save leaves it, then `set_monthly_budget`. `AddCategory` and `EditCategory` carry a `BudgetChange` and check before writing anything, so a refused budget writes no Category. The refusal is a typed `MonthlyBudgetRefusal` (not an expense leaf, read-only, unavailable).
- **`CategoryReassign`** — delete, with `CategoryReassigned` and `ReassignRefusal`. Delete re-points Splits and Bill Plans to Uncategorised and clears Payee defaults, so it is no single-domain Interactor.
- **`CategoryUsage`** — Splits and Budget Limits per Category, for the Dialogs' wording. Infallible (0 on nothing): the refusal comes from the reassign port.

The three ports the Desktop answers through `LedgerStore` need the gpui context, so `categories::ports::LedgerPorts { ledger, today, cx }` is built around each call and lent to the Presenters, at open for reads and to a separate `confirm(port, router)` for writes, rather than held. The Presenters hold `SharedCategories = Rc<RefCell<dyn CategoryRepository>>`, the Desktop's existing `SharedCategoriesStore`, so `build`, `LedgerStore`, Bills and Payees didn't change.

## UI

The Desktop reads and writes Categories through `lib-database`; the TUI renders its own stub.

- **TUI** — `crates/bins/bin-tui/src/category/` holds a `CategoryStore` seam over an in-memory tree, with `fixture.rs` supplying the data. The seam exists precisely so the screen and popups can be moved onto real persistence later by adding a second `impl CategoryStore`, with no UI rewrite. The screen itself is `view/categories.rs`. State that changes (tree shape, fold state, form drafts) lives in the owning `View`/popup and is mutated in its own `handle_key`/`update`, never routed through `view::Action` — `Action` only carries a `RowID` when signalling a transition across the `Shell` boundary. `docs/ux/tui-mockups/03-categories/README.md` is the authority on the model: two fixed, non-deletable roots (income and expenses), kind inherited from the root and never stored per node, unlimited depth, and each node carrying both a direct amount and a computed rollup.
- **Desktop** — `crates/bins/bin-desktop/src/categories/` (`ports.rs`, `router.rs`, `view/page.rs` with `page/render.rs`, and `view/dialog.rs` with `dialog/category_form.rs` and `dialog/delete_category.rs`) draws `lib-categories`' Presenters (#859), built against `docs/ux/desktop-mockups/18-categories/`: the 5a landing page (tree, sections, folds), the 5b Add and 5c Edit Dialogs (one form: name, type, parent, Monthly budget, type-locking, usage notice) and the 5d Delete Dialog (typed-back name, destructive treatment). The page reads and writes the Categories store through the Interactors, committed with the input, and refreshes the Ledger's read copy before and after each event. The Parent category field is a working select, Enter no longer confirms a name a sibling already has (the message shows under Name as you type), and refusals show as Messages (`categories-` in `lib-categories`' catalogue) rather than `CategoryError`'s `Display`. The Monthly budget and Category delete (reassign) are `lib-categories` ports the Desktop answers from the Ledger (`ports::LedgerPorts`), delete by running `lib_ledger::DeleteCategory` through the forwarding shims until `lib-reassign` (#813). Only leaf Categories are assigned to Splits; a parent exists to roll up and to filter by.

The two stubs are independent and do not share a model. Desktop differs from TUI in depth (3 vs. unlimited), structure (flat keys vs. semantic roots), and data (12 fixed vs. dynamic fixture).

> **Desktop navigation:** `Categories` is no longer a rail noun and has no `g` binding. The page is the Categories module's `categories/view/page.rs`, mounted by the paged Settings surface (`SettingsPage::Categories`); its Dialogs are Entities on the Dialog host (`categories/view/dialog.rs`). It is reached from the Settings index, `:settings categories` or the `:categories` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

| Requirement | Code | Test |
| --- | --- | --- |
| CAT-001 | `lib_categories::AddCategory`, `categories/view/dialog/category_form.rs` | `settings_categories_keyboard.rs` |
| CAT-002 | `lib_categories::tree_order`, `categories/view/page.rs` | `settings_categories_keyboard.rs` |
| CAT-003 | `lib_categories::EditCategory` | `settings_categories_keyboard.rs` |
| CAT-004 | `lib_categories::CategoryReassign`, `categories/view/dialog/delete_category.rs` | `settings_categories_keyboard.rs` |
| CAT-005 | `lib_categories::insert_category`, `move_exceeds_depth` | `lib-categories` unit tests |
| CAT-006 | `bin-tui/src/view/categories.rs` (the TUI's stub; the Desktop tree shows names and actions only) | |
| CAT-008 | `lib-database`'s `adapter/categories.rs` | `smoke_categories_persist.rs` |
| CAT-009 | `categories::ports::LedgerPorts`, `lib_ledger::DeleteCategory` | `settings_categories_keyboard.rs` |

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — why Tag is a separate cross-cutting label rather than a grouping layered over Category. Relevant here because `GLOSSARY.md`'s Category entry previously listed Tag under `_Avoid_`.
- `GLOSSARY.md` — Category, Split, Transaction.
- FR.35 in `docs/product-requirements.md` — per-Category totals.

## Open questions and known gaps

- **The TUI isn't wired to `lib-database`.** Its stub disagrees with the Desktop's tree — two fixed roots and unlimited depth against 12 nodes three levels deep — until its redesign (#805).
- **Per-Category totals (FR.35) have no reader.** The old module's `totals.rs` went with the old table.
- **A Ledger file made before #858 can't migrate.** `create_categories_table.sql` was rewritten in place, so its checksum changed and an old file needs a fresh Ledger (viper-architecture.md, Migrating a domain, step 15).
- **Delete is a store write, not a port write.** `CategoryRepository` has no delete, so `CategoriesStore::delete_category` is a store-level method the forwarding shim calls, after the Payees, Transactions and Bills writes whose foreign keys name the Category. `lib-reassign` (#813) decides whether it becomes a port write.
- **Order changed from added order to tree order.** The Ledger's read copy, `BillCategories` and `PayeeCategories` come in the store's tree order, so a child added later under an early parent lists among its siblings in the Bills and Payees selects.
- **Where the rules live twice.** `lib-ledger` still runs the same entity rules over its own rows for Category `LedgerCommand`s (reassign's Uncategorised and delete). The `docs/diagrams/workspace-dependencies.html` diagram is pinned at 2ef177e and still draws `lib-categories`' old edges to `lib-accounts` and `lib-transactions`; it is redrawn at the slice's close.
- **No Change Set emission.** Like every other entity, Categories writes do not emit Change Sets, so nothing here syncs yet.
