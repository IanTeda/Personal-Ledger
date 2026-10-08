# Claude Code: payees view

Work plan for implementing the payees design in `IanTeda/Personal-Ledger`. The **specification is `README.md`** in this folder — this file is only the order of work, the files each stage touches, and what "done" means. Don't implement from this file alone; each stage points at the README section that governs it.

## Before you start

1. Read `README.md` end to end. Three sections carry decisions you cannot infer from the drawings: **"Schema work this design requires"**, **"A correctness bug this design surfaces"**, and **"Changes from what's implemented today"** (which names the test each change breaks).
2. Open `Ledger TUI Payees.dc.html` in a browser (`support.js` beside it). It is a **design reference drawn in HTML**, not code to port. Target is Rust + ratatui + crossterm in the existing `bin-tui` screen/action architecture. Read all geometry in **terminal cells**; px values are drawing artifacts.
3. Re-read [ADR-0012](https://github.com/IanTeda/Personal-Ledger/blob/main/docs/adr/0012-payee-entity-with-rename-aliases.md) and `payees/resolve.rs`. Everything on this screen follows from auto-creation plus the three-step resolution order.

Copy this folder to `docs/ux/payees/`.

**Stage 1 is a prerequisite for stages 3–6.** Stage 2 can ship on today's schema.

## Stage 1 — Migration

**Files** new `crates/libs/lib-database/migrations/client/<ts>_payee_metadata_and_manual_aliases.sql`, `payees/{model,builder,insert,update}.rs`, `payee_aliases/{model,mod}.rs`
**Spec** README § *Schema work this design requires*

```sql
ALTER TABLE payees ADD COLUMN website TEXT;
ALTER TABLE payees ADD COLUMN icon_url TEXT;
ALTER TABLE payees ADD COLUMN default_category_id UUID REFERENCES categories(id);
ALTER TABLE payee_aliases ADD COLUMN source TEXT NOT NULL DEFAULT 'rename';
```

Done when: all four are nullable-or-defaulted with **no backfill**; `Payees` and `PayeeAliases` carry the new fields with `PayeesBuilder` support; `PayeeAliases` gains `insert` and `delete_by_id` (it has neither today, deliberately — the module docs say why, so update them); deleting a Category clears a `default_category_id` rather than blocking; `source` is `'rename'` for every existing row and for anything `Payees::rename` writes.

Do **not** widen `rename`'s contract: it stays the only way a name changes, so an alias can never be skipped.

## Stage 2 — The list

**Files** `crates/bins/bin-tui/src/screen/payees_list.rs`, `payees/totals.rs`, `action.rs`
**Spec** README § *8a → Left pane*

Replace the flat `Name / Active` table with flag(1) / name(Min 0) / `M`(2) / `TOTAL`(11, right, tabular), **sorted by `abs(total)` descending**.

Done when: totals arrive in bulk via `PayeeTotalsLoaded` and alias counts in bulk too — **nothing per-row at render time**; inactive payees are hidden with `za` to reveal them (`find_all` returns them, so the filter is this screen's job); the flag column shows `·` for no default category and `!` for a pattern that also matches another payee, with a legend row beneath the list; `/` filters on **name and alias pattern**; `TOTAL` is signed and stated in the base unit.

The `!` flag needs the cross-payee conflict check from stage 5 — ship the column first and light it up then.

## Stage 3 — The record and the category mix

**Files** `payees_list.rs`
**Spec** README § *8a → the selected payee's record*, § *Right pane*

Six-line record box, then the category mix, then this payee's transactions.

Done when: the record's `icon` row shows **`from website`** when derived rather than typed; the mix is this payee's transactions grouped by category, biggest first, proportional block-glyph bars capped at 16 cells; the mix's footer line states the default and its agreement with the mix (`default follows the mix · 84% groceries`) and **says so when they disagree** instead of overriding either; transaction status is a glyph (`○`/`✓`) with no reliance on colour; `enter` opens transactions filtered to this payee.

The mix is the justification for the default category — it is why `c` lives next to it. A payee spread evenly across four categories should end up with **no** default; make sure the UI can express that.

## Stage 4 — New and edit forms

**Files** `crates/bins/bin-tui/src/screen/payee_detail.rs`
**Spec** README § *8b*, § *8c*

Centred overlay (`Clear` + bordered `Block`, ~88% width) over the dimmed list, `INSERT` in the status line. `FIELDS` goes from `[Name, IsActive]` to `[Name, Website, IconUrl, DefaultCategory, IsActive]`.

Done when: `icon url` defaults to **`[×] derive from website`** with the derived value shown, and unchecking exposes a free-text URL; a URL is validated in shape but a save never blocks on reachability; the name field in edit mode is **boxed, focused and captioned "changing this is a rename"**, with the `on save` block showing the rename, the exact stored pattern, and *184 txns show the new name at once — no rows rewritten, they join by id*; a name colliding with another payee is caught **before** submit and names the holder, rather than surfacing a unique-constraint error; a no-op rename (same name, different case) writes no alias; `8b` carries the "why this form is rare" note verbatim in substance — a payee is created by typing it on a transaction, and this form is for pre-seeding.

**Render the stored pattern exactly.** `update.rs` builds `format!("(?i)^{}$", regex::escape(&current.name))`, and Rust's `regex::escape` escapes only meta characters — a space is not one, so `WW Metro` stores as `(?i)^WW Metro$` with no backslash. Developers copy this format from the UI.

**Make `save_payee` one transaction.** Today it awaits `rename` then `set_active` independently; with five fields a half-applied save becomes possible. `rename`'s internal transaction is the pattern.

`Enter` stops saving — `^s` does. Extend `tab_cycles_focus_forward_and_wraps` to 5 fields and add a `^s` case.

## Stage 5 — Rename matches

**Files** new `crates/bins/bin-tui/src/screen/payee_matches.rs`, `payee_aliases/find.rs`, `action.rs`
**Spec** README § *8d*, § *A correctness bug this design surfaces*

A list-inside-an-overlay: pattern / `FROM` / `HITS`, an add-and-test box, the resolution order, and the conflict block.

Done when:
- **`as` offers `(•) exact text` / `( ) regex`** and `stores` shows the compiled result either way — a curating user thinks in literal text, and exact-text mode must escape and anchor exactly as `rename` does.
- **`test` runs the real resolution order** (`resolve_or_create`'s three steps) and names the payee it lands on.
- **Adding a pattern checks it against every other payee's patterns and refuses on collision** — this is the whole point of the stage. `resolve_or_create` takes the first match from `find_all` in arbitrary order, so two payees matching one text resolves nondeterministically and imports misfile silently.
- **A pattern equal to another payee's name is refused or warned** — step 1 (exact name) always wins, so it can never fire.
- **A `source = 'rename'` alias is protected**: `d` removes it, `e` refuses. Its UUIDv7 is the only record of when that rename happened.
- `HITS` is real or absent. Count transactions that resolved through the alias; if that isn't tracked, add a counter — don't display a fabricated number.

## Stage 6 — Delete overlay

**Files** `payee_detail.rs` or new `payee_delete.rs`, `payees/delete.rs`
**Spec** README § *8e*

Done when: the overlay **leads with deactivate**, preselected, with `delete` shown and its condition stated (`needs 0 txns, 0 matches`); the refusal is attributed to **the foreign-key pragma, not a UI rule**, and states that a renamed payee can never be deleted because its own rename match references it; the `after deactivate` block lists what survives (transactions keep the payee and its name, matches keep resolving, nothing is rewritten) and that `a` reverses it; confirm is the payee's name typed out, matching the unit, category and account overlays; the closing line redirects to `m` matches for folding a duplicate.

Don't add a UI-side guard that duplicates the pragma — check references to *decide which option is available*, and let the database remain the authority.

## Stage 7 — Command grammar

**Files** the shell's command registry (see `design_handoff_ledger_shell`)
**Spec** README § *Command grammar*

`payee` · `new` · `edit` · `rename` · `match [add]` · `default` · `off|on` · `delete`. `<payee>` completes on **name and alias pattern** — an old name should find its payee, which is the entire point of aliases. Each command reaches the same code path as its keybinding, not a parallel one.

## Throughout

- **Theme roles, not hex.** The README's style table maps the wireframe's colours to roles. Use the repo's own theme module.
- **Never colour alone** — negatives carry a minus sign, status and flags are glyphs with a legend, refusals are stated in words.
- **Bulk aggregates.** Totals, alias counts and the category mix are per-payee aggregates; load them once per list load, like `AccountBalancesLoaded`.
- **Cells clip, they don't wrap.** Row heights are fixed; a too-long payee name truncates.
- Degrade below 96 columns: drop the mix's amount column → drop the transactions' `CATEGORY` → drop the mix → collapse to the list alone with the record on `enter`.
- No mouse support required.

## Out of scope

**Merge two payees** — the operation auto-creation guarantees you will need, and the one this design deliberately doesn't cover: matches prevent future duplicates but don't fold the 40 `WOOLIES` transactions you already have. Also out: the suggestion picker on transaction entry (issue #71), the FR.36 ranked payee-totals report, and rendering the icon (that's `bin-desktop`'s job — terminal graphics protocols are out of scope). See README § *Not yet designed*.
