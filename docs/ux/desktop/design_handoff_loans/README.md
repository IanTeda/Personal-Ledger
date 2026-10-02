# Handoff: Loans

## Overview
**Loans** (`g n`, under NET WORTH) shows each loan's balance, how every repayment splits between interest and principal over the whole term, the payoff date, and the interest still to pay. It also runs a read-only **What if…** for extra repayments.

There is one loan per view; the title is the loan switcher.

Frames are section 8 of `Ledger Desktop Shell.dc.html`:
- **9a:** Loan view (Home loan).
- **9b:** What if… (extra-repayment panel).
- **9c:** Loan switcher.

## About the design files
`Loans.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Loans depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample figures are illustrative (AUD). **Correct one data slip in the mock:** the TODAY rule reads 30 Sep 2026, but 15 Sep is still "Scheduled". The rule must sit between the last paid row and the first scheduled one; derive it from today's date.

## Frame & shell
- **Frame:** 1280 × 800.
- **Header:** 48px. Use the standard header: breadcrumb `net worth › loans`, sync status, window controls.
- **Primary rail:** 206px. Loans is active (NET WORTH: Cash `g c` · Inventory `g o` · **Loans `g n`** · Credit cards `g k` · Investments `g i`).
- **Status bar:** the mockups omit it. Add the standard 28px status bar (see Keyboard).
- **Content pane:** `flex:1; min-width:0; padding:18px 28px; flex column; overflow:hidden`. 9b adds a 224px side panel to the right of it.

---

## 9a · Loan view

### 1. Heading
`flex; align-items:flex-end; justify-content:space-between; margin-bottom:14px`.
- **Left: loan switcher.**
  - `h2` "Home loan" (28px/800), then a chevron-down icon, then a 4px gap, then the rate-type chip. Gap 8, `cursor:pointer`, `margin-bottom:4px`.
  - **Rate-type chip:** `#201e1d` / `#f3f2f2`, `800 10px/1 Archivo .1em; padding:4px`. Values: **VARIABLE** · FIXED · SPLIT.
  - Clicking the title or pressing `l` opens 9c.
  - Subline (11.5px #9b9797): "Macquarie · 6.14% variable · 30 years from Mar 2021 · offset: ANZ Offset 61,204.10", i.e. lender · rate · term and start · offset account and balance. Omit the offset clause when the loan has none.
- **Right:** **What if…** (secondary) · **Loan settings** (secondary) · **+ Record repayment** (primary). All `.btn`, 36px, gap 8.

### 2. Figures strip
`flex; border:1px solid rgba(32,30,29,.30); margin-bottom:14px`. Four cells, `flex:1; padding:10px 14px; border-right:1px solid rgba(32,30,29,.30)`.
- **Cell content:**
  - Caps label: `800 10px/1 Archivo .11em #9b9797; margin-bottom:6px`.
  - Value: 19px/800, tabular.
  - Note: 11px #9b9797, `margin-top:2px`.

| Label | Value | Note | Logic |
| --- | --- | --- | --- |
| BALANCE | −517,517.93 | of 560,000.00 borrowed | Shown negative, as a liability. |
| REPAYMENT | 3,408.05 | monthly, 15th | Scheduled repayment and frequency. |
| PAID OFF | Feb 2051 | 24y 6m to go | Amortise from today at the current rate, repayment and offset. |
| INTEREST TO GO | 484,449.64 | offset saves ≈ 313.16 / mo | Sum of projected interest. Offset saving = offset balance × rate ÷ 12. |

### 3. Chart panel
`border:2px solid #201e1d; padding:12px 16px 8px; margin-bottom:14px`.
- **Title row** (12px, `margin-bottom:8px`): "Interest vs principal, per repayment" (800).
- **Legend** (right-aligned, #605d5d, gap 14): swatches are 10×10.
  - `#ec3013` Interest
  - `#201e1d` Principal
  - a 14px `2px solid #201e1d` line for Balance (right axis)
- **SVG:** 736 × 210. Plot x 44 → 684, y 8 (top) → 186 (baseline).
  - **Gridlines:** `rgba(32,30,29,.18)` at 0 / half / full.
  - **Axis labels:** 10px #605d5d.
    - Left axis: the repayment (0 · 1,704 · 3,408).
    - Right axis: the balance (0k · 280k · 560k).
  - **Stacked area per repayment across the whole term:**
    - The **principal** polygon (`#201e1d`) fills from the baseline up to the repayment line.
    - The **interest** polygon (`#ec3013`) sits on top of it. It is largest at the start and shrinks as the balance falls.
    - Interest plus principal is the flat repayment band at the top.
  - **Past repayments** are drawn pale. A vertical **today rule** separates past from future.
  - **Balance line:** a `#201e1d` 2px path read on the right axis, descending to 0 at payoff.
- **Above the table:** a caps label **REPAYMENTS** on the left. On the right, 11px #9b9797: "**78%** of the next repayment is interest · rate set by lender, variable" (the percentage is 800 #201e1d).

### 4. Repayments table
`border:1px solid rgba(32,30,29,.30)`.
- **Header:** `padding:6px 14px; #eae9e9; 800 10px Archivo .1em #605d5d`.
- **Columns:**

  | Column | Flex | Align |
  | --- | --- | --- |
  | DATE | 1.1 | left |
  | STATUS | 1.25 | left |
  | REPAYMENT | 1 | right |
  | RATE | .7 | right |
  | INTEREST | 1 | right |
  | PRINCIPAL | 1 | right |
  | BALANCE | 1.05 | right |
- **Row:** `padding:5px 14px; 12.5px`, tabular, with a 1px bottom rule.
  - **Past (Paid)** rows are #605d5d, and the status reads "Paid · ANZ Everyday" (the paying account).
  - **Scheduled** rows are ink, with the interest cell in `#ae1800`.
- **TODAY divider:** `padding:3px 14px; background:#201e1d; color:#f3f2f2`, a caps label "TODAY · <date>" on the left. On the right, "63 more paid earlier · open history" (400, #bab6b6) opens the full history.
- **Rows shown:** the last 3 paid, the divider, then the next 3 scheduled.

| Date | Status | Repayment | Rate | Interest | Principal | Balance |
| --- | --- | --- | --- | --- | --- | --- |
| 15 Jun 2026 | Paid · ANZ Everyday | 3,408.05 | 6.14% | 2,659.52 | 748.54 | 519,026.52 |
| 15 Jul 2026 | Paid · ANZ Everyday | 3,408.05 | 6.14% | 2,655.69 | 752.37 | 518,274.15 |
| 15 Aug 2026 | Paid · ANZ Everyday | 3,408.05 | 6.14% | 2,651.84 | 756.22 | 517,517.93 |
| **TODAY** | | | | | | |
| 15 Sep 2026 | Scheduled | 3,408.05 | 6.14% | 2,647.97 | 760.09 | 516,757.85 |
| 15 Oct 2026 | Scheduled | 3,408.05 | 6.14% | 2,644.08 | 763.98 | 515,993.87 |
| 15 Nov 2026 | Scheduled | 3,408.05 | 6.14% | 2,640.17 | 767.88 | 515,225.99 |

---

## 9b · What if… (extra repayment)
Opens a **224px side panel** on the right: `border-left:2px solid rgba(32,30,29,.38); padding:16px; flex column; gap ~12`. The **What if…** button becomes **Close what-if**.

### Panel
- **Heading:** caps label WHAT IF, then "Pay extra each month" (800 15px).
- **Extra per month:** label 11.5px #605d5d. The input is 32px, `border:2px solid #201e1d; #f3f2f2`, reading "+500.00", with a right-aligned "aud" (11px #9b9797).
- **Slider:** a 4px `#d7d3d3` track with a `#201e1d` fill (35% here) and a 6×14 `#ec3013` thumb. Range 0 → the repayment amount, step 50. It is bound to the input.
- **Divider:** 2px rule.
- **Interest saved:** 11px #605d5d label, value **135,254.63** (22px/800, tabular).
- **Paid off sooner by:** **6y 0m** (22px/800).
- **Note** (11.5px/1.45 #605d5d): "Nothing is saved to the loan. Apply it as a recurring extra repayment under **Loan settings**."
- **Bottom of the panel:** **Set as extra repayment** (primary, 36px) and **Clear** (secondary, 36px).

### Effect on the view (live, read-only)
- **Figures:**
  - REPAYMENT 3,908.05, note "incl. extra · monthly, 15th".
  - **PAID OFF Feb 2045** in `#ae1800`, note "18y 6m to go".
  - INTEREST TO GO 349,195.01.
- **Chart:**
  - A grey **extra** band (`#bab6b6`) is added on top of principal.
  - The balance line drops earlier.
  - The current plan stays as a **dashed** `#605d5d` balance line.
  - The legend adds "Extra" and "Balance (current plan dashed)".
- **Table:** scheduled rows recompute (15 Sep: 3,908.05 · 2,647.97 · 1,260.09 · 516,257.85, and so on). The summary prefix reads "with extra …".

---

## 9c · Loan switcher (`l`)
- **Overlay:** a dimmer `rgba(32,30,29,.18)` over the content area (`inset:48px 0 28px 206px`). Align it with the shell's popover treatment.
- **Popover:** 400px, anchored under the title (`left:234px; top:106px`), `#f3f2f2; border:2px solid #201e1d`.
- **Search row:** `padding:9px 14px; border-bottom:1px solid #d7d3d3`, with a search icon, the placeholder "find a loan" and a right-aligned key hint "l" (11px).
- **Loan rows:** `flex; align-items:center; gap:10px; padding:9px 14px`.
  - **Check column:** 12px; the active loan shows ✓ (800).
  - **Main:** the name in 800, then "lender · rate · paid off <Mon YYYY>" (11px #605d5d).
  - **Right column:** the balance (800, tabular) above the rate-type chip (`border:1px solid rgba(32,30,29,.35); 800 9.5px .1em`).
  - **Active row:** `#201e1d` / `#f3f2f2`. The meta becomes #d7d3d3 and the chip border `rgba(243,242,242,.5)`.
  - Sample: **Home loan** (Macquarie · 6.14% · paid off Feb 2051, −517,517.93, VARIABLE) · **Car loan** (ANZ · 7.49% · paid off Feb 2029, −14,208.90, FIXED).
- **PAID OFF group:** a caps label, then muted rows (#9b9797), indented 36px, with a right-aligned "CLOSED MAR 2025" (11px). Sample: Personal loan 2022. Paid-off loans are never deleted.
- **Footer:** `border-top:2px solid rgba(32,30,29,.38)`, split in two:
  - **+ New loan** (800) with the key hint `n`.
  - "Total owed **−531,726.83**" (#605d5d, with the value in 800 ink).

---

## Interactions & behaviour
- **Amortisation** is computed per loan from balance, rate, repayment, frequency, interest method (daily / monthly) and the offset balance. Recompute after each repayment, rate change, offset change or settings edit. Variable-rate projections assume today's rate, and the subline says so.
- **Record repayment:** opens a dialog (not drawn) using the shared dialog shell.
  - Fields: Date · Amount · From account · Split (auto, editable: interest / principal / fees).
  - It creates a transfer from the paying account and marks the scheduled row Paid.
- **Bills link:** the loan's scheduled repayment also appears in Bills and in the Cash projection (7a), so the amounts must agree.
- **What if:**
  - Changes nothing until **Set as extra repayment**. That writes a recurring extra to Loan settings and closes the panel.
  - **Clear** resets the panel to 0.
  - `esc` closes it.
- **Loan settings:** lender, rate and type, term and start, repayment and frequency, offset account, interest method, and fixed-rate expiry (raises a notification before expiry).
- **Switcher:** `j`/`k` moves, `enter` selects, typing filters, `esc` closes. **+ New loan** opens the loan form.
- **Fixed / split loans:**
  - **Fixed:** the subline shows "fixed until <date>" and the table marks the reversion month.
  - **Split:** shows the portions in the figures notes.
- **Empty state:** "No loans" with **+ New loan**.

## Keyboard
| Key | Action |
| --- | --- |
| `g n` | Open Loans (last loan viewed). |
| `l` | Loan switcher. |
| `w` | What if… open / close. |
| `r` | Record repayment. |
| `,` | Loan settings. |
| `h` | Open full repayment history. |
| `j` / `k` | Move rows (table) or loans (switcher). |
| `enter` | Select (switcher). |
| `esc` | Close the panel or popover. |
| `n` | New loan (in the switcher). |

**Status bar (to add):**
- **Left:** `NORMAL` · `l switch · w what if · r record repayment · , settings · h history`
- **Right:** "rates as of 30 sep · variable assumes no change"

## State
```
loans
  loanId: LoanId
  whatIf: Option<{ extraPerPeriod: Money }>
  switcherOpen: bool, switcherQuery: String
  dialog: None | RecordRepayment(Draft) | LoanSettings(LoanId) | NewLoan

Loan { id, name, lender, principal, startOn, termMonths, rateType: Variable|Fixed{until}|Split{parts},
       rate, repayment, frequency: Monthly|Fortnightly|Weekly, dayOfPeriod,
       interestMethod: Daily|Monthly, offsetAccountId?, extraRepayment?: Money,
       payingAccountId, closedOn? }
Repayment { loanId, date, status: Paid|Scheduled, amount, rate, interest, principal, balanceAfter, transactionId? }
Schedule (derived) { rows: Vec<Repayment>, payoffOn, interestToGo, offsetSavingPerPeriod }
```
- Past rows come from transactions. Future rows are derived.
- The What-if schedule is a second derived schedule, discarded on close.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · table head `#eae9e9` · track `#d7d3d3` · extra band `#bab6b6`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6` / `#d7d3d3`.
  - Interest `#ec3013` · text-safe accent `#ae1800` (scheduled interest, changed payoff).
  - Rules: strong `rgba(32,30,29,.38)` · border `rgba(32,30,29,.30)` · grid `rgba(32,30,29,.18)` · row `rgba(32,30,29,.20)`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 9.5 / 10 / 11 / 11.5 / 12 / 12.5 / 15 / 19 / 22 / 28px.
  - Caps labels `.11em` (chips and table head `.1em`). Tabular numerals on every figure.
- **Spacing:** 3, 4, 5, 6, 8, 9, 10, 12, 14, 16, 18, 28px.
- **Radius:** 0. **Shadows:** none; the popover relies on its 2px border.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Focused inputs use `border:2px solid #201e1d`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary` from Modernist (`styles.css`).

## Assets
- **Icons:** the rail uses a 14px Lucide-style percent / diagonal mark (1.5 stroke). The mock also uses a chevron-down (switcher) and a search icon (popover).
- **Chart:** inline SVG. Rebuild it with GPUI paths.
- No images.

## Files
- `Loans.dc.html` — design reference: 9a Loan view, 9b What if…, 9c Loan switcher.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
