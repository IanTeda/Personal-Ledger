# Handoff: Budgets — Progress, Plan, History, Detail, Edit, Fill, Stop

## Overview
This package covers **Personal Ledger's Budgets** view. It has three tabs: **Progress** (the current period), **Plan** (a month grid for setting amounts ahead) and **History** (budget against spend per month). It also has four modals. There are seven variants, 9a–9g.

A budget is a monthly amount per category. It is the **same value** as the category's *Monthly budget* field (Categories, 5c), so editing either one updates both. Unpaid bills feed each budget as **known costs**; see the Bills handoff.

## About the design files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype**. It is a design reference, not production code. Recreate the screens in the target codebase (Rust + GPUI) using its own patterns, and follow the mockup's layout, type, colour and behaviour.

## Fidelity
**High fidelity.** Colours, typography, spacing and states are final.

## Frame
Every variant is drawn at **1280 × 800**. The shell is the same as in the other packages: header (48px), primary rail (206px), body, and a 28px status bar. Budgets is the active nav item. Its badge shows the number of categories over budget (**1**).

---

## Concepts
- **Budget amount** — a monthly figure per category, stored as effective-dated values. An edit takes effect from a chosen month, either for that month only or from that month onward. Past months are never rewritten.
- **Spent** — signed expense transactions in the category (and its children, for a parent) during the period.
- **Known costs** — unpaid **Due/Overdue** Bill Schedule rows in this period that match the budget's Category and Unit. A Paid row stops counting as a known cost once its linked transaction lands in Spent, so nothing is counted twice. Budget vs Actual in Reports (10d) leaves known costs out.
- **Left** = budget − spent − known costs. Settled in #385: Spent is signed (refunds net off), counts every Transaction Status on the Budget's on-budget Accounts; Known Costs are current-month Due plus carried-in Overdue (past months 0, future months their unpaid entries), matched by Category and on-budget Account; **over budget** is Spent > effective budget at leaf level only (parents shown, not counted), and Left < 0 from Known Costs alone is **at risk** (shown, not counted); per-day uses days remaining including today (so the mock's "9 days" on day 21 of 30 reads 10); elapsed is 100% past / 0% future; unbudgeted children are listed but not rolled into the parent's Spent; one shared `period_figures(budget, month, today)` in the desktop `budgets.rs` model feeds every surface.
- **Rollover**, per category: `None` | `Carry unspent` | `Carry unspent & overspend`. The carried amount is added to the next month's budget (for example, Electricity's September budget is 90.00 plus 90.00 carried). Settled in #384: the carry is a closed month's effective budget minus Spent (Known Costs excluded); it compounds uncapped, may go negative with overspend, is reset by a Stop or unbudgeted month, is held per Budget Amount record, and rolls up into parents and BUDGETED.
- **Unbudgeted** — spending in a category with no budget. It is always listed and never hidden.
- **Parents roll up** — a parent row sums its children, following the Categories hierarchy (5a).

---

## Screens

### 9a — Progress (default tab)
- Header: "Budgets", with the meta line "September 2026 · day 21 of 30 · **1** over budget · 58.00 unbudgeted". The `Edit plan` button jumps to the Plan tab.
- Tab row: `Progress` (active, dark fill) / `Plan` / `History`. A legend (spent · known costs · elapsed · over) sits beside the tabs, and the period nav `‹ September 2026 ›` sits on the right.
- **Stat strip**, a 4-column grid with equal cells and 2px rules:
  - BUDGETED 4,130.00 (10 categories · 90.00 carried in)
  - SPENT 3,137.60 (76% of budget · 70% of period)
  - KNOWN COSTS 673.51 (5 unpaid bills due this period, with a link to Schedule)
  - LEFT TO SPEND 318.89 (≈ 35.43 / day for 9 days)
