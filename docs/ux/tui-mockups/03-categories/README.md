# Handoff: Personal Ledger TUI — categories

## Overview
The categories view for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud). It re-hosts inside the shell specified in `design_handoff_ledger_shell` and follows the same conventions (single full-bleed view region, `:` palette, footer hint bar).

**The model this design encodes: two fixed roots — `income` and `expenses` — and below each, a tree of arbitrary depth.** Kind is inherited from the root, never set per category. Parents accept postings as well as children, so every node carries two figures: **direct** (posted to it) and **rollup** (itself plus all descendants).

Four states:

| id | state |
| --- | --- |
| `5a` | **Categories screen** — the tree across both roots, the selected category's summary, its spend line and its transactions |
| `5b` | **Move** — reparenting a subtree, with the landing previewed and the refusals named |
| `5c` | **New** — parent prefilled from the selection, kind inherited and not settable |
| `5d` | **Edit** — rename / note / active; parent and kind read-only, delegating to move |

## About the design files
`Ledger TUI Categories.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, using the repo's existing crate layout, domain types and SQLite access layer.

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about the category model (fixed roots, inherited kind, direct vs rollup, move constraints), pane structure, keybindings, the command grammar, and what each element implies as a ratatui widget. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All names, amounts and dates in the mock are fake.

---

## The category model

```
category(id) → parent_id | NULL      NULL means it IS a root
root(id)     → walk parents to the top
kind(id)     = kind of that root      income | expense — inherited, never stored per node
```

- **Exactly two roots, seeded by migration and not deletable**: `income` and `expenses`. They are rows like any other, with `parent_id IS NULL`. The tree renders them as the two top-level sections; they are never editable, never movable, never archivable.
- **Depth is unlimited.** The mock shows depth 4 (`expenses / food / groceries`, and `expenses / housing / mortgage / interest`); nothing in the schema or the UI caps it. Do not add a depth limit — cap the *rendering* (indent guides) rather than the data.
- **Kind is derived, not stored.** A category cannot be half income: it is whatever its root is. The new form shows `kind` read-only as "inherited from root", and the only way to change a category's kind is to move it — which, if it has transactions, is refused (below).
- **Parents are postable.** A parent with children can still have transactions posted directly to it. This is the single most consequential rule in the view, because it means every node needs two numbers:

```
direct(id)  = sum of transactions posted to id
rollup(id)  = direct(id) + Σ rollup(children)
```

The tree's `12M` column shows **rollup**; the summary box shows **both**, labelled. For a leaf they are equal — show them merged as one `direct · rollup 12m` row rather than printing the same figure twice. For a parent they differ, and that difference is the whole reason the pair exists: a user seeing `Housing 52 400` with no transactions of its own has no other way to understand where the number came from.

Compute rollups with a recursive CTE, not in application code:

```sql
WITH RECURSIVE sub(id) AS (
  SELECT :id UNION ALL
  SELECT c.id FROM category c JOIN sub ON c.parent_id = sub.id
)
SELECT sum(amount) FROM txn WHERE category_id IN (SELECT id FROM sub);
```

### Table shape

```sql
CREATE TABLE category (
  id        INTEGER PRIMARY KEY,
  parent_id INTEGER REFERENCES category(id),   -- NULL only for the two roots
  name      TEXT NOT NULL,                     -- unique among siblings, case-insensitive
  note      TEXT,
  active    INTEGER NOT NULL DEFAULT 1,        -- 0 = archived
  UNIQUE (parent_id, name)
);
```

`UNIQUE (parent_id, name)` is the sibling-uniqueness rule the new and edit forms enforce. Full paths (`expenses/food/groceries`) are **derived for display and for command completion**, never stored — storing them would need rewriting on every move.

Guard against cycles on write: a move must reject any target that appears in the moving node's own descendant set. Do this in SQL (the recursive CTE above, run on the *source*) rather than trusting the UI to have filtered completion.

---

## Terminal geometry

Drawn at **96 × 30 cells** (one cell ≈ 6.6 × 16.5 px in the HTML), inside the shell's view region.

```
row 0        status line     "ledger · categories / expenses / food / groceries"
                             right: "34 categories · 12m to sep 26 · AUD"
