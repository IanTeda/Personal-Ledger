# Handoff: Personal Ledger TUI — shell, command palette & help

## Overview
The application shell, its navigation model and its help window for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud dependency), built against `docs/product-requirements.md` in `IanTeda/Personal-Ledger`.

Four states are specified, and together they are the entire navigation model:

| id | state |
| --- | --- |
| `1a` | **Shell at rest** — status line, full-bleed chart-led dashboard, idle command line, keybind hints |
| `2a` | **`:help` window** — search + the full command list, at rest |
| `2b` | **`:help` window** — filtered by a search term |
| `3a` | **Command palette** — floating fuzzy-find window, the only way to navigate |

The command palette is included even though the ask was "shell and help": with no menus and no persistent nav, the palette *is* the shell's navigation, and help is its slower, browsable twin. All three read from **one shared action registry** — implement that registry first.

The remaining screens (account ledger, transaction form, accounts, categories, budgets, reconcile, spending report, units) are wireframed separately and re-host inside this same shell.

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
row n-2      command line      ":" prompt; a hint when idle, greyed while a window is open
row n-1      keybind hint bar   dim; contextual to the focused view
```

Everything is a `Layout` of `Constraint::Length` / `Percentage` / `Min`. No absolute positioning; no fixed heights on anything holding a table.

Degrade below 96 columns in this order: drop sparkline columns → drop the right-hand chart column and stack it under the main pane → single-column stack. Below 24 rows, show the headline row + net-worth chart only.

---

## 1a — Shell at rest (dashboard / financial position)

The default view. Chart-led by design: the tables live on their own screens, and this one answers "where do I stand" in one glance. Priority order, top to bottom:

**1. Headline row** — 3 rows. Net position as a large figure (bold, flush left), then 30-day delta, assets, liabilities as label-over-value pairs on one line. Labels dim and uppercase; values bold. Liabilities render in the accent.

**2. Net worth · 18 months** — `Chart` with one `Dataset`, `GraphType::Line`, 18 monthly points. ~60% width, 7 rows. X-axis labelled at first / mid / last month (`mar 25 · sep 25 · sep 26`). No Y-axis labels — the headline figure carries the magnitude.

**3. Income vs expense · 6 months** — divergent bars, right pane ~28 cols, sharing row 2's vertical band. A 3-col month gutter down the middle: expense grows leftward, income grows rightward. Label the two directions under the bars. Two horizontal `BarChart`s mirrored about the gutter, or draw the bars directly.

**4. Where it went · 30 days** — doughnut, left pane ~26 cols. Not in ratatui core: draw with `canvas::Circle` (outer arc + background-filled inner) or half-block arc segments. Legend of exactly 5 slices as `swatch category NN%`, largest first, `other` always last.

**5. Budgets this period** — up to 4 rows, right of the doughnut, each row: category label (12 cols), a **spend bar with a period-progress marker**, then `actual / limit` right-aligned.

The bar is one track, not two: fill length is `actual / limit`, and a 1-cell `│` marker sits at `elapsed_days / period_days`. Fill short of the marker is behind pace, fill past it is ahead of pace, and over-budget clamps the fill full and flips fill, figure and label to the accent. That single comparison is the whole point of the widget — do not split it into two stacked bars.

Not a `Gauge` (it can't carry the marker). Draw the track as a `Line` of block glyphs and overwrite the marker cell:

```
track  20 cells   ░ empty · ▓ fill (dim) · █ fill (accent, over budget)
marker 1 cell     │ at round(elapsed_ratio * 20), replacing that cell's glyph
```

The section header carries the period and its own miniature elapsed bar: `sep · day 21/30  ▓▓▓▓▓▓▓│░░░  70% elapsed`. One dim legend row under the list explains the marker, and must fit one line: `│ = period progress · bar = spent / budget`.

Marker contrast matters: fill is dim, the marker is full-strength ink, so it stays visible inside the filled region. On an over-budget (accent) bar the marker still renders in ink.

**6. Needs attention** — 2–3 lines only, each naming the command that resolves it (`14 unreconciled transactions → :reconcile`). Deliberately last, and deliberately not a table.

**Footer hints (1a):** `: command · / search · a add txn · ? help` plus the dim note `no persistent nav — ↑ recalls history`.

---

## 3a — Command palette

`:` opens a **centred floating overlay** — `Clear` + bordered `Block`, ~78% of terminal width, anchored in the top third. It is a window, not an inline dropdown, and it is the primary navigation affordance.

Contents, top to bottom:

1. **Prompt row** — `> bud▌` flush left, match count right-aligned (`7 of 62`), separated from the results by a strong rule. Block cursor in the accent.
2. **Results** — up to 7 rows, each `action → effect → binding`:
   - the matched substring is emphasised **anywhere in the string**, not just as a prefix — `bud` also surfaces `report budget variance` and `category set budget…`
   - direct-noun matches rank above incidental ones; the selected row is a full-width reversed block
   - the binding column shows the shortcut where one exists, `—` where none does
3. **Argument preview** — one row above the footer, naming the next expected argument and resolving it against real data: `arg 1 <category> — dining · limit 300.00 · actual 412.00` with `over by 112.00` right-aligned in the accent. **Show this before the command runs.** In a UI with no menus it is the only confirmation the user gets that they are about to act on the right record.
4. **Window footer** — its own dim hint row inside the border: `↑↓ select · tab complete · enter run · ^r history · esc close`.

While the window is open:
- the view behind it stays **visible and heavily dimmed** (`Modifier::DIM`), never hidden or cleared — context is the point
- the app's `:` command line greys out and echoes the typed text
- the app's own footer hints grey out too and gain `esc close command window`
- the status line shows `COMMAND` as the mode

Keys: `Esc` close · `↑`/`↓` select · `Tab` complete · `^r` history · `Enter` run.

---

## 2a / 2b — `:help` window

Same floating-window treatment as the palette, but a **reference rather than a runner**: the palette is for people who know the verb, help is for people who don't. Toggled with `?`.

**No title row.** The search row *is* the header — it carries the strong 2px rule and the match count:

```
/ ▌ help — filter by command, binding or description              62 of 62
```

Typed state (2b) replaces the hint with the term and folds the group count into the same right-hand slot: `/ bud▌` … `6 of 62 · 2 groups`.

**Body** — the command list, grouped by noun with **NAVIGATE first**, each row `:command  <binding>  description`:

```
NAVIGATE
:dashboard                 g d    financial position — the default view
:account ledger <name>     g l    transaction table for one account
:account list              g a    accounts and their balances
:category list             g c    categories and 30-day totals
:unit list                 g u    units, details and weekly prices
:budget list [period]      g b    budgets vs actual for the period
:report category [range]   g r    spending report and charts
:reconcile [account]       g k    balance checks and clearing
:txn recent                g t    last 50 transactions, all accounts
TRANSACTIONS
:txn new [account]         a      add a transaction from anywhere
:txn edit                  e      edit the highlighted transaction
:txn delete                d      delete — confirms by payee and amount
:txn status <state>        c R o  cleared · reconciled · reopen
UNITS & PRICES
:unit new <code> <type>    n      add a unit — code is permanent
…
```

Screen jumps are their own group at the top rather than scattered through their nouns — "how do I get to X" is the most common reason the window is open.

**Search matches command, binding and description.** `bud` surfaces `:report variance` on its description alone, with the match highlighted there. Groups collapse to what matched, and a dim line names the groups with no hits so the absence is explicit rather than ambiguous.

**Footer rows** — a counter and scroll hint (`showing 14 of 62 · j/k scroll · g group jump`) with the status-glyph legend right-aligned, then the window's own hint row: `/ search · ↑↓ select · enter run it · y copy command · esc close`.

`Enter` runs the highlighted command, so help doubles as a slower palette; 2b states what Enter will do (`enter runs :budget list — help closes and the view opens`) so it isn't a surprise.

**Sizing:** the window is **content-sized** — it is as tall as its rows. Do not give the list a leftover height inside a fixed-height window; a partially drawn row is wrong on a character grid, where a row is whole or absent. Compute how many rows fit the terminal, draw that many whole, and scroll the rest.

---

## The action registry

The palette, the help list, the footer hint bars and the keymap must all be generated from **one registry**, so a new action gets its palette entry, its binding column, its help row and its hint for free. Each entry needs:

```
name          ":budget edit"        canonical, noun-first
args          [<category>, [limit]] with a resolver per arg for the preview line
effect        "set limit"           palette's middle column
description   "change the limit or period"   help's third column
binding       Some("b e") | None    rendered as "—" in help when None
group         Navigate | Transactions | Units | Budgets | …
```

### Command grammar
Noun-first, space-separated, tab-completable at every position:

```
account   ledger <name> | list | new | edit <name> | reconcile <name> | deactivate <name>
budget    list [period] | new <category> <limit> | edit <category> [limit] | deactivate <category> | period <len>
category  list | new <name> <type> | edit <name> | merge <from> <into>
unit      list | new <code> <type> [precision] | edit <code> | delete <code>
price     set <unit> <date> <close> | import <path.csv>
report    category [range] | payee [range] | networth [range] | variance
check     new <account> <date> <balance> | import <path.csv>
txn       new [account] | edit | delete | status <state> | recent
sync      now | status
dashboard
help
```

**Every action must be reachable by name.** Keybindings are shortcuts, not the only path — a hard requirement given the shell has no visible menu.

---

## Interactions & behaviour
- **Modal, vim-flavoured.** `NORMAL` at rest, `INSERT` only inside forms, `COMMAND` while the palette is open, `HELP` while help is open. Show the mode in the status line whenever it is not NORMAL.
- **Motion:** `j/k` row · `g/G` top/bottom · `^d/^u` page · `l`/`Enter` open · `h` back.
- **Jumps:** `g d` dashboard · `g a` accounts · `g l` account ledger · `g c` categories · `g u` units · `g b` budgets · `g r` reports · `g k` reconcile · `g t` recent transactions.
- **Global:** `:` palette · `?` help · `/` search · `a` add txn from anywhere · `S` sync now · `y` yank as CSV · `q` close view (returns to the dashboard) · `Q` quit.
- Destructive commands confirm in a small overlay that names the record.
- No mouse support required. No animation beyond an optional 1-cell sync spinner.
- Redraw is event-driven (key or DB change), not on a timer.

## State
Shell: focused view id · view stack for `q` · mode. Palette: input buffer · filtered candidates + selection · history ring (`^r`) · resolved argument preview. Help: search buffer · filtered registry + selection + scroll offset · collapsed-group list. Dashboard: the derived aggregates (net position, 30-day delta, 18-month series, 6-month income/expense, 30-day category split, active budgets with elapsed ratio, attention items) — query them, do not cache in view state. Plus last sync time and result for the status line.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, a single accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3` for dim text, and a dark variant on `#161413`. In the terminal map these to **theme roles, not literal RGB**, so the user's own terminal theme wins:

