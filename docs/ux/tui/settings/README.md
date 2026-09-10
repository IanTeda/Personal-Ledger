# Handoff: Personal Ledger TUI — settings (database-backed)

## Overview
The settings view for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud). It re-hosts inside the shell specified in `design_handoff_ledger_shell` and follows the same conventions (single full-bleed view region, `:` palette, footer hint bar).

**The architectural decision this design encodes: settings live in the database, not in a config file.** Static config is reduced to the handful of keys needed *before* the database can be opened. Everything else is a row in a `settings` table that overrides a default compiled into the binary.

Three states:

| id | state |
| --- | --- |
| `4a` | **Settings at rest** — groups pane, the selected group's settings, the `settings` rows behind them |
| `4b` | **Editing in place** — a row opens where it sits, with a live preview and the exact upsert it will commit |
| `4c` | **Base unit guard** — the one setting whose commit invalidates derived data; states the cost first |

## About the design files
`Ledger TUI Settings.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, using the repo's existing crate layout, domain types and SQLite access layer.

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about the settings model (defaults in code, overrides in the DB), the resolution and reset semantics, pane structure, keybindings, the command grammar, and what each element implies as a ratatui widget. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All values, dates and counts in the mock are fake.

---

## The settings model

This is the part to get right before drawing anything.

```
value(key) = row in `settings` where key = ?    →  the override
           else DEFAULTS[key]                  →  compiled into the binary
