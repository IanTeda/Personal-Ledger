# Handoff: Personal Ledger TUI — payees

## Overview
The payees view for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud). It re-hosts inside the shell specified in `design_handoff_ledger_shell` and follows the same conventions (single full-bleed view region, `:` palette, footer hint bar, centred form overlays over a dimmed view).

**The one idea this design turns on:** a Payee is the only entity the user never deliberately creates. `Payees::resolve_or_create` makes one the first time its name is typed on a Transaction ([ADR-0012](https://github.com/IanTeda/Personal-Ledger/blob/main/docs/adr/0012-payee-entity-with-rename-aliases.md)), so the list fills itself — and fills itself with near-duplicates, because bank feeds spell the same shop three ways. **This screen is therefore a curation screen, not a CRUD screen.** Its two jobs are to fold variant spellings back onto one canonical Payee, and to teach each Payee what it should default to. Creating and deleting are the rare edge cases, not the spine.

What the repo gives us today (`crates/libs/lib-database/src/payees/`, `payee_aliases/`, migration `20260905090000_create_payees_table.sql`):

- **`payees`** is `id`, `name` (`TEXT NOT NULL UNIQUE COLLATE NOCASE`), `is_active`, `created_on`, `updated_on`. Nothing else.
- **`payee_aliases`** is `id`, `payee_id`, `pattern` — a **regex** matched against typed text. Write-once: inserted only by a rename, never updated or deleted, so its `id`'s UUIDv7 stands in for when the rename happened.
- **Renaming is not a field update.** `Payees::rename` changes `payees.name` *and* inserts the prior name as an alias **in one transaction**, so it can never be skipped. There is deliberately no plain "set name". 184 Transactions show the new name immediately because they join by `payee_id` — no rows are rewritten.
- **Resolution order** (`resolve.rs`): exact case-insensitive name match → first matching alias pattern → create a new Payee from the typed text verbatim.
- **`is_active`** excludes a Payee from suggestions; existing Transactions keep working.
- **Delete is a bare `DELETE`** guarded only by SQLite's `foreign_keys` pragma, which refuses it while any Transaction *or the Payee's own alias history* references it.
- **Payee totals exist** (`payees/totals.rs`, FR.36, `Action::PayeeTotalsLoaded(Vec<(RowID, Money)>)`).

Five states:

| id | state |
| --- | --- |
| `8a` | **Payees screen** — list sorted by total, attention flags, the selected payee's record, its category mix and its transactions |
| `8b` | **New** — rare by design; the form says why it exists and what auto-creation would have done |
| `8c` | **Edit** — name / website / icon / default / active, with the rename-and-alias write spelled out |
| `8d` | **Rename matches** — the alias list, its provenance, add-and-test, and the resolution conflict |
| `8e` | **Delete** — refused by the FK pragma while referenced, so it leads with deactivate |

## About the design files
`Ledger TUI Payees.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, in the repo's existing `bin-tui` screen/action architecture (`screen/payees_list.rs`, `screen/payee_detail.rs`, `Action::OpenPayees` / `OpenPayeeDetail` / `PayeesLoaded` / `PayeeSaved` / `PayeeDeleted` / `PayeeAliasesLoaded` / `PayeeTotalsLoaded`).

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about the payee model (auto-creation, rename-leaves-an-alias, resolution order, active vs delete), pane structure, keybindings, the command grammar, and what each element implies as a ratatui widget. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All names, amounts and dates in the mock are fake.

---

## Schema work this design requires

**Three of the four things asked for are not in `payees` today.** They need one migration, all nullable, no backfill:

```sql
ALTER TABLE payees ADD COLUMN website TEXT;
ALTER TABLE payees ADD COLUMN icon_url TEXT;
ALTER TABLE payees ADD COLUMN default_category_id UUID REFERENCES categories(id);
```

- **`website`** — plain host or URL, the user's own note of who this is. Nothing fetches it in the TUI.
- **`icon_url`** — **derived from `website` by default** (`bunnings.com.au` → `bunnings.com.au/favicon.ico`), which is the whole reason the two fields belong on the same form: the user never has to go and find an icon URL. A TUI cannot render it; `bin-desktop` can, and this is the column it will read. Store it, validate it as a URL, never block a save on it being reachable.
- **`default_category_id`** — pre-fills Category on Transaction entry. Nullable is the honest default: a Payee that hasn't been seen enough has no business asserting one. The FK must be `ON DELETE SET NULL` in effect — if a Category is deleted, the default should evaporate, not block it.

**`payee_aliases` needs one more column** for `8d` to be safe:

```sql
ALTER TABLE payee_aliases ADD COLUMN source TEXT NOT NULL DEFAULT 'rename';  -- 'rename' | 'manual'
```

ADR-0012 already anticipates hand-broadened patterns ("so a future cycle can let a pattern be hand-broadened … without a schema change"), and `8d` is that cycle. But making patterns editable breaks two properties the current table leans on, and `source` is what keeps them:

1. **Write-once / `id`-as-timestamp.** A rename-generated alias must stay untouched for its UUIDv7 to still mean "when the rename happened". `8d` therefore lets `d` remove a `rename` alias but makes `e` **refuse** it — edit a hand-authored one instead.
2. **Provenance is user-visible information.** "This came from your rename in March" and "you typed this" warrant different confidence, and the `FROM` column says which.

Editable patterns also mean `PayeeAliases` needs `insert` and `delete_by_id` — today the module has neither, deliberately.

---

## A correctness bug this design surfaces

`resolve_or_create` iterates `PayeeAliases::find_all(pool)` and takes **the first pattern that matches**, in whatever order the query returns. With only auto-generated anchored-exact patterns that is safe — two Payees cannot hold the same exact old name, because `payees.name` is unique. **The moment a pattern can be hand-authored, it is not safe:** two Payees can both carry a match for `WOOLIES`, and which one wins is arbitrary and can change between runs. Imports would misfile silently.

`8d` treats this as a first-class state, not an edge case:

- On add, **check the candidate pattern against every other Payee's patterns** and refuse the save on a collision.
- A pattern already live on another Payee is flagged in the list with `!` and stated in words: *"WOOLIES" also matches Coles Central — two payees matching one text resolves arbitrarily.* The `!` also surfaces on the `8a` list so it is findable without opening each Payee.
- **A match equal to another Payee's name is dead on arrival** — step 1 of resolution (exact name) always wins before step 2 (patterns). `8d` states the resolution order for exactly this reason, and should warn rather than silently store a pattern that can never fire.

---

## Terminal geometry

Drawn at **96 × 30 cells** (one cell ≈ 6.6 × 16.5 px in the HTML), inside the shell's view region.

```
row 0        status line     "ledger · payees / woolworths"
                             right: "142 payees · 130 active · base AUD"
rows 1..n-2  two panes       left 41 cols fixed · right Min(0)
row n-1      keybind hints   j/k · enter txns · n new · e edit · m matches
                             · c default · a off · / filter        (one row, always)
```

`Layout::horizontal([Constraint::Length(41), Constraint::Min(0)])`. Each pane is a stack of `Constraint::Length` blocks over a `Min(0)` list — no fixed heights on anything holding a list.

Degrade below 96 columns: drop the mix's amount column → drop the transactions' `CATEGORY` column → drop the category mix entirely → collapse the left pane to the list alone, with the record reachable on `enter`.

---

## 8a — Payees screen

### Left pane (41 cols) — the list

Four columns: flag (1), name (Min 0), `M` (2, the match count), `TOTAL` (11, right, tabular).

```
F NAME                 M        TOTAL
  Sunrise Payroll      0  +165 360.00
  Home Loan Direct     1   −42 180.00
  Woolworths           2   −18 402.55   ← selected: full-width reversed block
· Coles Central        1    −9 118.40
  Origin Energy        0    −5 784.60
· Telstra              1    −3 204.00
! WOOLIES              1      −920.10
```

- **Sorted by `abs(total)` descending**, not by name. With 142 auto-created Payees, alphabetical order buries the ones worth curating; spend order puts them first. (Today's `payees_list.rs` sorts by name — see the changes table.)
- **`TOTAL` is signed** and comes from `payees/totals.rs` / `PayeeTotalsLoaded`, in the base unit. A Payee with Transactions in several units is the same problem the accounts view has — state the base-unit total and don't silently mix.
- **The flag column is the curation queue.** `·` = no default category, `!` = a match that also matches another Payee. A legend row sits under the list; without it the glyphs are noise.
- `M` is the alias count, so a Payee that has absorbed variants is visible at a glance.
- **Inactive Payees are hidden** by default (`za` shows them, dim, with a trailing `· inactive`). `Payees::find_all` returns them, so the filter is the screen's job.

### Left pane — the selected payee's record

Six lines. The first carries what never changes; the rest are what curation edits.

```
active · seen 184 times · since oct 24
──────────────────────────────────────
total      −18 402.55 AUD
website    woolworths.com.au
icon       ·/favicon.ico · from website
default    expenses:groceries
matches    2 · m manage
```

`icon` shows **`from website`** when it was derived rather than typed — the user should be able to tell at a glance that they never set it, and that changing the website will change it.

### Right pane — where the money goes, then the evidence

**Category mix** — this Payee's Transactions grouped by Category, biggest first, as proportional bars:

```
expenses:groceries   ████████████████ 84%  −15 458.15
expenses:household   ██ 11%                 −2 024.30
expenses:alcohol     ▌ 5%                     −920.10
default follows the mix · 84% groceries  ·  c change
```

This is not decoration — **it is the justification for the default category**, and the reason `c` sits next to it. A Payee whose mix is 84% one Category should default to it; a Payee spread evenly across four should not have a default at all, and the mix is how the user sees which they're looking at. Bars are block glyphs (`█`, `▌` for the remainder), widths proportional to share, max 16 cells.

Where the mix and the stored default disagree, say so rather than silently overriding either — the user decides.

**Transactions** — this Payee's Transactions across all accounts, newest first: status glyph (1), `DATE` (6), `CATEGORY` (Min 0), `AMOUNT` (9, right, signed). `enter` opens the full transactions view filtered to this Payee. Status is a **glyph** (`○` open, `✓` reconciled), never colour alone.

**Footer rows** — the span (`184 txns · oct 24 → sep 26`), then the line that prevents the most likely misreading of this screen: *payees are created by typing them on a transaction*. Then the command hints.

---

## 8b — New (`:payee new`)

`n` from the list. Centred floating overlay (`Clear` + bordered `Block`, ~88% width) over the dimmed list.

| field | rule |
| --- | --- |
| `name` | required; **case-insensitively unique** (`COLLATE NOCASE`) — a collision resolves to the existing Payee rather than creating a second |
| `website` | optional, free text |
| `icon url` | `[×] derive from website` checked by default, showing the derived value beneath; unchecking exposes a free-text URL field |
| `default` | optional Category, `tab` to search |
| `active` | `[×]` by default |

The note block exists because **this form is the rare path and looks like the main one**: a payee is created automatically the first time its name is typed on a transaction, so come here only to pre-seed a default or a match before an import lands. Without that sentence a user will reasonably assume they must create every payee by hand.

Keys: `tab` next field · `^s` create · `^a` create and another · `esc` cancel.

## 8c — Edit (`:payee edit <payee>`)

`e` on a row. All five fields are editable — but **the name field is a rename**, and the form must never let that be a surprise. It is boxed, focused, and captioned *"changing this is a rename"*, with the consequence spelled out below:

```
on save                          one transaction
rename       Woolworths → Woolworths Group
match kept   (?i)^Woolworths$ · auto
184 txns     show the new name at once
             no rows rewritten — they join by id
```

- **The pattern is shown in its exact stored form.** `update.rs` builds it as `format!("(?i)^{}$", regex::escape(&current.name))`; Rust's `regex::escape` only escapes meta characters, and a space is not one, so `WW Metro` stores as `(?i)^WW Metro$` — no backslash. A developer will copy this format from this cell, so it has to be right.
- **A no-op rename writes no alias** — `rename` returns early when the name is unchanged case-insensitively (`rename_to_the_same_name_case_insensitively_is_a_no_op`). Don't add a second alias for `Woolworths` → `WOOLWORTHS`.
- A rename **colliding with another Payee's name** fails on the unique constraint. Validate before submitting and say which Payee holds it — the raw SQLite error is not an answer.
- `read-only`: id and `created`. `m` jumps to `8d`, `^d` to `8e`.

Keys: `tab` next field · `^s` save · `^a` deactivate · `esc` cancel.

## 8d — Rename matches (`:payee match`)

`m` from the list or from `8c`. A list-inside-an-overlay, so it gets its own state rather than being crammed into the edit form.

```
matches  Woolworths                          :payee match
typed text that resolves to this payee

  PATTERN                        FROM      HITS
> (?i)^Woolworths$                rename     12
  (?i)^WOOLIES$                   manual      3
  (?i)^Woolworths Metro$          manual      0

┌ add      WW Metro▌
│ as       (•) exact text   ( ) regex
│ stores   (?i)^WW Metro$
└ test     "ww metro" → Woolworths ✓

resolve order
1 an exact payee name   2 these matches
3 create a new payee from what was typed
a match equal to another payee's name is never
reached — that payee already wins at step 1

! "WOOLIES" also matches Coles Central          ← accent
  two payees matching one text resolves
  arbitrarily — remove one of them

a rename match is protected — d removes, e refuses
```

- **`as` is the important control.** `pattern` is a regex, but a user curating payees is thinking in literal text. `(•) exact text` escapes and anchors what they typed (exactly what `rename` does); `( ) regex` passes it through for someone who means it. `stores` shows the result either way, so the regex is never a mystery.
- **`HITS` is the column that makes this list actionable** — a pattern with 0 hits after an import either isn't needed or isn't working. Count Transactions whose Payee resolved through that alias; if that isn't tracked, count `payee_aliases` matches at resolve time and store a counter, or drop the column rather than faking it.
- **`test` resolves live** through the real `resolve_or_create` order and names the Payee it lands on — the only honest way to show what a regex will do.
- The conflict block and the resolve-order note are specified under *A correctness bug this design surfaces* above.

Keys: `j/k` match · `a` add · `e` edit · `d` remove · `t` test · `esc` close.

## 8e — Delete (`:payee delete <payee>`)

`d` on a row, or `^d` from `8c`. **The design's honest position is that this operation almost always cannot run**, so the overlay leads with the alternative instead of pretending otherwise:

```
references   184 txns · 2 matches
             the database refuses this delete      ← accent

action       (•) deactivate
             ( ) delete · needs 0 txns, 0 matches  ← accent on the condition

whose rule this is
the foreign-key pragma rejects deleting a payee
a transaction or its own match history points at
— not a UI guard. A renamed payee can never be
deleted: its own rename match references it.

after deactivate
suggestions   not offered when posting
184 txns      keep this payee and its name
2 matches     keep resolving · nothing rewritten
reversible    a on the row turns it back on

confirm       Woolworths▌
to fold a duplicate into this payee, add its name
as a match instead — m matches
```

- **Naming the pragma as the source of the refusal matters.** A user told "you can't delete this" assumes the app is being protective and looks for an override. Told that the database rejects it, and why, they stop looking.
- **A renamed payee can never be deleted** — its own alias references it (`delete_by_id_rejects_a_payee_still_referenced_by_an_alias`). This is a permanent property, not a temporary state, and the overlay says so.
- **Deactivate is reversible and lossless**, and the `after` block says exactly what survives. Confirm is the name typed out, matching the unit, category and account delete overlays.
- The closing line redirects the actual intent: someone reaching for delete on a duplicate wants a **merge**, and the nearest thing that exists is adding the duplicate's name as a match on the survivor.

Keys: `tab` next field · `^s` deactivate · `m` matches · `esc` cancel.

---

## Changes from what's implemented today

`payees_list.rs` and `payee_detail.rs` exist and are tested. This design changes five things; each breaks or moves a named test.

| today | this design | why | test affected |
| --- | --- | --- | --- |
| Flat `Table`, columns `Name / Active`, sorted by name, inactive included | Sorted by `abs(total)` desc, flag / name / `M` / `TOTAL`, inactive hidden behind `za` | 142 auto-created payees make alphabetical order useless; spend order and the flag column are the curation queue | `renders_loaded_without_panicking` still passes; add sort, flag and hidden-inactive cases |
| `enter` → `OpenPayeeDetail(Some(..))` (the rename form); footer reads "Enter: rename" | `enter` opens **transactions filtered to this payee**; edit moves to `e` | Matches the accounts view (`enter` = see the rows, `e` = edit) and is the more common intent | `enter_opens_an_edit_form_for_the_selected_payee` — rewrite as `e_opens_an_edit_form…`, add an `enter` → filtered-transactions case |
| `d` arms `pending_delete`, a footer line takes a single `y` | `d` opens the `8e` overlay: refusal stated, deactivate preselected, name typed to confirm | The single `y` silently hits an FK error on any referenced payee — which is nearly all of them | `d_arms_delete_confirmation_and_a_non_y_key_cancels_it` — keep the arming concept, replace the confirm |
| `FIELDS = [Name, IsActive]` | `[Name, Website, IconUrl, DefaultCategory, IsActive]` | The three new columns | `tab_cycles_focus_forward_and_wraps` — extend from 2 fields to 5 |
| `Enter` saves the form | `^s` saves; `enter` is free | Consistent with `7b`/`7c` and unambiguous once a form holds several text inputs | add a `^s` save case |

**`save_payee` needs to become one transaction.** Today it awaits `rename` then `set_active` independently — fine for two fields where the first is itself transactional, but with website, icon and default joining them, a half-applied save becomes possible. Wrap the whole save in one transaction; `rename`'s internal transaction is the pattern to follow.

Two things this design does **not** change: `name` stays case-insensitively unique (no second Payee for a different casing), and `rename` stays the only way to change a name, so an alias can never be skipped.

## Command grammar

Extends the shell's registry (noun-first, tab-completable at every position; `<payee>` completes on name *and* on alias patterns, since an old name should find its payee):

```
payee                                  opens 8a
payee new    <name>                    opens 8b prefilled
payee edit   <payee>                   opens 8c
payee rename <payee> <new>             renames, keeping the old name as a match
payee match  <payee>                   opens 8d
payee match add <payee> <text>         adds an exact-text match
payee default <payee> <category>       sets the default category
payee off|on <payee>                   is_active = 0 / 1
payee delete <payee>                   opens 8e
```

## Interactions & behaviour
- **Modal**: `NORMAL` in the list and in `8d`, `INSERT` inside a form overlay. Overlays dim the view behind and grey the app command line, like the shell's palette.
- **Motion**: `j/k` row · `g/G` top/bottom · `za` show inactive · `/` filters by name **and by alias pattern**.
- **Operations**: `n` new · `e` edit · `m` matches · `c` default category · `a` toggle active · `d` delete · `enter` transactions filtered to this payee.
- Redraw is event-driven: `PayeesLoaded`, `PayeeAliasesLoaded`, `PayeeTotalsLoaded`, `PayeeSaved`, `PayeeDeleted`.
- **Totals, alias counts and the category mix are per-payee aggregates — load them in bulk, once**, the way `AccountBalancesLoaded` does. Never compute during render.
- No mouse support required.

## State
List: payees with totals and alias counts, ordered by `abs(total)` · selection · show-inactive flag · filter string · the conflict set (patterns live on more than one payee). Selection detail: the `Payees` row, its first-seen date and transaction count, its category mix, its recent transactions. Forms: field drafts, the derive-icon-from-website flag, resolved `default_category_id`, per-field validation. Matches: the payee's aliases with source and hit counts, the add draft with its exact/regex mode, the compiled candidate pattern, the live test result, and the cross-payee conflict check.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3`. Map to **theme roles, not literal RGB** (same table as the shell handoff):

| role | use here | ANSI |
| --- | --- | --- |
| accent | negative totals, the `·`/`!` flags, the conflict block, the delete refusal, the cursor | red |
| dim | column heads, derived values, notes, inactive payees | dark grey / `DIM` |
| selection | current payee, current match, current transaction | reversed |
| header bar | status line | reversed |

Never rely on colour alone: negatives carry a minus sign, transaction status is a glyph, flags are glyphs with a legend, refusals are stated in words.

## Not yet designed
- **Merge two payees** — the operation auto-creation guarantees you will need. Matches only prevent *future* duplicates; folding 40 existing `WOOLIES` transactions onto `Woolworths` means repointing `payee_id` and then deleting the loser, which the FK pragma allows only once it holds nothing. This is the obvious next piece.
- **The suggestion picker on transaction entry** — the other half of resolution (CC-TUI-007, issue #71), where `PayeeAliasesLoaded` is consumed as the user types.
- **Payee totals report** (FR.36) as its own report view; `8a` shows the per-payee number, not the ranked report.
- **Icon rendering** — `bin-desktop` reads `icon_url`; terminal graphics protocols are deliberately out of scope.

## Files
- `Ledger TUI Payees.dc.html` — turn 8: `8a` screen, `8b` new, `8c` edit, `8d` matches, `8e` delete. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- `CLAUDE_CODE.md` — the staged work plan: order of work, files per stage, acceptance criteria. This README is the specification; that file is the order of work.
- Companion bundles: `design_handoff_ledger_shell` (the shell, palette and action registry), `design_handoff_ledger_accounts` (the same list+detail+overlay pattern, and `enter`/`e` precedent), `design_handoff_ledger_categories` (a payee's default category comes from there), `design_handoff_ledger_settings`, `design_handoff_ledger_units`.
