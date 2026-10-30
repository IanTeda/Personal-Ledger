# Tags (development)

End-user documentation: [Tags](../tags.md).

## Overview

A Tag is a freeform, cross-cutting label attached many-to-many to Splits, independent of their Category and Payee. [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) settled the domain model and vocabulary. `lib-tags` is a VIPER module (slice 5, [#847](https://github.com/IanTeda/Personal-Ledger/issues/847)): it owns the Entity and its rules, the Interactors, the ports, the Presenters, the keymap and the Messages, toolkit-free and I/O-free, and it is in `MIGRATED`, so it depends on no other feature crate. Tags persist through `lib-database` ([#862](https://github.com/IanTeda/Personal-Ledger/issues/862)), and the Desktop's Tags page and Dialogs run on its Presenters ([#863](https://github.com/IanTeda/Personal-Ledger/issues/863)).

Two halfway states remain, each a row in [During the transition](viper-architecture.md#during-the-transition):

- Tag remove and merge rewrite Splits, so they are `lib-ledger` commands that the Dialogs confirm through a port, `TagReassign`, until `lib-reassign` ([#813](https://github.com/IanTeda/Personal-Ledger/issues/813)).
- The Transactions View and Import still read Tags from a `Ledger` read copy, until they read them through their own domain.

## Data model

`tags` in `create_tags_table.sql`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `TagId` (UUIDv7, ADR-0019) |
| `name` | TEXT | Not null, not unique: the normalised-name rule is an Interactor rule |
| `color` | TEXT | The canonical `#RRGGBB`, or null for no swatch |
| `is_active` | BOOLEAN | Not null, defaults to true |
| `created_on`, `updated_on` | TEXT | ISO-8601 UTC, `updated_on` maintained by `trg_tags_set_updated_on` |

Rows read back in rowid order, the order they were added in, which an upsert keeps.

`split_tags` (`create_split_tags_table.sql`) joins a Split, by `(transaction_id, position)`, to a Tag in the order the Split lists them (`tag_position`). A Tag attaches to a **Split**, not to a Transaction, so a Transaction counts once for a Tag however many of its Splits carry it. `tag_id` references `tags(id)` with no cascade: Tag delete and merge untag the Splits first, and the key is the backstop, pinned by an adapter test. Deleting a Split or its Transaction deletes its `split_tags` rows.

A Ledger file made before #862 needs a fresh Ledger: `create_split_tags_table.sql` changed in place, and `create_tags_table.sql` sorts before it.

Invariants the schema leaves to `lib-tags`:

- **Unique by normalised name** (ADR-0015's #354 amendment): Unicode lower-cased, letters and digits only, and at least one must remain (`lib_tags::name_error`). The stub seed holds `work-trip` and `Work Trip` on purpose, so the duplicate flag and merge have something to show.
- **`is_active` is a toggle, not the delete path** (#353): removing a Tag always hard-deletes it and its join rows, used or not. An inactive Tag's name stays taken.
- **Merge** (#354): each Split with the source gains the target if absent and loses the source, then the source is deleted, in one transaction.
- **No rename aliases**, unlike Payee. Nothing auto-creates a Tag from parsed import text that a rename would need to reconcile, so a plain in-place rename is enough.
- **Never sum across Units** (#355). A Tag's total sums only Transactions sharing one Unit; a Tag spanning Units is `Total::Mixed` and the page says "mixed units".

## Domain types

- `lib_tags::Tag` — `id`, `name`, `color: Option<HexColor>` and `is_active`; `default_tags()` is the stub seed. `normalise`, `name_error`, `resolve_split_tag` (reuse an exact case-insensitive match, else create), `sorted_by_usage` and `duplicate_groups`/`duplicate_of` (the safety-net flag and suggested target) are the rules.
- `lib_tags::TagUse` — `transactions`, `total` and `last_used`, what a Tag's carrying Splits add up to. Its `Default` is hand-written, since `Total` has none.
- `lib_core::TagId` — the `RowID` newtype a Tag is keyed by (ADR-0040), and `lib_core::HexColor`.

A Tag's name is user data, not a translated Message, so there is no `Label` implementation in `lib-locale`.

## Persistence

`lib-database`'s `adapter/tags.rs` ([ADR-0041](../adr/0041-lib-database-one-file-per-entity.md), [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md)) loads every Tag into memory and queues each `TagRepository` write on the shared writer. `lib-tags`' contract suite runs on it under `sqlx::test`, failing adapter included, beside the in-memory fake.

`TagUsage` is answered there from the file: the carrying Splits with their Account's Unit code, summed in Rust (`amount` is a decimal string), from `split_tags`, `splits`, `transactions`, `accounts` and `units`. A Split listing a Tag twice counts once. The read is infallible, so a failed one is logged and answered as unused. It is a blocking read per Tag per render, as Payees' is; a grouped query or cache is worth doing if it shows.

The stub seed is written after the Categories and before the Payees and Transactions whose Splits carry it, in the first-launch seed and `db-seed`: Units, Institutions, Accounts, Categories, Tags, Payees, Transactions, Bills. Every adapter test factory that writes Split Tags stores the stub Tags first (`test_support::store_stub_references`).

## Interactors and ports

`lib-tags` declares, and the bins or `lib-database` answer:

- **`TagRepository`** — the persisted port. It has no delete, so `TagsStore::delete_tag` is a store write that the forwarding shim sends, not a port write.
- **Interactors** — `AddTag`, `EditTag` (name and colour only, leaving Active), `DeactivateTag` and `ReactivateTag`. The Edit Dialog maps its Active checkbox onto the last two.
- **`TagUsage`** — a Tag's `TagUse`, infallible. `TagsPorts` is `TagRepository + TagUsage`, as owned reads, so a store behind `Rc<RefCell<dyn TagsStore>>` can implement it by borrowing per call (`tags::ports::StorePorts`).
- **`TagReassign`** — `remove_tag` and `merge_tags`, with `TagRemoved` and `TagMerged`. Both rewrite Splits, so no Interactor does them: the rules are private functions in `lib-ledger`'s `tag_commands.rs`, with their rule tests.

The Desktop answers `TagReassign` by running `lib_ledger::RemoveTag` or `MergeTags` through `LedgerStore::apply`. That needs the gpui context, so `tags::ports::LedgerPorts` is built around each call and lent to the Presenters, at open for reads and to `confirm(port, router)` for writes (checklist step 17). Tag delete and merge's Transaction changes go to the Transactions store first and `TagsStore::delete_tag` after them (`tags::ports::forward_deletes`), in the input's one commit, so a Tag's Splits are untagged or retagged before it goes.

## UI

The Desktop reads and writes Tags through `lib-database`; the TUI renders its own stub.

- **TUI** — `crates/bins/bin-tui/src/tag/` (`mod.rs`, `fixture.rs`) holds the model and data; the screen is `view/tags.rs`. The left pane is a flat, alphabetical list, because a Tag has no classification dimension to group by. The right pane (`docs/ux/tui-mockups/06-tags/README.md`, `9a`) has **Tagged spend** (a monthly sparkline), **Where it lands** (a category breakdown) and a summary box for the selected Tag. It runs on fixtures and `lib-tags` isn't wired in.
- **Desktop** — `crates/bins/bin-desktop/src/tags/` (`ports.rs`, `router.rs`, `view/page.rs` with `page/render.rs`, and `view/dialog.rs` with `dialog/tag_form.rs`, `dialog/remove_tag.rs` and `dialog/merge_tags.rs`) draws `lib-tags`' Presenters (`TagsPageViewModel`, `TagFormPresenter`, `RemoveTagPresenter`, `MergeTagsPresenter`), built against `docs/ux/desktop-mockups/19-tags/` (7a–7e, map [#351](https://github.com/IanTeda/Personal-Ledger/issues/351)). The keymap (`keymap.rs`) is the flat `new` and `edit` plus the module's own `tags.remove` (`x`) and `tags.merge` (`m`). Remove stays `x`, not the flat `delete`, because `lib-config` defaults `delete` to `d` and an override on `x` would move the page's key. Messages are in `lib-tags/i18n/en-US/tags.ftl`.
  - `Enter` is the `select` command: `TagsPageEvent::OpenTransactions` asks `TagsRouter::open_transactions`, which calls `transactions::router::show_filtered` with `TransactionFilters::for_tag`, the Tag's id rather than a name substring. `navigation/command.rs` adds the `tags merge` palette verb.
  - `src/transactions/view/table.rs` draws a Tag's swatch on its chip.

> **Desktop navigation:** `Tags` is no longer a rail noun and has no `g` binding. The page is mounted by the paged Settings surface (`SettingsPage::Tags`), reached from the Settings index, `:settings tags` or the `:tags` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Tests

- `lib-tags` unit tests and its contract suite (`contract-tests`), run against the fake and the adapter.
- `bin-desktop/tests/ui/`: `settings_tags_keyboard.rs` and `settings_tags_mouse.rs` for the page and Dialogs (including a failing write), and `smoke_tags_persist.rs` for an added Tag and a removed used Tag surviving a restart, with a used Tag's usage read from the adapter.

## Traceability

Desktop locations for the ticked requirements. The TUI's Tags code lives in `crates/bins/bin-tui/src/tag/` and `src/view/tags.rs`.

| Requirement | Desktop location |
| --- | --- |
| TAG-005 | `tags/view/page/render.rs`, `lib_tags::TagUsage`, `lib_tags::sorted_by_usage` |
| TAG-006 | `lib_ledger::MergeTags`, `tags/view/dialog/merge_tags.rs` |
| TAG-007 | `lib_tags::EditTag`, `tags/view/dialog/tag_form.rs` |
| TAG-008 | `Tag::color`, `tags/view/dialog/tag_form.rs`; TUI `tag/colour.rs` (`ColourDraft`, `swatch_span`) |
| TAG-009 | `lib_ledger::RemoveTag`, `tags/view/dialog/remove_tag.rs` |
| TAG-010 | `lib_tags::DeactivateTag`, `lib_tags::ReactivateTag`, `tags/view/dialog/tag_form.rs` |
| TAG-011 | `lib_tags::duplicate_groups`, `tags/view/page/render.rs` |
| TAG-012 | `Tags` adapter (`lib-database`'s `adapter/tags.rs`), `tests/ui/smoke_tags_persist.rs` |

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — Tag as an independent, many-to-many label rather than a grouping over Category or Payee. Its amendments add the optional colour (#352), hard remove with `is_active` as a toggle (#353), normalised-name uniqueness and merge (#354), and the usage figures (#355).
- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — the contrast that explains why Tag needs no aliases.
- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md) and [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) — the adapter and the writer.
- `GLOSSARY.md` — Tag, Category, Payee, Split, Unit.

## Open questions and known gaps

- **Tag remove and merge aren't Interactors.** They answer `TagReassign` with a `LedgerCommand` until `lib-reassign` (#813).
- **The Transactions View and Import read Tags from `Ledger`**, a read copy refreshed after each Tags write and on rollback (`LedgerStore::reload_tags`, `Ledger::refresh_tags`).
- **`TagRows` is still public in `lib-ledger`**, now only behind `Ledger::tag_uses`, which recounts an in-memory store. It goes when the Transactions View reads its Splits through its own domain.
- **`lib-budgets` lists `lib-tags` as a dev-dependency** that nothing in its sources or tests uses (a finding of #864, left for the Budgets slice).
- **A malformed `tags.color` fails the whole load** rather than degrading to no colour.
- **`Stores::fake` takes eight arguments** and carries `#[expect(clippy::too_many_arguments)]`; give it a builder or a struct when the next domain adds a ninth.
- **No transaction form on the desktop**, so `resolve_split_tag` has no UI call site and inactive Tags' exclusion from tagging is untested in practice.
- **The 4b filter builder shows no swatch** yet, though #352 asks for one.
- **The TUI doesn't manage Tags** the desktop way; it shows and sets the optional colour (#363) on fixtures only.
- **The two Clients' stubs are independent** and share no model.
