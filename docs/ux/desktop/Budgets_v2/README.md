# Handoff: Budgets v2 — Multiple budgets, switcher, four methods

## Overview
This package extends **Personal Ledger's Budgets** so a user can keep **several budgets** and switch between them. Each budget has a **method**. It covers Turn 11, options **11a–11f**, and builds on the existing Budgets handoff (`design_handoff_budgets`, 9a–9g; its written spec is kept alongside as [`limits-9a-9g.md`](limits-9a-9g.md)), which is unchanged except where noted below.

| Method | Idea | Screen |
| --- | --- | --- |
| **Envelope** | Zero-based, YNAB-style. Every dollar received is assigned to a category. | 11a |
| **Category limits** | The existing monthly cap per category (9a–9g). | 9a–9g |
| **Percentage split** | Buckets (50/30/20) measured against income received. | 11d |
| **Project** | One total over a date range, split into lines. | 11e |

Screens: 11a Envelope · 11b Switcher · 11c New budget · 11d Split · 11e Project · 11f Manage budgets.

## About the design files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype**. It is a design reference, not production code. Recreate the screens in the target codebase (Rust + GPUI) using its own patterns. Open it and scroll to **Turn 11**, which is at the top. Turns 9 and 8 are included as context.

## Fidelity
**High fidelity.** Colours, type, spacing and states are final. Every frame is **1280 × 800** with the same shell as the other packages. Budgets is the active nav item, and the title bar reads `Personal Ledger | Budgets`.

---

## Core model
**Budgets are views over one ledger.** A budget never owns or copies transactions. A transaction counts in every budget whose accounts and categories cover it, so any number of budgets can overlap.

```
budget
  id, name, unit: CurrencyCode (locked after creation)
  method: Envelope | Limits | Split | Project
  accountIds: Set<AccountId>        // "on budget" accounts
  isDefault: bool, archivedAt: Option<Date>
  config: EnvelopeCfg | LimitsCfg | SplitCfg | ProjectCfg
```
- Exactly **one default** budget. It is used on first launch, and by the Dashboard and Reports Budget vs Actual (10d).
- The **last opened** budget is remembered per session and is restored on return.
- **Archive, don't delete**: there is no confirmation, because it only affects the budget and never the ledger. History is kept, and a budget can be restored.
- Unit is fixed on creation, like Bill plans. Accounts in another Unit can't be added.
- The existing Category limits data migrates to a budget named **Personal spending**. Categories → *Monthly budget* (5c) keeps reading and writing **that** budget only.

## Shared header (all Budgets screens)
- **Title is the switcher**: budget name (28px/800) + chevron + a **method tag** (`ENVELOPE`, `LIMITS`, `SPLIT`, `PROJECT`; 10px/800, +.1em, solid ink). Click, or press `b`, to open 11b.
- Below it: a one-line meta summary specific to the method.
- Right side: period nav `‹ September 2026 ›` (not for Project) and the primary actions.
- Tabs sit under the header, with a 2px rule below. Active tab is solid ink.

---

## Screens

### 11a — Envelope (default for the Household budget)
**Ready to Assign banner** (solid ink strip, full width)
- Figure 26px/800: **297.50** = income received this month into on-budget accounts − total assigned (+ any unassigned carried in).
- Two-line breakdown: "4,480.00 received in September / − 4,182.50 assigned to categories".
- Buttons: `Assign underfunded · 223.75` (assigns the shortfall to every underfunded target) and `Assign…`.
- Meta line: "5 accounts on budget · **2** overspent · 223.75 underfunded".

**Tabs:** Budget (active) / Targets / History. A legend shows funded (solid) / underfunded (outlined) / overspent (accent).

