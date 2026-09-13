# Handoff: Shell & Navigation (Personal Ledger desktop)

## Overview

The desktop client for Personal Ledger — a single-user, local-first finance ledger that
already has a TUI client. This handoff covers the **application shell**: window chrome, the
two navigation rails, the command palette, the status line, and the keyboard model binding
them together. View interiors (dashboard widgets, the transaction table, settings panes) are
drawn here for context but specified only far enough that the shell's focus and layout
contract is unambiguous.

The governing constraint: the desktop client and the TUI are **visual siblings**. Both speak
the same vocabulary — mode line, `g`-jumps, `:` palette, typographic status glyphs — so a
user moving between them re-uses their muscle memory. Where this document and the TUI
handoff (`docs/ux/tui/README.md`) disagree, the TUI is the older contract and wins unless
this document calls out the divergence deliberately.

## About the Design Files

The files in this bundle are **design references created in HTML** — prototypes showing
intended look and behaviour, not production code to copy. `Ledger Desktop Shell.dc.html`
opens directly in a browser (no build step); `support.js` is the runtime that renders it and
is **not** application code — do not port it, read it, or treat it as a dependency.

The target environment for this feature is **Rust + gpui** (see ADR-0007), so the task is to
**recreate these designs in gpui using the patterns already established in the repo** — not
to translate HTML or CSS structures literally. Flexbox descriptions below map onto gpui's
`div().flex()` builders; treat the pixel values as the specification and the HTML as the
picture of it.

## Fidelity

**High-fidelity.** Colors, typography, spacing, and row metrics are final and are stated
exactly below. Recreate the chrome pixel-accurately. The two places where fidelity is
deliberately loose:

- **View interiors** — the dashboard's chart shapes, the sample transactions, and the
  settings fields are representative content, not final designs. Match the *frame* (headers,
  column widths, rules), not the sample data.
- **Icons** — the mockup contains hand-drawn placeholder glyphs. Use the named Lucide icons
  in the Assets section instead; do not trace the mockup's SVG paths.

## Screens / Views

The mockup contains four explored options. **Option 1a is the accepted direction** — build
that. Options 1b, 1c, and 1d are retained as provenance and as the spec for two states of
1a (the collapsed rail and the palette), and are described here for that reason only.

---

### 1a — Dashboard, both rails expanded (ACCEPTED)

**Purpose.** The landing screen. The user reads their net position and current-period
budget health, and moves to any noun from here.

**Layout.** Window 1280 × 800. A vertical flex column of three bands:

| Band | Height | Background | Edge |
| --- | --- | --- | --- |
| Top bar | 48px fixed | `#eae9e9` | 2px `rgba(32,30,29,.38)` bottom |
| Content row | `flex: 1`, `min-height: 0` | — | — |
| Status line | 28px fixed | `#eae9e9` | 2px `rgba(32,30,29,.38)` top |

Content row is a horizontal flex of three columns, each separated by a 2px
`rgba(32,30,29,.38)` right border:

| Column | Width | Background |
| --- | --- | --- |
| Primary rail | 206px fixed | `#eae9e9` |
| Context rail | 238px fixed | `#f3f2f2` |
| View area | `flex: 1`, `min-width: 0` | `#f3f2f2` |

View area padding: `20px 24px 0`. It owns its own scrolling; the rails and bands never scroll
with it.

**Components.**

*Top bar* — horizontal flex, `align-items: center`, `gap: 12px`, `padding: 0 10px 0 12px`.
- **Rail toggle**: 28 × 28 hit area, 16px `panel-left` icon, `#201e1d`, stroke 1.5.
- **Brand mark**: "LEDGER" at weight 800, then the open file name at `#605d5d`;
  `gap: 8px`, `align-items: baseline`.
- **Spacer**: `flex: 1`.
- **Palette hint**: `padding: 4px 9px`, 1px `rgba(32,30,29,.30)` border, 12px text at
  `#605d5d`, reading `:` (weight 800, `#201e1d`) + "run a command". Clickable — opens the palette.
- **Sync indicator**: 12px `#605d5d`, `gap: 6px`, `margin-left: 6px`.
- **Window controls**: GNOME-supplied. In the mockup these are drawn as three 26 × 26 cells
  with `rgba(32,30,29,.08)` fills at `gap: 2px` — do not build these, let the compositor draw them.