- Table columns: CATEGORY (flex) / BUDGET / SPENT / KNOWN / LEFT (right-aligned, tabular) / PROGRESS (190px bar) / ACTIONS (`edit`).
- **Progress bar**: a solid ink segment for spent, then a **hatched** segment for known costs, set against the budget width, with a 2px vertical tick at the elapsed share of the period (70%). When spent is over the budget, the bar fills in accent and LEFT shows "68.40 over" in `#ae1800`.
- Parent rows (`▾ Utilities`, `▾ Food`) carry a "rollup" label and have no edit action. Child rows are indented. Electricity shows "incl. 90.00 carried".
- An unbudgeted row (Household, spent 58.00) reads "unbudgeted" and its action is `set`.
- Sample rows: Rent, Utilities (Electricity, Water, Internet), Food (Groceries, Dining — over), Transport, Subscriptions, Health & Fitness, Insurance, Household.
- Status bar: `NORMAL` · `j/k row · enter detail · e edit budget · [ ] period · tab switch view` · right: "September 2026 · 14 categories".

### 9b — Plan tab
- Meta: "Monthly amounts per category · past months are read-only · bold = differs from the category default". Actions: `Fill October from…` (opens 9f) and `+ Budget a category`.
- Range nav `‹ Jul – Dec 2026 ›`.
- Grid columns: CATEGORY / JUL / AUG / SEP · NOW / OCT / NOV / DEC / ROLLOVER. Rows are grouped under uppercase section labels (HOUSING, UTILITIES, FOOD, OTHER).
- **Past months** (Jul, Aug) are muted and read-only. The current month has a highlighted header.
- A **cell override** is shown in bold. Editing a cell carries the new value forward until the next override; in the sample, Dining changes to 300.00 from October. The cell being edited shows as an inline input with an accent border.
- Insurance is budgeted only in Sep and Dec, the months its quarterly bill falls due.
- ROLLOVER is a per-row control cycling `—` / carry unspent / carry both. It shows the current month's value; changing it writes an Onward record from the current month with the same amount and the new Rollover (#384).
- Footer rows: **Total budgeted** per month, and **Unallocated of 5,850.00 avg. income**, so over-allocation shows before the month starts.
- Status bar: `INSERT` · `enter save cell · esc cancel · tab next month · shift+enter this month only`.
- Settled in #386: the grid has **Normal** mode (`h/j/k/l` cell cursor, past cells selectable but not editable; `enter`/`i` edit; `r` cycle Rollover; `[ ]` shift the 6-month range; `tab` switch view; `f` Fill; `backspace`/`x` clear) and **Insert** mode (`enter` save Onward, `shift+enter` save Month-only, `tab`/`shift+tab` save Onward and move to next/previous month, `esc` cancel); clicking a future cell enters Insert. `0` writes an explicit 0.00; clearing a cell with `enter` (or `backspace`/`x`) writes a **Stop** from that month, with `shift+enter` it removes that month's own Month-only record. ROLLOVER cycles `—` → carry unspent → carry both; unbudgeted rows have none. **+ Budget a category** opens 9e with a Category picker offering leaf Expense Categories with no Budget Amount in the current month (never budgeted or Stopped), defaulting to current month, Onward, Rollover None; `set` on an unbudgeted Progress row opens it preselected. **Unallocated** = average signed Income Splits (transfers excluded) on the on-budget Accounts in the Budget's Unit over the last 3 closed months, minus each month's Total budgeted (Budget Amounts only, no carry); negative shows in accent as over-allocated.

