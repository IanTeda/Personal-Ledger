# Handoff: Transactions — Cross-Account Ledger, Filters & Running Total

## Overview
This package covers **Personal Ledger's Transactions** surface: the cross-account transaction table with filter chips and a running total, plus the full filter-builder popover. Two variants (4a resting state, 4b filter popover open).

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** — a design reference, not production code. Recreate these designs in the target codebase (Rust + GPUI) using established patterns, informed by the mockup's layout, typography, color and behavior.

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states. Recreate faithfully; translate HTML structure into GPUI idioms.

## Frame
Both variants are drawn at **1280 × 800**, the reference desktop window. Vertical stack: header (48px) / body (flex:1) / status bar (28px). Same shell as the other handoff packages — Transactions is the active nav item.

---

## Screens

### 4a — Resting state
**Purpose**: Cross-account view — unlike the per-account ledger (see Desktop Shell & Navigation, screen 1b), this table shows transactions from every account with an **ACCOUNT** column, and accumulates a **RUNNING TOTAL** column over whatever's currently filtered. Shown filtered to "groceries" across all 7 accounts, newest first.

**Layout**:
- Header (48px): app glyph, wordmark, `transactions` breadcrumb, `:` command affordance, 3 window controls
- Primary rail (206px): Transactions active (dark treatment)
- Main pane, split into a fixed filter/header block and a scrolling table:
  - **View header** (padding `16px 28px 14px`, `border-bottom:2px solid rgba(32,30,29,.38)`):
    - Title row: "Transactions" (26px/800) + "1,248 transactions across 7 accounts" (11.5px `#9b9797`) left; `add transaction` primary button (32px, trailing kbd hint `a` at `opacity:.75`) right
    - Filter chip row (`gap:8px`, wraps): `account: all accounts ▾` (outline tag), `category: groceries ✕` (**accent** tag — the one active filter, with a clickable ✕ to remove), `payee: any ▾`, `tag: any ▾`, `this year ▾`, `status: all ▾` (all outline tags), then `clear filters` link, a flex spacer, and a right-aligned search input (28px tall, placeholder "/ search payee or memo")
  - **Column header** (`padding:8px 28px`, 10px/800, letter-spacing `.11em`, `#9b9797`, `border-bottom:1px solid #d7d3d3`): status-dot spacer (20px) / DATE (70px) / ACCOUNT (116px) / PAYEE (flex:1) / CATEGORY (92px) / TAGS (86px) / AMOUNT (96px, right) / RUNNING (104px, right)
  - **Rows** (`padding:9px 28px`, tabular-nums):
    - Status glyph: `○` open, `◐` cleared, `●` reconciled, `⚑` flagged (legend lives in the status bar)
    - The **selected/focused row** gets the dark treatment: `background:#201e1d;color:#f3f2f2`, secondary text `#d7d3d3`/`#9b9797` instead of `#605d5d`/`#9b9797`; payee is 800 weight
    - Unselected rows: 1px bottom rule `#eae9e9`, payee 400 weight, account/category in `#605d5d`
    - TAGS column: empty `—` (`#9b9797`) normally, or a `tag tag-neutral` chip (e.g. "shared") when present
    - AMOUNT: signed, right-aligned tabular figure
    - RUNNING: right-aligned tabular figure, accumulating down the visible (filtered) list; 800 weight on the selected row
  - **Footer bar** (`padding:11px 28px`, `border-top:2px solid rgba(32,30,29,.38)`, `background:#eae9e9`): left "8 of 96 groceries transactions shown" (11.5px `#605d5d`); right "RUNNING TOTAL" label (11px letter-spacing `.06em` `#9b9797`) + the grand total figure (800, 20px, tabular-nums, colored `#ae1800` when negative)
- Status bar (28px): `NORMAL` mode chip; legend `j/k row · enter open · e edit · / filter · R reconcile`; right-aligned status-glyph legend `○ open · ◐ cleared · ● reconciled · ⚑ flagged`

**Sample data shown**: 8 rows, all category "groceries", spanning ANZ Everyday / Amex Platinum / ANZ Offset accounts, 12 Sep down to 18 Aug; running total ends at −745.97; footer states "8 of 96" (i.e. more rows exist below the fold / pagination applies).

---

### 4b — Filter builder popover
**Purpose**: Clicking any chip (or pressing `f`) drops one popover with **every** filter dimension at once, so a multi-condition query (e.g. account + tag + date range) doesn't require chip-by-chip dropdown hunting one at a time. Applied filters still render as chips behind it.

**Layout**:
- Shell behind: header/rail full opacity; the transactions content pane is **blurred + dimmed** (`filter:blur(1.5px);opacity:.55`) rather than using the palette's flat dimmer — shows just the view header with its two active chips (`category: groceries`, `this year`) and empty body
- Popover: 400px wide, positioned `top:96px;left:466px` (i.e. anchored near the chip row, not centered like the command palette/dialogs), `background:#f3f2f2;border:1px solid rgba(32,30,29,.30);box-shadow:0 18px 44px rgba(32,30,29,.40)`
  - Title bar: "Filter transactions" (800 13.5px, `border-bottom:2px solid rgba(32,30,29,.38)`, padding `14px 18px`)
  - Body (padding `16px 18px`, gap 14px), all `.field` + `.input` design-system components:
    - **Account** — select (All accounts / ANZ Everyday / ANZ Offset / Amex Platinum)
    - **Category** — select (Groceries selected / Transport / Dining)
    - **Payee** — text input, placeholder "any payee"
    - **Tag** — text input, placeholder "any tag"
    - **From** / **To** — two side-by-side date fields (`gap:10px`, each `flex:1`), values "01 jan 2026" / "today"
    - **Status** — `.seg` control: all / open / cleared / reconciled
  - Footer (`padding:14px 18px;border-top:1px solid #d7d3d3`, right-aligned): `reset` (ghost) + `apply` (primary, trailing kbd hint `enter`)