*Primary rail* — vertical flex, `padding: 14px 0 10px`.
- **Group headings**: `font: 800 10px/1 Archivo`, `letter-spacing: .11em`, `#9b9797`.
  Padding `0 14px 8px` for the first heading, `16px 14px 8px` for subsequent ones.
  Three groups, in order: **LEDGER** (Dashboard, Transactions, Accounts, Reconcile),
  **PLAN** (Budgets, Reports), **RECORDS** (Categories, Payees, Units). Then a 2px
  `rgba(32,30,29,.20)` divider with `margin: 8px 0`, then **Settings** ungrouped.
- **Rail row**: horizontal flex, `align-items: center`, `gap: 9px`, `padding: 7px 14px`.
  14px icon, then the label (`flex: 1`), then the jump key at 11px.
  - *Selected*: `#201e1d` fill, label `#f3f2f2` weight 800, jump key `#bab6b6`.
  - *Unselected*: no fill, label `#201e1d` weight 400, jump key `#9b9797`.
  - *Hover* (unselected only): `rgba(32,30,29,.06)` fill.
  - *Focus-visible*: 2px `#ec3013` outline, 2px offset.
- **Reconcile badge**: `#ec3013` fill, `#f3f2f2` text, `font: 800 10px`, `padding: 1px 5px`,
  `margin-right: 4px`, sits between label and jump key. Reconcile has **no jump key**.
  Suppress the badge entirely at zero — never render "0".
- Jump keys as drawn: `g d`, `g t`, `g a`, (none), `g b`, `g r`, `g c`, `g p`, `g u`, `g s`.

*Context rail* — vertical flex, scoped to the active noun. On Dashboard it shows accounts.
- **Header**: `padding: 14px 14px 8px`, `justify-content: space-between`,
  `align-items: baseline`. Label `font: 800 10px/1 Archivo`, `letter-spacing: .11em`,
  `#201e1d` ("ACCOUNTS"); count 11px `#9b9797` ("7 active"). Followed by a full-bleed 2px
  `rgba(32,30,29,.38)` rule.
- **Entity row**: `padding: 9px 14px`, vertical flex, `gap: 2px`, 1px `#d7d3d3` bottom
  border (omitted on the last row). Line one: name left, balance right at weight 800,
  tabular-nums. Line two: 11px `#9b9797` meta, `·`-joined ("bank · aud", "credit card · aud",
  "investment · vas").
- **Negative balances**: `#ae1800` at weight 800, minus sign U+2212 ("−2,318.44"), never the
  raw accent.
- **Footer**: `flex: 1` spacer above it, then `padding: 10px 14px`, 1px `#d7d3d3` top border,
  11.5px `#605d5d`, reading the affordance then its command in `#201e1d` weight 800 —
  "+ new account · **:account new**". Every context rail has one; it is how the command
  grammar gets taught.

*View area (Dashboard)* — for frame reference only.
- **Header**: `justify-content: space-between`, `align-items: baseline`. Title 19px
  ("Financial position"); meta 11.5px `#9b9797` ("13 september 2026 · all accounts · aud").
- **Figure row**: `gap: 36px`, `align-items: flex-end`, `margin-top: 14px`. Kicker
  (`font: 800 10px/1`, `.11em`, `#9b9797`, `margin-bottom: 5px`) over the value. Net position
  at `font: 800 38px/1`, `letter-spacing: -.02em`, tabular-nums. Secondary figures at
  `gap: 28px`, `padding-bottom: 4px`.
- **Chart band**: net-worth line (`flex: 1`, 124px tall, 2px `#201e1d` polyline, 1px
  `#d7d3d3` baseline, `#ec3013` 3r end dot) beside a 266px in/out bar pair.
- **Lower band**: `gap: 24px`, `padding: 14px 0 0`. 250px donut (88px box, 15px stroke,
  ramp `#ec3013` → `#201e1d` → `#605d5d` → `#9b9797`) beside the budget list (`flex: 1`).
- **Budget track**: 12px tall, `#e2e0df` inset; fill `#605d5d`, or `#ec3013` at 100%+;
  period marker is a 2px `#201e1d` bar at `left: 70%`, `top: -2px`, `bottom: -2px`.
  Amount column 118px, right-aligned, tabular-nums; over-budget amounts `#ae1800` weight 800.

*Status line* — `gap: 14px`, `padding: 0 12px`, 11.5px `#605d5d`.
- **Mode badge**: `#201e1d` fill, `#f3f2f2` text, `font: 800 10px`, `letter-spacing: .1em`,
  `padding: 2px 7px`. In COMMAND mode the fill becomes `#ec3013`.
