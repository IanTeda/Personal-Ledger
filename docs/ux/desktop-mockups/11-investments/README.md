# Handoff: Investments

> **Package 11 of 20 · Investments** — frames 11a–11b (11a, 11b). Open `Investments.dc.html` in a browser from this folder. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

## Overview
**Investments** (`g i`, under NET WORTH) shows a portfolio's market value against the money put in, allocation against target, and holdings with dated prices. Trades are recorded against **purchase lots**, so cost base and the Australian 50% CGT discount (held 12+ months) are tracked per lot.

There is one portfolio per view; the title is the portfolio switcher.

Frames are section 10 of `Investments.dc.html`:
- **11a:** Portfolio view (Share portfolio).
- **11b:** Record trade — Sell.

## About the design files
`Investments.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder.

The shell (header, primary rail, status bar) is specified in `../01-shell + 16-settings/`. Only what Investments depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample figures are illustrative (AUD, prices at 29 Sep 2026 close). The sums are internally consistent.

## Frame & shell
- **Frame:** 1280 × 800.
- **Header:** 48px. Use the standard header: breadcrumb `net worth › investments`, sync status, window controls.
- **Primary rail:** 206px. Investments is active (NET WORTH: Cash `g c` · Inventory `g o` · Loans `g n` · Credit cards `g k` · **Investments `g i`**).
- **Status bar:** the mockups omit it. Add the standard 28px status bar (see Keyboard).
- **Content pane:** `flex:1; min-width:0; padding:18px 28px; flex column; overflow:hidden`.

---

## 11a · Portfolio view

### 1. Heading
`flex; align-items:flex-end; justify-content:space-between; margin-bottom:14px`.
- **Left: portfolio switcher.**
  - `h2` "Share portfolio" (28px/800), then a chevron-down, a 4px gap and the **type chip**. Gap 8, `cursor:pointer`.
  - Clicking or pressing `i` opens the switcher. Build it like the Loans switcher (Loans handoff, 9c): value, gain and chip per portfolio, a CLOSED group, **+ New portfolio**, and a footer total.
  - **Type chip:** `#201e1d` / `#f3f2f2`, `800 10px/1 Archivo .1em; padding:4px`. Values: **BROKERAGE** · SUPER · CRYPTO · MANAGED.
  - **Subline** (11.5px #9b9797): "CMC Markets · 5 holdings + cash · prices at 29 Sep close, entered 30 Sep 09:12 · **update prices**".
    - **update prices** is a link (`#ec3013`) that opens price entry / refresh from the unit's price source (Settings › Units).
    - When any price is older than 3 trading days, the date clause turns `#ae1800`: "prices 6 days old".
- **Right:** **Record income** (secondary) and **+ Record trade** (primary). Both `.btn`, 36px, gap 8.

### 2. Figures strip
`flex; border:1px solid rgba(32,30,29,.30); margin-bottom:14px`. Four cells, `flex:1; padding:10px 14px; border-right:1px solid rgba(32,30,29,.30)`.
- **Cell content:**
  - Caps label: `800 10px/1 .11em #9b9797; mb 6`.
  - Value: 19px/800, tabular.
  - Note: 11px #9b9797.

| Label | Value | Note | Logic |
| --- | --- | --- | --- |
| MARKET VALUE | 215,005.95 | holdings + cash | Σ units × latest price, plus cash. |
| COST BASE | 176,957.55 | what you put in, incl. brokerage | Σ remaining lot cost bases, plus cash. |
| UNREALISED GAIN | +38,048.40 | +21.5% · not taxed until sold | Value − cost base. Ink when positive; `#ae1800` with "−" when negative. |
| INCOME THIS FY | 1,284.60 | dividends & distributions · 38% franked | Sum of income records in the AU financial year (from Settings › General FY start). |

### 3. Chart + Allocation row
`flex; gap:14px; margin-bottom:14px`.

#### Chart panel
632px, `border:2px solid #201e1d; padding:12px 14px 6px`.
- **Title row** (12px): "Value vs money put in" (800).
- **Legend** (right, #605d5d):
  - a 14px `2px #201e1d` line: Market value
  - a 10×10 `#d7d3d3` swatch: Contributed
  - a 10×10 `#ec3013` swatch: Gain today
- **Series:**
  - **Contributed:** a stepped grey (`#d7d3d3`) area of cumulative net contributions. It steps up on buys and down on sell proceeds withdrawn.
  - **Market value:** a `#201e1d` 2px line over the same period.
  - **Gain today:** a red vertical rule at the right edge, from the contributed top to the value line, with its value labelled.
- **Range:** since inception by default. `[` / `]` change the range (1y · 3y · 5y · all).

#### Allocation vs target
`flex:1; border:1px solid rgba(32,30,29,.30); flex column`.
- **Header:** "Allocation vs target" (800) on the left; "drift ±2% allowed" (11px #9b9797) on the right.
- **Rows:** `padding:12px 14px; gap:13px`. Each row has:
  - a name/value line (12.5px): class name on the left; **actual %** (800) with "/ target %" (#9b9797) on the right;
  - an 8px `#d7d3d3` track, with the fill `#201e1d` = actual (scaled so the largest target is near full width);
  - a 2×14 target tick at the target position. The tick is `#201e1d` within drift and `#ec3013` when |actual − target| > drift.

  | Class | Actual | Target |
  | --- | --- | --- |
  | Aus shares | 41.9% | 40% |
  | Intl shares | 44.7% | 45% |
  | Bonds | 11.8% | 12% |
  | Cash | 1.6% | 3% (drift flag) |
- **Footer:** `padding:8px 14px; 1px top rule; 11.5px #605d5d`. A one-line note on the largest drift and what will change it: "Cash is 1.4% under target. Next distribution (VAS, Oct) lands in cash."
- Each holding maps to one asset class. Targets are set in portfolio settings.

### 4. Holdings table
- **Above the table:** a caps label HOLDINGS on the left; "Select a holding for its lots, trades and income" (11px #9b9797) on the right.
- **Container:** `border:1px solid rgba(32,30,29,.30)`.
- **Header:** `padding:6px 14px; #eae9e9; 800 10px .1em #605d5d`.
- **Columns:**

  | Column | Flex | Align |
  | --- | --- | --- |
  | CODE | .6 | left, 800 |
  | NAME | 1.9 | left |
  | UNITS | .8 | right |
  | PRICE | .9 | right |
  | VALUE | 1.1 | right |
  | COST BASE | 1.1 | right |
  | GAIN | 1.2 | right |
  | WEIGHT | .7 | right |
- **Rows:** `padding:5px 14px; 12.5px`, tabular, 1px bottom rule.
- **Gain cell:** the amount, then the percentage (11px #9b9797). Gains are ink; losses are `#ae1800` with "−".
- **Selected row:** `background:#eae9e9` (VGS in the mock).
- **Cash row:** code "—", name "Cash · CMC Cash Account", value = cost, gain "—".
- **Total row:** `padding:7px 14px; 800`, top rule. The label spans CODE through PRICE.

| Code | Name | Units | Price | Value | Cost base | Gain | Weight |
| --- | --- | --- | --- | --- | --- | --- | --- |
| VAS | Vanguard Australian Shares | 820 | 103.42 | 84,804.40 | 71,340.00 | +13,464.40 18.9% | 39.4% |
| VGS | Vanguard Intl Shares | 610 | 142.18 | 86,729.80 | 63,125.00 | +23,604.80 37.4% | 40.3% |
| VAF | Vanguard Aus Fixed Interest | 540 | 46.95 | 25,353.00 | 26,190.00 | −837.00 −3.2% | 11.8% |
| NDQ | Betashares Nasdaq 100 | 180 | 52.60 | 9,468.00 | 6,840.00 | +2,628.00 38.4% | 4.4% |
| CSL | CSL Limited | 22 | 238.10 | 5,238.20 | 6,050.00 | −811.80 −13.4% | 2.4% |
| — | Cash · CMC Cash Account | | | 3,412.55 | 3,412.55 | — | 1.6% |
| **Total** | | | | **215,005.95** | **176,957.55** | **+38,048.40** | **100%** |

- **Opening a holding** (`enter`) shows a holding detail: lots (bought, units left, cost/unit, held, gain, discount eligibility), trades, income, and a price history. Build it from the 11b lot table pattern; it is not drawn.

---

## 11b · Record trade — Sell
- **Overlay:** a dimmer over the content area (`inset:48px 0 0 206px; rgba(32,30,29,.18)`).
- **Dialog:** 540px, at `left:400px; top:96px`, `#f3f2f2; border:2px solid #201e1d`. **Align with the shared dialog shell:** full-window `rgba(32,30,29,.30)` dimmer, top-anchored, shadow `0 16px 48px rgba(32,30,29,.40)`.
- **Header:** `padding:14px 18px 10px; border-bottom:2px solid rgba(32,30,29,.38); flex space-between`.
  - **Left:** "Record trade · VGS" (800 17px), then "610 units held · cost base 63,125.00" (11.5px #605d5d).
  - **Right:** a **Buy | Sell** segmented control (`border:1px solid rgba(32,30,29,.30)`, segments `padding:6px 12px; 12px`). The active segment is `#201e1d` / `#f3f2f2` 800.
- **Body:** `padding:14px 18px; flex column; gap:12px`. Field labels are 11.5px #605d5d, `mb 5`. Inputs are 32px. The focused input is `border:2px solid #201e1d`; the others are 1px `rgba(32,30,29,.30)`.
  - **Row 1:** Units (90px, `200`, focused) · Price (100px, `142.18`, prefilled from the latest price) · Brokerage (90px, `9.50`, the portfolio's default) · Date (`flex:1`, `30 Sep 2026`).
  - **Row 2:**
    - **Proceeds to:** a select, `flex:1`, "CMC Cash Account ▾".
    - **Net proceeds** (150px, right): label, then **28,426.50** (19px/800, `line-height:32px`).
    - Net proceeds = units × price − brokerage.
  - **2px rule.**
  - **Lots sold header:** "Lots sold" (800) on the left. On the right, "Match" (11px #605d5d) and a segmented control: **Oldest first** (active) · Least gain · Pick lots.
    - **Pick lots** turns each lot row's UNITS cell into an input.
  - **Lots table:** `border:1px solid rgba(32,30,29,.30)`.
    - **Header:** `padding:5px 10px; #eae9e9; 800 9.5px`.
    - **Columns:** BOUGHT 1.2 · UNITS .8 (right; "150 of 150") · COST/UNIT .9 · HELD .9 (#605d5d; "5y 6m") · GAIN 1.1 · DISCOUNT .9.
    - **Discount cell:** "**50% off**" when held ≥ 12 months. Otherwise "none" in `#ae1800`, and the row is flagged.

    | Bought | Units | Cost/unit | Held | Gain | Discount |
    | --- | --- | --- | --- | --- | --- |
    | 12 Mar 2021 | 150 of 150 | 88.20 | 5y 6m | +8,097.00 | **50% off** |
    | 8 Feb 2022 | 50 of 120 | 96.40 | 4y 7m | +2,289.00 | **50% off** |
  - **Summary strip:** `flex; border:1px solid rgba(32,30,29,.30)`. Three cells with `padding:8px 12px`, a caps label (9.5px) and a 16px/800 value:
    - CAPITAL GAIN **10,376.50**: Σ lot gains − brokerage.
    - AFTER 50% DISCOUNT **5,188.25**: discount-eligible gains halved, plus ineligible gains in full.
    - UNITS LEFT **410**.
  - **Note** (11.5px/1.45 #605d5d): "Estimate for your records, not tax advice. Lots held under 12 months get no discount and are flagged."
- **Footer:** `padding:12px 18px; border-top:2px solid rgba(32,30,29,.38); gap:8`. **Record sale · 28,426.50** (primary, `flex:1`, 36px) and **Cancel** (secondary).

**Buy mode:**
- Fields: Units · Price · Brokerage · Date · **Paid from** (cash account).
- Shows **Total cost** (units × price + brokerage).
- There is no lots table. It creates a new lot.
- Confirm: **Record purchase · <total>**.

**On sale confirm:**
- Reduce the matched lots (the remaining cost base stays per lot).
- Create a realised-gain record with its discount eligibility.
- Credit the proceeds account.
- Update holdings, allocation and the figures.

**Validation:** units must not exceed the units held; price > 0; Pick lots must total the units sold.

---

## Interactions & behaviour
- **Prices** are per Unit (Settings › Units, with price sources). Each price carries an `asOf` date. Market value uses the latest price, and the subline states when it was taken.
- **Record income:** a dialog (not drawn) using the shared shell.
  - Fields: Holding · Type (Dividend / Distribution / Interest) · Amount · Franking % / credits · Date · Paid to (cash account / DRP).
  - **DRP** creates a new lot at the reinvestment price.
- **Allocation drift** past the threshold can raise a notification. Add a rule under Notifications › Rules › Investments if wanted (not in the defaults yet).
- **Net worth / Dashboard** read the market value here (Dashboard 2c "Investments 215,005.95", and "What moved": market change since 1st).
- **Switcher:** multiple portfolios (brokerage, super, crypto). The footer total is the sum of their market values.
- **Empty state:** "No holdings yet" with **+ Record trade**.

## Keyboard
| Key | Action |
| --- | --- |
| `g i` | Open Investments (last portfolio). |
| `i` | Portfolio switcher. |
| `t` | Record trade (Sell when a holding is selected). |
| `b` / `s` | Record trade, Buy / Sell. |
| `r` | Record income. |
| `u` | Update prices. |
| `j` / `k` | Move holdings. |
| `enter` | Open holding detail. |
| `[` / `]` | Chart range. |
| `esc` | Close a dialog or switcher. |

**Status bar (to add):**
- **Left:** `NORMAL` · `i switch · t trade · r income · u update prices · enter holding · [ ] range`
- **Right:** "prices at 29 Sep close"

## State
```
investments
  portfolioId: PortfolioId
  selectedHoldingId: Option<UnitCode>
  chartRange: Y1 | Y3 | Y5 | All
  dialog: None | Trade(TradeDraft) | Income(IncomeDraft) | Prices | Settings

Portfolio  { id, name, broker, kind: Brokerage|Super|Crypto|Managed, cashAccountId,
             defaultBrokerage, targets: Map<AssetClass, Percent>, driftPercent }
Holding    { portfolioId, unit: UnitCode, name, assetClass }
Lot        { id, holding, boughtOn, units, unitsRemaining, costPerUnit, brokerage }
Trade      { id, holding, side: Buy|Sell, date, units, price, brokerage, cashAccountId,
             lotsMatched: Vec<(LotId, units)> }
RealisedGain { tradeId, lotId, gain, heldDays, discountEligible }
Income     { holding, type, amount, frankingPct?, date, toAccountId?, drp: bool }
Price      { unit, price, asOf, source }
TradeDraft { side, units, price, brokerage, date, cashAccountId, match: Oldest|LeastGain|Pick(Vec) }
```
- Value, gain, weight, allocation and the chart series are all derived.
- `discountEligible` = held ≥ 365 days. Store the full lot history; never net lots together.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · table head / selected `#eae9e9` · contributed area / track `#d7d3d3`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797`.
  - Accent `#ec3013` (gain rule, drift tick, links) · text-safe accent `#ae1800` (losses, stale prices, no-discount).
  - Rules: strong `rgba(32,30,29,.38)` · border `rgba(32,30,29,.30)` · row `rgba(32,30,29,.15)`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 9.5 / 10 / 11 / 11.5 / 12 / 12.5 / 16 / 17 / 19 / 28px.
  - Caps labels `.11em` (chips and table head `.1em`). Tabular numerals on every figure.
- **Spacing:** 3, 4, 5, 6, 8, 10, 12, 13, 14, 18, 28px.
- **Radius:** 0.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Focused inputs use `border:2px solid #201e1d`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary` from Modernist (`styles.css`).

## Assets
- **Icons:** the rail uses a 14px Lucide-style bar chart (1.5 stroke). The mock also uses a chevron-down (switcher).
- **Chart:** inline SVG. Rebuild it with GPUI paths.
- No images.

## Files
- `Investments.dc.html` — design reference: 11a Portfolio view, 11b Record trade (Sell).
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
