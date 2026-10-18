# Units (development)

End-user documentation: [Units](../units.md).

## Overview

A Unit is what a balance, principal or holding quantity is denominated in — fiat money like AUD, or a tradeable non-currency instrument like AAPL or bitcoin. Every Account has exactly one Unit, fixed at creation. `lib-core` owns the `UnitKind` enum, `lib-database` owns the `units` table and its CRUD, and the desktop Client has the most complete Unit UI in the codebase: a real Settings section with working add, edit and delete dialogs.

The defining constraint on this domain is that **Personal Ledger does not convert between Units in V1.** There is no exchange-rate table and no price-snapshot table, and the units migration says so explicitly.

## Data model

`units` (`migrations/client/20260904100000_create_units_table.sql`):

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `code` | TEXT | Unique, not null — e.g. `AUD`, `AAPL` |
| `name` | TEXT | Not null — note **not** unique, unlike `code` |
| `unit_kind` | TEXT | Not null; no `CHECK` constraint, unlike `categories.category_type` |
| `decimal_places` | INTEGER | Not null |
| `is_active` | BOOLEAN | Not null, defaults true |
| `created_on` | TEXT | ISO-8601 UTC, defaults to now |
| `updated_on` | TEXT | ISO-8601 UTC, maintained by `trg_units_set_updated_on` |

Invariants the schema does not enforce:

- **`unit_kind` has no `CHECK` constraint.** `categories.category_type` constrains its five tokens at the storage layer; `units.unit_kind` does not, so validity rests entirely on `lib_core::UnitKind` parsing on the way in. A row written by anything other than `lib-database` could hold an unparseable kind.
- **A Unit referenced by an Account must not be deleted**, only deactivated via `is_active` — the same lifecycle as Category, Payee and Account. `delete.rs` performs a hard delete and does not check for references itself.
- **`decimal_places` is not range-checked.** Nothing rejects a negative or absurd value.
- **No pricing or conversion tables exist, by decision.** FR.2 and FR.3's cross-Unit exchange-rate and weekly-price-snapshot machinery is superseded by the "no cross-Unit conversion in V1" rule in `GLOSSARY.md`'s Unit entry and the Constraints section of `docs/product-requirements.md`. The migration comment records this so nobody re-adds them casually.

## Domain types

- `lib_core::UnitKind` (`unit_kind.rs`) — the six Unit Kinds: `Currency`, `Crypto`, `Equity`, `FixedIncome`, `Commodity`, `Custom`, stored as `currency`, `crypto`, `equity`, `fixed_income`, `commodity`, `custom`. Parsing it is the only guard on the `unit_kind` column.
- `lib_core::PriceSourceId` (`ids.rs`) — a Price Source's id. Manual entry and the built-ins have fixed ids (`lib_units::MANUAL_ENTRY`, `COINGECKO`, `YAHOO_FINANCE`, `FRANKFURTER`).
- `lib_core::Money` (`money.rs`) — a `BigDecimal`-backed amount, stored as arbitrary-precision TEXT. Amounts are converted with `to_plain_string()` and never through `f64`, and are summed in Rust rather than by SQL `SUM()`.
- `lib_core::RowID` — UUIDv7 primary key.

`UnitKind` has a `Label` implementation in `lib-locale` (`vocabulary.rs`), so variants render through `unit_kind.label()` rather than their stored token. Currency formatting is Locale-aware and lives in `lib-locale`'s `format/currency.rs`: the **Unit** decides what is shown (its code and fraction digits), the **Locale** decides how (separators, symbol placement, disambiguation such as `A$` for an `en-US` reader). Fiat Units go through the ICU4X currency formatter; non-fiat Units format the quantity as a decimal and place the Unit code through a Message. See [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md).

## Price Sources