| role | use | ANSI |
| --- | --- | --- |
| ink / ground | body text | default fg / bg |
| accent | negatives, over-budget, variance, cursor, matched substring, focused border | red |
| dim | labels, column heads, the greyed footer and command line | dark grey / `DIM` |
| selection | current row | reversed |
| header bar | status line | reversed |

One accent only. Never rely on color alone: negatives also carry `−`, over-budget also overshoots its track, flagged rows also carry `⚑`.

## Status & flag glyphs
`○` open · `◐` cleared · `●` reconciled · `⚑` flagged (independent of status). Provide an ASCII fallback (`o` `/` `x` `!`) behind a config flag for terminals without the glyphs — the same flag should cover the `│` budget marker and the `🔒` locked-field glyph used on the unit forms.

## Files
- `Ledger TUI Shell.dc.html` — turns 1 (shell), 2 (help) and 3 (command palette). Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.

## Suggested Claude Code prompt
> Read `docs/ux/tui/README.md` and open `docs/ux/tui/Ledger TUI Shell.dc.html` in a browser for reference. Build, in this order: (1) the action registry described in the README, since the palette, help window, keymap and footer hints all generate from it; (2) the shell — status line, single full-bleed view region, command line, keybind hint bar, mode handling; (3) the floating command palette with its fuzzy matcher and argument-preview row; (4) the `:help` window over the same registry; (5) the chart-led dashboard view. Follow the existing crate layout and SQLite layer, put the shell, each window and each view in their own modules with their own state structs, and use the theme roles from the README rather than the hex values. The budget bar is a custom track with a period-progress marker cell, not a `Gauge`.
