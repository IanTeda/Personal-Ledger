# Handoff: Cash

## Overview
**Cash** is the first destination under NET WORTH (`g c`). It answers four questions about money the household can reach:

1. **What's free to spend** before the next payday?
2. **Where is the low point** ahead?
3. **How long is the runway** at the current spend?
4. **How much is the offset saving** this month?

There is one cash position, so there is one view and no switcher. Two dialogs work from it: **Transfer…** (6b) and **Set buffer** (6c).

Frames are section 6 of `Ledger Desktop Shell.dc.html`:

| Frame | Shows |
| --- | --- |
| 6a | Cash view |
| 6b | Transfer dialog over the view |
| 6c | Set buffer dialog over the view |

## About the design files
`Cash.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Cash depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample data is illustrative (as of 14 Sep 2026, AUD).

## Frame & shell
- **Frame:** 1280 × 800. Header 48px, body `flex:1`, status bar 28px.
- **Header breadcrumb:** `net worth › cash`.
- **Primary rail:** 206px. Cash is active (first in NET WORTH: Cash `g c` · Inventory `g o` · Loans `g n` · Credit cards `g k` · Investments `g i`).
- **Content pane:** `flex:1; min-width:0; padding:22px 28px; overflow:hidden; flex column`. There is no second rail.

---

## 6a · Cash view
Top to bottom:

### 1. Page heading
`flex; align-items:flex-end; justify-content:space-between; gap:16px; margin-bottom:16px`.
- **Left:** `h2` "Cash" (28px/800, `margin:0 0 4px`), with the subline "4 accounts · aud · as of 14 sep 2026" (11.5px #9b9797).
- **Right:** **Record balance** (secondary) · **Set buffer** (secondary) · **Transfer…** (primary). All are `.btn`, 32px, gap 8.

Then a 2px rule, `rgba(32,30,29,.38)`, `flex:none`.

### 2. Headline figures
- **Grid:** `grid-template-columns:repeat(4,minmax(0,1fr))`, `border-bottom:2px solid rgba(32,30,29,.38); margin-bottom:16px`.
- **Cell:** `padding:10px 16px` (the first cell has 0 left padding), `flex column; gap:8px`. Cells 2–4 carry `border-left:2px solid rgba(32,30,29,.38)`.
- **Cell content:**
  - Caps label: `800 10px/1 Archivo; .11em; #9b9797`.
  - Value: 24px/800, `letter-spacing:-.01em`, tabular, `line-height:1`.
  - Note: 11.5px #605d5d, `text-wrap:pretty`.