rows 1..n-2  two panes       left 41 cols fixed · right Min(0)
row n-1      keybind hints   j/k row · h/l fold · n new child · N sibling · m move
                             · r rename · X merge · a archive · tab tree↔txns
```

`Layout::horizontal([Constraint::Length(41), Constraint::Min(0)])`. Each pane is a stack of `Constraint::Length` blocks over a `Min(0)` list — no fixed heights on anything holding a list.

Degrade below 96 columns: drop the `N` (child count) column → drop the transactions `ACCOUNT` column → drop the spend graph → collapse the left pane to the tree alone, with the summary box reachable on `enter`.

---

## 5a — Categories screen

### Left pane (41 cols) — the tree, then the summary

**Tree list.** Three columns: name (Min(0), carrying the indent guides), `N` (2 cols, right — direct child count, `—` for a leaf), `12M` (8 cols, right — rollup). Column heads dim and uppercase. The header reads `tree   10 of 34 · depth 4 · 11 folded` — visible rows, total categories, deepest path, folded branches — and carries the shell's 1-col scrollbar when the tree overflows.

Rows, as drawn:

```
▾ INCOME              9   142 100      ← root, bold, not editable
├─▸ Salary            2   118 400
└─▸ Investments       3    23 200

▾ EXPENSES           21   118 640
├─▸ Housing           5    52 400
├─▾ Food              2    18 240
│ ├── Groceries       —    12 480      ← selected: full-width reversed block
│ └── Restaurants     —     5 760
├─▸ Transport         6    16 900
└─▸ Health            3     8 200
```

- **Roots are bold and carry a blank line above** the second one, so the income/expense split reads as two sections rather than one long list. They are not indented and have no guide glyph.
- **Guides are box-drawing**: `├─`/`└─` for the last sibling, `│ ` continuing through deeper levels. Render from the ancestor chain, not from a fixed indent per level.
- **Fold marker sits before the name**: `▾` expanded, `▸` folded, nothing for a leaf. A folded parent still shows its child count and its rollup — folding hides structure, never money.
- Fold state is **per node, session-local** (not a DB column): `h`/`l` fold and unfold, `zR`/`zM` all. Roots default expanded, everything else folded on first open.
- Archived categories render dim with a trailing `· archived` and are hidden entirely unless `za` toggles them on.

**Summary box** — directly under the tree, the selected category's full record. These are exactly the fields the edit form (`5d`) exposes, so the user rarely needs to open it to *read* anything:

```
expenses / food / groceries
──────────────────────────────
direct · rollup 12m   12 480.40      ← merged when equal; two rows when they differ
──────────────────────────────
kind · depth          expense · 3
children              none · leaf
transactions          148 · direct
first · last          oct 25 · 08 sep
note                  supermarket, greengrocer
active                [×] · offered
```

`active` states the consequence (`offered`) rather than just the flag, because "active" alone doesn't tell the user what it controls: an inactive category is not offered when categorising, but keeps every transaction and every total.

### Right pane — the money, then the evidence

**Direct spend line** — `oct 24 – sep 26`, monthly, ~24 points. A single polyline over a dashed average rule and a baseline, with the last point marked; labels beneath read `oct 24  712` / `avg 1 040` / `sep 26  904`. In ratatui this is `Chart` + `Dataset` with `GraphType::Line`, `Marker::Braille`, or a sparkline row if braille is unavailable.

It plots **direct** spend, not rollup, and the header says so — a parent's rollup line would otherwise silently include its children and contradict the tree.

**Transactions list** — `10 of 148 · newest first`, four columns: `DATE` (6 cols), `PAYEE` (Min(0)), `ACCOUNT` (10), `AMOUNT` (8, right, tabular). Selected row is a full-width reversed block; the 1-col scrollbar sits on the right edge, drawn only when the list overflows. Footer row: `148 direct · 0 in subtree`  ·  `enter open txn`.

`0 in subtree` is the leaf case. **For a parent, this is where direct and rollup become legible** — `82 direct · 1 240 in subtree` — and `s` toggles the list between direct-only and whole-subtree (in subtree mode each row gains its category, which is the one time PAYEE gives up width).

**Command hints** — dim, two rows at the bottom: `:cat new <name> [parent] · :cat move <cat> <parent>` and `:cat merge restaurants dining`.

---

## 5b — Move (`:cat move <cat> <parent>`)

`m` on a row. **The only operation that can break a tree**, so it previews the landing and states its refusals rather than discovering them on submit. Centred floating overlay (`Clear` + bordered `Block`, ~88% width, top-anchored) over the dimmed tree; the view behind keeps its greyed `esc close move form` hint row — the pattern every form in this design follows.

```
moving        Groceries · leaf · 148 txns
from          expenses / food
┌ new parent  expenses/food/daily▌                    ← focused
└ completion  daily · dining · delivery  · tab