**Category table** (grouped, collapsible; group rows are on `#f3f2f2`, 800 weight, and sum their children):
`CATEGORY / TARGET (200px, one line, ellipsis) / ASSIGNED (96) / ACTIVITY (96) / AVAILABLE (110)`.
- **Available = carried in + assigned + activity.** It rolls forward every month, so unspent money stays in its envelope.
- **Assigned** is an inline-editable figure with a dotted underline (`enter` to edit).
- **Activity** is signed spend for the month. Inflow shows `+`.
- **Available pill:**
  - funded → solid ink, light text
  - underfunded (a target exists and Available is below what's still needed) → 1.5px outlined pill
  - zero → plain grey `0.00`
  - **negative → accent fill `#ec3013`, light text**
- **Target** text: "10,000 by Dec 2027", "2,200 / month", or "no target". A target with a shortfall adds a bold **needs 125.00**.
- Groups in the sample: Fixed costs, Everyday, Savings goals, Credit card payments.
- A selected row is inverted (ink fill), and the inspector opens for it.

**Inspector** (252px column, right of the table, 2px border) for the selected envelope:
- Name + status tag (`OVERSPENT` in accent).
- Ledger: Carried in / Assigned / Activity / **Available**. Available has a 1px rule above and a **double underline** below (the ledger's total mark).
- **COVER x FROM** list: Ready to assign, then the envelopes with the most available. Each is a `cover →` row that moves money in one click.
- Note: uncovered *cash* overspending is deducted from next month's Ready to Assign; *card* overspending becomes card debt.
- QUICK ASSIGN: target amount, assigned last month, 3-month average spent. Clicking a row fills Assigned.

**Rules**
1. Income into an on-budget account (a transaction with an Income category) increases Ready to Assign. Transfers between two on-budget accounts don't.
2. **Credit cards:** spending on a card *moves* money from the category into the card's **payment envelope** (under "Credit card payments"). Paying the card spends from that envelope. A card with a balance owed shows its payment envelope as underfunded.
3. Money moved between envelopes is a **budget move**: it isn't a transaction and never appears in reports.
4. Ready to Assign **can't go negative silently**: assigning more than is available is blocked with an inline message.
5. Targets: `monthly amount`, `by date` (amount needed per month = remaining ÷ months left), `weekly`. Underfunded is computed from the target.
6. Known costs from Bills reduce nothing in Envelope, but an unpaid Due bill in an envelope with too little Available counts as **underfunded** and is flagged.

### 11b — Switcher (popover, 380px)
Opened with `b` or a click on the title; anchored under the header, with the page dimmed.
- Search field ("find a budget").
- One row per active budget: check (current), name (800) + `default`, a one-line health summary, and a method tag on the right. Current row is inverted.
  - Household — 297.50 ready to assign · 2 overspent
  - Personal spending — 1 over · 318.89 left to spend
  - 50/30/20 — needs 63% · wants 9% · savings 14%
  - Kitchen reno — 9,210 of 12,000 · ends 31 Dec
- `ARCHIVED` section, dimmed.
- Footer, split 50/50: `+ New budget` (`n`, opens 11c) and `Manage budgets…` (opens 11f).
- Keys: `j/k` move · `enter` open · `n` new · `esc` close.

### 11c — New budget (modal, 640px)
- **Name** and **Unit** (2 columns).
- **Method**: a 2×2 grid of radio cards, each with a title, a short tag (`ZERO-BASED`, `CAPS`, `50/30/20`, `ONE-OFF`) and a two-line description. Selected card has a 2px ink border and `#eae9e9` fill.
- **Accounts on budget**: chips (selected = solid ink with ✓). Help text: income into these accounts becomes Ready to Assign, and off-budget accounts still count toward net worth. Not shown for Project (all accounts).
- **Start from**: segmented `Empty` / `Copy categories from <budget>` / `Last 3 months' spending`. Help text: seeds targets and amounts from average spend, and you still assign the month's income yourself.
- Footer: `Cancel` + `Create budget`. Status bar mode: `INSERT`.
- Config that appears per method after the choice: **Split** asks for bucket names + percentages (default 50/30/20); **Project** asks for start date, end date and total.

### 11d — Percentage split
- Meta: "measured against income received on budget accounts".
- Tabs: This month / Buckets / History. Action `Edit split`.
- **Stat strip** (4 cells): INCOME RECEIVED 4,480.00 · SPENT + SAVED 3,815.60 · UNSPENT 664.40 (14.8%, counts as savings at month end) · KNOWN COSTS 614.51.
- **Stacked bar** (22px): one segment per bucket at its actual share of income, with the remainder in grey. **Accent ticks** mark the cumulative aim boundaries (50%, 80%) with labels underneath.
- **Three bucket columns** (NEEDS 50 / WANTS 30 / SAVINGS 20), each with a swatch, aim (% and amount), actual amount and %, then a variance line, then its categories.
  - Variance: "x over aim" is accent for Needs/Wants; Savings reads "x short of aim" in neutral (under-saving isn't an error state). "+ y known costs due" is added where bills are still coming.
  - Bucket swatches: solid ink / mid grey / hatched, so they don't rely on colour.
- Every budgeted category belongs to **exactly one** bucket. Drag a row to another column, or `shift+h/l`.
- **Savings** = transfers into goal/savings accounts + categories flagged as savings. Unspent income counts toward savings at month end.
- Percentages must total 100 (validated in Edit split).

### 11e — Project
- Meta: "1 Mar – 31 Dec 2026 · day 214 of 306 · all accounts". Actions `Add line`, `Edit project`. No period nav, and no monthly reset.
- Tabs: Lines (active) / Transactions / Timeline. Legend: spent · committed (hatched) · time elapsed (tick).
- **Stat strip:** TOTAL 12,000.00 · SPENT 9,210.00 (76.8% of total, 70% of time) · COMMITTED 1,200.00 (scheduled Bill rows, link to Schedule) · LEFT 1,590.00 (≈ 173 / month to end date).
- **Lines table:** `LINE · MATCHES / BUDGET / SPENT / COMMITTED / LEFT / PROGRESS (200px)`. The second line under each name says what it matches (a tag, category, or payee list, e.g. "tag kitchen-reno · Appliances"). Total row has double underlines.
- **Progress bar:** solid spent + hatched committed, with a tick at the elapsed share of the time range.
- **Unmatched strip** (dashed border): tagged transactions matching no line are listed with `assign to line` and `put in contingency`. They are never silently dropped.
- Line matching: any of tag / category / payee, plus an optional account filter. A transaction is claimed by the **first** matching line, in order.
- Left = budget − spent − committed. Project end date passing marks it `ENDED` and moves it to Archived after 30 days (no destructive action).

### 11f — Manage budgets
- Header: "4 active · 1 archived · opens to **Household**", and `+ New budget`.
- Table: `BUDGET / METHOD / PERIOD / ACCOUNTS / SCOPE / (default marker) / ACTIONS`.
  - Actions: `open`, `duplicate`, `set default` (hidden on the default), `archive`. An archived row is dimmed and offers `restore` + `duplicate`.
- Three notes underneath: **Budgets are views**, **Switching** and **Archive, not delete**.
- Keys: `j/k` row · `enter` open · `n` new · `d` duplicate · `*` set default · `x` archive.

---

## Components
| Part | Spec |
| --- | --- |
| Method tag | 10px/800 Archivo, `letter-spacing:.1em`, padding 4×7, solid ink fill (inverse / outlined when placed on ink or in a list) |
| Title switcher | h2 28px/800 + 14px chevron, cursor pointer, tooltip "switch budget (b)" |
| Ready to Assign | full-width ink strip, padding 7×16, figure 26px/800 tabular, light text |
| Available pill | min-width 74px, padding 1×8, line-height 16px, 800 tabular. Funded ink fill · underfunded 1.5px outline · negative `#ec3013` fill |
| Envelope row | padding 3×12, line-height 16px, 1px `#d7d3d3` rule, 12.5px text. All 18 rows plus groups must fit 1280×800 |
| Inspector | 252px, 2px border, 14px padding, 12px text |
| Ledger total | 1px ink rule above, 3px double rule below (the app's total mark) |
| Bucket bar | 22px high, 1px border, accent boundary ticks |
| Popover / Modal | 380px popover; 640px modal. `border:2px solid #201e1d`, offset shadow `6px 6px 0 rgba(32,30,29,.18)`, dimmer `rgba(32,30,29,.18–.35)` |

## Design tokens
Shared with the rest of the shell:
- Ground `#f3f2f2`, chrome `#eae9e9`
- Ink `#201e1d`, secondary `#605d5d`, tertiary `#9b9797`
- Accent `#ec3013`, and `#ae1800` where accent is used for text
- Rules `#d7d3d3`, border `rgba(32,30,29,.30)`, section rule `rgba(32,30,29,.38)`

Archivo, weights 400 and 800 only. Tabular numerals on every figure. Radius 0. Status meaning is never colour alone: hatching, outline or text always accompanies it.

## Interactions
- **Global (Budgets):** `b` switcher · `n` new budget · `[ ]` previous/next period · `tab` next tab.
- **Envelope:** `j/k` row · `enter` edit Assigned · `m` move money · `c` cover · `u` assign underfunded.
- **Split:** `h/l` bucket · `shift+h/l` move category · `e` edit split.
- **Project:** `a` add line · `e` edit · `enter` line transactions.
- Switching a budget keeps the active tab if the new method has it, otherwise it opens the method's first tab.

## State
```
budgets: Vec<Budget>; currentId; defaultId
envelope
  assigned[(categoryId, YearMonth)]: Decimal       // user input
  available(cat, month) = carriedIn + assigned + activity   // derived, rolls forward
  readyToAssign(month) = incomeReceived(onBudget) - sum(assigned) + carriedReady
  cardPaymentEnvelope[cardAccountId]                // derived from card spending
  target[categoryId]: Monthly(amt) | ByDate(amt, date) | Weekly(amt)
  move[(from, to, amount, month)]                   // budget moves, not transactions
split
  buckets: Vec<{name, pct, categoryIds}>            // pct sums to 100
project
  start, end, total, lines: Vec<{name, budget, match: {tags, categoryIds, payeeIds, accountIds}}>
```

## Implementation notes (GPUI)
1. Use the HTML as a reference only. Build from GPUI elements.
2. All figures are **derived on read** from transactions + budget config. Store only user inputs (assigned, targets, moves, config), never computed balances.
3. Keep Known costs as the single shared Bill Schedule query so nothing double-counts once a bill is Paid.
4. Budget moves and assignments are **not transactions**, so Reports never include them. A balance adjustment is still a real transaction.
5. Category limits keeps the effective-dated monthly amounts from the original handoff.
6. Persist `currentId` and `defaultId`. Dashboard and Reports 10d read `defaultId`.
7. Keep the Envelope table virtualised if the category count grows, since 18 rows only just fit the frame.
8. Use tabular numerals, radius 0, 2px section rules and 1px row rules. Icons are Lucide at 14px with a 1.5px stroke.

## Open items
- Dashboard's "budget" tile and Reports 10d need a small "showing: <default budget>" label. They are not redrawn here.
- Envelope History, Targets and Split's Buckets/History tabs are named but not drawn.
- Multi-Unit budgets are out of scope (one Unit per budget).

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (Turn 11, options 11a–11f)
- `README.md` — this document

## Settled for the Category limits build (#400)

- **11c:** all four method cards are shown, with Envelope, Split and Project disabled ("coming later") and not focusable, so Category limits is the only selectable method. **Start from** offers `Empty`, `Copy categories from <budget>` (the source's effective current-month amounts plus Rollover) and `Last 3 months' spending` (#386's Fill average on the chosen Accounts, with 0 averages left Unbudgeted, unlike Fill's 0.00). Each writes Onward Budget Amounts from the current month, with Rollover None except for Copy. **Accounts** offers open Accounts in the chosen Unit, all preselected, and changing the Unit re-selects. An **edit** mode of 11c, with Unit and Method locked, is the one place to rename a Budget or change its Accounts.
- **11b health (Limits):** "N over · X left to spend" for the current month. N is the count of over-budget leaves from `period_figures`, and X is the effective budget minus Spent minus Known Costs over budgeted leaves, which can be negative. "· M at risk" is appended when Known Costs alone push a leaf over. Archived rows show "archived <date>".
- **11f:** actions are `open`, `edit`, `duplicate`, `set default` and `archive`/`restore`. `duplicate` names the copy "<name> copy" (numbered when that name is taken), copies Unit, Method, Accounts, full Budget Amount chains and Rollover, is never the default, and opens edit mode. The default can't be archived, so an active Budget always exists and there is no empty state. `set default` applies only to active Budgets.
- **Keys:** bare `b` stays the rail toggle and `tab` stays focus cycling. On Budgets, `B` or a click on the title opens 11b, `n` opens 11c, `[ ]` change the period and `1`/`2`/`3` pick the tab. In 11f, `e` edits, `d` duplicates, `*` sets the default and `x` archives or restores. Palette commands: `budgets switch`, `budgets new`, `budgets manage`, `budgets edit`, `budgets duplicate`, `budgets set-default`, `budgets archive`, `budgets restore`.
- **Switching** keeps the active tab if the new method has it, and otherwise opens its first tab (trivial with one method, recorded for later maps). The last opened Budget is a Client-scoped Preference.
