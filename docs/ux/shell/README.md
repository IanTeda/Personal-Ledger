# Handoff: Personal Ledger TUI — shell & dashboard

## Overview
The application shell and default view for `personal-ledger`, a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud dependency), built against `docs/product-requirements.md` in `IanTeda/Personal-Ledger`.

Two states are specified, and they are the whole navigation model:
- **2a — resting state.** Status line, one full-bleed chart-led dashboard, idle command line, keybind hint bar.
- **2b — command open.** The same screen with a centred floating fuzzy-find command window over it.

Scope of this handoff is the shell, the dashboard view, and the command window. The other nine screens (account ledger, transaction form, accounts, categories, budgets, reconcile, spending report, help) are wireframed separately and re-host inside this same shell.

## About the design files
`Ledger TUI Shell.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, using the repo's existing crate layout, domain types and SQLite access layer.

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about information priority, pane structure, keybindings, the command grammar, and which ratatui widget each element implies. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All numbers, payees and account names are fake.

---

## Terminal geometry

Drawn at a **96 × 30 cell** minimum (one cell ≈ 6.6 × 16.5 px in the HTML).

```
row 0        status line       reversed Block — "ledger · position"   right: net · sync · date
rows 1..n-3  view region       exactly ONE view, full bleed — no tab strip, no sidebar, no tree
row n-2      command line      ":" prompt; a hint when idle, greyed while the command window is open
row n-1      keybind hint bar   dim; contextual to the focused view
```

Everything is a `Layout` of `Constraint::Length` / `Percentage` / `Min`. No absolute positioning; no fixed heights on anything holding a table.

Degrade below 96 columns in this order: drop sparkline columns → drop the right-hand chart column and stack it under the main pane → single-column stack. Below 24 rows, show the headline row + net-worth chart only.

---

## 2a — Resting state (dashboard / financial position)

The default view. Chart-led by design: the tables live on their own screens, and this one answers "where do I stand" in one glance. Priority order, top to bottom:

**1. Headline row** — 3 rows. Net position as a large figure (double-height or bold, flush left), then 30-day delta, assets, liabilities as label-over-value pairs on one line. Labels are 9px-equivalent dim uppercase; values bold. Liabilities render in the accent.

**2. Net worth · 18 months** — `Chart` with one `Dataset`, `GraphType::Line`, 18 monthly points. ~60% width, 8 rows. X-axis labelled at first / mid / last month (`mar 25 · sep 25 · sep 26`). No Y-axis labels — the headline figure carries the magnitude.

**3. Income vs expense · 6 months** — divergent bars, right pane ~28 cols, sharing rows 2's vertical band. A 3-col month gutter down the middle: expense grows leftward, income grows rightward. Label the two directions under the bars (`expense` right-aligned on the left half, `income` left-aligned on the right half). Implement as two horizontal `BarChart`s mirrored about the gutter, or draw the bars directly.

**4. Where it went · 30 days** — doughnut, left pane ~26 cols. Not in ratatui core: draw with `canvas::Circle` (outer arc + background-filled inner) or half-block arc segments. Legend of exactly 5 slices as `swatch category NN%`, largest first, `other` always last.

**5. Budgets this period** — up to 5 `Gauge`s, right of the doughnut. Each row: category label (12 cols), the ratio bar, then `actual / limit` right-aligned. Over-budget clamps the bar full and flips bar, figure and label to the accent. Header carries the period and elapsed days (`sep · day 2/30`).

**6. Needs attention** — 2–3 lines only, each naming the command that resolves it (`14 unreconciled transactions → :reconcile`, `balance check variance −12.40 → 31 aug`). Deliberately last, and deliberately not a table.

**Footer hints (2a):** `: command · / search · a add txn · ? help` and the dim note `no persistent nav — ↑ recalls history`.

---

## 2b — Command open (floating command window)

`:` opens a **centred floating overlay** — `Clear` + bordered `Block`, ~78% of terminal width, anchored in the top third. It is a window, not an inline dropdown, and it is the only navigation affordance the app has.

Contents, top to bottom:

1. **Prompt row** — `> bud▌` flush left, match count right-aligned (`7 of 62`), separated from the results by a strong rule. Block cursor in the accent.
2. **Results** — up to 7 rows, each `action → effect → binding`:
   - `budget edit <category> [limit]` · `set limit` · `b e`
   - the matched substring is emphasised **anywhere in the string**, not just as a prefix — `bud` also surfaces `report budget variance` and `category set budget…`
   - direct-noun matches rank above incidental ones; the selected row is a full-width reversed block
   - the binding column shows the keyboard shortcut where one exists, `—` where none does
3. **Argument preview** — one row above the footer, naming the next expected argument and resolving it against real data: `arg 1 <category> — dining · limit 300.00 · actual 412.00` with `over by 112.00` right-aligned in the accent. **Show this before the command runs.** In a UI with no menus it is the only confirmation the user gets that they are about to act on the right record.
4. **Window footer** — its own dim hint row inside the border: `↑↓ select · tab complete · enter run · ^r history · esc close`.

While the window is open:
- the dashboard behind it stays **visible and heavily dimmed** (`Modifier::DIM`), never hidden or cleared — context is the point
- the app's `:` command line greys out and echoes the typed text
- the app's own footer hints grey out too and gain `esc close command window`
- the status line shows `COMMAND` as the mode

Keys: `Esc` close · `↑`/`↓` select · `Tab` complete · `^r` history · `Enter` run.

### Command grammar
Noun-first, space-separated, tab-completable at every position:

```
account   ledger <name> | new | edit <name> | reconcile <name> | deactivate <name>
budget    list [period] | new <category> <limit> | edit <category> [limit] | deactivate <category>
category  list | new <name> <type> | edit <name> | merge <from> <into>
report    category [range] | payee [range] | networth [range] | variance
check     new <account> <date> <balance> | import <path.csv>
txn       new [account] | edit | delete
sync      now | status
```

**Every action must be reachable by name.** Keybindings are shortcuts, not the only path — a hard requirement given the shell has no visible menu. The palette is generated from the same action registry the keymap reads, so a new action gets its palette entry, its binding column and its help row for free.

---

## Interactions & behaviour
- **Modal, vim-flavoured.** `NORMAL` at rest, `INSERT` only inside forms, `COMMAND` while the window is open. Show the mode in the status line whenever it is not NORMAL.
- **Motion:** `j/k` row · `g/G` top/bottom · `^d/^u` page · `l`/`Enter` open · `h` back.
- **Jumps:** `g a` accounts · `g b` budgets · `g r` reports · `g c` categories · `g t` recent transactions.
- **Global:** `:` command · `/` search · `a` add txn from anywhere · `S` sync now · `y` yank as CSV · `?` help · `q` close view (returns to the dashboard) · `Q` quit.
- Destructive commands confirm in a small overlay that names the record.
- No mouse support required. No animation beyond an optional 1-cell sync spinner.
- Redraw is event-driven (key or DB change), not on a timer.

## State
Shell: focused view id · view stack for `q` · mode. Command window: input buffer · filtered candidate list + selection index · history ring (`^r`) · resolved argument preview. Dashboard: the derived aggregates (net position, 30-day delta, 18-month series, 6-month income/expense, 30-day category split, active budgets, attention items) — query them, do not cache in view state. Plus last sync time and result for the status line.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, a single accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3` for dim text, and a dark variant on `#161413`. In the terminal map these to **theme roles, not literal RGB**, so the user's own terminal theme wins:

| role | use | ANSI |
| --- | --- | --- |
| ink / ground | body text | default fg / bg |
| accent | negatives, over-budget, variance, cursor, matched substring, focused border | red |
| dim | labels, column heads, the greyed footer and command line | dark grey / `DIM` |
| selection | current palette row | reversed |
| header bar | status line | reversed |

One accent only. Never rely on color alone: negatives also carry `−`, over-budget also overshoots its track.

## Files
- `Ledger TUI Shell.dc.html` — the two states, 2a and 2b. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.

## Suggested Claude Code prompt
> Read `docs/ux/shell/README.md` and open `docs/ux/shell/Ledger TUI Shell.dc.html` in a browser for reference. Implement the shell (status line / single full-bleed view region / command line / keybind hint bar), the floating command window with its action registry and fuzzy matcher, and the chart-led dashboard view. Follow the existing crate layout and SQLite layer; put the shell, the command window and each view in their own modules with their own state structs. Drive the palette, the binding column and the help sheet from one shared action registry. Use the theme roles from the README, not the hex values. Build the shell + view routing + command parser first, then the dashboard widgets against the repo's existing queries.
