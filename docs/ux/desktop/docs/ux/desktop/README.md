# Desktop UX — Shell & Navigation

Implementation handoff for the gpui desktop client. Scope of this document is the
**shell**: window chrome, the two navigation rails, the command palette entry point, the
status line, and the keyboard model that binds them. View interiors (dashboard widgets,
transaction table, settings panes) are specified separately — this document defines the
frame they mount into and the focus contract they must honour.

Companion documents: [`../tui/README.md`](../tui/README.md) (sibling client, shared
vocabulary), ADR-0007 (gpui), ADR-0013 (Shell/View navigation).
Visual reference: [`Ledger Desktop Shell.dc.html`](./Ledger%20Desktop%20Shell.dc.html) in this
folder, option **1a** — open it in a browser, no build step. All four explored options are
kept in that file as provenance for §1; `support.js` is its runtime, not application code,
and `styles.css` is the Modernist token sheet (its `:root` block is the authoritative source
for §8).

---

## 1. Decision

Option **1a — grouped primary rail + context rail** is the accepted direction.

- The primary rail groups nouns under three headings and carries its `g`-jump key in the
  right column of each row, so the binding is discoverable without opening help.
- The context rail is scoped to the active noun and holds *entities*, not records — the
  account list, the budget list, the report list. It is the desktop equivalent of the TUI's
  second pane.
- The command palette (`:`) is the escape hatch for everything not on a rail. The rails are
  a convenience over the palette, never a replacement for it.

Rejected and why, for the record: 1b put records in the context rail, which breaks down on
Transactions (thousands of rows); 1c's icon-only rail loses the `g`-jump affordance that
makes the keyboard model teachable; 1d dropped persistent navigation entirely.

---

## 2. Window

| Property | Value |
| --- | --- |
| Default size | 1280 × 800 logical px |
| Minimum size | 960 × 640 |
| Decoration | GNOME client-side, server-drawn controls at trailing edge |
| Theme | Light only for v1 (dark deferred; do not branch tokens yet) |
| Base type | Archivo, 13px body, `font-variant-numeric: tabular-nums` on every amount |

Vertical bands, top to bottom: **top bar 48px**, content row (`flex: 1`), **status line 28px**.
Both bands are `#eae9e9` with a 2px `rgba(32,30,29,.38)` rule on their content-facing edge.
Nothing in the shell has a corner radius and nothing casts a shadow except the palette.

Content row, left to right: primary rail **206px**, context rail **238px** (present only on
nouns that own entities), view area (`flex: 1`, `min-width: 0`, `padding: 20px 24px 0`).
Rails are separated from their neighbour by the same 2px rule. The dashboard has no
entities, so it runs primary rail + view only — see §4 rule 4.

---

## 3. Component tree

```
LedgerWindow                     root, owns AppState, global key dispatch
├── TopBar                       48px
│   ├── RailToggle               icon button, 28×28, toggles primary rail
│   ├── BrandMark                "LEDGER" + active file name
│   ├── PaletteHint              ": run a command", click opens palette
│   ├── SyncIndicator            last-write time / dirty state
│   └── WindowControls           GNOME-supplied
├── ContentRow
│   ├── PrimaryRail              206px, collapsible
│   │   ├── RailGroup "LEDGER"   Dashboard · Transactions · Accounts
│   │   ├── RailGroup "PLAN"     Budgets · Reports
│   │   ├── RailGroup "RECORDS"  Categories · Payees
│   │   ├── RailDivider          2px
│   │   └── RailItem Settings
│   ├── ContextRail              238px, omitted entirely when the noun has no entities
│   │   ├── ContextHeader        noun label + count ("ACCOUNTS", "7 active")
│   │   ├── ContextList          entity rows, 1px #d7d3d3 separators
│   │   └── ContextFooter        affordance + its command ("+ new account · :account new")
│   └── ViewHost                 mounts the active View
└── StatusLine                   28px
    ├── ModeBadge                NORMAL / INSERT / COMMAND / SEARCH
    ├── HintStrip                context-sensitive binding hints
    └── Breadcrumb               "ledger · dashboard"

CommandPalette                   overlay, sibling of ContentRow, z above all
```