```

- **Defaults live in code**, as a static registry — one entry per key. The mock shows 42 of them.
- **A row exists only when the user has changed something.** The mock's "3 overridden" is literally `SELECT count(*) FROM settings`.
- **The red dot in the left gutter means "a row exists for this key"** — nothing more. It is not "differs from default" computed by comparison; it is the presence of the override. (These coincide in practice, but implementing it as row-presence is what makes `r` coherent.)
- **`r` deletes the row.** It does not write the default value back. After `r` there is no row for that key and the view falls through to the default. `R` deletes every row in the focused group.
- **Commits are transactional and immediate.** Accepting an edit performs one upsert and the next render reads the new value. There is no save step, no dirty buffer, no reload — the words "unsaved" and "write config" do not appear anywhere in this view; `uncommitted`, `commit` and `overridden` do.
- **Every commit is logged and undoable.** `u` reverses the last settings commit; `H` opens the change log. `undo depth` is itself a setting, kept in the database (default 50 commands).

### Table shape

```sql
CREATE TABLE settings (
  key        TEXT PRIMARY KEY,      -- "general.base_unit", dotted, group.name
  value      TEXT NOT NULL,         -- always TEXT; the registry owns the type
  changed_at TEXT NOT NULL          -- ISO 8601; renders as "14:02 today" / "02 sep"
);
```

`value` is stored as TEXT and parsed against the registry entry's type on read (`Bool`, `Int`, `Enum(&[&str])`, `Unit`, `Date`, `Path`, `String`). A row whose value no longer parses — an enum variant removed by an upgrade, a unit code since deleted — must **fall back to the default and surface as a warning in the change log**, never panic and never block startup.

### Registry entry

The groups pane, the settings list, the `selected` explainer box, validation, and `:set` completion all generate from **one registry**, the same way the palette generates from the action registry. Each entry needs:

```
key           "general.negatives"
group         General | Display | UnitsPrices | FilesBackup | Reconcile | Keys | About
label         "negatives"                  the list's SETTING column
note          "minus · brackets · trailing"  the list's NOTE column — one line, dim
kind          Enum(["minus","brackets","trailing"])
default       "brackets"
explain       "how a negative amount prints everywhere"   the selected box's prose
consequence   None | Guard(GuardKind)      routes the commit through a dialog
affects       &[Redraw | ClearDerived | RebindKeymap]  what to invalidate on commit
```

`affects` is what makes the commit cheap or expensive, and it drives the guard: a display setting is `Redraw`, `general.base_unit` is `ClearDerived`.

### Bootstrap config — the only static file

`ledger.toml` keeps exactly the keys needed before the database is open, and the settings view shows it **read-only**, as two rows in the left pane's box:

| key | why it can't live in the DB |
| --- | --- |
| `db_path` | needed to find the database |
| `log_level` | logging starts before the connection |
| `migrate` | whether to run migrations on open |
| `color_depth` | terminal init happens before the first query |

Editing these is an `$EDITOR` job outside the app. Do not add a fifth key without the same justification — "needed before the DB is open" is the whole test.

---

## Terminal geometry

Drawn at **96 × 30 cells** (one cell ≈ 6.6 × 16.5 px in the HTML), inside the shell's view region.

```
row 0        status line     "ledger · settings / general"   right: "3 overridden · ✓ committed 14:02"
rows 1..n-2  two panes       left 28 cols fixed · right Min(0)
row n-1      keybind hints   j/k setting · tab groups↔settings · enter edit · r drop override · u undo commit · H change log
```

`Layout::horizontal([Constraint::Length(28), Constraint::Min(0)])`. Both panes are stacks of `Constraint::Length` blocks over a `Min(0)` list — no fixed heights on anything holding a list.

Degrade below 96 columns: drop the NOTE column → drop the `settings table` block → collapse the groups pane into a header breadcrumb reachable with `h`.

---

## 4a — Settings at rest

### Left pane (28 cols)

**`groups` list** — 7 rows, each `label` + right-aligned override-eligible count (`general 8`, `display 7`, `units & prices 6`, `files & backup 5`, `reconcile 4`, `keys 12`, `about —`). Selected row is a full-width reversed block. `about` has no settings and carries `—`; it is a view, not a group.

**`where values live` box** — the model, stated on screen, because a user cannot otherwise tell where a value came from:

```
ledger.db · table
settings
─────────────────────
overrides       3 rows      ← accent
defaults        42 in code
last commit     14:02
─────────────────────
bootstrap       4 keys
ledger.toml · read-only
```

The header carries `H log`. The rule above `bootstrap` separates the mutable store from the static file — they are different things and must not read as one list.

**`reset` block** — header note `deletes the row`, then `r this setting` / `R whole group`. The note is the important word: it tells the user reset is a deletion, not a write.

### Right pane

**Settings list** — four columns: a 1-col override gutter, `SETTING` (18 cols), `VALUE` (14 cols), `NOTE` (Min(0), dim). Column heads dim and uppercase. Selected row is a full-width reversed block. The gutter holds `·` in the accent where a row exists in `settings`, blank otherwise.

The eight `general` settings as drawn:

| setting | value | note |
| --- | --- | --- |
| **base unit** ● | `AUD` | every total converts to this |
| fiscal year starts | `01 jul` | drives year-to-date and reports |
| week starts | `monday` | w/c label on weekly prices |
| **date input** ● | `dd/mm/yyyy` | accepted when typing a date |
| number format | `1 234.56` | space groups · dot decimal |
| **negatives** ● | `−1 234.56` | minus · brackets · trailing |
| confirm deletes | `type the name` | off falls back to y/n |
| undo depth | `50 commands` | kept in the database |

● = overridden (row exists). `VALUE` renders the setting **as it will appear in the app**, not as it is stored — `negatives` shows `−1 234.56`, not `minus`; `date input` shows `dd/mm/yyyy`, not `dmy`. The stored form is shown separately, below.

**`selected` box** — the registry entry's `explain` prose (two lines max), then a ruled block of facts:

```
default        USD
accepts        any active currency unit — 2 available
changing it    re-converts every historical total     ← accent
```

`accepts` resolves against live data (how many units qualify), so it is a query, not static text. `changing it` is present only where the entry has a `consequence`.

**`settings table` block** — the actual rows behind the list, `KEY` / `VALUE` / `CHANGED`. Values render in the **stored** form and in the accent-adjacent "positive" tone: `general.base_unit  AUD  14:02 today`, `general.date_input  dmy  02 sep`. Header shows `3 rows · 2 shown` — it is a truncated view; carry the shell's 1-col scrollbar when it overflows, on the same rules as the palette (drawn only when the list overflows).

This block is the answer to "what will `git diff` on my ledger show" now that there is no config file to diff. Keep it.

**Command hint row** — dim, bottom of the pane: `:set base <unit> · :set negatives brackets · :settings log`.

---

## 4b — Editing in place

`enter` on a row opens the editor **in the row's position** — the list above and below stays put and dims, and the opened row becomes a bordered focused box. No dialog, no separate screen. The status line shows `EDIT · uncommitted` and the breadcrumb extends to `settings / general / negatives`.

The focused box, for an enum setting:

```
· negatives      how a negative amount prints everywhere
  value          [minus] brackets trailing   · ←→
  preview        −320 334.10                        ← accent
  was            (320 334.10) · brackets, the default
  on accept      upsert general.negatives = "minus"
```

- `value` is a segmented row of the enum's variants; the selection is a reversed block, `←`/`→` moves it. Free-text and integer settings take a 1-line input with the same `preview` / `was` / `on accept` rows beneath.
- `preview` renders a **real figure from the user's data** in the candidate format, not lorem — the net position is the obvious choice.
- `was` names the current value *and* whether it is the default, in one line. If there is no row yet, `was` reads `… , the default`; if there is, it reads the overridden value.
- **`on accept` spells out the write**: `upsert general.negatives = "minus"`, or `delete general.negatives` when the candidate equals the default (choosing the default value is a delete, not a write of the same string — the row must not linger).

**`applies to` box** — below the list, the same candidate shown in the three places it lands, so scope is legible before committing:

```
net position   −320 334.10
ledger row     Woolworths  −184.20  groceries
csv export     unaffected — always machine format
```

The `csv export` line matters: display settings never touch export, and someone will otherwise assume they do.

**Status row** — `uncommitted: enter commits and redraws — esc reverts to brackets`. Name the value `esc` returns to; "cancel" alone is not enough information.

**The command line echoes the equivalent command**: `:set negatives minus▌ — same edit, typed`. Every setting must be settable both ways, and showing the command form here is how the user learns it.

Keys: `←→` choose · `enter` commit · `esc` revert · `r` drop override.

---

## 4c — Base unit guard

`general.base_unit` is the one `general` setting with `affects: ClearDerived`. Its commit is a single row like any other, but it invalidates every cached total — so it routes through a **centred floating overlay** (`Clear` + bordered `Block`, ~78% width, anchored in the top third) over the dimmed settings view, exactly like the shell's palette. Status line: `confirm base unit`.

Header: `base unit  AUD → USD` with `:set base` right-aligned.

Body — prose first (three short lines), then the facts, each resolved by query:

```
transactions are stored in their own units and are
not touched. Every reported total is re-converted
at the weekly USD close.

