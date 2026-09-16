# Handoff: Desktop Shell & Navigation

## Overview
This package contains the design for **Personal Ledger's application shell** — the window chrome, the primary navigation rail, the secondary (contextual) rail, the command palette, and the no-ledger-open empty state. The shell explores a **hybrid model**: a conventional graphical rail for discovery, paired with vim-style keybindings (`g`-jumps, `:` command palette, `/` filter) for speed. Four variants (1a–1d) present different answers to the same question — how much structure the rails should carry.

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** showing intended look, layout, and interaction model. It is a **design reference, not production code**. Recreate these designs in the target codebase (Rust + GPUI) using established patterns, informed by the mockup's layout, typography, color, and behavior.

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states. Recreate faithfully; translate HTML structure into GPUI idioms.

## Frame
All four variants are drawn at **1280 × 800** — the reference desktop window size. The shell is a vertical stack: header (48px) / body (flex:1) / status bar (28px).

---

## Screens

### 1a — Grouped primary rail, no ledger open
**Purpose**: The shell's structural baseline, shown in its **cold-start state**. The primary rail groups the domain nouns under headings and carries each item's `g`-jump binding in a right-hand column. No second rail: the landing view has no entity to scope to.

**Layout**:
- **Header** (48px): app glyph, "Personal Ledger" wordmark, `:` run-a-command affordance, sync indicator, three window controls. No file/unit label — there is no ledger loaded.
- **Primary rail** (206px): grouped nav
  - `LEDGER` — Dashboard `g d`, Transactions `g l`, Accounts `g a` (+ red count badge), Categories `g c`, Payees `g p`, Tags `g t`
  - `PLAN` — Bills `g w`, Budgets `g b`, Reports `g r`
  - Spacer, 2px rule, then Settings `g s` pinned to the bottom