- **Hint strip**: keys in `#201e1d` weight 800, descriptions in `#605d5d`, `·`-joined:
  "`:` command · `/` search · `a` add txn · `?` help · `b` toggle sidebar".
- **Spacer**, then **breadcrumb**: "ledger · dashboard".

---

### 1b — List–detail rail (REJECTED, reference only)

Flat 178px primary rail with no groups; 250px context rail holding the *records* of the
active noun behind a `/ filter accounts` input (`.input`, 30px tall, 12.5px). Rejected
because it collapses on Transactions, where the record count is unbounded. Two details are
worth keeping for the transaction **view**: the column frame — DATE 84px, PAYEE `flex: 1`,
CATEGORY 120px, AMOUNT 104px right-aligned, header at `font: 800 10px/1`, `.11em`, `#9b9797`
with a 1px `#d7d3d3` rule and rows at `padding: 9px 0` with 1px `#eae9e9` rules — and the
convention that a noun's own actions live in the view header, not the rail.

### 1c — Collapsed rail (this is 1a's `b` state — BUILD THIS)

The collapsed primary rail: **52px** wide, `#eae9e9`, `align-items: center`,
`padding: 12px 0`, `gap: 2px`. Rows are 36 × 34 centred icon boxes, 16px icon, stroke 1.5.
Selected row is a `#201e1d` fill with a `#f3f2f2` icon. Reconcile's badge degrades to a
6 × 6 `#ec3013` square at `top: 2px; right: 1px` of its row. Settings sits after a `flex: 1`
spacer, pinned to the bottom.

**Tooltip.** On hover (500ms delay) the row shows `#201e1d` fill, `#f3f2f2` label at weight
800, binding at `#bab6b6` 11px, `padding: 5px 10px`, `gap: 8px`, `white-space: nowrap`. It
opens **to the right of the rail, vertically centred on its row** — in the HTML,
`position: relative` on the row and `left: calc(100% + 12px); top: 50%;
transform: translateY(-50%)`. It must never clip at the window edge or overlap the context
rail. This was a live bug in an earlier draft of the mockup; do not reintroduce it by
anchoring the tooltip to anything but its own row.

Collapsing preserves focus and selection. The context rail is unaffected (232px in this
option, 238px in 1a — use **238px**).

### 1d — Command palette (this is 1a's `:` state — BUILD THIS)

The shell behind the palette drops to **30% opacity**; it is not blurred and not covered by a
colour wash. The palette is `position: absolute`, `left: 50%`, `top: 96px`,
`transform: translateX(-50%)`, **820px** wide, `#f3f2f2` fill, **2px `#201e1d` border**,
`box-shadow: 0 12px 32px rgba(45,43,43,.30)`. It is the only element in the shell that floats.

- **Input row**: `padding: 12px 16px`, `gap: 10px`. Leading `>` at 15px weight 800, then the
  query at 15px weight 800 `letter-spacing: -.01em`, then an 8 × 17 `#ec3013` block caret.
  Right-aligned match count at 11.5px `#605d5d` tabular-nums ("7 of 62"). Followed by a 2px
  `#201e1d` rule.
- **Result row**: `padding: 8px 16px`. Command string (`flex: 1`) with the matched substring
  in `#ec3013` weight 800; description column 200px at `#605d5d`; binding column 64px
  right-aligned at `#9b9797` 11.5px, or an em dash when the command has no binding.
- **Selected result**: `#201e1d` fill, `#f3f2f2` text, matched substring `#ff9783`,
  description `#d7d3d3`, binding `#bab6b6`. First result is pre-selected.
- Results are ranked across kinds, not grouped — a substring match on `bud` surfaces
  `:budget edit`, `:budget list`, `:report variance`, and `:category set budget…` together.
  Showing related commands the user did not know to ask for is the point.
- **Status line in COMMAND mode**: badge turns `#ec3013`; the live query echoes at `#201e1d`
  weight 800 with a 2 × 13 `#ec3013` caret; right side reads "esc close command window".

## Interactions & Behavior

**Focus zones.** `PrimaryRail | ContextRail | View`. `Tab` cycles forward, `Shift-Tab` back,
skipping zones with no content. The focused zone shows a 2px `#201e1d` inner edge on its
border-facing side. `j`/`k` act **only** on the focused zone — no zone scrolls while another
is focused.

**Movement** (focused zone): `j`/`k` or `Down`/`Up` next/previous; `gg`/`G` first/last;
`Ctrl-d`/`Ctrl-u` half-page; `Enter` activates — on the primary rail that means select the
noun *and* move focus to the view; on the context rail it selects the entity and leaves focus
where it is.

