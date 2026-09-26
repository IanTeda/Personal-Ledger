# Handoff: Payees (screens 6a–6e)

## Overview
Payee management for the Personal Ledger desktop app: a flat list of payees, Add / Edit / Delete dialogs, and the payee-matching step of bank-statement import. Each payee has an optional **default category** (auto-fills new transactions) and a set of **match rules** (raw statement strings that map to this payee on import).

## About the design files
`Payees.html` is a **design reference built in HTML** — it shows intended look and behaviour, not production code. Recreate these screens in the target codebase's existing environment and patterns (e.g. GPUI/Rust desktop), or pick the most suitable framework if none exists. Open `Payees.html` in a browser; each screen is a 1280×800 frame labelled 6a–6e. `styles.css` is the Modernist design-system stylesheet the frames use for `.btn` / `.tag` classes.

## Fidelity
**High-fidelity.** Colours, type, spacing and copy are final. Match them exactly.

---

## App shell (shared by all screens)
- **Title bar** 48px, bg `#eae9e9`, bottom border 2px `rgba(32,30,29,.38)`, padding `0 10px 0 12px`, gap 12px. Contents: 16px panel icon · "Personal Ledger" 13.5px/800 · context word 12px `#9b9797` ("payees", or "transactions › import statement.csv" on 6e) · spacer · command box ("**:** run a command", 12px, 1px border `rgba(32,30,29,.30)`, padding 4px 9px) · three 26×26 window buttons (bg `rgba(32,30,29,.08)`, gap 2px).
- **Sidebar** 206px, bg `#eae9e9`, right border 2px `rgba(32,30,29,.38)`, padding `14px 0 10px`. Section labels ("LEDGER", "PLAN") 10px/800 Archivo, letter-spacing .11em, `#9b9797`. Items: padding 7px 14px, gap 9px, 14px line icon (stroke 1.5), label, shortcut hint 11px `#9b9797` (g d, g l, g a, g c, g p, g t, g w, g b, g r, g s). Active item: bg `#201e1d`, text `#f3f2f2`, weight 800, hint `#bab6b6`, count badge inverted (bg `#f3f2f2`, text `#201e1d`, 10px/800, padding 1px 5px). Accounts has an alert badge (bg `#ec3013`). Settings pinned to bottom under a 2px `rgba(32,30,29,.20)` rule.
- **Content** padding 22px 28px.
- **Status bar** 28px, bg `#eae9e9`, top border 2px `rgba(32,30,29,.38)`, 11.5px `#605d5d`. Left: mode pill (NORMAL / DIALOG / IMPORT — bg `#201e1d`, text `#f3f2f2`, 10px/800, letter-spacing .1em, padding 2px 7px), then key hints (keys in `#201e1d`/800). Right: context summary.
- **Modal state:** title bar dimmed to opacity .38, sidebar and content to .30; scrim `rgba(32,30,29,.30)` over the content area; dialog centred.

## Dialog pattern (6b, 6c, 6d)
- Panel bg `#f3f2f2`, border 2px `#201e1d` (danger: `#ec3013`), shadow `0 16px 48px rgba(32,30,29,.40)`, no radius.
- Header: padding 18px 20px, 16px/800 Archivo, bottom border 2px `rgba(32,30,29,.30)` (danger: `#ec3013`, title `#ae1800`).
- Body: padding 20px, column gap 16px (14px on delete).
- Labels 12px/800, margin-bottom 6px; optional hint "(optional)" `#9b9797` weight 400.
- Inputs/selects: full width, padding 8px 10px, 1px `rgba(32,30,29,.30)`, 13px, bg `#f3f2f2`.
- Callout: 11.5px `#605d5d`, padding 10px, bg `#eae9e9`, left border 2px `#201e1d` (warning/usage: `#ec3013`).
- Footer: padding 16px 20px, top border 1px `#d7d3d3`, right-aligned, gap 10px. Secondary: 1px `rgba(32,30,29,.30)`, transparent, 800. Primary: bg `#201e1d`, text `#f3f2f2`, 800. Destructive: bg `#ec3013`. Button padding 8px 16px.

