# screen/ vs view/ migration audit

Resolves [#157](https://github.com/IanTeda/Personal-Ledger/issues/157), a child of the [TUI Keybindings & Shared Navigation Grammar](https://github.com/IanTeda/Personal-Ledger/issues/155) map. Feeds [#161](https://github.com/IanTeda/Personal-Ledger/issues/161) (retire `app.rs`/`screen/` wherever this audit clears it).

## Method

`main.rs` boots `Shell::new().run()` only; `mod app;` and `mod screen;` are kept purely so the crate still compiles (per the module doc comment on `main.rs` and ADR-0013). Every file below was read in full (or to the point real vs. placeholder status was unambiguous), then cross-checked against its `view/` counterpart (found via `crates/bins/bin-tui/src/view/mod.rs`'s module list) for actual domain logic/data — not just a bordered box or a "not yet built" hint. `grep -rn` across `crates/bins/bin-tui/src` confirmed which types/modules are still referenced from `app.rs`, `view/`, `popup/`, or `shell.rs`.

`docs/ux/tui/navigation.md`'s "Current implementation status" section was not consulted for verdicts — it is known-stale (see the ticket) and this audit reads the actual code instead. One correction worth flagging: the map's own background notes assumed Units, like Accounts/Categories/Payees/Tags, already has a closed migration map with real `view/` content. The code does not bear that out — see the Units row below.

## Verdict summary

| screen/ file(s) | Domain | view/ counterpart | Verdict |
| --- | --- | --- | --- |
| `balance_check_detail.rs`, `balance_checks_list.rs` | Balance Checks | `view/balance_checks.rs` | Not migrated |
| `budget_detail.rs`, `budgets_list.rs` | Budgets | `view/budgets.rs` | Not migrated |
| `candlestick_chart.rs` | chart demo | none | Dead, no live purpose |
| `csv_import.rs` | Balance Checks CSV import | none | Not migrated |
| `dashboard.rs` | Dashboard | `view/dashboard.rs` | Partially migrated |
| `divergent_chart.rs` | chart demo | none | Dead, no live purpose |
| `doughnut_chart.rs` | chart demo | none | Dead, no live purpose |
| `help.rs` | Help | `view/help.rs` | Not migrated |
| `line_chart.rs` | chart demo | none | Dead, no live purpose |
| `reports.rs` | Reports | `view/reports.rs` | Not migrated |
| `settings.rs` | Settings | `view/settings.rs` | Migrated |
| `table.rs` | table-widget demo | none | Dead, no live purpose |
| `transaction_detail.rs`, `transactions_list.rs` | Transactions | `view/transactions.rs` | Not migrated |
| `unit_detail.rs`, `units_list.rs` | Units | `view/units.rs` | Partially migrated |

**`app.rs` cannot be fully deleted yet.** It still wires up (imports and `push`es) 12 of the 13 domain screens above (everything except the already-superseded `settings.rs`) — see its `use crate::screen::{...}` block. Ten of those domain screens are still the *only* place real CRUD/report logic exists at all (Balance Checks, Budgets, Transactions, Reports, Help, csv_import — not migrated — plus Dashboard and Units, partially). `app.rs` becomes deletable only once each of those gets a real `view/` implementation. The five standalone chart/table demo files are already unreferenced by `app.rs` itself (never wired into its navigation, confirmed below), so they and `screen/settings.rs` are the only pieces of `screen/` clear to delete today.

## Per-file findings

### `balance_check_detail.rs` + `balance_checks_list.rs` — Not migrated

Both files are real, `db`-backed CRUD (2 and 3 `db::` call sites respectively): a create/edit form mirroring `AccountDetailScreen`'s field-split pattern, and a list screen that also loads Account names for its column. `view/balance_checks.rs` (53 lines) is the literal placeholder example from the ticket's own definition: one `Block::bordered().title(" Balance Checks ")` and nothing else — no list, no form, no data. Nothing in `screen/`'s Balance Checks logic has been ported. The live command popup already defines a `check import <path.csv>` command (`popup/command/commands/balance_checks.rs`) that has no real handler yet, consistent with the view being unbuilt.

### `budget_detail.rs` + `budgets_list.rs` — Not migrated

Real, `db`-backed CRUD (3 and 4 `db::` call sites): create/edit form (limit/period/active-status only, per FR.26's field lock) and a list screen with the horizontal progress-bar-vs-"today"-line visualisation carried over from the TUI feasibility prototype, including live spend-progress computation. `view/budgets.rs` (52 lines) is the same bare-box placeholder pattern as Balance Checks. None of the real Budget logic is in `view/`.

### `candlestick_chart.rs` — Dead, no live purpose

A ratatui/`chandelier` feasibility demo (`#![allow(dead_code)]`, doc comment: "Not wired into the real navigation yet"). `app.rs`'s own `use crate::screen::{...}` import list does not include it — it was never wired into the dead `App` stack's navigation, let alone the live `Shell`. `view/units.rs` does use the `chandelier` crate directly for its own candlestick chart, but that's a fresh call site, not a reference to `screen::candlestick_chart`'s types (`grep -rn "CandlestickChartScreen"` outside `screen/` returns nothing). Safe to delete standalone.

### `csv_import.rs` — Not migrated

Real, `db`-backed (2 `db::` call sites) one-shot CSV import screen (FR.33) for Balance Checks, reached from `balance_checks_list.rs` via `i`. It is still wired into `app.rs` (`Action::OpenCsvImport => self.push(Box::new(CsvImportScreen::new()))`) and into `screen/balance_checks_list.rs`'s own key handling. No `view/` module or popup exists for it at all — not even a placeholder — though the live command popup already stages the command name (`check import <path.csv>`, `popup/command/commands/balance_checks.rs`) for when a real handler lands. Load-bearing: deleting this file today would lose the only working CSV-import logic in the crate.

### `dashboard.rs` — Partially migrated

`screen/dashboard.rs` is the real navigation hub: a fixed list of drill-in areas (each either wired to an `Action::OpenX` or flagged `not_yet_built_hint`), a live Accounts-balance snapshot loaded via `db` (reusing `AccountsLoaded`/`AccountBalancesLoaded`), and its own doc comments confirm Categories/Accounts/Payees rows were deliberately unwired here once those domains moved to `view/`. `view/dashboard.rs` (1415 lines) is visually a superset in scope — it lays out a net-worth chart, income-vs-expense, "where it went" pie, budget-progress rows and needs-attention items, matching `docs/ux/tui/README.md`'s pane structure — but every one of those panels is fed by hardcoded `fake_*()` functions (`fake_net_worth`, `fake_income_vs_expense`, `fake_spending`, `fake_budgets`, `fake_attention_items`), and the file's own module doc calls it "Wireframe stage... before any one region's real widget content... is built out." Critically, `view/dashboard.rs` implements no `handle_key` at all (confirmed via `grep`) — it has zero interactivity, so it cannot even navigate to another `View`, unlike `screen/dashboard.rs`'s drill-in menu. Missing from `view/`: any real data source (not even the one live query `screen/dashboard.rs` already has — Account balances), and all navigation/dispatch to other views.

### `divergent_chart.rs` — Dead, no live purpose

Same pattern as `candlestick_chart.rs`: a custom `Canvas`-based diverging-bar feasibility demo, `#![allow(dead_code)]`, never imported by `app.rs`, no references anywhere outside `screen/divergent_chart.rs` itself. Safe to delete standalone.

### `doughnut_chart.rs` — Dead, no live purpose

Same pattern: custom `Canvas`-based doughnut-chart feasibility demo, never wired into `app.rs`'s navigation, no outside references. (`view/dashboard.rs`'s own pie chart uses the third-party `tui_piechart` crate directly, not this module.) Safe to delete standalone.

### `help.rs` — Not migrated

`screen/help.rs` renders real, static content: the actual global-key and list-screen-vocabulary text as a `Paragraph`. `view/help.rs` (52 lines) is, again, the bare-box placeholder: `Block::bordered().title(" Help ")` and nothing else. None of the real help text has been ported into `view/`.

### `line_chart.rs` — Dead, no live purpose

Ratatui built-in `Chart`-widget feasibility demo for a line chart, `#![allow(dead_code)]`, never imported by `app.rs`, no outside references. Safe to delete standalone.

### `reports.rs` — Not migrated

The largest gap in the crate. `screen/reports.rs` is 1564 lines of real, `db`-backed logic (11 `db::` call sites): all five reports are implemented (Account Balance, Category-total, Payee-total, Budget-vs-actual, Balance-check-variance), with a unified `Field`-based focus model spanning the report picker and each report's own scope/date-range filter. `view/reports.rs` (52 lines) is the bare-box placeholder. None of this has been ported.

### `settings.rs` — Migrated

`screen/settings.rs` (43 lines) was itself never more than a stub: one hardcoded `Paragraph` string ("Default Unit for new Accounts: (none yet — build Units, issue #67, to pick one)") with no `db` import, no state, no interactivity — `update()` is a no-op. `view/settings.rs` (1172 lines) already exceeds this: a real two-pane layout (`docs/ux/tui/settings/README.md` §4a), `handle_key` wired to `e`/`Enter` opening real popup modules (`popup/settings/edit.rs`, `popup/settings/guard.rs`). It is still fake-data-backed (no live settings registry — its own doc comment says so explicitly), but that parity gap already existed in `screen/settings.rs` too, since no real Settings persistence was ever built in the old stack either. Deleting `screen/settings.rs` loses nothing not already superseded.

### `table.rs` — Dead, no live purpose

Ratatui built-in `Table`-widget feasibility demo rendering synthetic transaction rows, `#![allow(dead_code)]` (comment: "Reused by the Transactions build ticket, not yet wired into `App`"). Never imported by `app.rs`; `view/transactions.rs` is itself still a bare placeholder (see below), so the promised reuse never happened. No outside references. Safe to delete standalone.

### `transaction_detail.rs` + `transactions_list.rs` — Not migrated

Real, `db`-backed CRUD (5 and 4 `db::` call sites): the create/edit form has the most complex field-cycling logic in the crate (Reconciled-lock narrowing per FR.19), and the list screen loads Account/Category names for its columns. `view/transactions.rs` (52 lines) is the same bare-box placeholder pattern as Balance Checks/Budgets/Reports. None of this is in `view/`.

### `unit_detail.rs` + `units_list.rs` — Partially migrated

`screen/unit_detail.rs`/`units_list.rs` are real, `db`-backed CRUD (1 and 2 `db::` call sites) — the first entity CRUD built on the old stack (issue #66/CC-TUI-005). `view/units.rs` (1453 lines) plus its popup modules (`popup/unit/new.rs`, `edit.rs`, `delete.rs`) are visually and interactionally richer — a two-column layout with a live-looking unit list, weekly candlestick chart and price table, and real `n`/`e`/`d` key dispatch to `Action::OpenNewUnitPopup`/`OpenEditUnitPopup`/`OpenDeleteUnitPopup` — but every row is generated by `fake_units()`/synthetic weekly-price generators, and `popup/unit/edit.rs`'s own doc comment states plainly: "`view/units.rs` is fake-data only there, so it doesn't render one yet." No `AccountStore`-style seam (contrast `crate::account`/`crate::category`/`crate::payee`/`crate::tag`, which each have a `Store` trait over an in-memory fixture as an explicit swap-in-real-persistence seam) or any `lib_database`/`db` reference exists anywhere under `view/units.rs` or `popup/unit/`. Missing from `view/`: any real data source at all — no Store seam has even been started, let alone wired to `lib_database::Units`, so the list/candlestick/price content and the `n`/`e`/`d` popups all operate on synthetic data with no persistence.

## Domains with no screen/ counterpart at all (context, not part of the audited list)

Accounts, Categories, Payees and Tags all have real, `Store`-trait-backed `view/` implementations (`view/accounts.rs`, `categories.rs`, `payees.rs`, `tags.rs`, each 1300+ lines) with no corresponding files left in `screen/` — their old `screen/accounts_list.rs`/`account_detail.rs` etc. were already deleted as part of their own closed Wayfinder maps (issues #115, #106, #143, and the Tags equivalent). `screen/dashboard.rs`'s own doc comments explicitly confirm this for Accounts/Categories/Payees ("Retired along with `crate::screen::categories_list`/`category_detail`... the real Categories screen is being rebuilt under `crate::view::categories`"). These domains are genuinely done and don't appear in the table above because `ls crates/bins/bin-tui/src/screen/` confirms no such files exist to audit.

## Action enum analysis

This crate has two distinct `Action` enums that must not be conflated: `crate::action::Action` (`action.rs`), routed through the dead `App::update`, and `crate::view::Action` (`view/mod.rs`), routed through the live `Shell`. Several variant names are shared between them (same name, unrelated types) — `Tick`, `Quit`, `NoOp`, `OpenHelp`, `OpenUnits`, `OpenTransactions`, `OpenBudgets`, `OpenBalanceChecks`, `OpenReports`, `OpenSettings` all exist in both enums independently.

Every variant in `crate::action::Action` is still reachable from at least one `screen/` file this audit did not clear for deletion. Concretely:

- The Category/Account/Payee "loaded for reference/picker data" variants (`CategoriesLoaded`, `AccountsLoaded`, `PayeesLoaded`, `PayeeAliasesLoaded`, etc.) are no longer produced by any list/detail screen of their own (those were deleted with Accounts/Categories/Payees' migrations), but are still consumed as picker/reference data by `transaction_detail.rs`, `balance_check_detail.rs`, `budget_detail.rs`, `dashboard.rs` and `reports.rs` — all "not migrated" or "partially migrated" above, so still load-bearing.
- The full CRUD lifecycle variants for Units, Transactions, Balance Checks, Budgets (`UnitsLoaded`/`UnitSaved`/`UnitDeleted`/..., and the equivalents for the other three domains) are each used only by their own still-kept `screen/` file pair.
- The five report-computation variants (`AccountBalancesLoaded`, `CategoryTotalsLoaded`, `PayeeTotalsLoaded`, `BalanceCheckBalancesLoaded`, plus their `*Failed` counterparts) are used only by `reports.rs` (not migrated) and, for `AccountBalancesLoaded`/`AccountsLoadFailed`, also by `dashboard.rs` (partially migrated).
- `OpenCsvImport`/`BalanceChecksImported`/`BalanceChecksImportFailed` are used only by `csv_import.rs` and `balance_checks_list.rs`'s `i` key (not migrated).
- `OpenSettings` is the one variant tied to a file this audit does clear (`settings.rs`), but the variant itself is not orphaned by that deletion alone — it is still constructed by `screen/dashboard.rs`'s own menu and matched in `app.rs`; only the `app.rs` match arm and `use` of `SettingsScreen` would need a small edit.

Net effect: deleting only what this audit clears today (the five chart/table demo files, which reference zero `Action` variants, plus `screen/settings.rs`) orphans no `crate::action::Action` variant outright. The enum only starts shrinking once further domains (Dashboard, Units, and then Balance Checks/Budgets/Transactions/Reports/Help/csv_import) get real `view/` implementations of their own.