lands as                                      depth 4
└─▾ Food
  ├─▾ Daily
  │ └── Groceries · 12 480 moves with it
  └── Restaurants

recomputes    rollups on 3 ancestors
transactions  148 · unchanged, still this category
refuses       its own descendants · the other root
```

- **`new parent` completes on full paths**, and `^n` creates a missing parent inline (typing a path that doesn't exist is a common way to reorganise — don't force a separate `:cat new` trip).
- **`lands as` renders the real destination fragment** with the moved node in place, not a text description. This is the check the user actually performs.
- **`refuses` names both hard constraints up front**: a node cannot move into its own descendant set (cycle), and cannot cross into the other root **while it or any descendant has transactions** — that would silently flip income to expense. With no transactions anywhere in the subtree, a cross-root move is allowed; it is a re-classification of an empty branch.
- Transactions are **not** touched: they reference the category id, so a move is one `UPDATE category SET parent_id = ?` plus rollup invalidation on the old and new ancestor chains.

Keys: `tab` complete parent · `^n` new parent · `^s` move · `esc` cancel.

---

## 5c — New (`:cat new <name> [parent]`)

`n` (child of the selected row) or `N` (sibling of it). Same overlay treatment.

| field | rule |
| --- | --- |
| `name` | required, **unique among siblings** (case-insensitive); the sibling clash is the only validation error here |
| `parent` | prefilled from the selection, `tab` to change — full-path completion, same widget as `5b` |
| `kind` | **read-only**, `expense · inherited from root` |
| `depth` | read-only, informational: `3 · no limit` |
| `note` | optional free text |
| `active` | `[×]` by default, with the consequence spelled out: `offered when categorising` |

`lands as` previews the new node among its siblings (`Daily · new, empty`), so sort position is visible before creation — **siblings sort by name**, there is no manual ordering, and the footnote says so along with "kind cannot be set here — move it to change kind".

Keys: `tab` next field · `^s` create · `^a` create and start another sibling (bulk tree-building is the common case on first setup) · `esc` cancel.

## 5d — Edit (`:cat edit <cat>`)

`e` on a row. **Editable**: `name` (rename is safe — nothing references a category by name, only by id), `note`, `active`. **Read-only, with the key that owns them**: `parent` (`m` move), `kind` (`follows the root`), `children` (`n` new child).

The lower block states the destructive semantics before the user reaches for them:

```
archiving keeps all 148 transactions and totals;
it only stops the category being offered.
delete is refused while transactions exist        ← accent
X merge into another category instead
```

- **Archive is the soft path** (`active = 0`): history intact, hidden from pickers, still shown in reports covering its period.
- **Delete is refused** while any transaction references the category, and separately while it has children — offer merge instead of explaining the refusal twice.
- **Merge** (`X`) reassigns every transaction to the surviving category and deletes the source; it is its own confirmation flow (not drawn — see "Not yet designed").

Keys: `tab` next field · `^s` save · `^a` archive · `X` merge · `esc` cancel.

---

## Command grammar

Extends the shell's registry (noun-first, tab-completable at every position; `<cat>` and `<parent>` complete on full paths and on bare names where unambiguous):

```
cat                            opens 5a
cat new     <name> [parent]    parent defaults to the selection
cat edit    <cat>              opens 5d
cat move    <cat> <parent>     opens 5b prefilled, or executes if both given
cat rename  <cat> <name>
cat merge   <from> <into>      reassigns transactions, deletes <from>
cat archive <cat>              active = 0
cat tree    [root]             prints the subtree — scriptable/pipeable
```

## Interactions & behaviour
- **Modal**: `NORMAL` in the tree, `INSERT` inside a form overlay. Overlays dim the view behind and grey the app command line, exactly like the shell's palette.
- **Motion**: `j/k` row · `h/l` fold/unfold (`h` on a leaf jumps to its parent) · `zR`/`zM` unfold/fold all · `za` show archived · `g/G` top/bottom · `tab` tree↔transactions focus.
- **Operations**: `n` new child · `N` new sibling · `e` edit · `r` rename in place · `m` move · `X` merge · `a` archive · `enter` open the highlighted transaction.
- `/` filters the tree by name, keeping ancestors of every match visible so the hierarchy stays legible while filtered.
- Redraw is event-driven; a category write fires the same DB-change event as any other and invalidates rollups on the affected ancestor chains only.
- No mouse support required.

## State
Tree: the node list with derived depth/guide flags · fold set (session) · selected id · show-archived flag · filter string. Selection detail: the category record, its direct and rollup totals, child count, transaction count, first/last posting. Right pane: the monthly direct series for the graph · the transaction page (offset, direct-vs-subtree mode) · scroll offsets. Forms: field drafts, the resolved parent id, per-field validation, and for move the computed descendant set used to refuse targets.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3`, dark variant on `#161413`. Map to **theme roles, not literal RGB** (same table as the shell handoff):