- **Main pane** (flex:1): the empty state — vertically and horizontally centered
  - Title: "No ledger open" (800 26px/1.1, letter-spacing −.01em)
  - Body: "Run `:open` to load a ledger file, or `:new` to start one." (13px, #605d5d, max-width 380px, `text-wrap: pretty`) — the two command names are set 800 weight in #201e1d
  - Stack gap: 14px, `text-align: center`, pane padding 24px
- **Status bar** (28px): `NORMAL` mode chip, binding legend, right-aligned context label

**Why it matters**: The rail stays fully populated and legible with no ledger loaded — navigation is app structure, not document content. Only the main pane and the header's file label are empty.

### 1b — Flat primary rail + records rail (list–detail)
**Purpose**: List–detail. The second rail holds the **records of the current noun**, so an account's ledger is one selection away. Rail one is flat (no group headings); the noun's own actions sit in the view header, not the rail.

**Layout**:
- Header breadcrumb: `accounts › ANZ Everyday`
- Primary rail (206px): flat list, Dashboard active
- Secondary rail: filterable account list — `/ filter accounts` box at top, account rows with name + balance, selected row in the dark treatment
- Main pane: the selected account's transaction ledger

**Alignment note**: the rule under the `/ filter accounts` box lines up with the rule under the view header — the two rails share one horizontal datum.

### 1c — Collapsed icon rail + section index
**Purpose**: Rail one collapsed to icons (`b` toggles; hover reveals label + binding). The second rail becomes the screen's own table of contents — here, the settings index.

**Layout**:
- Primary rail: icon-only, ~48px wide
- Secondary rail (214px): filter box, `SETTINGS` label, then the settings pages — General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing (Logs), About
- Main pane: the selected settings pane (Display shown)

### 1d — Command palette
**Purpose**: The keyboard half of the hybrid. `:` floats the palette over the dimmed shell; one command registry drives results, bindings, and argument help.

**Layout**:
- Dimmer: `rgba(32,30,29,.30)` over the whole shell, z-index 10
- Palette: 820px wide, centered horizontally, `top: 96px`
  - Input row: `>` prompt, typed query (`bud`), 8px × 17px `#ec3013` block cursor, result count "7 of 62" right-aligned
  - 2px `#201e1d` rule
  - Result rows: selected row in dark treatment (description `#d7d3d3`, binding `#bab6b6`); unselected (description `#605d5d`, binding `#9b9797`)
  - Argument help row: current arg value, limit, actual, variance
  - Footer hint: `↑↓ select · tab complete · enter run · ^r history · esc close`
- Palette chrome: `background #f3f2f2; border: 2px solid #201e1d; box-shadow: 0 12px 32px rgba(45,43,43,.30)`

---

## Components

| Part | Spec |
| --- | --- |
| Header | 48px, `background #eae9e9`, `border-bottom: 2px solid rgba(32,30,29,.38)`, padding `0 10px 0 12px`, gap 12px |
| Wordmark | 800 13.5px, letter-spacing −.01em |
| Breadcrumb | 12px, `#9b9797` |
| Command affordance | `padding: 4px 9px; border: 1px solid rgba(32,30,29,.30)`, `:` in 800 #201e1d |
| Window controls | three 26px squares, `background rgba(32,30,29,.08)`, gap 2px |
| Primary rail | 206px (48px collapsed), `background #eae9e9`, `border-right: 2px solid rgba(32,30,29,.38)`, padding `14px 0 10px` |
| Group heading | `font: 800 10px/1 'Archivo'; letter-spacing: .11em; color: #9b9797; padding: 0 14px 8px` (16px top for later groups) |
| Nav item | `display:flex; align-items:center; gap:9px; padding:7px 14px`, icon 14×14, label `flex:1`, binding 11px `#9b9797` |
| Nav item (active) | `background:#201e1d; color:#f3f2f2`, label 800, binding `#bab6b6`, icon stroke `#f3f2f2` |
| Count badge | `background:#ec3013; color:#f3f2f2; font:800 10px; padding:1px 5px; margin-right:4px` |
| Rail footer rule | `height:2px; background:rgba(32,30,29,.20); margin:8px 0` |
| Secondary rail | 214–250px, `border-right: 2px solid rgba(32,30,29,.38)` |
| Filter box | `height:30px; font-size:12.5px; padding:8px 10px; border:1px solid rgba(32,30,29,.30)` |
| Status bar | 28px, `background #eae9e9`, `border-top: 2px solid rgba(32,30,29,.38)`, 11.5px `#605d5d`, gap 14px |
| Mode chip | `background:#201e1d; color:#f3f2f2; font:800 10px; letter-spacing:.1em; padding:2px 7px` |
| Section rule | `height:2px; background:rgba(32,30,29,.38)` — always `flex:none` inside a flex column |
| Row rule | `height:1px; background:#d7d3d3` |

## Design Tokens

### Color
| Role | Value |
| --- | --- |
| Ground | `#f3f2f2` |
| Chrome / rail | `#eae9e9` |
| Canvas (outside the window) | `#e2e0df` |
| Ink | `#201e1d` |
| Ink secondary | `#605d5d` |
| Ink tertiary | `#9b9797` |
| Ink on dark | `#f3f2f2` / `#bab6b6` (dimmed) |
| Accent | `#ec3013` |
| Accent (text-safe) | `#ae1800` |
| Rule (strong) | `rgba(32,30,29,.38)` |
| Rule (medium) | `#d7d3d3` |
| Border | `rgba(32,30,29,.30)` |
| Dimmer | `rgba(32,30,29,.30)` |

### Type
- Family: **Archivo** throughout (headings and body); monospace only for log output and inline command tokens
- Weights: 400, 800 — nothing between
- Sizes: 10px (labels), 11px (bindings), 11.5px (status/meta), 12px (breadcrumb), 13px (body), 13.5px (wordmark), 15px (palette input), 19–20px (view headings), 26px (empty-state title)
- Letter-spacing: `.11em` (all-caps labels), `−.01em` (display sizes), `.1em` (mode chip)
- Numbers: `font-variant-numeric: tabular-nums` on every figure

### Spacing
7, 8, 9, 10, 12, 14, 16, 20, 24, 28px. Rail items 7px vertical / 14px horizontal; pane padding 20–24px.

### Radius & elevation
- **Radius 0 everywhere.** No rounded corners.
- Palette shadow: `0 12px 32px rgba(45,43,43,.30)`
- Window card shadow (mockup only): `0 14px 40px rgba(32,30,29,.30)`

---

## Interactions

### Pointer
- Nav item click → activate that section; the clicked item takes the dark treatment
- Secondary rail row click → select that record / scroll to that section
- Filter box → live substring filter over the rail below it
- Hover on collapsed icon rail → reveal label + binding

### Keyboard
| Key | Action |
| --- | --- |
| `j` / `k` | move selection down / up |
| `g` + letter | jump to section — `d` Dashboard, `l` Transactions, `a` Accounts, `c` Categories, `p` Payees, `t` Tags, `w` Bills, `b` Budgets, `r` Reports, `s` Settings |
| `b` | toggle the primary rail between full and icon-only |
| `:` | open the command palette |
| `/` | focus the contextual filter |
| `a` | add transaction |
| `?` | help |
| `esc` | close palette / clear filter |

### Command palette
- `:` opens; typing filters the registry (fuzzy, substring-weighted)
- `↑` `↓` move selection, `tab` completes, `enter` runs, `^r` history
- Argument help updates per selected command — shows current value, limit, actual, variance
- One registry is the single source of truth for palette entries, rail bindings, and the status-bar legend

### Empty state
- The no-ledger state is reached on cold start and after `:close`
- Only the main pane and the header's file label empty out — rail and status bar stay fully rendered
- `:open` and `:new` are the only two paths forward; both are named in the body copy

---

## State

```
shell
  ledgerOpen: bool            // false → 1a empty state
  activeSection: Section      // Dashboard | Transactions | Accounts | …
  railCollapsed: bool         // b toggles
  secondaryRail: none | records | index
  selectedRecord: Option<Id>
  filterQuery: String
  mode: Normal | Insert | Command
palette
  open: bool
  query: String
  results: Vec<Command>
  selectedIndex: usize
  history: Vec<String>
```

---

## Implementation notes (GPUI)

1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **Rails are chrome, not content.** They render identically whether or not a ledger is open — only the main pane branches on `ledgerOpen`.
3. **One command registry.** Palette entries, `g`-jump bindings, and the status-bar legend must read from the same table, or they will drift.
4. **Zero radius.** Every corner is square — this is load-bearing to the look.
5. **Rules are 2px for section boundaries, 1px for row separators.** Do not soften either into a hairline.
6. **Tabular numerals** on every figure so columns align.
7. **Palette layering.** Dimmer and palette share a stacking context above the shell; the palette is horizontally centered with a fixed 96px top offset, not vertically centered.
8. **Flush left.** Labels — including labels inside wide buttons — start at the left padding edge. Never center them.
9. **Icons**: Lucide, 14×14 at rail size, 1.5px stroke, `fill: none`.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (1a–1d plus the settings surface in section 2)
- `README.md` — this document
