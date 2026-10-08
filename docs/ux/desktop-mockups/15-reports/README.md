# Handoff: Reports — Categories, Payees, Balances, Budget vs Actual, Balance Checks

> **Package 15 of 20 · Reports** — frames 15a–15i (15a, 15b, 15c, 15d, 15e, 15f, 15g, 15h, 15i). Open `Reports.dc.html` in a browser from this folder. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

## Overview
This package covers **Personal Ledger's Reports** view. It is one screen with five report kinds as tabs, matching the TUI's single picker (`reports.rs`). The desktop app opens on the **Category Total** tab; the TUI opens on Account Balance. There are four supporting dialogs, for nine variants in total (15a–15i).

## About the design files
`Reports.dc.html` is a **high-fidelity HTML prototype**. It is a design reference, not production code. Recreate the screens in Rust + GPUI using the codebase's own patterns.

## Fidelity
**High fidelity.** Colours, typography, spacing and states are final.

## Frame
Every variant is drawn at **1280 × 800** in the shared shell: header 48px, rail 206px, status bar 28px. Reports is the active nav item.

---

## Concepts
- **Scope**: `Unit` or `Account`, one at a time and never mixed. Categories and Payees share the same scope and date range, so switching between those tabs keeps the question.
- **Range and compare**: a date range plus a comparison range derived from it (for example 1–26 Sep compared with 1–26 Aug). `‹ shift range ›` steps both ranges together.
- **Units are never converted** (FR.34). Each Unit gets its own total, and there is no cross-Unit grand total.
- **Signed amounts**: expenses are negative and income is positive.
- **Uncategorised** spending is always listed, never hidden.
- **Balance adjustments** are real transactions in the "Balance adjustment" category, so a fix shows up in reports.

---

## Common header (Categories / Payees)
- Title "Reports" with a one-line description of the active tab, and `Export…` (opens 15i).
- Tab row: `Categories` / `Payees` / `Balances` / `Budget vs Actual` / `Balance Checks`, with the hint `1–5 switch report`.
- Filter row (30px controls):
  - `Unit | Account` seg
  - UNIT select ("AUD — Australian dollar")
  - RANGE ("This month · 1–26 Sep 2026", opens 15f)
  - COMPARE ("1–26 Aug 2026")
  - `‹ shift range ›`

## Screens

### 15a — Category Total (default)
- 4-cell stat strip:
  - EXPENSES −3,236.80 (39 txns · ▲ 27.91 vs 1–26 Aug)
  - INCOME +5,678.12 (3 txns)
  - NET +2,441.32 (43% of income kept)
  - UNCATEGORISED 41.20 (3 transactions to review, with a link to Transactions)
- Table: CATEGORY / TOTAL (110px) / SHARE OF EXPENSES (230px bar) / % (60px) / VS 1–26 AUG (70px) / TXNS.
- Sections EXPENSES ("9 categories · bars scaled to the largest") and INCOME. Bars are scaled to the largest row in their section.
- Parents roll up (`▾ Food` "rollup") with indented children, as in 18a.
- The VS column shows `▲`/`▼` with the change amount, or 0.00.
- `enter` on a row opens 15g.

### 15b — Payee Total
- Uses the same scope and range as 15a, with the caption "scope & range shared with Categories".
- Split into **PAID TO** ("17 payees · largest first") and **RECEIVED FROM**, so income doesn't sit on top of spending.
- Columns: PAYEE / TOTAL / RANKED (200px bar) / TXNS / AVG / MAIN CATEGORY / VS 1–26 AUG.
- The long tail folds into a single "N others" row.
- `enter` opens 15g split by category.