---

## 6a — Payees list
**Purpose:** browse and manage all payees.

- Header row: "Payees" 28px/800; subline 11.5px `#9b9797` — "24 payees · **3** without a default category" (count in `#ae1800`/800). Right: primary "+ Add payee" (36px high). Margin-bottom 24px, then a 2px `rgba(32,30,29,.38)` rule, margin-bottom 24px.
- Table: outer border 1px `rgba(32,30,29,.30)`. Header row bg `#eae9e9`, 10px/800 Archivo, letter-spacing .11em, `#605d5d`, padding 10px 16px.
- Columns: NAME (flex) · DEFAULT CATEGORY 130px · MATCH RULES 100px · TRANSACTIONS 100px right · TOTAL 110px right · ACTIONS 150px right. Numbers use tabular figures.
- Rows: padding 10px 16px, bottom border 1px `#d7d3d3`. Secondary text `#605d5d`, rule count `#9b9797`.
- Selected row: bg `#201e1d`, text `#f3f2f2`, name 800, category `#d7d3d3`, rules `#bab6b6`, total 800; action buttons get border `rgba(243,242,242,.35)` and light text.
- Missing default category: shows "no default category" in `#ae1800`/800.
- Row actions: "edit", "delete" — 11px, padding 4px 8px, 1px border.
- Footnote 11px `#9b9797`: payees without a default category require picking one each time; match rules drive import renaming (6e).
- Sample data: Woolworths (Groceries, 3 rules, 142, 3,204.18) · Coles · Netflix · BP · Origin Energy · Officeworks · Employer Pty Ltd · J Smith (none, 0 rules, 4, 640.00).
- Status bar: NORMAL · "j/k row · enter view transactions · e edit · d delete · n new" · right "24 payees · 3 without default category".

## 6b — Add payee (dialog, 460px)
- Title "Add payee".
- **Name** — placeholder "e.g. Aussie Candle Co".
- **Default category (optional)** — select, default "— none, ask every time —", then the leaf categories.
- **Match rules (optional)** — chip row (empty state: neutral chip "no rules yet"), then a monospace input (12.5px, placeholder "e.g. AUSSIE CANDLE") + "+ add" button (padding 8px 12px, 12px/800) with a 6px gap.
- Callout (dark border): rules match case-insensitively against the raw statement description; add one per format (e.g. `WOOLWORTHS`, `WW SUPERMARKET`).
- Buttons: Cancel · **Add payee**. Status bar: DIALOG · "esc cancel · enter confirm · tab next field".

## 6c — Edit payee (dialog, 460px)
- Title "Edit payee — Woolworths". Fields pre-filled.
- Existing rules as accent chips (`.tag-accent`, monospace 11.5px, padding 3px 8px) each with a "✕" remove affordance. Input placeholder "add another format…".
- Callout (red border): "142 transactions use this payee. Changing the default category only affects new transactions — existing ones keep whatever category they were assigned at the time."
- Buttons: Cancel · **Save**.

## 6d — Delete payee (danger dialog, 440px)
- Title "Delete payee — J Smith" in `#ae1800`; panel and header border `#ec3013`.
- Body text 13px: "This payee is used by **4 transactions**. Deleting it cannot be undone."
- Callout (red border): transactions keep amount and category but show **(no payee)**; its **N match rules** are also removed.
- Type-to-confirm: label "Type `J Smith` to confirm", input placeholder "J Smith". **Delete** stays disabled until the text matches exactly.
- Buttons: Cancel · **Delete payee** (bg `#ec3013`).

## 6e — Import: match payees (step 2 of 3)
**Purpose:** review how each raw statement line maps to a payee and category before committing.