In memory only so far (SQLite is #685, fetching is not built). `lib-units` (`price_source.rs`) holds the model, following [ADR-0043](../adr/0043-built-in-price-sources-are-code-referenced-by-id-and-api-keys-never-sync.md):

- `BUILT_IN_SOURCES` is a `const` table of `BuiltInSource`s, each with an `Endpoint` (address, request template with `{symbol}` and `{quote}`, a JSON-pointer price path, and `PriceSourceAuth`, which names an API key's parameter or header but never holds the key), default `Updates` and the Unit Kinds it prices.
- The Ledger stores `PriceSourceRecord`s: a `BuiltInOverride` (a built-in's id with the person's name and `Updates`) or a whole `CustomSource`. Manual entry is never stored.
- `price_sources` resolves the records into `PriceSource` views, sorted by current name ignoring case, with Manual entry sorted in. Manual entry's name is a Message, so the Client passes it in. `sources_for_kind` filters to the sources able to price a Kind; `last_fetched` is the newest `fetched_at` among a source's Units.
- Each `UnitRow` carries a `PriceLink` (`source_id`, `symbol`, `quote`) and an optional `LastPrice` (price and `fetched_at`, no history). Editing a Unit keeps its last price unless the quote Unit changes.

`lib-ledger` (`price_source_commands.rs`) has `AddPriceSource` (Custom only), `EditPriceSource` (refused for Manual entry, or for a built-in given an endpoint) and `DeletePriceSource` (refused for a built-in or Manual entry; it moves the source's Units to Manual entry, keeping their last price, and records an `UpsertUnit` for each). `AddUnit` and `EditUnit` carry the link and refuse one naming a source that doesn't exist.

## Persistence

`crates/libs/lib-database/src/units/` — `model.rs`, `builder.rs`, `find.rs`, `insert.rs`, `update.rs`, `delete.rs`, `mod.rs`. No `totals.rs`: totals are always per-Account or per-Category within a Unit, never across Units.

The one cross-domain behaviour worth knowing: `Preferences::get_or_create_default` (in `preferences/update.rs`) **seeds a default `USD` Unit row** the first time the preferences table is used, and reuses an existing `USD` row if one is already there. The seed happens in Rust rather than as an `INSERT` in the migration because a Unit's `id` must be a genuine UUIDv7, which plain SQL cannot generate. Three tests in `preferences/update.rs` cover this: the seeding itself, idempotency, and reuse of an existing `USD` row.

## UI

- **Desktop** — the most complete Unit surface in the codebase. The Settings Units page lives in one folder, `crates/bins/bin-desktop/src/settings/units/` (ADR-0037's amendment): `mod.rs` is the `UnitsPage` Entity, `render.rs` draws a UNITS table (CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS) with per-row edit and delete buttons and a "+ Add unit" button, plus a PRICE SOURCES table (NAME / SOURCE / LAST UPDATED, 16g) listing `Ledger::price_sources` alphabetically with a muted ✗ "Not fetched yet" for a source that hasn't fetched and — for Manual entry, `form.rs` and `source_form.rs` hold the forms, `apply.rs` and `source_apply.rs` run them as `lib_ledger` commands, and `dialogs/` holds `UnitsDialog` with the Add, Edit and Delete unit Dialogs (issues #184–#186) and `dialogs/sources/`'s `PriceSourceDialog` with Add and Edit price source for a Custom source (16z·custom), Edit price source for a built-in (16z·edit) and Delete price source (16z·del), #713. A built-in's row offers Test and Edit, a Custom source's Test, Edit and Delete, and Manual entry no menu. The API key field is validated and never stored. The page reads the Units and Price Sources straight from `LedgerStore`, seeded from `lib_units::default_units(now)` through `Ledger::stub()`, not from `lib-database`. This section absorbed the removed "Ledger & units" section's "Default unit for new entries" control, which is now a `default` pill in the FLAGS column rather than a standalone control. Spec: `docs/ux/desktop-mockups/16-settings/README.md`.
- **TUI** — `crates/bins/bin-tui/src/view/units.rs` is still at wireframe stage: four labelled, bordered placeholder boxes matching the pane structure and proportions of `docs/ux/tui-mockups/07-units/README.md` §4a — unit list and summary in a roughly 36-column left column, weekly close candlestick and weekly prices filling the rest. The candlestick uses the `chandelier` crate. No list, summary detail, price table or forms are built yet. There is an older `screen/unit_detail.rs` from the pre-[ADR-0013](../adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md) layout which does use `lib_database`, still compiling but disconnected from `main.rs`.

## Traceability

Omitted: [units.md](../units.md) is a design document and carries no requirement checklist, so there is nothing ticked to trace. Add the table here when `UNT-` requirements are written.

## Decisions

- [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md) — the Locale, not a Preference, owns how an amount is formatted; the Unit owns what is shown.
- [ADR-0013](../adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md) — why `view/` supersedes `screen/` in the TUI.
- [ADR-0014](../adr/0014-preferences-table-and-leaner-sync-server-config.md) — the Preferences table that holds `default_unit_id`.
- `GLOSSARY.md` — Unit, Account, Trade.

## Open questions and known gaps

- **Price Sources are in memory only and never fetch.** Persistence is #685; where fetching runs and where a Client keeps API keys are open (ADR-0043). The Desktop's Test price source and Test with symbol still say "not yet built", and a Price Sources row takes no keyboard selection until #714.
- **The TUI Units screen is a wireframe**, so the candlestick and price panes render placeholder content.
- **Neither Client's Unit list reads `lib-database`** — the desktop reads `lib_units::default_units(now)`, and the TUI's only database-backed Unit code is the disconnected `screen/unit_detail.rs`.
- **`unit_kind` is unconstrained at the storage layer**, unlike the comparable `categories.category_type`.
- **No Change Set emission**, so Unit writes do not sync.
