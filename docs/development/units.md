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
- **A Unit referenced by an Account, a Bill Plan or a Budget must not be deleted**, only deactivated via `is_active` — the same lifecycle as Category, Payee and Account. `delete.rs` performs a hard delete and does not check for references itself; the rule lives in `lib-ledger`'s `delete_unit` (below), so persistence has to be reached through that command.
- **`decimal_places` is not range-checked.** Nothing rejects a negative or absurd value.
- **No pricing or conversion tables exist, by decision.** FR.2 and FR.3's cross-Unit exchange-rate and weekly-price-snapshot machinery is superseded by the "no cross-Unit conversion in V1" rule in `GLOSSARY.md`'s Unit entry and the Constraints section of `docs/product-requirements.md`. The migration comment records this so nobody re-adds them casually.

## Domain types

- `lib_core::UnitKind` (`unit_kind.rs`) — the six Unit Kinds: `Currency`, `Crypto`, `Equity`, `FixedIncome`, `Commodity`, `Custom`, stored as `currency`, `crypto`, `equity`, `fixed_income`, `commodity`, `custom`. Parsing it is the only guard on the `unit_kind` column.
- `lib_core::PriceSourceId` (`ids.rs`) — a Price Source's id. Manual entry and the built-ins have fixed ids (`lib_units::MANUAL_ENTRY`, `COINGECKO`, `YAHOO_FINANCE`, `FRANKFURTER`).
- `lib_core::Money` (`money.rs`) — a `BigDecimal`-backed amount, stored as arbitrary-precision TEXT. Amounts are converted with `to_plain_string()` and never through `f64`, and are summed in Rust rather than by SQL `SUM()`.
- `lib_core::RowID` — UUIDv7 primary key.

`UnitKind` has a `Label` implementation in `lib-locale` (`vocabulary.rs`), so variants render through `unit_kind.label()` rather than their stored token. Currency formatting is Locale-aware and lives in `lib-locale`'s `format/currency.rs`: the **Unit** decides what is shown (its code and fraction digits), the **Locale** decides how (separators, symbol placement, disambiguation such as `A$` for an `en-US` reader). Fiat Units go through the ICU4X currency formatter; non-fiat Units format the quantity as a decimal and place the Unit code through a Message. See [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md).

## Price Sources

Saved to SQLite by both Clients (see Persistence); fetching is not built. `lib-units` (`price_source.rs`) holds the model, following [ADR-0043](../adr/0043-built-in-price-sources-are-code-referenced-by-id-and-api-keys-never-sync.md):

