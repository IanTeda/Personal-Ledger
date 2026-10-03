# Handoff: Dashboard

> **Package 02 of 20 · Dashboard** — frames 2a–2d (2a, 2b, 2c, 2d). Open `Dashboard.dc.html` in a browser from this folder. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

## Overview
The Dashboard (`g d`, first in LEDGER) is the landing destination. It has three views of the same moment, switched with `1` / `2` / `3` or the view tabs under the heading: **Today** (default), **Net worth** and **This month**. Before a ledger is open it shows an empty shell (2a).

**Nothing on the Dashboard is calculated only here.** Every figure is read from the destination that owns it (Cash, Budgets, Net worth areas, Bills, Notifications). Each block links to that destination.

## About the design files
`Dashboard.dc.html` is a **high-fidelity HTML design reference**, not production code. Recreate it in **Rust + GPUI** using the codebase's patterns. Exact px values are in the file; this README covers structure, data and behaviour.

## Fidelity
High fidelity. Colours, type, spacing, rules and states are final. Figures are sample data.

## Frame & shell
1280 × 800: header 48px (breadcrumb `ledger › dashboard`), primary rail 206px with Dashboard active, body `flex:1`, status bar 28px. Shell spec: `../01-shell/`.

### Page heading (2b–2d)
- `h2` "Dashboard" (28px/800), subline "monday 14 september 2026 · aud · 9 accounts" (11.5px #9b9797).
- **View tabs:** Today `1` · Net worth `2` · This month `3`. Active tab is 800 ink with a 3px ink underline; others #605d5d. The binding shows beside each label in 11px #9b9797.
- **Headline row:** four equal cells divided by 2px rules. Each is a caps label (800 10px .11em #9b9797), a large tabular figure (800), and an 11.5px #605d5d note.

---

## 2a · No ledger open
The same empty shell as 1a. The view tabs are hidden. The body reads "No ledger open" with "Run **:open** to load a ledger file, or **:new** to start one."
- Status bar: `NORMAL` · `: command · / search · b toggle sidebar · ? help`; right: "ledger · dashboard".
- After a ledger loads, the Dashboard opens on the last view used (Today the first time).

## 2b · Today (`1`, default)
Puts the next action first.
- **Headline:** FREE TO SPEND 55,204.61 ("until payday wed 23 sep · after bills and buffer", from Cash) · SPENT THIS MONTH 2,576.80 ("of 3,390.00 everyday budget · 76% with 47% of month gone", from Budgets) · NET WORTH 955,366.31 ("+4,120.55 since 1 sep") · NEXT PAYDAY wed 23 sep ("+4,210.00 · Dept of Education").
- **NEEDS YOU · 3** ("from Notifications · g m"): the unresolved notifications, one row each: a kind caps label (CARD / IMPORT / BUDGET), a bold title, an 11.5px #605d5d detail, and its own action button with a key hint:
  - CARD — Pay Amex 2,318.44 by fri 18 sep · **Pay card…** `p`
  - IMPORT — 4 ANZ transactions need a category · **Categorise** `c`
  - BUDGET — Groceries is at 92% with 16 days left · **Open budget** `b`
- **RECENT · LAST SYNC 08:40** ("8 of 23 this week"): DATE · PAYEE · CATEGORY · AMOUNT (right, tabular). Uncategorised rows show "uncategorised" in the category column.
- **NEXT 14 DAYS · INSTANT** ("rule = payday"): DATE · ITEM (with a kind tag: bill / card / income) · AMOUNT · BALANCE, starting from today's instant balance (77,836.65) and ending at payday. The note under it gives the lowest point (72,724.61 on 20 sep, 55,204.61 above the buffer) and links to Cash for the 60-day low.
- **THIS WEEK** strip: −345.95 spent since mon · 2 documents to file · +1.3% portfolio, 5 days.
- Status bar: `1/2/3 view · j/k row · p pay card · c categorise · b budget` …

## 2c · Net worth (`2`)
A balance sheet for the household.
- **Headline:** NET WORTH 955,366.31 (+4,120.55 since 1 sep · +43,266 over 12 months) · LIQUID NET WORTH 310,524.16 (cash and investments, less cards — no home, no contents) · DEBT TO ASSETS 35.2% (519,076.29 owed against 1,474,442.60) · HOME EQUITY 603,242.15 (1,120,000 valuation less the 516,757.85 loan).
- **12-month trend** chart, oct–sep, y-axis 920k–960k.
- **ASSETS** ("share of assets") and **LIABILITIES** ("share of debt"): one row per area, with name, an 11.5px source note, a share bar, the area's `g` key, and its value. Totals row at the bottom.
  - Assets: Home (manual valuation · 14 aug) 1,120,000.00 · Investments `g i` 215,005.95 · Cash `g c` 97,836.65 · Inventory `g o` 41,600.00 · total 1,474,442.60.
  - Liabilities: Home loan `g n` 516,757.85 · Credit cards `g k` 2,318.44 · total 519,076.29.
- **WHAT MOVED SINCE 1 SEP** (+4,120.55): Investments +2,884.10 · Home loan +760.09 · Cash +590.36 · Credit cards −114.00, each with a one-line reason.
- Status bar: `1/2/3 view · j/k row · enter open area · v update valuation · [ ] range`; right: "home is a manual valuation · last set 14 aug".

## 2d · This month (`3`)
The month as it happens.
- **Headline:** SPENT 2,576.80 (of 3,390.00 · 76%) · MONTH GONE 47% (day 14 of 30 · 16 days, 813.20 left) · IN · OUT +4,210.00 · −4,986.80 · BILLS 2,450.94 paid (9,928.25 still to come · 6 bills).
- **EVERYDAY BUDGET · BY CATEGORY** ("grey tick = where you'd be on pace"): CATEGORY · SPENT · BUDGET · USED with a bar per row and a grey pace tick at 47%. Sorted by % used, so over-budget rows (Home 138%) sit at the top; over-budget values use #ae1800. Footer: "9 categories · sorted by % used" · "2,576.80 of 3,390.00".
- **Suggestion note** under the table: "Home is over after the Bunnings run on 11 sep (312.80). Move 112.80 from Entertainment to cover it — Budgets › Edit." The link opens the budget edit.
- **CUMULATIVE SPEND** (september vs august): this month's line against last month (aug 3,512) and the budget line (3,390), x-axis 1 / 14 / 30 sep.
- **BILLS THIS MONTH** (3 paid · 6 to come): date · bill · status (paid, tomorrow, 4 days, 16 days) · amount, then "+ Telstra, Origin, home loan 29 sep" and "9,928.25 to come".
- Status bar: `1/2/3 view · j/k category · enter open in Budgets · [ ] month · m move money`; right: "everyday budget · envelope method".

## Interactions
- `1` / `2` / `3` switch views. Remember the last view per ledger.
- `j` / `k` move through rows of the focused block; `enter` opens the row in its owning destination.
- Action buttons on Needs you run the same command as in Notifications; resolving one removes it here and decrements the rail badge.
- `[` / `]` change the range (Net worth) or month (This month). `v` updates the home valuation. `m` opens the move-money dialog from Budgets.

## State
```
dashboard
  view: Today | NetWorth | ThisMonth      // persisted per ledger
  focusedBlock, focusedRow
  range: Months(12)                        // Net worth
  month: YearMonth                         // This month
```
All figures are derived queries over the owning destinations; don't store them.

## Files
- `Dashboard.dc.html` — design reference (section 2): 2a–2d.
- Shared `../styles.css`, `../_ds_bundle.js`, `../support.js`.
