# Handoff: Tags (screens 7a–7e)

## Overview
Tag management for the Personal Ledger desktop app. Tags are free-form, cross-cutting labels (no hierarchy, no budget) with a colour swatch. Screens: list, Add, Edit, Remove, and Merge (for folding typo-duplicates like `Shared` into `shared`).

## About the design files
`Tags.html` is a **design reference built in HTML** — it shows intended look and behaviour, not production code. Recreate these screens in the target codebase's existing environment and patterns (e.g. GPUI/Rust desktop), or pick the most suitable framework if none exists. Open `Tags.html` in a browser; each screen is a 1280×800 frame labelled 7a–7e. `styles.css` is the Modernist design-system stylesheet the frames use for `.btn` / `.tag` classes.

## Fidelity
**High-fidelity.** Colours, type, spacing and copy are final. Match them exactly.

---

## App shell (shared by all screens)
- **Title bar** 48px, bg `#eae9e9`, bottom border 2px `rgba(32,30,29,.38)`, padding `0 10px 0 12px`, gap 12px. Contents: 16px panel icon · "Personal Ledger" 13.5px/800 · context word "tags" 12px `#9b9797` · spacer · command box ("**:** run a command", 12px, 1px border `rgba(32,30,29,.30)`, padding 4px 9px) · three 26×26 window buttons (bg `rgba(32,30,29,.08)`, gap 2px).
- **Sidebar** 206px, bg `#eae9e9`, right border 2px `rgba(32,30,29,.38)`, padding `14px 0 10px`. Section labels ("LEDGER", "PLAN") 10px/800, letter-spacing .11em, `#9b9797`. Items: padding 7px 14px, gap 9px, 14px line icon, label, shortcut hint 11px `#9b9797`. Active item (Tags): bg `#201e1d`, text `#f3f2f2`/800, hint `#bab6b6`, count badge "18" inverted (bg `#f3f2f2`, 10px/800, padding 1px 5px).
- **Content** padding 22px 28px.
- **Status bar** 28px, bg `#eae9e9`, top border 2px, 11.5px `#605d5d`; mode pill (NORMAL / DIALOG — bg `#201e1d`, text `#f3f2f2`, 10px/800, letter-spacing .1em, padding 2px 7px), key hints, right-aligned summary.
- **Modal state:** title bar opacity .38, sidebar/content .30; scrim `rgba(32,30,29,.30)`; dialog centred.

## Dialog pattern
- Panel bg `#f3f2f2`, border 2px `#201e1d`, shadow `0 16px 48px rgba(32,30,29,.40)`, no radius.
- Header padding 18px 20px, 16px/800, bottom border 2px `rgba(32,30,29,.30)`.
- Body padding 20px, gap 16px. Labels 12px/800, margin-bottom 6px. Inputs full-width, padding 8px 10px, 1px `rgba(32,30,29,.30)`, 13px.
- Callout 11.5px `#605d5d`, padding 10px, bg `#eae9e9`, left border 2px `#201e1d` (irreversible: `#ec3013`).
- Footer padding 16px 20px, top border 1px `#d7d3d3`, right-aligned, gap 10px; buttons padding 8px 16px, 800. Secondary: 1px border, transparent. Primary: bg `#201e1d`, text `#f3f2f2`.
- Note: tags have **no red destructive dialog** — removing/merging never deletes transactions.

## Colour swatches (fixed set of 6)
`#ec3013` · `#7a8a6b` · `#c9922f` · `#4a7c9e` · `#8a6bb5` · `#9b9797`. Picker squares 26×26, gap 8px; selected gets a 2px `#201e1d` border. In tables the swatch is a 10×10 square before the name (gap 8px).

---

## 7a — Tags list
- Header: "Tags" 28px/800; subline 11.5px `#9b9797` — "18 tags · **2** likely duplicates — merge them" (count `#ae1800`/800; "merge them" is a link opening 7e). Right: primary "+ Add tag" (36px). Then 2px rule, 24px spacing.
- Table: border 1px `rgba(32,30,29,.30)`; header bg `#eae9e9`, 10px/800, letter-spacing .11em, `#605d5d`, padding 10px 16px.
- Columns: TAG (flex: swatch + name) · TRANSACTIONS 110px right · TOTAL 120px right · LAST USED 110px · ACTIONS 150px right. Sorted by usage, descending. Tabular figures.
- Rows padding 10px 16px, bottom border 1px `#d7d3d3`; last-used `#605d5d`.
- Selected row: bg `#201e1d`, text `#f3f2f2`, name and total 800, date `#d7d3d3`, action buttons bordered `rgba(243,242,242,.35)`.
- Likely-duplicate row: bg `#faf3ef` and an outline tag after the name: `looks like a duplicate of "shared"` (border `#ec3013`, text `#ae1800`).
- Row actions "edit", "remove" — 11px, padding 4px 8px, 1px border.
- Sample: shared (86, −4,208.90, 08 sep) · reimbursable (41) · tax-deductible (33) · Shared (9, duplicate) · work-trip (22) · gift (14) · one-off (3).
- Footnote 11px `#9b9797`: tags are free-form; removing a tag untags transactions and never deletes them; flagged tags are close spellings — use merge.
- Status bar: NORMAL · "j/k row · enter view transactions · e edit · x remove · n new" · right "18 tags · 2 likely duplicates".

