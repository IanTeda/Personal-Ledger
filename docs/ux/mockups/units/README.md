# Handoff: Personal Ledger TUI — units screen & unit forms

## Overview
The units view and its three forms for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud dependency), built against `docs/product-requirements.md` in `IanTeda/Personal-Ledger`.

A **unit** is anything a balance can be denominated in: a currency, an ETF, a share, a crypto asset. Every account fixes its unit at creation, and every transaction inherits it, which is why this screen is mostly about *references* — what points at a unit decides what you're allowed to do to it.

| id | state |
| --- | --- |
| `4a` | **Units screen** — list, summary of the highlighted unit, weekly price history |
| `4b` | **Add** — `:unit new`, code and price precision fixed at creation |
| `4c` | **Edit** — `:unit edit`, locked fields shown with their reason |
| `4d` | **Delete refused** — references exist; only deactivate is offered |
| `4e` | **Delete allowed** — nothing references it; typing the code confirms |

This screen re-hosts inside the shell specified in `../design_handoff_ledger_shell/` — status line, one full-bleed view, command line, keybind hint bar. Read that README first; it carries the shell geometry, the action registry, the theme roles and the glyph set that this screen assumes.

## About the design files
`Ledger TUI Units.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, using the repo's existing crate layout, domain types and SQLite access layer.

Every px value is an artifact of drawing. Read the geometry in **terminal cells** only: 96 × 30 minimum, one cell ≈ 6.6 × 16.5 px in the HTML.

## Fidelity
**Low-to-mid fidelity.** Authoritative about information priority, pane structure, column sets, keybindings, and the reference rules that gate editing and deletion. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All codes, prices and holdings are fake.

---

## 4a — Units screen

Two panes. Left column ~36 cols holds the list above the summary; the rest is price history.

### Unit list (top left)
`Table`, columns `CODE` (6) · `TYPE` (fill) · `LAST` (9, right-aligned, tabular).

The `TYPE` cell carries qualifiers rather than adding columns: `currency · base` marks the base unit, `share · inactive` marks a deactivated one. Inactive units render dim throughout and are hidden unless `i` is pressed. Sort: base unit first, then currencies, then by code.

### Summary (bottom left)
Bordered `Block` that redraws on every selection change — this is the screen's main payload, so it is field-per-row rather than prose:

```
VDHG                                    etf
Vanguard Diversified High Growth
symbol       VDHG.AX
precision    3 · price 4
priced in    AUD
source       manual entry
last price   72.41 · 31 aug
52w range    64.18 – 73.90
accounts     1 · Index Fund
holdings     561.204
──────────────────────────────
value        40 637.79 AUD
```

`value` sits below a rule and renders bold — it is the derived figure everything else explains. `accounts` and `holdings` are the reference counts that gate delete, so they are on this screen, not hidden in a dialog.

For a **currency**, `holdings`, `value` and `52w range` are meaningless: replace the price half of the screen with a single dim line (`currencies carry no price series — rates come from :sync`) rather than drawing an empty table.

### Weekly price history (right)
1. **Candlestick · 12 months** — `Canvas`, ~4 rows. Weekly OHLC: a 1-cell wick line with the open/close body over it; down weeks fill in the accent, up weeks in ink. X-axis labelled at first / mid / last with the first and last close (`sep 25 64.18 · mar 26 · sep 26 72.41`).
2. **Weekly prices** — the same series as a `Table`, newest first, flowing down **two side-by-side columns** (`W/C` · `CLOSE` · `Δ%`). Two columns fit ~19 weeks where one fits ~10; `j/k` scrolls, and the last cell of the second column states what is off-screen (`— 33 earlier weeks —`).
3. **Pagination row** — `showing 19 of 52 · j/k scroll` left, `[ sep 24 – sep 25   ] forward` right. `[` and `]` page whole 12-month windows and name the window they will move to, so the user never has to guess which year they are looking at.
4. **Command hints** — three dim lines, no header, directly under the pagination rule:
   ```
   :unit new <code> <type> <precision>  — add a unit
   :unit edit VDHG  — name, symbol, precision, active
   :price set VDHG <date> <close>  ·  :price import <path.csv>
   ```
   These name the commands rather than the keys because this screen's actions all take arguments; the single-key equivalents live in the hint bar.

Δ% is week-on-week, negative in the accent with a `−`. Close is right-aligned and tabular so the decimal points line up.

**Keys:** `j/k` unit · `Tab` list↔prices · `n` new · `e` edit · `p` set price · `I` import csv · `[` earlier 12m · `]` later.

---

## The forms

All three are centred floating overlays (`Clear` + bordered `Block`, ~88% width on the drawing) over the dimmed units screen — same window treatment as the command palette. Title row left, context right, the window's own dim hint row inside the border, and the app's footer greyed to `esc close unit form`.

### 4b — Add (`:unit new`)

Field order and rules:

| field | rule |
| --- | --- |
| `code` | uppercase, unique, **permanent** |
| `name` | free text |
| `type` | segmented `currency / etf / share / crypto`, `←`/`→` |
| `symbol` | optional, for the user's own reference — not a data source |
| `priced in` | another unit, `Tab` to pick |
| `qty precision` | decimals held |
| `price precision` | **permanent** |
| `active` | checkbox, default on |

A dim line closes the form: `code and price precision cannot change once a price or transaction exists`. State the constraint at creation — it is the only moment the user can still choose freely.

`^s` create · `Tab` next field · `Esc` cancel.

### 4c — Edit (`:unit edit <code>`)

Same field order, but **locked fields are shown, not hidden** — with the reason attached, so the form doesn't silently disagree with the add form:

```
code          VDHG  🔒 referenced by 412 transactions
type          etf   🔒 fixed while prices exist
priced in     AUD   🔒
```

The title row carries the reference counts (`1 account · 412 txns · 52 prices`).

**Lowering `qty precision` warns before it saves**, in a focused box:

```
lowering precision rounds 1 holding: 561.204 → 561.20
transactions are not rewritten — the rounding applies to display and new entry
```

`active` is the soft alternative to deletion: `clear to hide from pickers, keeps history`.

### 4d / 4e — Delete (`:unit delete <code>`)

**A unit can only be deleted when nothing references it.** Resolve the reference counts *before* drawing the dialog and branch — do not draw one dialog that fails on submit.

**4d — refused.** Title `cannot delete VDHG` in the accent. State the rule in one line, then every reference count, with the blocking ones in the accent:

```
transactions   412 · Index Fund
accounts       1 · Index Fund (unit fixed at creation)
prices         52 weekly closes
budgets        none
```

Then the way out, in a box: `x deactivate instead — hides VDHG from pickers, keeps every record`, plus `to delete: reassign or remove the 412 transactions first`. `Enter` opens the blocking transactions — a refusal should be navigable, not a dead end.

**4e — allowed.** Same counts, now zero, and the one consequence spelled out: `prices 18 weekly closes · deleted with the unit`. Price history is not a reference — it is owned by the unit and cascades — so say so rather than letting it vanish silently.

Confirm by **typing the code**, not by pressing `y`. `^s` deletes, `x` still offers deactivate, `Esc` cancels.

### Reference rules, in one place

| references | delete | edit code / type / priced-in | deactivate |
| --- | --- | --- | --- |
| none | allowed (cascades prices) | allowed | allowed |
| prices only | allowed (cascades prices) | `type` locked | allowed |
| accounts or transactions | **refused** | locked | allowed |

The same guard pattern applies to accounts and categories; categories additionally offer "move transactions to…" because they are reassignable, whereas a unit is not.

---

## State

Units view: selection index in the list · focused pane (list / prices) · price scroll offset · the 12-month window offset for `[` / `]` · a cached summary for the selected unit. Forms: field drafts with per-field validation errors · the resolved lock set (which fields are locked and why) · the reference counts, resolved once when the dialog opens · the typed-code confirmation buffer.

Balances, holdings, `value` and `52w range` are derived — query them, do not cache in view state.

## Style
Take the theme roles from the shell README (`../design_handoff_ledger_shell/README.md`) — ink/ground default, one accent (red) for negatives, blocked counts, down candles and cursors, dim for labels and locked values, reversed for the selected row. Never rely on color alone: negatives carry `−`, locked fields carry the lock glyph, down candles are filled where up candles are hollow.

The `🔒` glyph needs the same ASCII fallback as the status glyphs (`○ ◐ ● ⚑`) and the `│` budget marker — one config flag covers all of them.

## Files
- `Ledger TUI Units.dc.html` — the units screen and all four forms. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.

## Suggested Claude Code prompt
> Read `docs/ux/units/README.md` and `docs/ux/shell/README.md`, and open `docs/ux/units/Ledger TUI Units.dc.html` in a browser for reference. Implement the units view inside the existing shell: unit list, the summary block that redraws on selection, and the weekly price history (candlestick on a Canvas, then the two-column price table with `[` / `]` twelve-month paging). Then the add, edit and delete forms as floating overlays. Resolve reference counts before opening the delete dialog and branch between the refused and allowed variants — a unit is only deletable when no account or transaction references it, and its price history cascades. Locked fields render visibly with their reason rather than being hidden. Follow the existing crate layout and SQLite layer, keep the view and each form in their own modules with their own state structs, and use the theme roles from the shell README rather than hex values.
