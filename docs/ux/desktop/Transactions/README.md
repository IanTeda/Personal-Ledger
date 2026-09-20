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

## Acceptance pass

The closing walk of the [Desktop Transactions Surface](https://github.com/IanTeda/Personal-Ledger/issues/199) map: every 4a element and the 4b popover checked against this document, now that both have landed against `crates/bins/bin-desktop/src/view/transactions/`, `transaction_rows.rs`, `transaction_chips.rs`, `transaction_filter_form.rs` and `transaction_query.rs`, the same closing walk the Settings and Accounts bundles made. All data is stubbed and in-memory, as the map's Destination says.

- **4a: header.** Met. Title (26px, 800) with the count line beside it, the `add transaction` button (32px, trailing `n` hint at 75% opacity), then the six-chip row, `clear filters` (shown only once something differs from the defaults), a spacer and the 240px search box, all inside the `16px 28px 14px` block with the 2px rule under it. Outline chips carry `▾`; a chip that differs from its default is an accent chip with a `✕` that clears just that filter. The count line is computed (`700 transactions across 4 accounts`, the accounts that hold any), not the mockup's `1,248`.
- **4a: table.** Met. Column header (10px, 800, `#9b9797`, 1px rule) over the virtualised rows: status, flag, DATE, ACCOUNT, PAYEE (the flexible column, with a floor), CATEGORY, TAGS, AMOUNT, RUNNING. The selected row is the dark treatment (ink fill, payee and RUNNING at 800, secondary text stepped down); unselected rows take the 1px `#eae9e9` rule and the hover tint. TAGS shows `—` or a neutral chip with `+N` for more; a multi-Split row shows `split · N`; negatives are `#ae1800` (`#ff9783` on the dark row). `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` keep the selection in view.
- **4a: footer.** Met. `N of M transactions shown` on the left, `RUNNING TOTAL`, the figure (800, 20px, `#ae1800` when negative) and its Unit on the right, on the `#eae9e9` bar with the 2px rule. Mixed Units show `mixed units` instead of a figure. The footer total equals the last row's RUNNING figure.
- **4a: status bar.** Met, with departures below: `NORMAL` badge, the key legend, and the right-aligned glyph legend in the chosen glyph style.
- **4b: popover.** Met. 400px, anchored under the chip row rather than centred, with the title bar, the seven fields in the bundle's order (Account, Category, Payee, Tag, From / To side by side, Status as a segmented control) and the `reset` / `apply` footer. It edits a draft apart from the applied filters, so `Esc` cancels without changing them; `Enter` applies and the chips, table and running total recompute; `Tab` steps fields; the status bar switches to the `FILTER` badge and its legend.
- **One filter state, two entry points.** Met. Chip click, `▾`, and `f` all open the same popover (focused on the clicked field), and `✕` / `clear filters` write the same `TransactionFilters` the popover applies, so they cannot drift.
- **Selected-row contrast.** Met, checked numerically (WCAG ratios against the `#201e1d` row): primary `#f3f2f2` 14.9:1, secondary `#d7d3d3` 11.2:1, tertiary `#9b9797` 5.8:1, negatives `#ff9783` 7.9:1. On the ground, `#ae1800` negatives are 6.4:1 and `#605d5d` secondary 5.8:1. `#9b9797` on the ground (2.6:1) is the mockup's own tertiary ink for the column headers, count line and empty `—`; kept as designed and not used for anything the reader must act on.
- **Accounts hand-off.** `enter` or a click on an Accounts row opens Transactions filtered to that account (the account chip goes accent; the range stays this year), filling the "open ledger" stub the Accounts bundle left.
- **Gaps this pass found and fixed.** The Transactions status-bar legend read `/ filter`, but here `/` is search and `f` opens the filter popover; it now reads `j/k row · enter open · e edit · n add · / search · f filter`. A misplaced doc comment in `shell.rs` (the filter-popover legend's text had been spliced into the Transactions legend's) was also put right.

### Deliberate departures from the mockup

Each was decided on the map, with Ian, and recorded on its ticket:

- **Status and Flagged are independent.** The 20px status column shows `○` open / `◐` cleared / `●` reconciled, and `⚑` sits in a separate narrow column after it, so a flagged reconciled row reads `● ⚑`. The mockup's single four-state glyph column is dropped, since the glossary makes Flagged a separate flag. The status filter stays all / open / cleared / reconciled; a flagged-only filter is not designed.
- **`n` adds a transaction, not `a`** (the shell's `a` is insert mode). `f` opens the filter popover and `/` is search, so the legend's `/ filter` and `R reconcile` are replaced, and the button's kbd hint is `n`.
- **No reconcile.** `R` is an Accounts action, not a Transactions one, and is dropped from the legend and the interactions.
- **Mixed Units blank RUNNING.** The running column and footer total show only when every visible row shares one Unit (the footer then names it); otherwise the column is blank and the footer says `mixed units`. No second-Unit account is seeded, so this is covered by unit tests, not live.
- **The popover dims the page only.** gpui 0.2 has no blur, so `blur(1.5px) + opacity .55` becomes the `.55` opacity alone; the rail and header stay at full opacity.
- **One entry point, the popover.** A chip's `▾` opens the full popover focused on that field, not the mockup's separate single-field dropdown (`Interactions`, first bullet).
- **Splits are modelled.** One row per Transaction; Category, Payee and Tag filters match at Split level; a multi-Split row summarises (`split · 2`, `+N`); AMOUNT and RUNNING show the matching Splits' sum under a Split-level filter. Search matches the description and every Split's Payee at Transaction level.
- **Default range is this year**, shown as an outline `this year` chip; From / To take `today` and the chosen Date format, with ISO always accepted.
- **Display preferences are honoured**: date format, decimal separator, row density (26 / 34 / 42px) and status glyphs (unicode or ascii).
- **Column widths differ slightly** (DATE 92px not 70, CATEGORY 100px not 92, TAGS 96px not 86) so the longest compact date and a tag chip fit without truncating.
- **The footer reads `N of M transactions shown`**, without the mockup's `groceries` noun, since the filter is not always a category.
- **No context rail**, as for Accounts. Sorting is newest-first only; the bundle specifies no other.
- **`enter` (detail), `e` (edit) and add are "not yet built" stubs**; the bundle designs none of them.

### Verified live vs by code review

The resting page, the popover and the search were confirmed in a real window, driven by keystrokes injected through `gpui::Window::dispatch_keystroke` (a temporary, env-driven diagnostic, removed before each commit) and screenshotted: the 4a table against the mockup (dark selected row after `j j`, hover tint, tags chips, `split · N` cells, footer figure and legend); `f` opening the popover anchored under the chip row with the page dimmed and the `FILTER` badge; `Esc` closing it without changing anything; and `/` switching to `SEARCH` with the count and running total narrowing as text was typed. The Accounts `enter` hand-off was confirmed the same way. **Mouse clicks and hover** on chips, `✕`, rows and popover buttons are verified by code review and test coverage only (no synthetic mouse path here). **Flagged rows** and **mixed Units** are covered by unit tests (`transaction_rows`, `transaction_query`, `transaction_chips`), not seen in a screenshot. The pure logic (filter engine, Split rules, running total, chips, the popover's draft and date parsing, formatting) is covered by `cargo test -p bin_desktop`.

### Known gaps

- `gpui` 0.2 has no letter-spacing or `tabular-nums` hook, so the header's `.11em` tracking and the figures' tabular alignment are not reproduced (right-aligned, with Archivo's own digit metrics).
- The `◐` and `●` glyphs are not in Archivo and render from a fallback font, so they sit smaller than `○` and `⚑`.
- The popover's Status segmented control gives its last segment (`reconciled`) the leftover width.
- No blur behind the popover (above).
- The flagged mark can be seen but not set: no key or row action flags or unflags a transaction, and there is no flagged-state filter, though FR.18 lists one. Palette commands for transactions and search matching a Payee's former names are also not built.
- Data is stubbed: 700 seeded transactions over four of the seven accounts, with no `lib-database` wiring.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section `#t4`, options 4a–4b)
- `README.md` — this document