### 9c — History tab
- Meta: "Budget vs spent by month · the average and OVER count use closed months only". Action: `Export CSV`. Range nav `‹ Apr – Sep 2026 ›`.
- **Chart**: a 6-column grid with one pair of bars per month (budget and spent), labelled "3,512.40 / 3,620.00". Over-budget months are marked. The current month (September) is labelled "to date" and hatched. Caption: "over in 3 of the last 5 closed months".
- Table: CATEGORY / BUDGET / APR…SEP / AVG / OVER (e.g. `3/5`). Over-budget cells have a `▲` prefix and accent text. SEP is excluded from AVG and OVER.
- Each past month shows the budget that applied at the time. Plan edits never rewrite history.
- Status bar: "8 of 14 categories shown".
- Settled in #387: each cell and bar is the month's **effective budget** (Budget Amount plus Rollover carry in, as recorded then) against Spent, both recomputed from the Budget's current on-budget Accounts and Category tree; Known Costs are excluded throughout (as in 10d). The chart pair totals budgeted leaves only (unbudgeted spend is left out) and a month is marked over when its total Spent exceeds its total effective budget. Rows are the leaves with a Budget Amount in any month of the visible range, under their parents as rollup rows; a month in which a leaf is unbudgeted shows `—`. AVG is the mean Spent over the closed months shown in which the row was budgeted; OVER is months over / closed months budgeted; the current month counts in neither. The status line counts leaves shown out of leaves ever budgeted in this Budget. The range nav runs from the Budget's first Budget Amount month to the current month as the rightmost column (future months belong to Plan). `h/l` move the month cursor and `j/k` the row; `enter` opens 9d for that Category and month (a parent row shows its rollup). **Export CSV** is a real file: a save dialog, then the visible table (rows, months, AVG, OVER, plain numbers without `▲`) is written and a Toast names the path.

### 9d — Category detail (modal, 600px)
Opened with `enter` on a Progress row.
- Title "Dining — September 2026".
- 4-stat grid: BUDGET 250.00 / SPENT 318.40 / KNOWN 0.00 / OVER 68.40.
- A bar, and the line "127% spent · 70% elapsed".
- "12 TRANSACTIONS · LARGEST FIRST": the top 5 rows (date / payee / account / amount), a "+ 7 more · 57.50" row, and "no bill plans in this category".
- Track record: "Over in 4 of the last 6 months · 5-month average 280.14. Rollover is off, so this overspend doesn't reduce October."
- Footer: `open in Transactions →` (goes to 4a, pre-filtered), `Close`, `Edit budget`.
- Keys: `esc` close · `e` edit · `t` transactions · `j/k` transaction.

### 9e — Edit budget (modal, 480px)
- Fields:
  - Amount per month
  - Starting: select (September 2026 current / October / November)
  - Applies to: `.seg` with "That month only" or "That month onward"
  - Rollover: `.seg` with None / Carry unspent / Carry unspent & overspend
- **Before/after summary**: "September 250.00 unchanged" · "October → onward 250.00 → 300.00" · "Total budgeted, October 3,620.00 → 3,670.00".
- Note: this is the same figure as Monthly budget on the category (5c), and editing either updates both. Past months keep their amount.
- Footer: `Cancel` + `Save budget`.