`RailItem` props: `icon`, `label`, `jump_key: Option<char>`, `badge: Option<Badge>`,
`selected: bool`. The **Accounts** item carries an accent badge (`#ec3013` fill, `#f3f2f2`
text, 10px/800, `padding: 1px 5px`) showing the unreconciled count aggregated across
accounts; suppress the badge at zero rather than rendering `0`.

Selected `RailItem` is an inverted block — `#201e1d` fill, `#f3f2f2` label at weight 800,
jump key at `#bab6b6`. Unselected: transparent fill, `#201e1d` label at weight 400, jump
key at `#9b9797`. Hover on unselected is `rgba(32,30,29,.06)`. Row metrics: `padding:
7px 14px`, `gap: 9px`, 14px icon.

Group headings: `font: 800 10px/1 Archivo`, `letter-spacing: .11em`, `#9b9797`,
`padding: 0 14px 8px` (first) / `16px 14px 8px` (subsequent).

---

## 4. Navigation state

```rust
struct NavState {
    noun: Noun,                  // which rail item is active
    context: ContextSelection,   // which entity within the noun, if any
    focus: FocusZone,            // which zone owns j/k
    primary_rail: RailMode,      // Expanded | Collapsed
    mode: InputMode,             // Normal | Insert | Command | Search
}

enum Noun { Dashboard, Transactions, Accounts,
            Budgets, Reports, Categories, Payees, Settings }

enum FocusZone { PrimaryRail, ContextRail, View }
```

**Rules.**

1. `noun` is the single source of truth for what the context rail contains. Changing the
   noun resets `context` to the noun's first entity (or `None` where the noun has no
   entities — Dashboard, Settings) and moves focus to `View`.
2. Changing `context` never changes `noun` and never moves focus.
3. `focus` cycles `PrimaryRail → ContextRail → View → PrimaryRail` on `Tab`; the reverse on
   `Shift-Tab`. Zones with no content are skipped.
4. A noun with no entities renders no context rail at all — the view area takes the space.
   Do not render an empty rail.
5. Collapsing the primary rail (`b`) does not change focus. When collapsed the rail is
   48px, icon-only, tooltip on hover after 500ms — tooltips open to the **right** of the
   rail, vertically centred on their row, never clipped by the window edge.
6. `mode` transitions are global and pre-empt zone key handling. `Esc` from any mode
   returns to `Normal` and restores the pre-mode focus zone.

**Units are not a noun.** Unit definitions (currencies, share units, crypto) are
configuration, not records the user browses — they live in Settings under "Ledger & units",
as drawn in 1c. Nothing in the rail points at them; `:unit new` and `:unit list` still work
from the palette.

State persists across restarts: `noun`, `primary_rail`, and window geometry. `context`,
`focus`, and `mode` do not.

---

## 5. Keyboard model

The desktop client is keyboard-first and shares the TUI's grammar. Every binding below is
also reachable from the palette; the palette is authoritative when the two disagree.

**Movement** (acts on the focused zone)

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous item |
| `Down` / `Up` | Same as `j` / `k` |
| `gg` / `G` | First / last item |
| `Ctrl-d` / `Ctrl-u` | Half-page down / up |
| `Enter` | Activate — rail: select noun and move focus to view; context: select entity |
| `Tab` / `Shift-Tab` | Next / previous focus zone |

**Jumps** (`g` prefix, global, mode-independent in `Normal`)

`g d` Dashboard · `g t` Transactions · `g a` Accounts · `g b` Budgets · `g r` Reports ·
`g c` Categories · `g p` Payees · `g s` Settings

Reconciling is **not** a noun and has no rail item of its own: it is an action on an
account, reached from the account view header or by `:reconcile` on the selected account.
The Accounts badge is the standing nudge that work is waiting. `g` followed by an unbound key is a no-op that clears the pending prefix and
flashes the hint strip. Pending-prefix timeout: 1000ms.

**Modes and actions**

| Key | Action |
| --- | --- |
| `:` | Command palette, command mode |
| `/` | Search within the active view |
| `a` | Add transaction (opens the entry dialog in `Insert`) |
| `b` | Toggle primary rail |
| `?` | Keybinding help overlay |
| `Esc` | Leave mode / dismiss overlay / clear pending prefix |

Do not bind `Ctrl` chords for anything a bare key already covers — the TUI's grammar is the
contract, and divergence between clients is the failure mode to avoid. GNOME-owned chords
(`Ctrl-w`, `Ctrl-q`, `F10`) stay with the window manager.

