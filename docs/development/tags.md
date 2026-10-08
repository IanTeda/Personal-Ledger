# Tags (development)

End-user documentation: [Tags](../tags.md).

## Overview

A Tag is a freeform, cross-cutting label attached many-to-many directly to Transactions, independent of their Category and Payee. [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) settled the domain model and vocabulary and **deferred schema, UI and reporting to a future build effort**.

That deferral is the thing to know before working here: there is **no `tags` table, no `lib-core` type and no `lib-database` module**. Everything that exists is UI built against in-memory stubs: a read-only screen in the TUI and a full management surface in the desktop. Nothing persists.

## Data model

Not yet built. No migration exists in `migrations/client/`, and nothing in `lib-database` references Tags.

ADR-0015 fixes these properties for whoever builds the schema:

- **Globally unique by normalised name** (ADR-0015's #354 amendment): Unicode lower-cased, letters and digits only, and at least one must remain. `COLLATE NOCASE` alone is not enough; the schema needs a stored normalised column (or equivalent) with the unique index.
- **An optional colour** (#352), any hex value stored as `lib-core`'s `HexColor`; nullable.
- **Many-to-many with Transaction**, so a join table is required. Note the end-user documentation and the TUI model attach Tags at the **Split** level in places and the Transaction level in others — see [Open questions](#open-questions-and-known-gaps).
- **`is_active` is a toggle, not the delete path** (#353): removing a Tag always hard-deletes it and its join rows, used or not. An inactive Tag's name stays taken.
- **Merge** (#354): each Split with the source gains the target if absent and loses the source, then the source is deleted, in one transaction.
- **No rename aliases**, unlike Payee. Nothing auto-creates a Tag from parsed import text that would later need reconciling under a rename, so a plain in-place rename is enough and no `tag_aliases` table is needed.
- **Never sum across Units.** A Tag's total only sums across Transactions sharing one Unit; a Tag spanning mixed Units reports "mixed units" on the list and per-Unit subtotals via its filtered Transactions (#355). This extends the rule `docs/ux/tui-mockups/02-accounts/README.md` already established for type-grouped subtotals.

## Domain types

None. There is no `lib_core::Tag`, and no `Label` implementation in `lib-locale` — a Tag's name is user data, not a translated Message, so it correctly has none.

## Persistence

Not yet built.

## UI

Both Clients render Tags from in-memory fixtures.

- **TUI** — the more complete of the two. `crates/bins/bin-tui/src/tag/` (`mod.rs`, `fixture.rs`) holds the model and data; the screen is `view/tags.rs`. The left pane is a flat, alphabetically sorted list, because a Tag has no classification dimension to group by. The right pane (issue #133, from the "Tags right pane" map, issue #131) implements `docs/ux/tui-mockups/06-tags/README.md`'s `9a` spec with three widgets: **Tagged spend** (a monthly sparkline mirroring `view::accounts::render_balance_chart`), **Where it lands** (a proportional category breakdown), and a summary box for the selected Tag. Its data comes from the fixtures landed by issue #132.
- **Desktop** — the Tags surface (handoff `docs/ux/desktop-mockups/19-tags/`, screens 7a–7e, map [#351](https://github.com/IanTeda/Personal-Ledger/issues/351)), all on in-memory stubs:
  - `src/tags/mod.rs` — `gpui`-free model and rules: the seeded `Tag` (`id: u32`, `name`, `color: Option<HexColor>`, `is_active`), `normalise`, `name_error`, `insert_tag`/`edit_tag`/`set_active`/`remove_tag`/`merge_tags`, `resolve_split_tag` (reuse an exact case-insensitive match, else create), `usage` and `transaction_count` (computed live from the Transactions stub), `sorted_by_usage`, `duplicate_groups`/`duplicate_of` (the safety-net flag and suggested target). The dialog forms `TagForm`, `RemoveTagForm` and `MergeTagsForm` are in `src/tags/form.rs`. The seed includes `Work Trip`/`work-trip`, which bypasses the uniqueness rule on purpose so the flag and merge have something to show. Start here; it is unit-tested without a window.
  - `src/view/tags/` — `mod.rs` the list (7a) and the shared `swatch`, `add_dialog.rs` (7b), `edit_dialog.rs` (7c), `colour_field.rs` the swatch row and hex box, `remove_dialog.rs` (7d), `merge_dialog.rs` (7e).
  - `src/shell.rs` — `handle_tags_key` (`n`/`e`/`x`/`m`), `handle_tags_dialog_key`, `open_merge_tags_dialog`, and `open_tag_transactions`, which opens Transactions filtered by exact Tag id (`transactions::query`'s `tag_id`). `src/navigation/command.rs` adds the `tags merge` palette verb.
  - `src/view/transactions/table.rs` draws a Tag's swatch on its chip.
  - Messages in `i18n/en-US/tags.ftl`.

> **Desktop navigation:** `Tags` is no longer a rail noun and has no `g` binding. The page is `view/settings/tags.rs`, mounted by the paged Settings surface (`SettingsSection::Tags`); the dialogs and models stay in `view/tags/`. It is reached from the Settings index, `:settings tags` or the `:tags` alias, and a persisted `Noun` that no longer exists loads as its Settings page.

## Traceability

Desktop locations for the ticked requirements. The TUI's Tags code lives in `crates/bins/bin-tui/src/tag/` and `src/view/tags.rs`.

| Requirement | Desktop location |
| --- | --- |
| TAG-005 | `view/tags/mod.rs`, `tags::usage`, `tags::sorted_by_usage` |
| TAG-006 | `tags::merge_tags`, `view/tags/merge_dialog.rs` |
| TAG-007 | `tags::edit_tag`, `view/tags/edit_dialog.rs` |
| TAG-008 | `Tag::color`, `view/tags/colour_field.rs`; TUI `tag/colour.rs` (`ColourDraft`, `swatch_span`) |
| TAG-009 | `tags::remove_tag`, `view/tags/remove_dialog.rs` |
| TAG-010 | `tags::set_active`, `view/tags/edit_dialog.rs` |
| TAG-011 | `tags::duplicate_groups`, `view/tags/mod.rs` |

## Decisions

- [ADR-0015](../adr/0015-tag-as-independent-transaction-label.md) — Tag as an independent, many-to-many Transaction label rather than a grouping over Category or Payee, including why the rejected alternative cannot express a cross-cutting total over an untagged Category. Its amendments add the optional colour (#352), hard remove with `is_active` as a toggle (#353), normalised-name uniqueness and merge (#354), and the usage figures (#355).
- [ADR-0012](../adr/0012-payee-entity-with-rename-aliases.md) — the contrast that explains why Tag needs no aliases.
- `CONTEXT.md` — Tag, Category, Payee, Split, Unit.

## Open questions and known gaps

- **Nothing persists.** Schema, CRUD and reporting are all unbuilt, by ADR-0015's own scoping.
- **Transaction or Split?** ADR-0015 says Tags attach to Transactions, and [transactions.md](transactions.md)'s schema sketch has `split_tags` joining to a Split instead. These are different designs with different totals behaviour, and the discrepancy needs resolving before a migration is written.
- **No transaction form on the desktop**, so `resolve_split_tag` has no UI call site and inactive Tags' exclusion from tagging is untested in practice.
- **The 4b filter builder shows no swatch** yet, though #352 asks for one.
- **The TUI doesn't manage Tags** the desktop way; it does show and set the optional colour (#363) on its Tags screen, but has no Transactions tag chips to draw it on yet.
- **The two Clients' stubs are independent** and share no model.