## 7b — Add tag (dialog, 420px)
- Title "Add tag". **Name** placeholder "e.g. work-trip". **Color** — 6-swatch picker (first selected by default).
- Callout: names match case-insensitively — creating "Shared" when "shared" exists offers to reuse the existing tag.
- Buttons: Cancel · **Add tag**. Status: DIALOG · "esc cancel · enter confirm · tab next field".

## 7c — Edit tag (dialog, 420px)
- Title "Edit tag — shared". Name and colour pre-filled.
- Callout: "86 transactions use this tag. Renaming or recoloring updates all of them immediately — nothing needs re-tagging."
- Buttons: Cancel · **Save**.

## 7d — Remove tag (dialog, 400px)
- Deliberately lighter than delete elsewhere: **no typed confirmation**, neutral (not red) styling.
- Title "Remove tag — one-off". Body 13px: "This tag is on **3 transactions**. They'll keep their amount, category and payee — only the `one-off` label comes off."
- Buttons: Cancel · **Remove tag** (primary dark).

## 7e — Merge tags (dialog, 460px)
- Title "Merge tags".
- Row of two selects with a "→" (16px `#9b9797`) between: **Merge this tag** (source; border `#ec3013`; options show counts, e.g. "Shared (9 txns)") → **into this tag** (target, e.g. "shared (86 txns)"). Source can't equal target.
- Callout (red border): "All **9 transactions** tagged "Shared" will be retagged "shared" instead, using shared's color. "Shared" is then deleted. This can't be undone, but the transactions themselves are never touched beyond their tag."
- Buttons: Cancel · **Merge into "shared"** (label follows the target).
- Entry points: duplicate link on 7a (pre-fills source/target) or manually from the command palette.

---

## Behaviour
- **List:** j/k select; enter opens Transactions filtered by tag; e edit; x remove; n new.
- **Duplicate detection:** flag tags whose names are equal after lower-casing and stripping punctuation/whitespace/hyphens (e.g. `Shared`/`shared`, `work trip`/`work-trip`). Suggest the tag with more transactions as the merge target.
- **Add:** name required; if a case-insensitive match exists, offer to reuse it instead of creating.
- **Edit:** rename/recolour updates every display immediately (tags are referenced by id).
- **Remove:** untag all transactions, delete the tag.
- **Merge:** for each transaction with source: add target if absent, remove source; then delete source. Single transaction/undo-safe operation.

## State
- `tags: Tag[]` — `{ id, name, color, txnCount, total, lastUsed, duplicateOf|null }`
- `selectedIndex`, `dialog: none | add | edit | remove | merge`, form draft `{ name, color }`
- Merge: `{ sourceId, targetId }`

## Design tokens
- Ink `#201e1d` · muted `#605d5d` · faint `#9b9797` · on-dark `#bab6b6` / `#d7d3d3`
- Paper `#f3f2f2` · chrome `#eae9e9` · canvas `#e2e0df` · row rule `#d7d3d3` · duplicate highlight `#faf3ef`
- Strong rule `rgba(32,30,29,.38)` · control border / scrim `rgba(32,30,29,.30)`
- Accent `#ec3013` · accent text `#ae1800`
- Tag swatches: `#ec3013` `#7a8a6b` `#c9922f` `#4a7c9e` `#8a6bb5` `#9b9797`
- Type: Archivo (400/800). Sizes 10 / 11 / 11.5 / 12 / 13 / 13.5 / 16 / 28px.
- Radius 0. Modal shadow `0 16px 48px rgba(32,30,29,.40)`.

## Assets
Icons are inline stroke SVGs (stroke 1.5) — see `Tags.html`. No images.

## Files
- `Tags.html` — screens 7a–7e
- `styles.css` — design-system stylesheet (`.btn`, `.tag` variants)