**Focus ring.** `:focus-visible` equivalent on every interactive element: 2px `#ec3013`
outline, 2px offset. Never suppress it. The focused *zone* additionally shows a 2px
`#201e1d` inner edge on its border-facing side.

---

## 6. Command palette

Opens on `:` or on `PaletteHint` click. Overlay, **820px** wide, pinned 96px from the top,
horizontally centred, 2px `#201e1d` border, `box-shadow: 0 12px 32px rgba(45,43,43,.30)`.
The shell behind it drops to **30% opacity** — no blur, no colour wash. The one place in the
shell that floats.

- Input row (`padding: 12px 16px`) carries a leading `>` at 15px/800, the query at 15px/800,
  an 8×17 `#ec3013` block caret, and a right-aligned match count ("7 of 62") at 11.5px
  `#605d5d`. A 2px `#201e1d` rule closes the row.
- Results (`padding: 8px 16px`) are ranked across kinds, not grouped — matching `bud` surfaces
  `:budget edit`, `:report variance` and `:category set budget…` together. First result is
  pre-selected; `j`/`k` move, `Enter` runs, `Esc` closes.
- Each row is command string (`flex: 1`, matched substring `#ec3013`/800) + 200px description
  `#605d5d` + 64px right-aligned binding `#9b9797`, em dash when unbound. Selected row inverts
  to `#201e1d` with the substring in `#ff9783`. Showing the grammar is how users learn it.
- Commands come from the action registry, not a hand-written list. Every rail item, every
  context-rail footer affordance, and every view action registers there.

---

## 7. Icons

Lucide, 14px in rails, 16px in the top bar, `stroke-width: 1.5`, `currentColor`, no fill.
Replace the hand-drawn glyphs in the mockup with these exact names:

| Element | Lucide name |
| --- | --- |
| Dashboard | `layout-dashboard` |
| Transactions | `align-justify` |
| Accounts | `wallet` |
| Reconcile action (view header) | `check-check` |
| Budgets | `gauge` |
| Reports | `trending-up` |
| Categories | `tag` |
| Payees | `circle-user` |
| Units (settings pane) | `circle-dollar-sign` |
| Settings | `settings` |
| Rail toggle | `panel-left` |
| Palette / search | `search` |
| Add transaction | `plus` |
| Flagged status | `flag` |

Transaction status glyphs stay as the TUI's typographic marks — `○` pending, `◐` cleared,
`●` reconciled, `⚑` flagged — not icons. They are data, and they must read identically in
both clients.

---

## 8. Tokens

From the Modernist system. Do not introduce values outside this set in shell code.

| Role | Value |
| --- | --- |
| Ground | `#f3f2f2` |
| Chrome (bars, primary rail) | `#eae9e9` |
| Ink | `#201e1d` |
| Ink secondary | `#605d5d` |
| Ink tertiary / labels | `#9b9797` |
| Hairline (within-rail rows) | `#d7d3d3` |
| Structural rule (2px) | `rgba(32,30,29,.38)` |
| Accent | `#ec3013` |
| Accent text on ground | `#ae1800` (negative amounts, over-budget) |
| Inset track | `#e2e0df` |

Accent is for the primary action, the unreconciled badge, over-budget state, and the focus
ring. It is never a background field in the shell. Negative amounts use `#ae1800` at weight
800, never the raw accent — `#ec3013` on `#f3f2f2` does not clear 4.5:1 at body size.

---

## 9. Acceptance criteria

1. Every noun is reachable by `g`-jump, rail click, and palette, and all three land in the
   same state.
2. `Tab` visits exactly the zones that have content, in rail → context → view order, and
   the focused zone is visually unambiguous without hovering.
3. `j`/`k` act only on the focused zone; no zone scrolls while another is focused.
4. Collapsing the rail with `b` preserves focus and selection, and tooltips in the collapsed
   state open to the right without clipping at any window size ≥ minimum.
5. `Esc` from any mode or overlay returns to `Normal` with the pre-mode focus restored.
6. Nouns without entities render no context rail; the view area reflows to fill.
7. Restarting restores noun, rail mode, and window geometry — and nothing else.
8. Every amount in the shell is tabular-aligned and right-aligned within its column.
9. No rounded corners, no shadows outside the palette, no unthemed focus ring.