- Sidebar active item: Transactions. Content has no outer padding; sections pad 28px horizontally.
- Stepper 11px/800, letter-spacing .08em: "1 upload — **2 match payees** — 3 confirm" (done `#201e1d`, current `#ae1800`, upcoming `#9b9797`).
- Heading "Match payees" 26px/800; subline "18 rows from statement.csv — review how each description maps to a payee and category before importing". Section bottom border 2px `rgba(32,30,29,.38)`.
- Table (scrolls): header 10px/800 `#9b9797`, padding 8px 28px. Columns: RAW DESCRIPTION (flex, monospace 11.5px `#605d5d`) · PAYEE 150px · CATEGORY 130px · STATUS 110px · AMOUNT 90px right. Rows padding 9px 28px, border 1px `#eae9e9`.
- **Rule match** rows: payee bold, category `#605d5d`, neutral tag "rule match" (10px, padding 2px 7px).
- **Unmatched** rows: bg `#eae9e9`; payee select offers `+ create "<cleaned name>"` first then existing payees; category select "choose category…" with red 1px `#ec3013` border until chosen. Status: accent tag "new payee" (a suggestion exists) or outline tag "needs review" (border `#ec3013`, text `#ae1800`).
- Footer bar (bg `#eae9e9`, top border 2px, padding 11px 28px, gap 14px): "18 rows · 14 matched by rule · 2 new payees to create · **2 need review**" · checkbox (checked) "remember new payees' rules for next time" · back (ghost) · **continue** (primary, hint "enter").
- Status bar: IMPORT · "j/k row · enter accept suggestion · n create new payee" · right "2 rows need review before continuing".

---

## Behaviour
- **List:** j/k moves selection; enter opens Transactions filtered to the payee; e edit; d delete; n new; `:` command palette. Sidebar badge = payee count.
- **Add:** name required and unique (case-insensitive). Rules are trimmed, upper-cased for display, deduplicated. Enter in the rule input adds the chip instead of submitting.
- **Edit:** changing default category applies only to future transactions. Removing a rule doesn't change past transactions.
- **Delete:** confirm requires exact name. Transactions keep amount/category, payee becomes null; rules are deleted.
- **Import matching:** for each row, case-insensitive *contains* test of each rule against the raw description; first match wins → payee + its default category. No match → suggest a cleaned name (strip prefixes like `SP`, `TFR TO`, store numbers, locations, reference text), status "new payee"; if payee or category is still unresolved the row is "needs review". **continue** is disabled while any row needs review. On continue: create new payees; if "remember" is checked, add each raw description's cleaned token as a match rule on the new payee.

## State
- `payees: Payee[]` — `{ id, name, defaultCategoryId|null, rules: string[], txnCount, total }`
- `selectedIndex`, `dialog: none | add | edit | delete`, dialog form draft, `deleteConfirmText`
- Import: `rows: { raw, amount, payeeId|null, newPayeeName|null, categoryId|null, status: 'rule'|'new'|'review' }[]`, `rememberRules: bool`

## Design tokens
- Ink `#201e1d` · muted `#605d5d` · faint `#9b9797` · on-dark faint `#bab6b6` · on-dark muted `#d7d3d3`
- Paper `#f3f2f2` · chrome `#eae9e9` · canvas `#e2e0df` · row rule `#d7d3d3`
- Strong rule `rgba(32,30,29,.38)` · control border `rgba(32,30,29,.30)` · scrim `rgba(32,30,29,.30)`
- Accent `#ec3013` · accent text `#ae1800`
- Type: Archivo throughout (400/800); monospace `ui-monospace` for raw descriptions and rules. Sizes 10 / 11 / 11.5 / 12 / 12.5 / 13 / 13.5 / 16 / 26 / 28px.
- Radius 0 everywhere. Modal shadow `0 16px 48px rgba(32,30,29,.40)`.

## Assets
Icons are inline 16/14px stroke SVGs (stroke 1.5) — see `Payees.html`. No images.

## Files
- `Payees.html` — screens 6a–6e
- `styles.css` — design-system stylesheet (`.btn`, `.btn-primary`, `.btn-ghost`, `.tag`, `.tag-neutral`, `.tag-accent`, `.tag-outline`)
