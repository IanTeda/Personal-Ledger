# Handoff: Credit cards

## Overview
**Credit cards** (`g k`, under NET WORTH) answers one question first: **am I paying interest?** It shows the card's interest-free status, splits the balance into the statement due and spending since, charts each statement against what was paid, and lists statements with any interest charged. **Pay card…** records the payment.

There is one card per view; the title is the card switcher.

Frames are section 9 of `Ledger Desktop Shell.dc.html`:
- **9a:** Card view (ANZ Platinum).
- **9b:** Pay card… dialog.

## About the design files
`Credit Cards.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder. Links to other sections point to `../Ledger Desktop Shell.dc.html`.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Credit cards depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample figures are illustrative (AUD, today = 30 Sep 2026).

## Frame & shell
- **Frame:** 1280 × 800.
- **Header:** 48px. Use the standard header: breadcrumb `net worth › credit cards`, sync status, window controls.
- **Primary rail:** 206px. Credit cards is active (NET WORTH: Cash `g c` · Inventory `g o` · Loans `g n` · **Credit cards `g k`** · Investments `g i`).
- **Status bar:** the mockups omit it. Add the standard 28px status bar (see Keyboard).
- **Content pane:** `flex:1; min-width:0; padding:18px 28px; flex column; overflow:hidden`.

---

## 9a · Card view

### 1. Heading
`flex; align-items:flex-end; justify-content:space-between; margin-bottom:14px`.
- **Left: card switcher.**
  - `h2` "ANZ Platinum" (28px/800), then a chevron-down, a 4px gap and the **status chip**. Gap 8, `cursor:pointer`.
  - Clicking or pressing `k` opens the switcher. Build it like the Loans switcher (Loans handoff, 8c): balance, rate, status chip per card, and a CLOSED group.
  - **Status chip:** `800 10px/1 Archivo .1em; padding:4px`.

    | Chip | Style | Meaning |
    | --- | --- | --- |
    | **INTEREST-FREE** | `#201e1d` / `#f3f2f2` | Last statement paid in full by its due date. |
    | **PAYING INTEREST** | `#ec3013` / `#f3f2f2` | A statement was part-paid; interest accrues until a full statement balance is paid. |
    | **RESTORED** | outline | The month after interest-free is regained. |
  - **Subline** (11.5px #9b9797): "Visa ·· 4417 · 20.99% purchases · 55 days interest-free · statement closes the 21st · payment envelope in **Household**". **Household** is a link to the Envelope budget (Budgets v2, section 13).
- **Right:** **Card settings** (secondary) and **Pay card…** (primary). Both `.btn`, 36px, gap 8.

### 2. Figures strip
`flex; border:1px solid rgba(32,30,29,.30); margin-bottom:14px`. Four cells, `flex:1; padding:10px 14px; border-right:1px solid rgba(32,30,29,.30)`.
- **Cell content:**
  - Caps label: `800 10px/1 .11em #9b9797; mb 6`.
  - Value: 19px/800, tabular.
  - Note: 11px #9b9797.

| Label | Value | Note | Logic |
| --- | --- | --- | --- |
| BALANCE | −2,846.30 | incl. 731.70 since statement | Current balance (a liability, shown negative). |
| AVAILABLE | 9,153.70 | of 12,000.00 limit · 24% used | Limit − balance. A 4px `#d7d3d3` bar below the note, fill `#201e1d` = used %. The fill turns `#ec3013` at 80% or more (the "Card near limit" notification rule). |
| STATEMENT DUE | **2,114.60** (`#ae1800`) | by 16 Oct · minimum 42.30 | Unpaid closing balance of the latest statement. Ink when paid, with the note "paid 12 Oct". |
| PAYMENT ENVELOPE | 2,846.30 | covers the full balance | The card's envelope in the Envelope budget. If it holds less than the statement due, the note reads "short by …" in `#ae1800`. |

### 3. Chart + Since-statement row
`flex; gap:14px; margin-bottom:14px`.

#### Chart panel
632px, `border:2px solid #201e1d; padding:12px 14px 6px`.
- **Title row** (12px): "Statements, last 12" (800).
- **Legend** (right, #605d5d, gap 12, 10×10 swatches):
  - `#201e1d` Closing
  - `#bab6b6` Paid
  - `#ec3013` Short / interest
  - `1.5px dashed #605d5d` Due
- **SVG:** 600 × 196. Plot x 40 → 592, y 10 (top) → 174 (baseline).
  - **Gridlines:** `rgba(32,30,29,.18)` at 0 / 2,000 / 4,000. Left-axis labels are 10px #605d5d.
  - **Per statement month**, a pair of 18px bars, 2px apart, about 46px per month:
    - **Closing:** `#201e1d`.
    - **Paid:** `#bab6b6`, height = amount paid.
  - **Part-paid month:** the paid bar is followed by a red (`#ec3013`) segment for the unpaid shortfall. Interest charged the next month is also marked red.
  - **Current (unpaid) statement:** an outline-only dashed bar ("Due").
  - **Month labels** below the baseline: 10px #605d5d, Oct → Sep.

#### Since statement
`flex:1; border:1px solid rgba(32,30,29,.30); flex column`.
- **Header:** `padding:10px 14px`. "Since statement" (800) on the left; "22 Sep – 21 Oct · day 9 of 30" (11px #9b9797) on the right.
- **Rows:** `flex; gap:10px; padding:6px 14px; border-bottom:1px solid rgba(32,30,29,.15)`, tabular. Columns:
  - date: 44px, #9b9797
  - payee: flex, with its category inline after it (11px #9b9797, `margin-left:6px`)
  - amount: right-aligned

  Sample:

  | Date | Payee | Category | Amount |
  | --- | --- | --- | --- |
  | 29 Sep | Woolworths | Groceries | −212.40 |
  | 27 Sep | Uber | Transport | −34.80 |
  | 26 Sep | JB Hi-Fi | Kitchen reno | −389.00 |
  | 24 Sep | Spotify | Subscriptions | −13.99 |
  | 23 Sep | Coles | Groceries | −81.51 |
- **Footer:** `padding:8px 14px; 1px top rule`. "14 transactions · **all**" (#605d5d; **all** opens Transactions filtered to this card and period), with the total **−731.70** (800) on the right.

### 4. Statements table
- **Above the table:** a caps label STATEMENTS on the left; on the right, "Pay the closing balance by the due date to keep interest-free days · 31 statements" (11px #9b9797).
- **Container:** `border:1px solid rgba(32,30,29,.30)`.
- **Header:** `padding:6px 14px; #eae9e9; 800 10px .1em #605d5d`.
- **Columns:**

  | Column | Flex | Align |
  | --- | --- | --- |
  | STATEMENT | 1.1 | left |
  | CLOSING | 1 | right |
  | MINIMUM | 1 | right |
  | DUE | .8 | right |
  | PAID | 1 | right |
  | INTEREST | .8 | right |
  | STATUS | 1.5 | `padding-left:18px` |
- **Rows:** `padding:5px 14px; 12.5px`, tabular, 1px bottom rule.
- **Current statement row:** `background:#eae9e9`. Its status is a chip **DUE IN 16 DAYS** (`#201e1d` / `#f3f2f2`, `800 9.5px .1em`).
  - It turns `#ec3013` when overdue.
  - At 3 days or fewer it reads "DUE IN 3 DAYS" on `#ae1800`.
- **Status text:**
  - "Paid in full · <account>": #605d5d.
  - "Paid · interest-free restored": #605d5d.
  - "**Part-paid · interest charged next**": 800 `#ae1800`.
- **Interest amounts** are shown in `#ae1800`.

| Statement | Closing | Minimum | Due | Paid | Interest | Status |
| --- | --- | --- | --- | --- | --- | --- |
| 21 Sep 2026 | 2,114.60 | 42.30 | 16 Oct | — | — | `DUE IN 16 DAYS` |
| 21 Aug 2026 | 2,486.05 | 49.72 | 16 Sep | 2,486.05 | — | Paid in full · ANZ Everyday |
| 21 Jul 2026 | 1,958.60 | 39.17 | 16 Aug | 1,958.60 | — | Paid in full · ANZ Everyday |
| 21 Feb 2026 | 2,210.90 | 44.22 | 16 Mar | 2,210.90 | **38.12** | Paid · interest-free restored |
| 21 Jan 2026 | 3,120.20 | 62.40 | 16 Feb | 1,500.00 | — | **Part-paid · interest charged next** |

The mock skips Mar–Jun to show the interest story. The real table lists every statement newest first and scrolls.

---

## 9b · Pay card…
- **Overlay:** a dimmer over the content area (`inset:48px 0 0 206px; rgba(32,30,29,.18)`).
- **Dialog:** 440px, at `left:448px; top:132px`, `#f3f2f2; border:2px solid #201e1d`. **Align with the shared dialog shell:** full-window `rgba(32,30,29,.30)` dimmer, top-anchored, shadow `0 16px 48px rgba(32,30,29,.40)`.
- **Header:** `padding:14px 18px 10px; border-bottom:2px solid rgba(32,30,29,.38)`. "Pay ANZ Platinum" (800 17px), then "Records a transfer. Nothing is sent to the bank." (11.5px #605d5d).
- **Body:** `padding:14px 18px; flex column; gap:8px`.
  - **Amount options:** a radio list. Each option is `flex; align-items:center; gap:10px; padding:10px 12px; border:1px solid rgba(32,30,29,.30)`; the selected option has `border:2px solid #201e1d`.
    - **Radio:** a 14×14 square, `border:2px solid #201e1d`. When selected it shows a 6×6 `#201e1d` dot.
    - **Text:** the label in 800, a hint in 11px #605d5d, and the amount right-aligned (800, tabular).

    | Option | Hint | Amount |
    | --- | --- | --- |
    | **Statement balance** (default) | Keeps interest-free days · due 16 Oct | 2,114.60 |
    | Full balance | Includes 731.70 since statement | 2,846.30 |
    | Minimum | Interest charged on the rest, back-dated | 42.30 |
    | Other amount | — | becomes an input |
  - **From / Date:** a row, gap 10, `margin-top:6px`.
    - **From:** a select, `flex:1`, 32px, reading "ANZ Everyday" with "4,182.55 ▾" (11px #9b9797). Cash accounts only.
    - **Date:** 130px, default today ("30 Sep 2026").
  - **Effect note** (11.5px/1.45 #605d5d): "Payment envelope drops to **731.70**, covering what's spent since the statement." It recomputes with the amount.
    - Choosing **Minimum** or an amount below the statement balance replaces it with: "Interest will be charged on 2,072.30 from the purchase dates · ≈ 36.25 next statement" in `#ae1800`.
- **Footer:** `padding:12px 18px; border-top:2px solid rgba(32,30,29,.38); gap:8`. **Record payment · 2,114.60** (primary, `flex:1`, 36px; the label carries the amount) and **Cancel** (secondary).

**On confirm:**
- Create a linked transfer: From account → card, category Transfer.
- Mark the statement Paid (or Part-paid).
- Debit the payment envelope by the amount.
- Recompute the status chip.
- Resolve the "Interest-free period ending" notification when the statement balance is fully covered.

---

## Interactions & behaviour
- **Interest-free logic** (per standard AU card terms):
  - Paying the full **statement** closing balance by the due date keeps purchases interest-free.
  - A part payment loses interest-free days. Interest is charged on the unpaid balance and on new purchases from their transaction dates.
  - Interest-free returns after the next statement is paid in full ("interest-free restored").
- **Statements** are generated on the closing day from transactions in the period. The due date = closing + the card's due offset.
- **Notifications** (section 5) read from here: "Interest-free period ending" (N days before due) and "Card near limit".
- **Cash** (section 6) lists the statement payment in Next 30 days.
- **Envelope:** the payment envelope tops up as purchases are categorised in the Envelope budget, moving money from the spending category envelope to the card's payment envelope.
- **Card settings:** network and last 4 digits, limit, purchase rate, cash-advance rate, interest-free days, closing day, due offset, payment envelope, and the default paying account.
- **Selecting rows:** a statement row opens its transactions. A since-statement row opens the transaction.
- **Empty state:** "No credit cards" with **+ Add card** (opens Settings › Accounts).

## Keyboard
| Key | Action |
| --- | --- |
| `g k` | Open Credit cards (last card viewed). |
| `k` | Card switcher. |
| `p` | Pay card… |
| `,` | Card settings. |
| `j` / `k` | Move rows (while the switcher is closed, `k` moves rows inside a table only when the table has focus). |
| `enter` | Open the statement or transaction. |
| `1`–`4` | Pick an amount option in Pay card. |
| `esc` | Close the dialog or switcher. |

**Status bar (to add):**
- **Left:** `NORMAL` · `k switch · p pay card · , settings · enter open statement`
- **Right:** "statement closes the 21st · due 16 Oct"

## State
```
cards
  cardId: AccountId
  switcherOpen: bool
  dialog: None | Pay(PayDraft) | Settings(AccountId)

Card       { accountId, name, network, last4, limit, purchaseRate, cashRate,
             interestFreeDays, closingDay, dueOffsetDays, envelopeId?, defaultFromAccountId }
Statement  { cardId, closedOn, dueOn, closing, minimum, paid, interestCharged,
             status: Due|PaidInFull|PartPaid|Restored|Overdue }
PayDraft   { option: Statement|Full|Minimum|Other(Money), fromAccountId, date }
CardStatus (derived) = InterestFree | PayingInterest | Restored
```
- The balance, available credit, since-statement list, status and envelope coverage are all derived.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · table head / current row `#eae9e9` · track `#d7d3d3` · paid bars `#bab6b6`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797`.
  - Accent `#ec3013` (shortfall, interest bars, overdue) · text-safe accent `#ae1800` (statement due, interest, part-paid).
  - Rules: strong `rgba(32,30,29,.38)` · border `rgba(32,30,29,.30)` · grid `rgba(32,30,29,.18)` · row `rgba(32,30,29,.15)`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 9.5 / 10 / 11 / 11.5 / 12 / 12.5 / 17 / 19 / 28px.
  - Caps labels `.11em` (chips and table head `.1em`). Tabular numerals on every figure.
- **Spacing:** 2, 3, 4, 5, 6, 8, 9, 10, 12, 14, 18, 28px.
- **Radius:** 0.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. The selected option uses a 2px ink border.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary` from Modernist (`styles.css`).

## Assets
- **Icons:** the rail uses a 14px Lucide-style credit card (1.5 stroke). The mock also uses a chevron-down (switcher).
- **Chart:** inline SVG. Rebuild it with GPUI rects.
- No images.

## Files
- `Credit Cards.dc.html` — design reference: 9a Card view, 9b Pay card….
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