- Status bar: `FILTER` mode chip, legend `tab next field · enter apply · esc cancel`

---

## Components

| Part | Spec |
| --- | --- |
| Filter chip (outline) | `.tag.tag-outline`, `padding:2px 8px`, cursor pointer, trailing `▾` for chips that open a dropdown |
| Filter chip (active) | `.tag.tag-accent`, `padding:2px 8px`, inline-flex with a `✕` at `opacity:.85` to clear that one filter |
| Clear-filters link | 11.5px, uses the design system's default link color |
| Search input | `.input`, `height:28px;min-height:28px`, 12px, right-aligned in the filter row, placeholder prefixed `/` |
| Column header | `padding:8px 28px`, 10px/800, letter-spacing `.11em`, `#9b9797`, `border-bottom:1px solid #d7d3d3` |
| Table row (default) | `padding:9px 28px`, tabular-nums, `border-bottom:1px solid #eae9e9` |
| Table row (selected) | `background:#201e1d;color:#f3f2f2`; secondary columns step down to `#d7d3d3` (was `#605d5d`) / `#9b9797`; payee 800 weight |
| Status glyph | inline character, 20px column width: `○` `◐` `●` `⚑` |
| Tag chip in TAGS column | `.tag.tag-neutral`, `padding:1px 6px;font-size:10px` |
| Footer bar | `padding:11px 28px`, `border-top:2px solid rgba(32,30,29,.38)`, `background:#eae9e9` |
| Running-total figure | 800, 20px, tabular-nums, `#ae1800` when negative else ink |
| Filter popover | 400px, `border:1px solid rgba(32,30,29,.30)`, `box-shadow:0 18px 44px rgba(32,30,29,.40)`, anchored (not centered) |
| Popover title bar | padding `14px 18px`, `border-bottom:2px solid rgba(32,30,29,.38)`, 800 13.5px |
| Popover footer | padding `14px 18px`, `border-top:1px solid #d7d3d3`, right-aligned, `gap:10px` |
| Background dim (popover variant) | content pane `filter:blur(1.5px);opacity:.55` — distinct from the modal dimmer (`rgba(32,30,29,.30)` flat overlay) used elsewhere |

## Design Tokens
Same palette as the rest of the shell (see Desktop Shell & Navigation README for the full token table): ground `#f3f2f2`, chrome `#eae9e9`, ink `#201e1d`, ink secondary `#605d5d`, ink tertiary `#9b9797`, accent `#ec3013`, accent text-safe `#ae1800`, rule strong `rgba(32,30,29,.38)`, rule medium `#eae9e9`/`#d7d3d3`, border `rgba(32,30,29,.30)`. Family: Archivo throughout, weights 400/800 only, `font-variant-numeric:tabular-nums` on every date/amount/running figure. Radius: 0 everywhere.

## Interactions

### Table (4a)
- `j`/`k` move row focus; `enter` opens the transaction detail; `e` edits inline/opens edit
- `/` focuses the search box; typing filters by payee/memo substring
- Clicking a chip's `▾` opens a scoped single-field dropdown; clicking any chip (or pressing `f`) instead opens the **full filter popover** (4b) with all dimensions
- `✕` on the accent (active) chip clears just that filter; `clear filters` clears all
- `R` opens the reconcile flow for the focused row
- RUNNING TOTAL recomputes top-to-bottom over the currently filtered + sorted (newest-first) set; the footer grand total always equals the last row's running figure

### Filter popover (4b)
- Opens anchored near the triggering chip, not centered — distinct from the app's centered dialogs (Add/Edit/Delete) and the horizontally-centered command palette
- `tab` moves field to field, `enter` applies, `esc` cancels without changing the applied filters
- `reset` clears all fields in the popover (not the same as `clear filters` on the resting chips — reset only affects the open popover's draft state)
- On apply: popover closes, chip row updates to reflect the new filter set, table + running total recompute

## State
```
transactionsView
  filters: { account: Option<AccountId>, category: Option<CategoryId>, payee: Option<String>,
             tag: Option<String>, dateFrom: Date, dateTo: Date, status: All|Open|Cleared|Reconciled }
  searchQuery: String              // payee/memo substring, separate from filters
  rows: Vec<Transaction>           // filtered + sorted newest-first
  selectedId: Option<Id>
  runningTotal: Decimal            // recomputed over `rows` on every filter/search change
filterPopover
  open: bool
  draft: <same shape as filters>   // edited independently; only committed to `filters` on apply
```

## Implementation notes (GPUI)
1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **Two distinct overlay treatments exist in this app** — reuse them consistently: a flat `rgba(32,30,29,.30)` dimmer for modals/palette (see other handoff packages), and a `blur+opacity` dim on the content pane specifically for this anchored filter popover. Don't merge the two.
3. **One filter state, two entry points** — chip dropdowns (single field) and the full popover (all fields) must write to the same underlying filter state so they never drift out of sync.
4. **Tabular numerals** on every date/amount/running figure so columns align.
5. **Zero radius, 2px section rules, 1px row rules** — consistent with the rest of the shell.
6. **Selected-row contrast**: verify the stepped-down secondary text colors (`#d7d3d3`, `#bab6b6`-family) meet contrast against the `#201e1d` row fill in the real theme implementation.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section `#t4`, options 4a–4b)
- `README.md` — this document
