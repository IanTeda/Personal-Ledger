# Budgets (development)

End-user documentation: [Budgets](../budgets.md).

## Overview

A **Budget** is a named view over the one Ledger: a Method, one Unit locked at creation, a set of on-budget Accounts, a default flag and an archived date. A Category limits Budget holds a chain of effective-dated records per leaf Expense Category. [ADR-0028](../adr/0028-budget-amounts-are-effective-dated-and-never-rewrite-closed-months.md) and [ADR-0029](../adr/0029-budgets-are-named-overlapping-views-over-one-ledger.md) settled the design, and the [Desktop Budgets Surface](https://github.com/IanTeda/Personal-Ledger/issues/398) map settled the details (#383–#387, #399, #400).

The desktop app builds the whole surface (handoff `docs/ux/desktop-mockups/14-budgets-v2/`, screens 9a–9g and 11b, 11c, 11f) on in-memory stubs. There is no `lib-core` type for the v2 model, `lib-database`'s `budgets` table still has the pre-v2 shape and is not read, and the TUI's Budgets screen is a wireframe placeholder.

## Data model

The parts that are hardest to change later:

- **One chain per leaf Category per Budget, at most one record per month.** A record is **Onward** (its amount applies until the next record), **Month-only** (its month alone; the next month falls back to the carried Onward value) or **Stop** (no amount from its month). There is no stored "category default". No amount is Unbudgeted; 0.00 is budgeted.
- **Closed months are never rewritten.** Every write checks the month is the current one or later (`check_open_month`).
- **Rollover is held on each record** and applied on read. A closed month's carry is its effective budget (amount plus carry in) minus Spent, Known Costs excluded; it compounds, can go negative with Carry both, and resets on a Stop or an unbudgeted month. The current month never carries.
- **Figures are derived, never stored.** Spent is signed Splits of every Transaction Status on the on-budget Accounts. Known Costs are the open Bill Schedule entries on those Accounts: the current month's plus carried-in Overdue, none for a past month, a future month's own.
- **Parents only roll up.** A budgeted leaf that gains a child gets an automatic Stop from the current month (`Budgets::stop_new_parents`).
- **Exactly one default, and it can't be archived**, so an active Budget always exists. Archived Budgets are read-only (`BudgetError::Archived`).

## Domain types

None in `lib-core` for the v2 model yet. The desktop model's types (`Budget`, `Method`, `Rollover`, `Limit`, `LimitRecord`, `Budgets`, `Span`, `NewBudget`, `StartFrom`, `BudgetError`) live in `crates/bins/bin-desktop/src/budgets/` (`mod.rs` and `store.rs`) and are the starting point for real domain types.

## Persistence

Not yet built for v2. `budgets::default_budgets` seeds the stub (Personal spending and a second Budget) with an injectable `today`. The existing `lib-database` `budgets` entity predates ADR-0028 and ADR-0029 and needs reshaping before it can hold chains.

## UI

- **Desktop** — all on in-memory stubs, under `crates/bins/bin-desktop/`:
  - `src/budgets/` — the `gpui`-free model, one module per section (`figures`, `detail`, `history`, `plan`, `store`, `surface`, `seed`) re-exported from `mod.rs`: `applied` and `effective_budget` (what a month resolves to), `period_figures` (the one shared period query), `health`, `category_detail`, `history`, `history_range` and `history_csv`, `plan` and `PlanEdit`, `fill_target`/`fill_preview`/`Budgets::fill`, `three_month_average`, `unallocated`, `dashboard_bars`, the `Switcher` state and `switcher_ids`, the `Budgets` store (`create`, `edit`, `duplicate`, `archive`, `restore`, `set_default`, `set_amount`, `stop`, `save_cell`, `cycle_rollover`, `set_monthly_limit`), and the `BudgetsTab` and `BudgetsDialog` surface state.
  - `src/budgets/limit_form.rs` (9e and 9g) and `src/budgets/form.rs` (11c) — the dialogs' pure state.
  - `src/view/budgets/` — `mod.rs` the shared header and tabs, `progress.rs` (9a), `plan.rs` (9b), `history.rs` (9c), `detail_dialog.rs` (9d), `limit_dialog.rs` (9e), `fill_dialog.rs` (9f), `stop_dialog.rs` (9g), `switcher.rs` (11b), `budget_dialog.rs` (11c), `manage_dialog.rs` (11f).
  - `src/shell.rs` — `handle_budgets_key` (`B`, `n`, `c`, `e`, `s`, `[`, `]`, `1`–`3`, and on History `h`/`l`/`x`), `handle_budgets_plan_key` and `handle_budgets_plan_edit_key` (the grid's Normal and Insert modes), `handle_budgets_tab_key`, `handle_budgets_dialog_key` and its per-dialog handlers, `switch_budget`, `export_budgets_history` (the platform save dialog), `budgets_hints` (the status-line legend) and `save_category_budget` (Categories 5c's write).
  - `src/navigation/command.rs` — the `budgets switch|new|edit|manage|duplicate|set-default|archive|restore` palette commands (`BudgetsVerb`).
  - `src/chrome/rail/primary.rs` — the Budgets badge (the default Budget's over count); `src/view/dashboard.rs` — the budget list, from `budgets::dashboard_bars`.
  - Messages in `i18n/en-US/budgets.ftl`.
- **TUI** — a wireframe placeholder, `crates/bins/bin-tui/src/view/budgets.rs`.

`B` rather than `b` opens the Switcher because bare `b` is the rail toggle. `c` rather than `n` budgets a Category because `n` is New budget on this page (#400).

## Traceability

Desktop locations for the ticked requirements. Paths are under `crates/bins/bin-desktop/src/`; tests are in the named file's `tests` module.

| Requirement | Code | Test |
| --- | --- | --- |
| BUD-001 | `budgets::Budget`, `budgets::Budgets` | `budgets/tests.rs` |
| BUD-002 | `Budgets::create`, `budgets/form.rs`, `view/budgets/budget_dialog.rs` | `budgets/tests.rs`, `budgets/form.rs` |
| BUD-003 | `Budgets::edit`, `budgets::form::BudgetForm::from_budget` | `budgets/tests.rs`, `budgets/form.rs` |
| BUD-004 | `budgets::Switcher`, `budgets::health`, `view/budgets/switcher.rs`, `Shell::switch_budget` | `budgets/tests.rs` |
| BUD-005 | `Budgets::{duplicate, set_default, archive, restore}`, `view/budgets/manage_dialog.rs` | `budgets/tests.rs`, `view/budgets/manage_dialog.rs` |
| BUD-006 | `Budgets::set_amount`, `budgets/limit_form.rs`, `view/budgets/limit_dialog.rs` | `budgets/tests.rs`, `budgets/limit_form.rs` |
| BUD-007 | `Budgets::stop`, `budgets::limit_form::StopForm`, `view/budgets/stop_dialog.rs` | `budgets/tests.rs`, `budgets/limit_form.rs` |
| BUD-008 | `budgets::period_figures`, `view/budgets/progress.rs` | `budgets/tests.rs` |
| BUD-009 | `budgets::period_figures` (parent rows, `unbudgeted_spent`) | `budgets/tests.rs` |
| BUD-010 | `budgets::effective_budget`, `Budgets::cycle_rollover` | `budgets/tests.rs` |
| BUD-011 | `budgets::plan`, `Budgets::save_cell`, `view/budgets/plan.rs` | `budgets/tests.rs` |
| BUD-012 | `budgets::fill_preview`, `Budgets::fill`, `view/budgets/fill_dialog.rs` | `budgets/tests.rs` |
| BUD-013 | `budgets::history`, `budgets::history_range`, `view/budgets/history.rs` | `budgets/tests.rs` |
| BUD-014 | `budgets::history_csv`, `Shell::export_budgets_history` | `budgets/tests.rs` (the CSV text only) |
| BUD-015 | `Shell::open_budgets_detail_transactions`, `TransactionFilters::for_budget_category` | `transactions/query.rs` |
| BUD-016 | `chrome/rail/primary.rs` (`budget_over`), `budgets::dashboard_bars`, `view/dashboard.rs` | `budgets/tests.rs` |
| BUD-017 | `Budgets::{monthly_limit, set_monthly_limit}`, `Shell::save_category_budget`, `categories::BudgetLock` | `budgets/tests.rs` |

## Decisions

- [ADR-0028](../adr/0028-budget-amounts-are-effective-dated-and-never-rewrite-closed-months.md): Budget Amounts are effective-dated and never rewrite closed months.
- [ADR-0029](../adr/0029-budgets-are-named-overlapping-views-over-one-ledger.md): Budgets are named, overlapping views over one Ledger.
- [ADR-0019](../adr/0019-materialized-bill-schedule-transaction-linked.md): the Bill Schedule that Known Costs reads.
- `CONTEXT.md` terms: Budget, Method, Category Limit, Budget Amount, Stop, Unbudgeted, Rollover, Budget History, Known Costs (Bills).

## Open questions and known gaps

- The surface has not been walked live against the handoff. The Switcher's position under the title is computed from the rail widths (`BUDGETS_SWITCHER_TOP`, `Shell::render_budgets_switcher`), and the History export's save dialog has not been run end to end.
- The last opened Budget is Shell state only; persisting it as a Client-scoped Preference waits on real persistence.
- The Switcher's search takes the keys only after `/` or a click, because `j`, `k` and `n` are letters too. The handoff doesn't say how the two share the keyboard.
- The handoff doesn't say where Stop budgeting opens from. It is `s` on a Progress row and a link in Edit budget's footer.
- History's table cells show Spent only; the month's effective budget is in the chart, the Category detail and the CSV export.
- Whether an over-budget Category earns a row in the Dashboard's Needs Attention list ([ADR-0020](../adr/0020-needs-attention-as-derived-view.md)) is undecided.
- The Plan range starts no earlier than the Budget's first Budget Amount month and no later than 12 months ahead (`plan_start_bounds`, `PLAN_MAX_LEAD`); the handoff sets no bounds.
- The Dashboard budget list carries no label naming the default Budget it is showing yet.