- `BUILT_IN_SOURCES` is a `const` table of `BuiltInSource`s, each with an `Endpoint` (address, request template with `{symbol}` and `{quote}`, a JSON-pointer price path, and `PriceSourceAuth`, which names an API key's parameter or header but never holds the key), default `Updates` and the Unit Kinds it prices.
- The Ledger stores `PriceSourceRecord`s: a `BuiltInOverride` (a built-in's id with the person's name and `Updates`) or a whole `CustomSource`. Manual entry is never stored.
- `price_sources` resolves the records into `PriceSource` views, sorted by current name ignoring case, with Manual entry sorted in. Manual entry's name is a Message, so the Client passes it in. `sources_for_kind` filters to the sources able to price a Kind; `last_fetched` is the newest `fetched_at` among a source's Units.
- Each `UnitRow` carries a `PriceLink` (`source_id`, `symbol`, `quote`) and an optional `LastPrice` (price and `fetched_at`, no history). Editing a Unit keeps its last price unless the quote Unit changes.

`lib-ledger` (`price_source_commands.rs`) has `AddPriceSource` (Custom only), `EditPriceSource` (refused for Manual entry, or for a built-in given an endpoint) and `DeletePriceSource` (refused for a built-in or Manual entry; it moves the source's Units to Manual entry, keeping their last price, and records an `UpsertUnit` for each). `AddUnit` and `EditUnit` carry the link and refuse one naming a source that doesn't exist.

## Persistence

The Desktop saves Units, Price Source records and the base Unit through `lib-database`'s `UnitsAdapter` (`adapter/units.rs`) and the shared `Writer` (`writer.rs`), per [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md). At start `main` calls `open_storage` (`bin-desktop/src/units/store.rs`): `Writer::open_ledger` creates the `[Personal-Ledger]` `file` if it is missing and runs the client migrations, then the adapter loads, blocking. If any of that fails, the Desktop logs the error and exits. A Ledger with no base Unit is new, so it is first seeded with `InMemoryPorts::stub`'s Units and base Unit, which the stub Accounts, Bill Plans and Budgets name (#746's first-launch seed, removed once every domain persists). `build_shell` takes a `UnitsStore`, implemented by the adapter and by the in-memory ports the UI tests use. `ports::sync`, run after every Units event and Dialog confirm, commits the Interactor call's writes as one transaction. The app flushes the writer when it quits. `build_shell` also takes the writer's failure channel, which the Shell polls (`app/write_failures.rs`): when a failure names the Units, `ports::reload` puts the adapter back to the rows as stored and refreshes the `Ledger` read copy, so the Units page and the Bill Plan form's Unit picker redraw, and an error Toast says so. If the reload fails too, the rows in memory stand and the Toast says they aren't saved. The UI tests `a_failed_write_…` and `a_failed_reload_…` in `settings_units_keyboard.rs` drive this on a failing store (`Harness::with_failing_units`). The end-to-end smoke test, `tests/ui/smoke_units_persist.rs`, runs this start-up on a temporary file: it adds a Unit, restarts and finds it.

The older per-entity queries:

`crates/libs/lib-database/src/units/` — `model.rs`, `builder.rs`, `find.rs`, `insert.rs`, `update.rs`, `delete.rs`, `mod.rs`. No `totals.rs`: totals are always per-Account or per-Category within a Unit, never across Units.

The one cross-domain behaviour worth knowing: `Preferences::get_or_create_default` (in `preferences/update.rs`) **seeds a default `USD` Unit row** the first time the preferences table is used, and reuses an existing `USD` row if one is already there. The seed happens in Rust rather than as an `INSERT` in the migration because a Unit's `id` must be a genuine UUIDv7, which plain SQL cannot generate. Three tests in `preferences/update.rs` cover this: the seeding itself, idempotency, and reuse of an existing `USD` row.

## UI

- **Desktop** — the first View on a VIPER module (#751). `crates/bins/bin-desktop/src/units/` holds the Units Views and `impl UnitsRouter`; every rule, every piece of text and the keymap are `lib-units`'. `page.rs` wraps `lib-units`' page Presenter in a gpui Entity, which the Settings View hosts through `units_page`, the module's entry point in `router.rs`: it turns a key into chord text for the Units keymap (the Shell's global keys, `b` among them, stay global, and `g g` comes back as the router's movement), a click into an event, and draws the page view model (`page/render.rs`) and, at X-Large, its detail pane (`page/pane.rs`). `dialogs.rs` hosts each Dialog Presenter in today's Dialog host (`OpenDialog::Units`), keeping only gpui's widget state (which select's list is open); Confirm sends the Presenter its Confirm event, and a Dialog that closes hands its outcome back to the page, which raises the Toast or logs ACTIVITY. `build_shell` takes a `UnitsStore`: the app passes the SQLite adapter (see Persistence), and the UI tests pass `lib_units::fake::InMemoryPorts::stub`. `ports.rs` holds #746's halfway shims: `UnitUsage` and `UnitHoldings` answered from the `Ledger`, and the Ledger's read copy of the Units, the base Unit and the Price Sources refreshed after every Units event (`LedgerStore::refresh_units`, announcing `Domain::Units`), so Bills, Budgets, Transactions, the Dashboard and the Bill and Budget forms see a write at once. The General page's *Base unit* select runs `lib-units`' `SetBaseUnit` Interactor on the same ports. Tests: `settings_units_keyboard.rs` and `settings_units_mouse.rs` check wiring and drawing only; the rules are the Presenter tests'. Spec: `docs/ux/desktop-mockups/16-settings/README.md`.
- **TUI** — the second Client on the same Presenters (#755). `crates/bins/bin-tui/src/view/units.rs` holds `lib-units`' page Presenter and draws its view model in the §4a layout of `docs/ux/tui-mockups/07-units/README.md`: the Units and Price Sources tables above the selected row's summary on the left, the weekly close and weekly prices regions (which say no price history is recorded yet) above the keys box on the right. A key becomes chord text for the shared Units keymap; the arrows walk the tables and `Enter` edits. The Shell's global keys come first, so `ctrl+u` (open Units) and `g g` (Tags) never reach the keymap. `crates/bins/bin-tui/src/units/` holds the rest: `dialog.rs` hosts each Dialog Presenter as a floating window (text fields append and backspace, `←`/`→` step a select, `Tab` moves, `Enter` confirms, `Esc` cancels), and while one is open the View takes every key but `Ctrl+C` (`View::dialog_open`); `router.rs` is `impl UnitsRouter`, which collects what a Presenter asked for, a status reading as an Info Toast; `ports.rs` answers `UnitUsage` and `UnitHoldings` from `Ledger::stub()` (#746's shim; the TUI's other Views keep their fixtures, so there is no read copy to refresh); `store.rs` is the same `UnitsStore` as the Desktop's. `main` opens the `[Personal-Ledger]` `file` through `db::open_storage` before the terminal, as the Desktop does, commits after every Presenter call, polls the failure channel on each tick (rolling the Units back with an error Toast) and flushes the writer on quit. Tests: `tests/units_keyboard.rs` on the in-memory ports checks wiring and drawing; `tests/units_persist.rs` adds a Unit on a temporary file and finds it after a restart.

## Traceability

Omitted: [units.md](../units.md) is a design document and carries no requirement checklist, so there is nothing ticked to trace. Add the table here when `UNT-` requirements are written.

## Decisions

- [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md) — the Locale, not a Preference, owns how an amount is formatted; the Unit owns what is shown.
- [ADR-0013](../adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md) — why `view/` supersedes `screen/` in the TUI.
- [ADR-0014](../adr/0014-preferences-table-and-leaner-sync-server-config.md) — the Preferences table that holds `default_unit_id`.
- `GLOSSARY.md` — Unit, Account, Trade.

## Open questions and known gaps

- **Price Sources never fetch.** Where fetching runs and where a Client keeps API keys are open (ADR-0043). The Desktop's Test price source and Test with symbol still say "not yet built".
- **The TUI draws no price history**: its weekly close and weekly prices regions wait for a price history Presenter.
- **`unit_kind` is unconstrained at the storage layer**, unlike the comparable `categories.category_type`.
- **No Change Set emission**, so Unit writes do not sync.