**Jumps** (`g` prefix, global in Normal mode): `g d` Dashboard, `g t` Transactions,
`g a` Accounts, `g b` Budgets, `g r` Reports, `g c` Categories, `g p` Payees, `g u` Units,
`g s` Settings. Reconcile is deliberately unbound — it is a task, not a place; reach it by
rail click or `:reconcile`. `g` + an unbound key is a no-op: clear the pending prefix and
flash the hint strip. Pending-prefix timeout **1000ms**.

**Modes and actions**: `:` palette (COMMAND) · `/` search within the active view (SEARCH) ·
`a` add transaction, opening the entry dialog in INSERT · `b` toggle primary rail ·
`?` help overlay · `Esc` leave mode, dismiss overlay, or clear a pending prefix.

Do not add `Ctrl` chords for anything a bare key already covers — divergence from the TUI
grammar is the failure mode to avoid. GNOME-owned chords (`Ctrl-w`, `Ctrl-q`, `F10`) stay
with the window manager.

**Transitions.** The shell does not animate. Rail collapse is instant, palette open is
instant, selection is instant. The only timed behaviours in the shell are the 500ms tooltip
delay and the 1000ms prefix timeout. Do not add easing to chrome.

**Hover.** Rail rows tint `rgba(32,30,29,.06)`; palette results and context-rail rows tint
the same. Selected items do not respond to hover.

**Loading and error states.** The ledger is local-first — reads are synchronous and need no
spinners. Do not build skeleton states into the shell. A failed write surfaces in the status
line: mode badge stays as-is, the hint strip is replaced by the error at `#ae1800` weight
800, cleared by any keypress. An empty context rail is not an error state — see rule 4 below.

**Responsive behaviour.** Fixed-size desktop window: rails keep their pixel widths at every
size and the view area absorbs all remaining space. Minimum window 960 × 640. Below the
minimum, nothing reflows — the window simply does not shrink further. Do not add breakpoints.

## State Management

```rust
struct NavState {
    noun: Noun,                  // active rail item
    context: ContextSelection,   // active entity within the noun, if any
    focus: FocusZone,
    primary_rail: RailMode,      // Expanded | Collapsed
    mode: InputMode,             // Normal | Insert | Command | Search
}

enum Noun { Dashboard, Transactions, Accounts, Reconcile,
            Budgets, Reports, Categories, Payees, Units, Settings }

enum FocusZone { PrimaryRail, ContextRail, View }
```

**Rules.**

1. `noun` is the single source of truth for the context rail's contents. Changing it resets
   `context` to the noun's first entity (or `None` for nouns with none — Dashboard, Settings)
   and moves focus to `View`.
2. Changing `context` never changes `noun` and never moves focus.
3. `focus` cycles rail → context → view → rail, skipping empty zones.
4. A noun with no entities renders **no context rail at all** — the view area takes the
   space. Never render an empty rail.
5. Collapsing the rail does not change focus or selection.
6. Mode transitions are global and pre-empt zone key handling. `Esc` returns to `Normal` and
   restores the pre-mode focus zone.

**Persistence.** `noun`, `primary_rail`, and window geometry survive restart. `context`,
`focus`, and `mode` do not — every launch starts in Normal mode with focus in the view.

**Data.** The shell itself fetches nothing. It needs three counts from the domain layer to
render: unreconciled transaction count (Reconcile badge), active entity count per noun
(context header), and the last-write timestamp (sync indicator). Everything else belongs to
the views.

**Command registry.** The palette must be driven by a registry, not a hand-written list.
Every rail item, every context-rail footer affordance, and every view action registers a
command with: command string, description, kind (`navigate` / `create` / `run` / `setting`),
optional binding, and a handler. The palette is authoritative — if a binding and the registry
disagree, the registry wins.

## Design Tokens

From the Modernist design system. Introduce no values outside this set in shell code.

| Role | Value |
| --- | --- |
| Ground | `#f3f2f2` |
| Chrome (bars, primary rail) | `#eae9e9` |
| Inset track | `#e2e0df` |
| Ink | `#201e1d` |
| Ink secondary | `#605d5d` |
| Ink tertiary / labels | `#9b9797` |
| Ink on dark (inverted rows) | `#f3f2f2`; secondary `#bab6b6`, tertiary `#d7d3d3` |
| Hairline (rows within a rail) | `#d7d3d3`; lighter table rule `#eae9e9` |
| Structural rule (2px) | `rgba(32,30,29,.38)`; in-rail divider `rgba(32,30,29,.20)` |
| Hover tint | `rgba(32,30,29,.06)` |
| Accent | `#ec3013` |
| Accent on dark (matched substring) | `#ff9783` |
| Accent text on ground | `#ae1800` |