### 15c — Account Balance
- An AS OF control ("Today · 26 Sep 2026") replaces the range. There is an `Active | Include inactive` seg, and UNIT reads "Not applicable" because every account reports in its own Unit.
- 3-cell strip, one cell per Unit: AUD 23,080.24 (5 accounts · ▲ 482.78 in 30 days) / USD 1,240.55 / BTC 0.04210000.
- Table grouped by Unit, with columns ACCOUNT / TYPE / 30 DAYS / LAST BALANCE CHECK / BALANCE.
- The last-check cell reads "31 Aug · ⚑ −42.50" for a mismatch, "31 Aug · ✓", or "31 Jul · 57 days" when the account hasn't been checked for a while. It links to 15e.
- Inactive accounts are muted and labelled "· inactive".
- A side panel shows the month-by-month net change, switchable per Unit.
- Amounts use each Unit's own precision (BTC to 8 dp).

### 15d — Budget vs Actual
- Actions: `Export…` and `Open in Budgets`. PERIOD "Current · September 2026", UNIT AUD, and the note "no filters: each budget carries its own category, unit and period".
- Columns: # / CATEGORY / LIMIT / SPENT / REMAINING / USED bar (tick at 70% elapsed) / USED % / STATUS.
- Rows are ranked by share used. STATUS is "Over by 68.40" (accent), "Spent in full", "On pace" and so on.
- **Known costs from Bills are excluded**; this report is spend only. It is the flat, exportable form of 13a, and 13a remains the place to act.

### 15e — Balance Check Variance
- Actions: `+ Record a check`, `Export…`.
- Filter: `All 10 | Mismatched 2` seg and an ACCOUNT select, with the note "sorted by size of variance, then newest".
- 4-cell strip:
  - CHECKS 10 across 6 accounts
  - MISMATCHED 2
  - LARGEST VARIANCE −42.50 (ANZ Everyday · 31 Aug)
  - LONGEST UNCHECKED 57 days (Joint)
- Columns: ACCOUNT / UNIT / CHECK DATE / ASSERTED / COMPUTED / VARIANCE / ACTIONS.
- Mismatched rows get `⚑` and an `explain` action (opens 15h). Passing rows get `✓` and stay listed as an audit trail.
- **Variance = asserted − computed**, calculated at the check's own date, in the check's Unit, and never summed across Units.

### 15f — Date range picker (popover, 700px)
- Opened with `r` or by clicking RANGE.
- Presets (210px list):
  - This month, Last month, Last 30 days, This quarter
  - Financial year to date, Last financial year (**AU financial year starts 1 July**)
  - Last 12 months, All time, Custom range
- A two-month calendar (7×30px grid, weeks start on Monday) and typed From/To fields (YYYY-MM-DD) edit the same range. The comparison range re-derives from it.
- Future days are shown but dimmed.
- Status bar: `INSERT` · `j/k preset · h/l day · tab from / to / compare · enter apply · esc cancel`.

### 15g — Category drill-down (modal)
- Title "Dining — 1–26 Sep 2026 · AUD". It follows the report's range and Unit, unlike 13d, which is fixed to the budget month.
- Stats: TOTAL −318.40 / TRANSACTIONS 12 / AVERAGE 26.53 / VS 1–26 AUG ▲ 109.50.
- BY PAYEE list with TXNS count, and the tail folded into "4 others".
- LAST 6 MONTHS bar chart (Apr–Sep) with a dashed budget line at 250.00. September is hatched because it is to date.
- Footer: `open 12 in Transactions →`, `Close`, `Budget detail` (opens 13d).
- Keys: `t` / `b` / `j/k`.

### 15h — Explain a variance (modal)
- Title "ANZ Everyday — balance check, 31 Aug 2026". Stats: ASSERTED 5,104.86 / COMPUTED 5,147.36 / VARIANCE −42.50.
- Narrowing: "The previous check on 31 Jul matched exactly, so the gap is within the 38 transactions dated 1–31 Aug", followed by a plain-language direction ("ledger is higher than the statement…").
- LIKELY CAUSES, most likely first ("3 OF 38"), in this order:
  1. An **exact-amount pending** transaction (◐ Uber Eats −42.50)
  2. **Near the cut-off** (Woolworths −61.80)
  3. **A possible duplicate** (Transfer to Joint −200.00, which also appears on Joint)
