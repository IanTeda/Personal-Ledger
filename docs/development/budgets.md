# Budgets (development)

End-user documentation: [Budgets](../budgets.md).

## Overview

A **Budget** is a named view over the one Ledger: a Method, one Unit locked at creation, a set of on-budget Accounts, a default flag and an archived date. A Category limits Budget holds a chain of effective-dated records per leaf Expense Category. [ADR-0028](../adr/0028-budget-amounts-are-effective-dated-and-never-rewrite-closed-months.md) and [ADR-0029](../adr/0029-budgets-are-named-overlapping-views-over-one-ledger.md) settled the design, and the [Desktop Budgets Surface](https://github.com/IanTeda/Personal-Ledger/issues/398) map settled the details (#383–#387, #399, #400).

Budgets are a VIPER module (slice 8, [#950](https://github.com/IanTeda/Personal-Ledger/issues/950); [Migrating a domain](viper-architecture.md#migrating-a-domain)). `lib-budgets` holds the model, the figures, one Interactor per intent and the Presenters for the whole View and its Dialogs, and depends on no feature crate. `lib-database` persists them, and `bin-desktop/src/budgets/` draws them. The desktop app builds the whole surface (handoff `docs/ux/desktop-mockups/14-budgets-v2/`, screens 9a–9g and 11b, 11c, 11f). The TUI's Budgets screen is a wireframe placeholder.

## Data model

The parts that are hardest to change later:

- **One chain per leaf Category per Budget, at most one record per month.** A record is **Onward** (its amount applies until the next record), **Month-only** (its month alone; the next month falls back to the carried Onward value) or **Stop** (no amount from its month). There is no stored "category default". No amount is Unbudgeted; 0.00 is budgeted.
- **Closed months are never rewritten.** Every write checks the month is the current one or later (`check_open_month`).
- **Rollover is held on each record** and applied on read. A closed month's carry is its effective budget (amount plus carry in) minus Spent, Known Costs excluded; it compounds, can go negative with Carry both, and resets on a Stop or an unbudgeted month. The current month never carries.
- **Figures are derived, never stored.** Spent is signed Splits of every Transaction Status on the on-budget Accounts. Known Costs are the open Bill Schedule entries on those Accounts: the current month's plus carried-in Overdue, none for a past month, a future month's own.
- **Parents only roll up.** A budgeted leaf that gains a child gets an automatic Stop from the current month, with any later record dropped, in every active Budget (`Budget::stop_new_parent`, run by the `StopNewParent` Interactor). Closed months keep their amounts and archived Budgets are untouched. `lib-categories`' Add and Edit call it through `CategoryBudget::stop_new_parent` (#957).
- **Exactly one default, and it can't be archived**, so an active Budget always exists. Archived Budgets are read-only (`Error::Archived`).

## Domain types

In `lib-budgets` (`crates/libs/lib-budgets/src/`), none in `lib-core`:

- `entity/`, the Entity: every rule the Budgets aggregate keeps, with no I/O (#996). `budget.rs`: `Budget`, `Method`, `Rollover`, `LimitRecord`, `applied` (what a month resolves to). `chain.rs`: the chain rules (`set_amount`, `stop`, `stop_new_parent`, `clear_month_only`, `cycle_rollover`, `save_cell`, `fill`, `set_monthly_limit`) as methods on `Budget`, with `Span`, `check_leaf` and `check_open_month`. `budgets.rs`: the `Budgets` set (`create`, `edit`, `duplicate`, `archive`, `restore`, `set_default`, which need the whole set for names and the one default; one-line wrappers over the chain rules), `NewBudget`, `StartFrom`, `check_accounts`. `seed.rs`: the sample Budgets and `PERSONAL_SPENDING_ID`.
- `error.rs`: `Error` and `Result<T>`, the Refusals an Interactor returns; a Presenter words each as a Message and `debug_assert!`s on the ones it can't receive.
- `figures.rs`: `effective_budget`, `period_figures` (the one shared period query), `health`, `dashboard_bars`, `is_over_budget`. `plan.rs`: `plan`, `PlanEdit`, `fill_preview`, `three_month_average`, `unallocated`, `AVERAGE_MONTHS`. `history.rs`: `history`, `history_range`, `history_csv`. `detail.rs`: `category_detail`.
- `protocols.rs`: the read ports `BudgetCategories`, `BudgetAccounts`, `BudgetSplits`, `BudgetBills`, `BudgetUnits` and `BudgetPayees` (bundled as `BudgetReads`) with their row types, the write port `BudgetRepository` (`upsert_budget` only: a Budget is never deleted), `BudgetsPorts`, `BudgetsOutcome` and `BudgetsRouter`.
- `interactor/<intent>.rs`, one file per Interactor with its tests: `CreateBudget`, `EditBudget`, `DuplicateBudget`, `ArchiveBudget`, `RestoreBudget`, `SetDefaultBudget`, `SetAmount`, `StopBudgeting`, `ClearMonthOnly`, `CycleRollover`, `SaveCell`, `Fill`, `SetMonthlyLimit`, `StopNewParent`. `support.rs` holds the logging and the write-a-copy helpers they share. A chain Interactor runs the rule on a copy of one Budget and upserts it only if it changed; a set-wide one writes `changed_since`.
- `presenter/`: `BudgetsPagePresenter` (one Presenter for the three tabs, whose view models are in `progress.rs`, `plan.rs` and `history.rs`), `SwitcherPresenter`, `ManagePresenter`, `BudgetFormPresenter`, `LimitFormPresenter`, `StopPresenter`, `FillPresenter` and `DetailPresenter`. Every confirming Dialog raises its own Toast through `Notify`.

## Persistence

`lib-database`'s `adapter/budgets.rs` holds the Budgets in memory and sends each write to the shared writer thread (ADR-0046), implementing `lib_budgets::BudgetRepository`; `BudgetsStore` is the store the bins share. Three tables hold a Budget (`migrations/client/`):

- `budgets`: the row, its Unit (no cascade, so a Unit a Budget uses can't be deleted) and the default flag, one default kept by a partial unique index.
- `budget_accounts`: the on-budget Accounts, `ON DELETE CASCADE` from `accounts`, so a deleted Account leaves every Budget. The adapter's memory keeps the dead id until it next loads; `Budget::counts` ignores an id with no Account, and an upsert skips it.
- `budget_limit_records`: one row per chain record, keyed by Budget, Category and month. The Category doesn't cascade, so a Category a chain names can't be deleted until the chain is gone.

An upsert writes a Budget whole: its row, then its Accounts and records replaced. `Budgets::changed_since` orders a write so the Budget losing the default goes before the one taking it. `lib_budgets::contract` (feature `contract-tests`) is the suite both the in-memory fake and the adapter pass.

`seed::sample_budgets` (Personal spending and a second Budget) is the sample's. Creating a Ledger writes an empty Personal spending Budget in the base Unit on no Accounts. `open_storage` writes no Budget: it refuses a Ledger file with none as an incomplete Ledger (ADR-0054's 2026-11-07 amendment), so an open Ledger always has one active Budget.

The Figures read the other domains through the six read ports, which `lib-database`'s `StoredBudgetReads` answers from the shared Categories, Accounts, Transactions, Bills, Units and Payees stores. They return rows, not sums, so Spent, the Rollover carry and Known Costs stay one query in `lib-budgets`.

**The Categories Monthly budget writes two domains in one commit, with no cross-domain module.** Saving a Category's Monthly budget writes the Category and the Personal spending Budget. The Desktop answers `lib-categories`' `CategoryBudget` port with `SetMonthlyLimit` over the Budgets store (`budgets::ports::set_monthly_limit`), and both writes join the one pending batch `input.rs`'s `send` commits after the Presenter's `handle`. That commit already makes them atomic, so a module such as `lib-bill-payments` would add a crate and an atomic port for nothing. This is deliberate, not an oversight; a workflow that has to write two domains outside a single `send` (a Presenter that commits for itself, or a TUI without `send`) would need one.

## UI

- **Desktop** — under `crates/bins/bin-desktop/src/budgets/`:
  - `view/page.rs` — the page Entity: header and tabs, its keys through `lib-budgets`' keymap (`budgets_page_keymap`), and the page's status line. `view/progress.rs` (9a), `view/plan.rs` (9b), `view/history.rs` (9c), `view/snapshot.rs` (what the UI tests read back).
  - `view/dialog.rs`, `view/switcher.rs` (11b), `view/manage.rs` (11f) — the Switcher and Manage budgets, Dialogs on the Entity host. `view/form_dialog.rs` and `view/form_dialog/{budget,limit,stop,fill,detail}.rs` — one hosted Entity, `FormDialog`, for the Budget form (11c), Edit budget (9e), Stop budgeting (9g), Fill from… (9f) and the Category detail (9d).
  - The ports are `lib-database`'s `StoredBudgetsPorts` over the Budgets store and `StoredBudgetReads`, which `LedgerPorts` hands the page and its Dialogs ([#1076](https://github.com/IanTeda/Personal-Ledger/issues/1076)); the input door announces a Budgets write from the commit, and the Categories' Monthly budget is set through `StoredCategoriesPorts`. Category delete drops Limits through `lib-category-deletion`'s port (#1005). `router.rs` — the `BudgetsRouter` implementation: it builds the Presenters, owns the History export's platform save dialog, the write and the outcome Toast (`ExportHistory`), and answers `show_transactions` through `transactions::router::show_filtered`.
  - The `budgets switch|new|edit|manage|duplicate|set-default|archive|restore` palette verbs are the Budgets keymap's module Commands (`navigation/command.rs`). `shell/view/rail/primary.rs` draws the Budgets badge (the default Budget's over count) and the Dashboard its budget list, both through `StoredBudgetReads` on the Budgets store; no read copy of the Budgets is kept ([#1028](https://github.com/IanTeda/Personal-Ledger/issues/1028)).
  - Messages are in `lib-budgets`' `i18n/en-US/` (`budgets.ftl` for the View, `dialogs.ftl` for the Dialogs) and the Desktop's `i18n/en-US/budgets.ftl` (History's pair label and export outcomes).
- **TUI** — a wireframe placeholder, `crates/bins/bin-tui/src/view/budgets.rs`. Its Units ports count Budgets from the Budgets store.

The Presenter takes `today` from the Shell's Clock on each event and refreshes on `month_turned`, so the View moves on when the month turns. `B` rather than `b` opens the Switcher because bare `b` is the rail toggle. `c` rather than `n` budgets a Category because `n` is New budget on this page (#400).

## Traceability

Desktop locations for the ticked requirements. Paths are under `crates/libs/lib-budgets/src/` (`lib-budgets`) or `crates/bins/bin-desktop/` (`desktop`); each test sits beside the code it tests, in that file's `tests` module, sharing the fixtures in `test_support.rs` (the rules and figures) or `interactor/test_support.rs` (the Interactors).

| Requirement | Code | Test |
| --- | --- | --- |
| BUD-001 | `entity/budget.rs::Budget`, `entity/budgets.rs::Budgets` | `entity/budgets.rs` |
| BUD-002 | `interactor::CreateBudget`, `presenter/budget_form.rs`, desktop `src/budgets/view/form_dialog/budget.rs` | `entity/budgets.rs`, `interactor/create_budget.rs`, `presenter/budget_form.rs`, desktop `tests/ui/budgets_keyboard.rs` |
| BUD-003 | `interactor::EditBudget`, `presenter/budget_form.rs` | `interactor/edit_budget.rs`, `presenter/budget_form.rs` |
| BUD-004 | `presenter/switcher.rs`, `figures::health`, desktop `src/budgets/view/switcher.rs` | `presenter/switcher.rs`, `figures.rs` |
| BUD-005 | `interactor::{DuplicateBudget, SetDefaultBudget, ArchiveBudget, RestoreBudget}`, `presenter/manage.rs` | `interactor/{duplicate,set_default,archive,restore}_budget.rs`, `presenter/manage.rs` |
| BUD-006 | `interactor::SetAmount`, `presenter/limit_form.rs`, desktop `src/budgets/view/form_dialog/limit.rs` | `interactor/set_amount.rs`, `presenter/limit_form.rs` |
| BUD-007 | `interactor::StopBudgeting`, `presenter/stop.rs`, desktop `src/budgets/view/form_dialog/stop.rs` | `interactor/stop_budgeting.rs`, `presenter/stop.rs` |
| BUD-008 | `figures::period_figures`, `presenter/progress.rs`, desktop `src/budgets/view/progress.rs` | `figures.rs`, `presenter/budgets_page.rs` |
| BUD-009 | `figures::period_figures` (parent rows, `unbudgeted_spent`) | `figures.rs` |
| BUD-010 | `figures::effective_budget`, `interactor::CycleRollover` | `figures.rs`, `interactor/cycle_rollover.rs` |
| BUD-011 | `plan::plan`, `interactor::SaveCell`, `presenter/plan.rs`, desktop `src/budgets/view/plan.rs` | `plan.rs`, `entity/chain.rs`, `interactor/save_cell.rs`, `presenter/budgets_page.rs` |
| BUD-012 | `plan::fill_preview`, `interactor::Fill`, `presenter/fill.rs` | `plan.rs`, `interactor/fill.rs`, `presenter/fill.rs` |
| BUD-013 | `history::history`, `history::history_range`, `presenter/history.rs`, desktop `src/budgets/view/history.rs` | `history.rs`, `presenter/budgets_page.rs` |
| BUD-014 | `history::history_csv`, `presenter/history.rs` (`ExportHistory`), desktop `src/budgets/router.rs` | `history.rs` (the CSV text), `presenter/budgets_page.rs` (the request); the save dialog is untested |
| BUD-015 | `presenter/detail.rs` (`BudgetsOutcome::Transactions`), `BudgetsRouter::show_transactions` | `presenter/detail.rs`, desktop `tests/ui/budgets_mouse.rs` |
| BUD-016 | `figures::dashboard_bars`, desktop `src/shell/view/rail/primary.rs` (`budget_over`) | `figures.rs`, desktop `tests/ui/dashboard_keyboard.rs` |
| BUD-017 | `interactor::SetMonthlyLimit`, `lib-database`'s `ports/categories.rs` (`CategoryBudget`) | `interactor/set_monthly_limit.rs`, desktop Categories UI tests |
| BUD-018 | `lib-ledger-creation` `NewLedger`, `lib-database` `open_storage` (refuses a Ledger with no Budget) | `lib_budgets::contract`, desktop `tests/ui/smoke_budgets_persist.rs` |

## Decisions

- [ADR-0028](../adr/0028-budget-amounts-are-effective-dated-and-never-rewrite-closed-months.md): Budget Amounts are effective-dated and never rewrite closed months.
- [ADR-0029](../adr/0029-budgets-are-named-overlapping-views-over-one-ledger.md): Budgets are named, overlapping views over one Ledger.
- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md): the Bill Schedule that Known Costs reads.
- [ADR-0041](../adr/0041-lib-database-one-file-per-entity.md), [ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md) and [ADR-0048](../adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md): the one-file adapter, the shared writer and Notices.
- `GLOSSARY.md` terms: Budget, Method, Category Limit, Budget Amount, Stop, Unbudgeted, Rollover, Budget History, Known Costs (Bills).

## Open questions and known gaps

- The Dialogs and tabs have been driven only by the headless UI tests, not walked live against the handoff, and the History export's save dialog has not been run end to end.
- The last opened Budget is not kept: the Presenter holds the current Budget in memory and opens on the default. Persisting it as a Client-scoped Preference is out of scope (#950).
- The Switcher's search takes the keys only after `/` or a click, because `j`, `k` and `n` are letters too. The handoff doesn't say how the two share the keyboard.
- The handoff doesn't say where Stop budgeting opens from. It is `s` on a Progress row and a link in Edit budget's footer.
- History's table cells show Spent only; the month's effective budget is in the chart, the Category detail and the CSV export.
- Whether an over-budget Category earns a row in the Dashboard's Needs Attention list ([ADR-0020](../adr/0020-needs-attention-as-derived-view.md)) is undecided.
- The Plan range starts no earlier than the Budget's first Budget Amount month and no later than 12 months ahead (`plan_start_bounds`, `PLAN_MAX_LEAD`); the handoff sets no bounds.
- The Dashboard budget list carries no label naming the default Budget it is showing yet.