**Type.** Archivo throughout — 400, 600, 800. Sizes: 38px/800 hero figure, 19px view title,
15px/800 palette query, 13px body, 12.5px inputs, 11.5px meta, 11px jump keys and row meta,
10px/800 `letter-spacing: .11em` section kickers. Every amount carries
`font-variant-numeric: tabular-nums`. Minus sign is U+2212, not a hyphen.

**Spacing.** Rail rows `7px 14px`; context-rail rows `9px 14px`; palette rows `8px 16px`;
view padding `20px 24px 0`; band gaps 14px (status), 12px (top bar); figure gaps 36px/28px;
section gaps 24px/40px.

**Radius.** `0` everywhere. No exceptions — this is a system rule, not a preference.

**Shadows.** Exactly one in the shell: the palette's `0 12px 32px rgba(45,43,43,.30)`. The
collapsed-rail tooltip carries `0 3px 10px rgba(45,43,43,.25)`. Nothing else elevates.

**Accent discipline.** Accent is for the primary action, the Reconcile badge, over-budget
state, the block caret, matched substrings, the focus ring, and the COMMAND mode badge. It is
never a background field in the shell. `#ec3013` on `#f3f2f2` does not clear 4.5:1 at body
size — use `#ae1800` for accent-coloured text at 13px and below.

## Assets

No image assets. Icons are **Lucide** (https://lucide.dev) — use the repo's existing icon
pipeline, not the mockup's hand-drawn SVG paths. 14px in rails, 16px in the top bar,
stroke 1.5, `currentColor`, no fill.

| Element | Lucide name |
| --- | --- |
| Dashboard | `layout-dashboard` |
| Transactions | `align-justify` |
| Accounts | `wallet` |
| Reconcile | `check-check` |
| Budgets | `gauge` |
| Reports | `trending-up` |
| Categories | `tag` |
| Payees | `circle-user` |
| Units | `circle-dollar-sign` |
| Settings | `settings` |
| Rail toggle | `panel-left` |
| Palette / search | `search` |
| Add transaction | `plus` |
| Flagged status | `flag` |

**Transaction status glyphs are not icons.** `○` pending, `◐` cleared, `●` reconciled,
`⚑` flagged — typographic marks, as in the TUI. They are data, and must read identically in
both clients.

**Fonts.** Archivo. The mockup loads it from Google Fonts for portability; in the app, bundle
it rather than fetching at runtime — the client is local-first and must render offline.

## Files

In this bundle:

- `README.md` — this document.
- `Ledger Desktop Shell.dc.html` — the mockup, all four options. Open in a browser; no build
  step. Option ids `1a`–`1d` are anchors, so `#1a` scrolls to the accepted direction.
- `support.js` — the mockup's rendering runtime. **Not application code.**

In the repository:

- `docs/ux/desktop/README.md` — the gpui-facing spec (component tree, state machine,
  keybindings, acceptance criteria). The same decisions as this document, written for the
  repo rather than for a handoff bundle.
- `docs/ux/tui/README.md` — the sibling client's handoff. Read it before implementing the
  keyboard model; the grammar originates there.
- ADR-0007 (gpui), ADR-0013 (Shell/View navigation), `docs/product-requirements.md`
  FR.34–38, `CONTEXT.md` (domain glossary).

## Acceptance criteria

1. Every noun is reachable by `g`-jump, rail click, and palette, and all three land in the
   same state.
2. `Tab` visits exactly the zones that have content, in rail → context → view order, and the
   focused zone is unambiguous without hovering.
3. `j`/`k` act only on the focused zone; no zone scrolls while another is focused.
4. `b` preserves focus and selection, and the collapsed-rail tooltip opens to the right,
   vertically centred, without clipping or overlapping the context rail at any window size
   ≥ minimum.
5. `Esc` from any mode or overlay returns to Normal with the pre-mode focus restored.
6. Nouns without entities render no context rail; the view area reflows to fill.
7. Restart restores noun, rail mode, and window geometry — and nothing else.
8. Every amount is tabular-aligned, right-aligned in its column, and uses U+2212 for negatives.
9. No rounded corners, no shadows outside the palette and tooltip, no unthemed focus ring,
   no animated chrome.
10. The palette is registry-driven: adding a view action makes it appear in the palette with
    no palette-side code change.