- Each cause has an `open` link.
- Nothing changes automatically. Fixing the cause re-computes the variance live.
- Footer: `open 1–31 Aug in Transactions →`, `Close`, `Add adjustment −42.50`, `Edit check`. The adjustment is a real transaction categorised **Balance adjustment**.

### 15i — Export (modal)
- Opened with `x` on any report. Format radio: **CSV — for spreadsheets** (default) or **PDF — for printing**.
- Include checkboxes:
  - ✓ Subcategory rows
  - ✓ Comparison column
  - Transaction list under each category
  - Share bars (PDF only)
- The file name encodes scope, Unit, range and report, so exports sort cleanly.
- Summary: Scope Unit · AUD; Range 2026-09-01 to 2026-09-26; Rows 13 · signed amounts, 2 dp, no currency symbol.
- Exports exactly what's on screen. The Unit is written into the file and never converted.
- Footer: `Cancel` + `Export CSV…` (the label follows the chosen format).

---

## Components
| Part | Spec |
| --- | --- |
| Report tabs | same as the Budgets/Bills tab switcher (active dark fill) |
| Filter control | 30px high, label 11px/800 uppercase `#9b9797` + value 800 + `▾` |
| Stat strip | `repeat(3 or 4, minmax(0,1fr))`, 2px rules, figure 22px/800 tabular |
| Share bar | 10px track `#eae9e9`, fill `#201e1d`, width = row / section max |
| Change indicator | `▲`/`▼` + amount; over-budget or worse direction in `#ae1800` |
| Check status | `⚑` mismatch (accent) / `✓` pass (`#605d5d`) |
| Cause row | status glyph ◐ pending / ○ unreconciled, date, payee, status, amount, one-line reason, `open` |
| Modal | `border:2px solid #201e1d`, `box-shadow:0 16px 48px rgba(32,30,29,.40)`, dimmer `rgba(32,30,29,.30)` |

## Design tokens
The palette is shared:
- Ground `#f3f2f2`, chrome `#eae9e9`
- Ink `#201e1d` / `#605d5d` / `#9b9797`
- Accent `#ec3013`, and `#ae1800` where accent is used for text
- Rule `#d7d3d3`

Archivo 400/800, tabular numerals, radius 0.

## Interactions
- `1–5` switch report · `r` range picker · `x` export · `[ ]` shift range · `j/k` row · `enter` drill down (15g) or explain (15h)
- Scope and range persist between Categories and Payees. Balances uses AS OF instead, and Budget vs Actual has no filters.
- Changing the range re-slices any open drill-down.

## State
```
reportsView
  tab: Categories | Payees | Balances | BudgetVsActual | BalanceChecks
  scope: Unit(CurrencyCode) | Account(Id)
  range: { from: Date, to: Date, preset: Option<Preset> }
  compare: Option<Range>          // derived from range
  asOf: Date                      // Balances only
  includeInactive: bool
  checksFilter: All | Mismatched, checksAccount: Option<Id>
balanceCheck
  id, accountId, date, asserted: Decimal
  computed: Decimal  // derived as of date
  variance = asserted - computed
exportRequest
  format: Csv | Pdf, includeSubcategories, includeComparison, includeTransactions, includeBars
  fileName  // derived from tab + scope + range
```

## Implementation notes (GPUI)
1. Use the HTML as a reference only; build from GPUI elements.
2. Never convert between Units, and never show a cross-Unit total.
3. Leave known costs out of Budget vs Actual.
4. Compute each variance as of the check's own date. Keep every check (they are an audit trail).
5. Explain never mutates data. An adjustment creates a normal transaction in the "Balance adjustment" category.
6. Exports must reproduce the on-screen state exactly.
7. Use tabular numerals, radius 0, 2px section rules and 1px row rules, with Lucide icons at 14px.

## Files
- `Reports.dc.html` — the prototype (Turn 10, options 15a–15i)
- `README.md` — this document