| role | use here | ANSI |
| --- | --- | --- |
| accent | the spend line, the `refuses` row, the cursor | red |
| dim | guides, column heads, notes, read-only form fields | dark grey / `DIM` |
| selection | current tree row, current transaction | reversed |
| header bar | status line | reversed |

Never rely on colour alone: fold state is a glyph (`▾`/`▸`), the selected row is reversed, and refusals are stated in words.

## Not yet designed
- **Merge** (`X`) — reassigning 148 transactions into a surviving category, and what it says about the source's descendants.
- **Delete** of an empty category, versus archive.
- The **transactions view** filtered to a category and its subtree (`enter` from 5a lands there).
- **Rules / auto-categorisation** on import — deliberately out of scope until the import flow is designed.

## Files
- `Ledger TUI Categories.dc.html` — turn 5: `5a` screen, `5b` move, `5c` new, `5d` edit. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- Companion bundles: `design_handoff_ledger_shell` (the shell, palette and action registry this view sits in), `design_handoff_ledger_units` (units and prices), `design_handoff_ledger_settings` (database-backed settings — `week starts` and `general.base_unit` affect this view's figures).

## Suggested Claude Code prompt
> Read `docs/ux/categories/README.md` and open `docs/ux/categories/Ledger TUI Categories.dc.html` in a browser for reference. Build, in this order: (1) the `category` migration with the two seeded non-deletable roots, `UNIQUE (parent_id, name)`, and recursive-CTE helpers for descendants, derived kind, and rollup totals; (2) the tree widget — ancestor-derived box-drawing guides, session fold state, child count and rollup columns, roots as bold sections; (3) the summary box, the direct-spend chart and the paged transactions list with its direct-vs-subtree toggle; (4) the new and edit forms, with kind and parent read-only in edit and sibling-uniqueness validation; (5) the move overlay last — cycle and cross-root refusals enforced in SQL against the descendant set, with the landing fragment previewed. Follow the existing crate layout and SQLite layer, and use theme roles rather than the hex values.
