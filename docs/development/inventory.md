# Inventory (development)

End-user documentation: [Inventory](../inventory.md).

## Overview

The Inventory is the Ledger's register of possessions: **Locations**, each with an ordered list of **Areas** and the **Items** in them (`GLOSSARY.md`). `lib-inventory` is a VIPER module (slice 5, [#847](https://github.com/IanTeda/Personal-Ledger/issues/847)): it owns the Entity and its rules, the Interactors, the ports, the Presenters, the keymap and the Messages, toolkit-free and I/O-free, and it is in `MIGRATED`, so it depends on no other feature crate. Documents depend on Inventory (a Document Link names an Item by id), never the reverse. The ports and Interactors landed in [#865](https://github.com/IanTeda/Personal-Ledger/issues/865), persistence in `lib-database` in [#866](https://github.com/IanTeda/Personal-Ledger/issues/866), and the Desktop's Inventory page and Dialogs run on its Presenters ([#867](https://github.com/IanTeda/Personal-Ledger/issues/867)).

Two halfway states remain, each a row in [During the transition](viper-architecture.md#during-the-transition):

- Remove Location drops the Document Links to its Items, so it is a `lib-ledger` command that the Dialog confirms through a port, `InventoryReassign`, until `lib-reassign` ([#813](https://github.com/IanTeda/Personal-Ledger/issues/813)).
- Documents resolve a Link to an Item from a `Ledger` read copy, until they read Items through their own port ([#812](https://github.com/IanTeda/Personal-Ledger/issues/812)).

The Desktop's Settings › Inventory page is the only Inventory UI. The Inventory register, where Items and a Location's address and Cover are edited, is a placeholder, so no Client calls the Item and Cover Interactors yet.

## Data model

Three tables, created in this order by `create_locations_table.sql`, `create_areas_table.sql` and `create_items_table.sql` in `lib-database`'s `migrations/client/`.

`locations`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `LocationId` (UUIDv7, ADR-0040) |
| `name` | TEXT | Not null, not unique: uniqueness ignoring case is an Interactor rule |
| `address` | TEXT | Not null, empty when there is none |
| `unit_id` | UUID | Not null, references `units(id)` with no cascade; a fiat Unit, fixed on Add |
| `cover_insurer`, `cover_policy_no`, `cover_renews_on`, `cover_sum_insured`, `cover_item_limit` | TEXT | The Cover inline, all NULL when there is none. Amounts are decimal strings in the Unit |
| `created_on`, `updated_on` | TEXT | ISO-8601 UTC, `updated_on` maintained by `trg_locations_set_updated_on` |

Two CHECKs pin the Cover: insurer and sum insured are both set or both NULL, and the other three fields need an insurer.

`areas`: `id`, `location_id` (references `locations(id)`), `name`, `position` (the Area's place in its Location's tabs, from 0, `CHECK (position >= 0)`) and the timestamps, with `UNIQUE (id, location_id)`. `position` is not unique, because a Move Area swap would collide mid-write.

`items`: `id`, `location_id`, `area_id`, `name`, `replacement` (a decimal string in the Location's Unit) and the timestamps. It references its Location, its Area and the pair `(area_id, location_id)` against `areas UNIQUE (id, location_id)`, so the file refuses an Item in another Location's Area.

Nothing cascades. Remove Area moves its Items first, and Remove Location deletes the Items, then the Areas, then the Location, so each key is the backstop to an Interactor rule, and each refusal is pinned in an adapter test (`UnknownArea`, `NeedsDestination`).

Rows read back in rowid order (the order added, which an upsert keeps), and a Location's Areas in `position` order.

A Ledger file made before #866 needs a fresh Ledger. The tables are new, and the first-launch seed runs only on a new file, so an older file opens with an empty Inventory and the read copy then empties the Inventory page and drops Documents' Item chips.

Invariants the schema leaves to `lib-inventory`:

- **Names are unique ignoring case**: Location names across the Ledger, Area names within a Location. A blank name is refused.
- **A Cover validates as a whole** (`CoverDraft::validate`): no insurer clears all five fields, otherwise a sum insured above zero is needed, and a single-item limit above zero and at most the sum insured.
- **An Item** needs a name, a replacement of zero or more, and an Area of its Location.
- **Remove Area** with Items needs a destination Area of the same Location, other than itself, and refuses the Location's last Area while it holds Items (`LastAreaHoldsItems`).
- **Move Area swaps** two positions rather than shifting, so only a delta of 1 is a tab move.

## Domain types

- `lib_inventory::Inventory` — the `locations` and `items`, with the rules as methods (`add_location`, `edit_location`, `remove_location`, `add_area`, `edit_area`, `remove_area`, `move_area`, `set_cover`, `add_item`, `edit_item`, `remove_item`) and the counts and sums the page shows. `default_inventory(today)` is the stub seed.
- `Location` (`id`, `name`, `address`, `unit: UnitId`, `cover: Option<Cover>`, `areas`), `Area`, `Item` and `Cover`, with their drafts (`LocationDraft`, `CoverDraft`, `ItemDraft`) and refusal types (`LocationError`, `AreaError`, `ItemError`, `NameError`, `CoverError`).
- `lib_core::LocationId`, `AreaId`, `ItemId` — the `RowID` newtypes (ADR-0040).

`Location.unit` is a `UnitId`, not a code, so a Unit rename leaves the Location where it was. With no empty Unit to refuse there is no `LocationError::UnitMissing`: the Add form reports a missing Unit itself when the select is empty, and probes the name rules with a throwaway `UnitId::new()`.

## Persistence

`lib-database`'s `adapter/inventory.rs` ([ADR-0041](../adr/0041-lib-database-one-file-per-entity.md), [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md)) is the first multi-table adapter. It holds `locations`, `areas` and `items` as one in-memory `Inventory` and queues each write on the shared writer. Writing a Location rewrites its Areas with their `position` and deletes any it no longer lists, so Area order is the `Vec` order and needs no Area-level write. `lib-inventory`'s contract suite runs on it under `sqlx::test`, failing adapter included, beside the in-memory fake.

`InventoryStore::delete_location` is a store write rather than a port one, as `TagsStore::delete_tag` is, because Remove Location stays a `lib-ledger` command (Q18). It deletes the Items, the Areas and the Location in one change, and the forwarding shim sends it after the Document Links go.

The stub seed is written last, after the Transactions and Bills: Units, Institutions, Accounts, Categories, Tags, Payees, Transactions, Bills, Inventory, in the first-launch seed and `db-seed`. Inventory names only the Units, but the stub Documents name its Items. `seed::fake` writes no Inventory.

`db-seed` re-seeds each stub Location's Areas as the stub has them, so an Area someone added by hand to a stub Location in the dev DB, holding Items, would make it fail on the `items` key. The stock `db-fresh` then `db-seed` is unaffected.

## Interactors and ports

`lib-inventory` declares, and the bins or `lib-database` answer:

- **`InventoryRepository`** — the persisted port: `locations`, `items`, `upsert_location`, `upsert_item` and `delete_item`. It has no delete Location (see above).
- **Interactors** — `AddLocation`, `EditLocation`, `AddArea`, `EditArea`, `RemoveArea`, `MoveArea`, `AddItem`, `EditItem`, `RemoveItem` and `SetCover`. The Desktop calls the Location and Area ones; the Item and Cover ones are waiting for the register.
- **`InventoryUnits`** — the fiat Units the Add form offers, lent at open.
- **`LocationUsage`** — the Items and Documents a Location holds, which the Remove Dialog quotes, lent at open like Categories' usage. Documents depend on Inventory, so the count is the bin's.
- **`InventoryReassign`** — `remove_location`, with `LocationRemoved`. The Desktop answers it by running `lib_ledger::RemoveLocation` through `LedgerStore::apply`, which also drops the Document Links to the Location's Items; `inventory::ports::LedgerPorts` is built around each call and lent to the Presenters (checklist step 17).

`inventory::ports::forward` is the Inventory shim: it sends `DeleteLocation` to the store in the input's one commit. `UpsertLocation`, `UpsertArea` and `UpsertItem` have left `LedgerChange` with `lib-ledger`'s Add, Edit, Remove Area and Move Area commands, which `lib-inventory`'s Interactors replaced.

## UI

- **TUI** — no Inventory screen.
- **Desktop** — `crates/bins/bin-desktop/src/inventory/` (`ports.rs`, `router.rs`, `view/page.rs` with `page/render.rs`, and `view/dialog.rs` with a Dialog Entity per form) draws `lib-inventory`'s Presenters: `InventoryPageViewModel`, the Location and Area form Presenters, `RemoveLocationPresenter`, `RemoveAreaPresenter` and the last-Area notice (`AreaBlockedPresenter`). The Dialogs are Entities on the Dialog host, reading their keys through `lib-shell`'s Dialog keymap. The page is mounted by the paged Settings surface (`SettingsPage::Inventory`), designed in `docs/ux/desktop-mockups/16-settings/` (frame 16q). Messages are in `lib-inventory/i18n/en-US/inventory.ftl`.
  - The keymap (`keymap.rs`) uses the flat movement names and `new` and `edit`, plus the module's own `inventory.remove` (`x`, since the flat `delete` defaults to `d`), `inventory.add_area` (`a`), `inventory.move_area_down` and `_up` (`J`, `K`), `inventory.expand` (`right`) and `inventory.collapse` (`left`). None is a key the Shell keeps. `left` with nothing to close or climb to stays the Settings index's, through `takes`.
  - The Presenter holds which row the context menu is on and its entries; the pointer position stays in the View.
  - Cover and Item Dialogs belong to the Inventory register's ticket, not this page.

## Tests

- `lib-inventory` unit tests and its contract suite (`cargo test -p lib_inventory --features contract-tests`), run against the fake and the adapter.
- `lib-database`'s `storage.rs` test writes and rereads an Item through the Interactors.
- `bin-desktop/tests/ui/`: `settings_inventory_keyboard.rs` and `settings_inventory_mouse.rs` for the page and Dialogs (including a failing write), and `smoke_inventory_persist.rs`, which renames a Location that holds Items and adds a Location with an Area across a restart, with `DocumentsSnapshot::selected_chips` showing a Document Link still resolving.

## Traceability

Desktop locations for the ticked requirements.

| Requirement | Code | Test |
| --- | --- | --- |
| INV-001 | `lib_inventory::interactor::AddLocation`, `inventory/view/dialog/` | `settings_inventory_keyboard.rs` |
| INV-002 | `lib_inventory::interactor::EditLocation`, `inventory/view/dialog/` | `settings_inventory_keyboard.rs` |
| INV-003 | `lib_inventory::interactor::{AddArea, EditArea, MoveArea}`, `inventory/view/dialog/` | `settings_inventory_keyboard.rs` |
| INV-004 | `lib_inventory::interactor::RemoveArea`, `inventory/view/dialog/` | `settings_inventory_keyboard.rs` |
| INV-005 | `lib_ledger::RemoveLocation`, `lib_inventory::InventoryReassign`, `inventory/ports.rs` | `settings_inventory_keyboard.rs` |
| INV-006 | `lib_inventory::Inventory` counts, `inventory/view/page/render.rs` | `settings_inventory_keyboard.rs` |
| INV-007 | `InventoryAdapter` (`lib-database`'s `adapter/inventory.rs`) | `smoke_inventory_persist.rs` |

## Decisions

- [ADR-0040](../adr/0040-ledger-ids-are-uuidv7-rowid-newtypes.md) — the id newtypes.
- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the adapter and the writer.
- [ADR-0047](../adr/0047-interactors-log-ledger-changes-and-presenters-never-log.md) — Interactors log, Presenters don't.
- `GLOSSARY.md` — Location, Area, Inventory Item, Document Link, Unit.

## Open questions and known gaps

- **Remove Location isn't an Interactor.** It answers `InventoryReassign` with a `LedgerCommand` until `lib-reassign` (#813).
- **Documents read Items from `Ledger`**, a read copy refreshed after each Inventory write and on rollback (`LedgerStore::reload_inventory`, `Ledger::refresh_inventory`), until #812.
- **The register isn't built**: no Item or Cover Dialog, so `AddItem`, `EditItem`, `RemoveItem` and `SetCover` have no Client caller, and nothing on the Desktop adds an Item. The smoke test works around it by renaming a Location that already holds Items.
- **Duplicated conversion and `retain`s.** `InMemoryInventory` ↔ `Inventory` is written out in three places, and the Location delete's two `retain`s appear in both the adapter and the in-memory store. An `Inventory::remove_location_rows` or a `From` impl would fold them.
- **`Stores::fake` still lists every store** in the older `with_failing_*` constructors; `with_failing_inventory` overrides one store on `Stores::fake(stub_fakes())` and the others could follow it.
- **No Inventory on the TUI.**
