# Handoff: Personal Ledger TUI — tags

## Overview
The tags view for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud). It re-hosts inside the shell specified in `design_handoff_ledger_shell` and follows the same conventions (single full-bleed view region, `:` palette, footer hint bar, centred form overlays over a dimmed view). It is deliberately the **categories view with the tree taken out** — same list-plus-detail-plus-evidence skeleton, same overlay grammar — so the two screens read as siblings.

**The one idea this design turns on:** a Category is exactly one per Transaction; a Tag is **any number**. Everything that differs from `design_handoff_ledger_categories` falls out of that single cardinality change:

- **No tree, no kind, no parent.** A Tag is only a name, so there is nothing to inherit and nothing to move. `5b` (move) has no counterpart here; the sharp operation is instead `9d` (merge).
- **Rows overlap.** A Transaction carrying two Tags is counted in both rows, so **the `12M` column does not sum to spend**. Categories never had this problem — its `direct` / `rollup` pair was exactly the arithmetic that made a tree add up. The tags screen has to state the opposite, in words, on the screen.
- **One Unit for every Tag.** An Account is fixed to one Unit; a Category spans Accounts and therefore Units. A Tag does too — but a Tag is only useful if the list is sortable and comparable, so **the whole view is denominated in one Unit** (the Ledger's default Unit, stated once in the status line as `AUD only`) and a Tag reaching a Transaction in another Unit has that Transaction **excluded from the total and counted out loud** (`2 txns in JPY · not in this total`, flag `!` on the list). The Ledger does not convert Units (CONTEXT.md, **Unit**), so silently summing across them would be a lie; dropping them without saying so would be worse.

### Assumptions to confirm
`tags` does not exist in the repo at `main` as of this writing, and `CONTEXT.md` on `main` still reads `_Avoid_: Tag, group` under **Category** — the updated glossary entry was not visible when this was drawn. The design therefore assumes the following; correct the README and the wireframe if the glossary says otherwise:

1. A Tag is a name, a note, and an active flag — nothing else. No colour, no group, no type.
2. A Transaction carries **zero or many** Tags, stored as a join table, not a column.
3. A Tag is **never auto-created** (unlike a Payee). It is created deliberately — from `9b`, or inline while tagging, which is the same insert with a different entry point.
4. A Tag is **orthogonal to Category Type**: a Tag carries no kind and can sit on income and expense Transactions alike. Its total is therefore a magnitude in one Unit, not a signed income/expense figure.
5. Renaming a Tag is a plain field update, with no alias history (the Payee Alias mechanism exists because bank feeds type Payee names; nothing types a Tag on the user's behalf).

Five states:

| id | state |
| --- | --- |
| `9a` | **Tags screen** — flat list by 12m total, the untagged remainder, the selected tag's record, the categories it cuts across, its transactions |
| `9b` | **New** — one real field; the form names the three things a category form would ask for and a tag has no place for |
| `9c` | **Edit** — rename / note / active, with delete's refusal and the two operations that replace it |
| `9d` | **Merge** — the dedupe arithmetic: transactions carrying both tags collapse to one link |
| `9e` | **Apply** — bulk add or remove across a filter, reporting what is already tagged before it writes |

## About the design files
`Ledger TUI Tags.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, in the repo's existing `bin-tui` screen/action architecture (`screen/categories_list.rs` and `screen/category_detail.rs` are the closest precedent to copy).

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about the tag model (many-per-transaction, no tree, one unit, overlap, dedupe on merge), pane structure, keybindings, the command grammar, and what each element implies as a ratatui widget. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All names, amounts and dates in the mock are fake.

---

## Schema work this design requires

Nothing exists today. One migration, two tables:

```sql
CREATE TABLE tags (
    id          BLOB PRIMARY KEY NOT NULL,              -- RowID / UUIDv7, as elsewhere
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    note        TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_on  TEXT NOT NULL,
    updated_on  TEXT NOT NULL
);

CREATE TABLE transaction_tags (
    transaction_id BLOB NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    tag_id         BLOB NOT NULL REFERENCES tags(id),
    created_on     TEXT NOT NULL,
    PRIMARY KEY (transaction_id, tag_id)
);
CREATE INDEX idx_transaction_tags_tag ON transaction_tags(tag_id);
```

Four decisions in that DDL that the screens depend on:

- **The composite primary key is what makes `9d` and `9e` safe.** A Transaction can never carry the same Tag twice, so merge and bulk-apply are idempotent by construction — `INSERT OR IGNORE` — rather than needing a UI guard. The counts the forms show (`6 txns already carry both`, `2 already tagged`) are the rows that key rejects, and they must be **counted before the write and reported**, not discovered afterwards.
- **`ON DELETE CASCADE` on `transaction_id`, but *not* on `tag_id`.** Deleting a Transaction should take its tag links with it; deleting a Tag must be **refused** while links exist, by the same `foreign_keys` pragma that guards Payees and Accounts. `9c` states that refusal in words and offers the two things the user actually meant (`A` remove-from-filter, `X` merge).
- **`name` is `UNIQUE COLLATE NOCASE`**, matching `payees.name`. `Holiday-Japan` resolves to `holiday-japan` rather than making a second Tag.
- **No `unit_id` on `tags`.** A Tag is not denominated; the *view* is. Keeping the Unit out of the table is what lets one Tag legitimately touch a JPY Transaction — the screen then excludes it from the total and says so. Putting a Unit on the Tag would instead make that Transaction untaggable, which is the wrong answer to the right problem.

**Change Sets:** `transaction_tags` is a link table with no scalar fields, which the field-granularity Change Set model ([ADR-0009](https://github.com/IanTeda/Personal-Ledger/blob/main/docs/adr/0009-lww-sqlite-change-set-log.md)) does not obviously express — a link either exists or does not, and last-write-wins on "the row" is not the same as LWW on a field. **This needs deciding before implementation**, and it is the one genuinely open question in this design. The pragmatic reading: treat a link row as a single boolean field keyed by `(transaction_id, tag_id)`, so an add on one Client and a remove on another resolve by HLC timestamp like anything else. A bulk `9e` apply then emits one Change Set per link, which is correct but chatty — a 400-transaction apply is 400 Change Sets.

**Aggregates:** the list needs per-Tag transaction count and total, restricted to the view's Unit, plus the co-occurrence counts and the untagged remainder. Load them **in bulk, once**, the way `AccountBalancesLoaded` does. Never compute during render.

---

## Terminal geometry

Drawn at **106 × 30 cells** (one cell ≈ 6.6 × 16.5 px in the HTML), inside the shell's view region.

```
row 0        status line     "ledger · tags / holiday-japan"
                             right: "8 tags · 12m to sep 26 · AUD only"
rows 1..n-2  two panes       left 45 cols fixed · right Min(0)
row n-1      keybind hints   j/k · enter txns · n new · e edit · A apply
                             · X merge · a archive · / filter · tab   (one row, always)
```

`Layout::horizontal([Constraint::Length(45), Constraint::Min(0)])`. Each pane is a stack of `Constraint::Length` blocks over a `Min(0)` list — no fixed heights on anything holding a list.

Degrade below 106 columns: drop the category-mix amount column → drop the transactions' `CATEGORY` column → drop the category mix entirely → collapse the left pane to the list alone, with the record reachable on `enter`.

---

## 9a — Tags screen

### Left pane (45 cols) — the flat list

Four columns: flag (1), `TAG` (Min 0), `N` (4, right — the transaction count), `12M` (10, right, tabular).

```
F TAG                     N       12M
  tax-deductible         86     9 480
! holiday-japan          41     7 240   ← selected: full-width reversed block
  investment-property    62     6 118
  reimbursable           23     2 884
  medical                31     1 962
  subscription           96     1 344
  gift                   18       940
· work-from-home         12       612
──────────────────────────────────────
  — untagged —          742    88 340
! txns outside AUD   · archived  za show
```

- **Sorted by 12m total descending**, not by name — same argument as the payees list: the vocabulary is small but its usefulness is not uniform, and `subscription` at 96 transactions and $1 344 is a different kind of tag from `tax-deductible` at 86 and $9 480. `/` filters by name for the alphabetical case.
- **`N` is a transaction count, not a child count.** In `5a` the same column position holds the number of descendants; a tag has none. Keeping the column shape and changing its meaning is deliberate — the two screens stay scannable as a pair — so the header letter differs (`N` vs the tree's fold marker) and the README says which.
- **The untagged row is the point of the screen.** A tag vocabulary is only trustworthy if you know how much of the Ledger it misses. It sits below a rule, is not selectable, and its figures are the complement of *any* tag — not the complement of the selected one. With 742 of 792 transactions untagged, the honest read of this Ledger is "tags are used for trips and tax, not for everything", and the screen should let the user see that without arithmetic.
- **The flag column carries the unit exception**: `!` = this tag reaches transactions outside the view's Unit, so its total is incomplete. `·` = archived (hidden by default; `za` shows them, dim). Legend row sits under the list — without it the glyphs are noise.
- **Totals are unsigned magnitudes.** A tag has no kind, so it can sit on income and expense alike; `subscription` summing to 1 344 means 1 344 moved on tagged transactions. Where a tag genuinely mixes directions, show the signed net in the detail pane rather than pretending the list column can carry both.

### Left pane — the selected tag's record

Seven lines, same box as `5a`'s category record:

```
active · 41 txns · mar 26 → sep 26
──────────────────────────────────────
TOTAL 12M · AUD              7 240.55
spans        9 categories · 3 accounts
also tagged  tax-deductible 4 · gift 2
off-unit     2 txns in JPY · not in this total   ← accent
note         apr trip · flights, rail, food
active       [×] · offered when tagging
```

- **`spans` replaces `5a`'s `kind · depth` / `children` pair.** It answers the question a flat vocabulary raises: is this tag a synonym for one category (in which case it is redundant), or does it genuinely cut across? Nine categories and three accounts says it earns its place.
- **`also tagged` is the overlap, named per-tag.** It is how the user finds the duplicate vocabulary that `9d` merges: two tags co-occurring on almost every transaction are one tag.
- **`off-unit` is only rendered when non-zero**, and it is in the accent — it is a statement that the big number above it is incomplete.

### Right pane — the trend, the crossing, the evidence

**Tagged spend** — monthly sparkline over the same 24-month window the rest of the app uses, flat at zero before the tag existed. The flat run is information: it dates the tag. Footer row reads `first used mar 26 · peak apr 3 180 · sep 26 148`.

**Where it lands** — the tag's transactions grouped by Category, biggest first, proportional block-glyph bars (`█`, max 16 cells), top 4 plus an `N more` roll-up:

```
travel:flights    █████████████ 41%   2 968.00
travel:lodging    ████████ 26%        1 882.40
food:restaurants  █████ 16%           1 158.75
transport:rail    ███ 9%                651.20
5 more            ██ 8%                 580.20
a tag crosses the tree — that is what it is for
```

This is the mirror of the payees screen's category mix, and it carries the same weight: it is the evidence that a tag is doing work a category could not. A tag landing 100% in one category is a redundant tag, and this block is where that becomes obvious.

**Transactions** — newest first: `DATE` (6), `PAYEE` (Min 0), `CATEGORY` (11), `T` (2, right), `AMOUNT` (9, right). **`T` is the count of *other* tags on that transaction** — it is the overlap made visible at row level, and the reason the footer's warning is believable rather than abstract. `enter` opens the full transactions view filtered to this tag.

**Footer rows** — the overlap statement first, in the accent: `12 of 41 carry another tag — rows overlap`, then `the 12M column does not sum to spend; untagged does`. That second line is the precise claim: the tag rows overlap each other, but *tagged + untagged* is the whole Ledger, so the untagged row is the one figure on screen that can be trusted to complete the set. Then the command hints.

---

## 9b — New (`:tag new`)

`n` from the list. Centred floating overlay (`Clear` + bordered `Block`, ~88% width) over the dimmed list.

| field | rule |
| --- | --- |
| `name` | required; **case-insensitively unique**; the only focused field |
| `note` | optional free text |
| `active` | `[×]` by default |

**The form's real content is the block listing what it does not ask for** — `parent — the list is flat`, `kind — it takes the category's`, `unit — one unit for all tags: AUD`. A user arriving from `5c` (new category) expects those three prompts; naming their absence is faster than letting them look for them, and it is where the flat-vs-tree difference is taught.

The second note block prevents the other likely misreading, in both directions: a tag **can** be created inline while tagging a transaction (typing an unknown name offers `create holiday-korea`, the same insert from a different entry point), but **unlike a Payee it is never auto-created** — nothing types a tag on the user's behalf, so the list never fills itself with near-duplicates the way `8a` does. That is why there is no alias/match machinery here and no curation queue.

Keys: `tab` next field · `^s` create · `^a` create and another · `esc` cancel.

## 9c — Edit (`:tag edit <tag>`)

`e` on a row. Three editable fields; the interesting half is read-only.

- **Rename is free and safe** — `41 links join by id`. This is the deliberate contrast with `8c`, where changing a Payee's name is a rename that must write an alias. Nothing resolves a tag from typed text at import time, so there is no history to keep. State it on the form so a developer doesn't copy the Payee pattern out of symmetry.
- **`read-only here`** carries `tagged` (41 txns · 9 categories · 3 accounts), `unit` (`AUD · 2 txns in JPY excluded`), and `created` (`12 mar 26 · with its first txn`).
- **The closing block is the delete story.** Archiving keeps all 41 links and totals and only stops the tag being offered. Delete is **refused while links exist** (the FK pragma, as with Payees and Accounts), and the two things the user probably meant are named with their keys: `A` apply in remove mode to strip the tag from 41 transactions, or `X` merge to fold it into another tag. Deleting a tag that has never been used is the one case that just works, and it needs no ceremony.

Keys: `tab` next field · `^s` save · `^a` archive · `X` merge · `esc` cancel.

## 9d — Merge (`:tag merge <from> <into>`)

`X` on a row. **This is the screen's counterpart to `5b` (move)** — the one operation whose result is not what a user would compute in their head, and therefore the one that has to show its arithmetic:

```
merging      japan-trip · 14 txns · 1 986.40
into         holiday-japan▌                        ← focused
completion   holiday-japan · gift · medical  · tab

the arithmetic                        NOT A SUM
links moved  14 → holiday-japan
deduped      6 txns already carry both            ← accent
net new      8 links written · 6 dropped
──────────────────────────────────────────────
was          41 txns  7 240.55
becomes      49 txns  8 388.15
not          55 txns  9 226.95 — that double-counts 6   ← accent
```

- **`deduped` is the whole reason this state exists.** Merging categories moves N transactions and adds N to the target, because a transaction has one category. Merging tags moves N *links*, and the composite primary key silently rejects those whose transaction already carries the target. Showing `was / becomes / not` — including the wrong number the user would otherwise expect — is the only way the result stops looking like a bug.
- **Count the collisions before writing.** `SELECT count(*) FROM transaction_tags a JOIN transaction_tags b USING (transaction_id) WHERE a.tag_id = :from AND b.tag_id = :into`. The preview must be the same query the write relies on, not an estimate.
- **The write is `INSERT OR IGNORE` then `DELETE`, in one transaction**, and the source Tag row is deleted once its last link moves — which is also the only way a used Tag ever gets deleted, given the FK guard in `9c`.
- **No Transaction row is edited** — only join rows. Say it, because "merge" sounds destructive and here it is not.
- Marked `one transaction · irreversible`. There is no alias trail to reconstruct the split, so the confirmation carries the weight.

Keys: `tab` complete target · `^s` merge · `esc` cancel.

## 9e — Apply (`:tag apply <tag> <filter>`)

`A` on a row. **Tagging is a bulk act** — nobody tags 41 transactions one at a time — so the primary way a tag gets used is a filter plus a verb, and it belongs on the tags screen rather than hidden in the transactions view.

- **`verb` is a two-option radio, `add` / `remove`**, and the remove half is what makes the whole operation safe to try: the undo for a bad apply is the same filter in remove mode, which the form says out loud.
- **`filter` reuses the transactions view's filter grammar** (`payee:` `cat:` `acct:` `26-04..26-05` `>100`) rather than inventing a second one. The grammar hint sits under the field.
- **The match list previews what will be written**, with a `T` column marking rows that **already carry this tag** (`✓`, accent) — skipped, not duplicated, by the composite key. Four rows plus an `+N more` roll-up; the header carries the full count and sum (`9 txns · 1 184.60 AUD`).
- **`on apply` states the delta before it happens**: links written vs already tagged, the tag's before/after count and total, that other tags are untouched (tags do not displace each other — the one-category-only rule has no analogue here), and the unit exception (`1 JPY txn matched the filter · not tagged`).
- **The unit exception is a design choice worth a second look.** The filter matched a JPY transaction; the view cannot total it. This design **refuses to tag it** and says so, keeping "every tag total is complete in AUD" true for anything created through this form. The alternative — tag it and let the `!` flag carry the incompleteness — is defensible and arguably more honest to the user's intent. Confirm which before implementing; `9a`'s `!` flag exists either way, for links created elsewhere.

Keys: `tab` next field · `space` verb · `^s` apply · `esc` cancel.

---

## Command grammar

Extends the shell's registry (noun-first, tab-completable at every position):

```
tag                                    opens 9a
tag new     <name>                     opens 9b prefilled
tag edit    <tag>                      opens 9c
tag rename  <tag> <new>                plain rename, no alias
tag merge   <from> <into>              opens 9d prefilled
tag apply   <tag> <filter>             opens 9e in add mode
tag remove  <tag> <filter>             opens 9e in remove mode
tag off|on  <tag>                      is_active = 0 / 1
tag delete  <tag>                      refused unless unused
```

## Interactions & behaviour
- **Modal**: `NORMAL` in the list, `INSERT` inside a form overlay. Overlays dim the view behind and grey the app command line, like the shell's palette.
- **Motion**: `j/k` row · `g/G` top/bottom · `za` show archived · `/` filter by name · `tab` moves focus between the list and the transactions list.
- **Operations**: `n` new · `e` edit · `A` apply · `X` merge · `a` toggle active · `enter` transactions filtered to this tag.
- Redraw is event-driven: `TagsLoaded`, `TagTotalsLoaded`, `TagSaved`, `TagsMerged`, `TagLinksChanged`.
- The untagged remainder, per-tag totals and co-occurrence counts are bulk aggregates — one load, never per-render.
- No mouse support required.

## State
List: tags with 12m total, transaction count, off-unit count and archived flag, ordered by total · the untagged remainder · selection · show-archived flag · filter string. Selection detail: the `Tags` row, its first/last use, its category mix, its co-occurring tags, its recent transactions with per-row other-tag counts. Forms: field drafts and validation (name uniqueness, `COLLATE NOCASE`); merge holds the target draft, the completion list and the pre-computed collision count; apply holds the verb, the filter string, its parsed form, the matched set with already-tagged marks, and the off-unit count.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3`. Map to **theme roles, not literal RGB** (same table as the shell handoff):

| role | use here | ANSI |
| --- | --- | --- |
| accent | the `!` off-unit flag, the off-unit line, the overlap warning, merge's `deduped` / `not` rows, the already-tagged `✓`, the cursor | red |
| dim | column heads, the untagged row, notes, archived tags, roll-up rows | dark grey / `DIM` |
| selection | current tag, current transaction, current match | reversed |
| header bar | status line | reversed |

Never rely on colour alone: the off-unit exception is stated in words as well as flagged, the overlap warning is a sentence, already-tagged rows carry a glyph, and merge prints both the right number and the wrong one.

## Not yet designed
- **The tag picker on transaction entry** — where a Transaction's tags are actually set one at a time, and where inline creation lives. `9e` covers the bulk path only.
- **Tags in the transactions view** — a column, a filter chip, and what `T` expands to. `9a`'s `T` count implies it.
- **Tag totals as a report** — a ranked report with the overlap stated once, the analogue of FR.36 for payees.
- **Tags on the dashboard** — probably a single "untagged share" figure rather than a chart; the vocabulary is too small and too overlapping to slice a pie with.
- **Splitting one tag into two** — the inverse of `9d`, which needs a filter to decide which links go where. `9e` in remove mode plus a new tag is the manual path.

## Files
- `Ledger TUI Tags.dc.html` — turn 9: `9a` screen, `9b` new, `9c` edit, `9d` merge, `9e` apply. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- Companion bundles: `design_handoff_ledger_categories` (the screen this one is derived from — read it first), `design_handoff_ledger_shell` (the shell, palette and action registry), `design_handoff_ledger_payees` (the rename/alias contrast, and the category-mix precedent), `design_handoff_ledger_accounts`, `design_handoff_ledger_units` (the Unit model this view's single-unit rule leans on), `design_handoff_ledger_settings`.