transactions   412 · unchanged
accounts       11 · 4 already in USD
re-converted   18 months of totals
missing rates  6 weeks · nov 25 – dec 25          ← accent
one write      general.base_unit = "USD"
then           clears the cached totals · u undoes it
```

`missing rates` is the reason this dialog exists: converting to a unit you have no closes for silently produces gaps. Name the count **and the range**, then offer the fix as a first-class option rather than an error:

```
those 6 weeks report as gaps until priced.
i import USD closes first — :price import
```

`one write` / `then` state that the commit is cheap and reversible — one row, plus a cache invalidation, undoable with `u`. That is the difference from the old config-file model, and it belongs on screen.

**Confirm by typing the unit code** — a focused 1-line input (`type the unit  USD▌`), matching the delete-confirmation convention on the unit forms (`confirm deletes` governs whether type-the-name or `y/n` applies).

Keys: `enter` commit and re-convert · `i` import first · `esc` cancel. The dimmed view behind keeps its own hint row, greyed, with `esc close dialog`.

---

## Command grammar

Extends the shell's registry (noun-first, tab-completable at every position):

```
set        <key> <value>      key completes on dotted registry keys and on bare labels
set        <key>              opens 4b for that key, no value typed
settings                      opens 4a
settings   log                the change log (H)
settings   reset <key>        equivalent of r — deletes the row
settings   reset <group>      equivalent of R
```

`:set` with no argument opens the palette filtered to settings keys. Value completion comes from the registry entry's `kind` — enum variants, or live data for `Unit`.

## Interactions & behaviour
- **Modal**: `NORMAL` in the list, `INSERT` inside an open editor, and the guard overlay behaves like the palette (view behind visible and heavily dimmed, app command line greyed).
- **Motion**: `j/k` setting · `tab` toggles groups↔settings focus · `h` back to groups · `g/G` top/bottom.
- **Commit** on `enter`; **revert** on `esc`; **drop override** on `r` (`R` for the group); **undo** on `u`; **change log** on `H`.
- Validation is per-field and inline, in the focused box — never a modal. An invalid value blocks the commit and leaves the editor open.
- Redraw is event-driven. A settings commit fires the same DB-change event as any other write, so open views pick up the new formatting on their next draw.
- No mouse support required.

## State
Settings view: focused pane · selected group · selected key · the resolved value list for that group (query, do not cache) · the override rows for the `settings table` block. Editor: the candidate value · the validation result · the derived preview figures. Guard: the resolved impact figures (transaction count, account count, months affected, missing-rate weeks and their range) and the typed confirmation buffer. Global: last settings commit for `u`, and the change log ring.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3`, dark variant on `#161413`. Map to **theme roles, not literal RGB** (same table as the shell handoff):

| role | use here | ANSI |
| --- | --- | --- |
| accent | the override dot, `missing rates`, `changing it`, the cursor | red |
| dim | labels, column heads, notes, the read-only bootstrap box | dark grey / `DIM` |
| selection | current group / setting row, the chosen enum variant | reversed |
| header bar | status line | reversed |

Never rely on colour alone: the override dot is a glyph as well as a colour, and the guard's warning also carries its count in words.

## Files
- `Ledger TUI Settings.dc.html` — turn 4: `4a` at rest, `4b` editing in place, `4c` base unit guard. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- Companion bundles: `design_handoff_ledger_shell` (the shell, palette and action registry this view sits in) and `design_handoff_ledger_units` (the units screen and its forms, which read `general.base_unit` and `week starts`).

## Suggested Claude Code prompt
> Read `docs/ux/settings/README.md` and open `docs/ux/settings/Ledger TUI Settings.dc.html` in a browser for reference. Build, in this order: (1) the settings registry — one static entry per key with group, label, note, kind, default, explain, consequence and affects — plus the `settings` table migration and a `value(key)` resolver that falls back to the default; (2) `r`/`R` as row *deletes*, the change log, and `u` undo; (3) the 4a two-pane view generated entirely from the registry; (4) the in-place editor with its preview / was / on-accept rows, including the rule that choosing the default deletes the row instead of writing it; (5) the base-unit guard overlay with its queried impact figures. Reduce the static `ledger.toml` to the four bootstrap keys listed in the README and show them read-only. Follow the existing crate layout and SQLite layer, and use theme roles rather than the hex values.