### 9f — Fill October from… (modal, 560px)
- Three radio sources, each with its resulting total:
  - **The plan** (defaults + overrides; carries are applied on read once the month closes, never filled, #384) — 3,739.97
  - **September as budgeted** (an exact copy) — 4,130.00
  - **3-month average spent** (Jul–Sep, rounded to the nearest 10) — 3,702.40
- "CHANGES FROM SEPTEMBER" diff: Dining 250.00 → 300.00 (+50.00); Insurance 420.00 → — (−420.00, no bill due); Electricity 180.00 → 159.97 (−20.03). Then "7 other categories unchanged".
- Rule: fill **never overwrites** cells the user has already edited.
- Settled in #386: the target is the first open month at or after the grid cursor's column (next month when the cursor is on a past month), and the button label follows it. **The plan** is dropped as a source; it becomes the diff baseline. Fill writes **Month-only** records for the target month only, for leaf Categories budgeted in the month before it; a Category with its own record in the target month (Onward, Month-only or Stop) is skipped by both sources and listed as "kept (edited)". The 3-month average uses the three closed months before the target (Spent from `period_figures`, Known Costs excluded), rounded to the nearest 10 half away from zero, floored at 0.00, and is shared with 11c's "Last 3 months' spending" (#400).
- Footer: `Cancel` + `Fill October`.

### 9g — Stop budgeting (modal, 420px)
- Non-destructive, so there is **no typed confirmation** (matches 7d).
- Field: From (September 2026 current / October 2026).
- Body: the category keeps its transactions. From that month its spending appears on Progress as **unbudgeted**, and its Bill Plans (Netflix, Streaming Bundle) still count as known costs. Past months keep their budget in History. Nothing is deleted.
- Footer: `Cancel` + `Stop budgeting` (solid dark, not accent).

---

## Components
| Part | Spec |
| --- | --- |
| Tab switcher | active `padding:8px 16px;background:#201e1d;color:#f3f2f2;font-weight:800`; inactive `#605d5d` |
| Stat strip | `grid-template-columns:repeat(4,minmax(0,1fr))`, 2px rules; label 11px/800 `.08em` `#9b9797`, figure 22px/800 tabular |
| Progress bar | 190×8px track `#eae9e9`; spent solid `#201e1d`; known costs hatched (45° ink stripes); over-budget fill `#ec3013`; elapsed tick 2×12px |
| Parent row | 800 weight, `▾` prefix, "rollup" tag, no action |
| Plan cell | right-aligned tabular; override bold; past muted `#9b9797`; editing = input with 2px accent border |
| Over marker | `▲` + `#ae1800` text |
| Modal | 420/480/560/600px, `border:2px solid #201e1d`, `box-shadow:0 16px 48px rgba(32,30,29,.40)`, dimmer `rgba(32,30,29,.30)` |

## Design tokens
The palette is shared with the rest of the shell:
- Ground `#f3f2f2`, chrome `#eae9e9`
- Ink `#201e1d`, secondary `#605d5d`, tertiary `#9b9797`
- Accent `#ec3013`, and `#ae1800` where accent is used for text
- Rule medium `#d7d3d3`, border `rgba(32,30,29,.30)`

Archivo throughout, weights 400 and 800 only, with tabular numerals on every figure. The radius is 0.

## Interactions
- **Progress**:
  - `j/k` row · `enter` 9d · `e` 9e · `[ ]` previous/next period · `tab` next view
  - `set` on an unbudgeted row opens 9e empty
- **Plan**:
  - Click a future cell to edit it. `enter` saves with carry-forward; `shift+enter` saves for that month only.
  - `tab` moves to the next month. Past cells ignore input.
- **History**: `enter` opens that month's detail; Export CSV exports the visible table.
- **9e/9c/Categories sync**: saving writes the effective-dated amount that the category's Monthly budget field reads.

## State
```
budgetAmount            // effective-dated
  categoryId: Id
  unit: CurrencyCode
  amount: Decimal
  effectiveFrom: YearMonth
  scope: MonthOnly | Onward
budgetCategory
  categoryId  // rollover lives on each Budget Amount record (#384); `active` superseded by Stop (#383)
budgetPeriodRow        // derived per render
  budget, carriedIn, spent, knownCosts, left, elapsedShare, overBudget: bool
planGrid
  range: [YearMonth; 6], editingCell: Option<(categoryId, YearMonth)>
fillModal
  targetMonth, source: Plan | CopyPrevious | Average3m, preview: Vec<Diff>
```

## Implementation notes (GPUI)
1. Use the HTML as a reference only. Build from GPUI elements and don't port the markup.
2. Store budgets as effective-dated records. History reads the value that applied in each month, and editing never mutates the past.
3. Known costs come from Bill Schedule. Keep a single query and share it with the Dashboard, so the figure can't double-count once a bill is Paid.
4. Rollover is applied when a period is read, not written into the next month's amount.
5. AVG and OVER use closed months only.
6. Use tabular numerals, radius 0, 2px section rules and 1px row rules. Icons are Lucide at 14px with a 1.5px stroke.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (Turn 9, options 9a–9g)
- `README.md` — this document
- `screenshots/` — one PNG per screen (9a–9g); not carried into `Budgets_v2/`, see Turn 9 of the prototype instead