| Label | Value | Note | Formula |
| --- | --- | --- | --- |
| CASH ON HAND | 97,836.65 | 77,836.65 instant · 20,000.00 on notice | Sum of all cash accounts. |
| FREE TO SPEND | 55,204.61 | instant, less 5,112.04 due before payday 23 sep and the 17,520.00 buffer | **instant − scheduled outflows dated before the next income event − buffer** (only when the buffer is held in instant). Floor at 0; when negative, show the shortfall in `#ae1800` as "short by …". |
| RUNWAY | 16.8 mo | at 5,840.00 average monthly spend, last 12 months | Cash on hand ÷ average monthly spend (same window as the buffer setting). One decimal place. |
| OFFSET WORKING | 310.61 | home-loan interest saved this month by 61,204.10 in offset | Offset balance × loan rate ÷ 12 (use the loan's daily-interest method if available). Hide the cell when no offset exists, and let the grid reflow to 3 columns. |

### 3. Balance chart (146px block, `margin-bottom:14px`)
- **Size:** the SVG is 130px tall, full width, drawn in a 1000×180 viewBox with `preserveAspectRatio:none`. All strokes are non-scaling.
- **Gridlines:** `#d7d3d3` 1px at 50k / 75k-equivalent steps. The y labels are 10px #9b9797, right-aligned (`75k`, `50k`).
- **Baseline:** 2px `rgba(32,30,29,.38)` at the bottom.
- **History:** 12 months of month-end **cash on hand**, solid `#201e1d` 2px.
- **Projection:** 60 days forward from today, `#201e1d` 2px dashed `3 4`, drawn over a `rgba(32,30,29,.05)` band.
  - The band carries the caps label PROJECTED (#605d5d) at its top-left.
  - The projection = today's balance plus scheduled Bills, recurring income and card statement payments, applied by date.
- **Buffer line:** `#605d5d` 1px dashed `5 4`, labelled "buffer 17,520" (10.5px #605d5d, on a `#f3f2f2` chip).
- **Projected low:**
  - An 8×8 `#ec3013` square on the line, with a `#ec3013` 1px drop line to the baseline.
  - A label in 11px 800 `#ae1800` beside it: "low 72,118.40", with a second line in 400: "30 sep · council rates".
- **X labels:** 10.5px #9b9797 month abbreviations, absolutely positioned at their month's x.
- If any projected day falls below the buffer, that segment and its rows in the next-30-days table turn `#ae1800`.

### 4. Lower split
`flex:1; min-height:0; flex; gap:28px`. The left column is `flex:3` and the right column `flex:2`. Each starts with a title row: a caps label on the left and an 11.5px #9b9797 meta on the right, `margin-bottom:10px`.

#### Left — ACCOUNTS · BY ACCESS ("4 accounts")
- **Table:** `border:1px solid rgba(32,30,29,.30); 12.5px`.
- **Tier header row:** `padding:7px 12px; #eae9e9; caps #605d5d; border-bottom:1px solid rgba(32,30,29,.30)`. It repeats the column heads: INSTANT · INSTITUTION 96 · RATE 104 · 30 DAYS 84 (right) · BALANCE 96 (right).
  - The NOTICE tier relabels the 30 DAYS column as MATURES.
- **Account row:** `padding:5px 12px; border-bottom:1px solid #d7d3d3`, tabular. The name is 800 and the other cells #605d5d (the balance is ink).
- **Tier subtotal:** `padding:7px 12px; #eae9e9; 1px top rule; 12px`. The label ("instant" / "notice") is #605d5d and the total 800.
- **Tiers:**
  - **Instant:** available today.
  - **Notice:** term deposits and notice savers. The mock shows "MATURES 14 dec".
  - **Locked** (not in the sample): other cash held for longer.
- **Sample:**

  | Tier | Account | Institution | Rate | 30 days / matures | Balance |
  | --- | --- | --- | --- | --- | --- |
  | INSTANT | ANZ Everyday | ANZ | — | +412.30 | 4,182.55 |
  | INSTANT | ANZ Offset | ANZ | offsets 6.09% | +1,850.00 | 61,204.10 |
  | INSTANT | Westpac Bonus Saver | Westpac | 4.75% | +49.20 | 12,450.00 |
  | NOTICE | Westpac Term Deposit | Westpac | 4.90% · 6 mo | 14 dec | 20,000.00 |
- **Footnote** (11.5px #605d5d, `margin-top:8px`): "Card limits aren't cash — Amex Platinum has 12,681.56 of credit available (**Credit cards**). Shares, super and crypto sit in **Investments**." Destination names are 800 ink and navigate there.

#### Right — NEXT 30 DAYS · INSTANT ("rule = payday")
- **Table:** `border:1px solid rgba(32,30,29,.30); 12px`.
- **Columns:** DATE 52 · ITEM flex · kind 46 (10.5px #9b9797: bill / card / income) · AMOUNT 76 (right) · BALANCE 78 (right, #605d5d). Rows are `padding:5px 12px`.
- **Payday rule:** the first income row gets `border-top:2px solid #201e1d`. This marks where Free to spend stops counting.
- **Low point row:** the lowest balance is shown in 800 `#ae1800` (item and balance).
- **Sample:**

  | Date | Item | Kind | Amount | Balance |
  | --- | --- | --- | --- | --- |
  | 15 sep | Home loan repayment | bill | −2,410.00 | 75,426.65 |
  | 17 sep | Telstra | bill | −99.00 | 75,327.65 |
  | 18 sep | Amex statement | card | −2,318.44 | 73,009.21 |
  | 20 sep | Origin Energy | bill | −284.60 | 72,724.61 |
  | 23 sep | Dept of Education | income | +4,210.00 | 76,934.61 |
  | 29 sep | Home loan repayment | bill | −2,410.00 | 74,524.61 |
  | 30 sep | **Council rates** | bill | −2,406.21 | **72,118.40** |
  | 07 oct | Dept of Education | income | +4,210.00 | 76,328.40 |

  The sum of the items before 23 sep is 5,112.04, which is the "due before payday" figure in Free to spend.

### Status bar (6a)
- **Left:** `NORMAL` · `j/k row · t transfer · u record balance · f set buffer · enter open account`
- **Right:** "projection from Bills · income every 2nd wed"

---

## Dialogs (6b, 6c)
- **Shell:** the view stays rendered underneath a dimmer, `position:absolute; inset:0; background:rgba(32,30,29,.30)`. The dialog is top-anchored with `padding-top:110px`.
- **Dialog box:** 440px wide, `#f3f2f2`, `border:2px solid #201e1d`, `box-shadow:0 16px 48px rgba(32,30,29,.40)`, square corners.
- **Title bar:** `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30); 800 16px Archivo .01em`.
- **Body:** `padding:20px; flex column; gap:16px`.
- **Field:** label 800 12px, `margin-bottom:6px`. Control `padding:8px 10px; border:1px solid rgba(32,30,29,.30); 13px`, tabular. Selects show a `▾` in #605d5d.
- **Info note:** `padding:10px 12px; #eae9e9; border-left:2px solid #ec3013; 11.5px/1.5 #605d5d`. Key figures in it are 800 ink.
- **Footer:** `padding:16px 20px; border-top:1px solid #d7d3d3; justify-end; gap:10px`. **Cancel** (outlined, `padding:8px 16px`, 800) and the confirm (`#201e1d` / `#f3f2f2`, 800).
- **Keys:** `esc` cancels. `enter` confirms when the form is valid. Focus moves to the first field and is trapped inside the dialog.

### 6b · Transfer between accounts (`t`)
- **From / To:** a 2-column grid, gap 12. Account selects show their live balance ("ANZ Offset · 61,204.10" → "ANZ Everyday · 4,182.55"). Only cash accounts can be picked, and From ≠ To.
- **Amount / Date:** a 2-column grid. Amount is "2,500.00 aud" (the unit suffix in #9b9797). Date defaults to today.
- **Memo (optional):** placeholder text in #9b9797.
- **Live preview note** (recomputes on every change): "After: Offset **58,704.10** — saves 297.92 this month (−12.69). Everyday **6,682.55**. Free to spend is unchanged; both accounts are instant."
  - The offset clause appears only when the offset is involved.
  - When the move crosses tiers (instant ⇄ notice), the note states the change to Free to spend instead.
- **Footnote** (11.5px #9b9797): "Recorded as a linked pair — one transaction in each account, category **Transfer**."
- **Validation:** the amount must be > 0. Warn, but do not block, when the transfer would take From below 0 or below the buffer; the warning replaces the note's last sentence in `#ae1800`.
- **Confirm:** **Transfer**. It creates two linked transactions (Transfer category, excluded from spend), closes the dialog, and selects the To account row.

### 6c · Set buffer (`f`)
- **Buffer is:** a full-width segmented control, `border:1px solid rgba(32,30,29,.30)`, segments `padding:7px 10px; 12px`. The active segment is `#201e1d` / `#f3f2f2` 800. Options are **months of spending** | **fixed amount**.
- **Months mode:** a grid of `110px 1fr`, gap 12. Fields:
  - **Months:** default 3.
  - **Average spend from:** a select, "last 12 months · 5,840.00 / mo". Other options: last 6 months, last 3 months, budget total.
- **Fixed mode:** replaces both fields with a single **Amount** field.
- **Result row:** `flex; space-between; padding:10px 0; border-top:2px solid rgba(32,30,29,.38); border-bottom:1px solid #d7d3d3`. The caps label BUFFER (#605d5d) is on the left. The value **17,520.00** (20px/800, tabular) and "aud" (12px 400 #605d5d) are on the right.
- **Held in:** checkboxes (14×14, square; checked = `#201e1d` fill with a white check):
  - **instant accounts — 77,836.65 · covered 4.4×**, checked.
  - **notice accounts — 20,000.00**.
  - Only a buffer held in instant is subtracted from Free to spend.
- **Note:** "Free to spend subtracts the buffer. Projected days that dip below it turn red in Next 30 days and on the chart."
- **Confirm:** **Save buffer**. It recomputes Free to spend, the buffer line and the red thresholds.

### Record balance (`u`)
The button exists in 6a but the dialog is not drawn. Build it with the same dialog shell:
- **Fields:** Account (cash accounts without a feed listed first) · Balance · As of date.
- **Note:** shows the difference from the ledger balance. On confirm it creates a balance-check entry, and the variance appears in Reports › Balance Check Variance.

---

## Interactions & behaviour
- **Selecting:** `j`/`k` moves through the account rows (tier headers and subtotals are skipped). `enter` opens that account's ledger in Transactions, filtered to the account.
- **Navigation:** clicking a Next-30-days row opens its source (the bill in Bills, the statement in Credit cards, the income's recurring transaction).
- **Chart hover:** a vertical guide with a date and balance chip (11px, `#201e1d` / `#f3f2f2`, `padding:3px 6px`). Hovering in the projected band also names the item that day.
- **Recalculation:** after sync, bill edits, transfer, record balance and buffer changes. Free to spend, Runway and the projected low must agree with the Dashboard (2b) and Notifications ("Projected low below buffer" rule).
- **Next income event** = the next scheduled income from recurring transactions ("rule = payday"). With no recurring income, Free to spend uses the next 30 days and the meta reads "rule = 30 days".
- **Empty state:** with no cash accounts, show "No cash accounts yet" plus **+ Add account** (opens Settings › Accounts).
- **Stale balance:** an account not updated in 7+ days shows its balance in #9b9797 with "as of 02 sep" in the 30 DAYS cell.

## Keyboard
| Key | Action |
| --- | --- |
| `g c` | Open Cash. |
| `j` / `k` | Next / previous account row. |
| `enter` | Open the account ledger. |
| `t` | Transfer… |
| `u` | Record balance… |
| `f` | Set buffer… |
| `esc` | Close a dialog. |

Palette: `:transfer <from> <to> <amount>`, `:buffer 3mo` / `:buffer 15000`.

## State
```
cash
  selectedAccountId: Option<AccountId>
  dialog: None | Transfer(TransferDraft) | SetBuffer(BufferDraft) | RecordBalance(RecordDraft)

CashAccount  { id, name, institution, unit, tier: Instant|Notice|Locked, rate?: Rate,
               isOffsetFor?: LoanId, maturesOn?: Date, balance, balanceAsOf, delta30d }
Buffer       { mode: Months(n, window: Last12|Last6|Last3|Budget) | Fixed(amount),
               heldIn: { instant: bool, notice: bool } }      // ledger Preference, syncs
Projection   { points: Vec<(Date, balance)>, low: (Date, balance, itemLabel), nextIncome: Date }
TransferDraft { from, to, amount, date, memo }
```
The headline figures, the tier subtotals and the projection are all derived; don't store them. Buffer is a ledger-scoped Preference.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · chrome / table head `#eae9e9` · gridline `#d7d3d3` · projection band `rgba(32,30,29,.05)`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797`.
  - Accent `#ec3013` (low marker, info rule) · text-safe accent `#ae1800` (low point, shortfalls).
  - Rules: strong `rgba(32,30,29,.38)` · border `rgba(32,30,29,.30)` · dimmer `rgba(32,30,29,.30)`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 10 / 10.5 / 11 / 11.5 / 12 / 12.5 / 13 / 16 / 20 / 24 / 28px.
  - Caps labels `.11em`. Tabular numerals everywhere.
- **Spacing:** 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 22, 28px.
- **Radius:** 0. **Shadow:** dialog `0 16px 48px rgba(32,30,29,.40)`.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary` from Modernist (`styles.css`).

## Assets
- **Rail icon (wallet):** Lucide-style, 14px, 1.5 stroke, drawn as a rect `1.5,3.5 13×10`, a path `M1.5 6.5h10.5` and a circle `11.5,10 r1`.
- **Chart:** inline SVG. Rebuild it with GPUI paths.
- No images.

## Files
- `Cash.dc.html` — design reference: 6a Cash view, 6b Transfer, 6c Set buffer.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
