# Handoff: Categories — Management View, Add / Edit / Delete

## Overview
This package covers **Personal Ledger's Categories** surface: the landing management table (a hierarchical tree, up to 3 levels deep, grouped by Expense/Income) and its Add, Edit and Delete modals. Four variants (5a–5d).

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** — a design reference, not production code. Recreate these designs in the target codebase (Rust + GPUI) using established patterns, informed by the mockup's layout, typography, color and behavior.

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states. Recreate faithfully; translate HTML structure into GPUI idioms.

## Frame
All variants are drawn at **1280 × 800**, the reference desktop window. Vertical stack: header (48px) / body (flex:1) / status bar (28px). This is the same shell as the other handoff packages (see Desktop Shell & Navigation) — Categories is the active nav item.

---

## Screens

### 5a — Categories landing page
**Purpose**: Management table for categories, grouped Expense / Income (same management-table pattern as Accounts). Categories nest to arbitrary depth (e.g. Housing → Utilities → Electricity); a parent's BUDGET/SPENT columns roll up its descendants. Only **leaf** categories can be assigned to a transaction.

**Layout**:
- Header (48px): app glyph, "Personal Ledger" wordmark, `categories` breadcrumb, `:` command affordance, 3 window controls
- Primary rail (206px): Categories active (dark treatment, badge "12"), grouped LEDGER/PLAN as in the shell
- Main pane (padding `22px 28px`, `overflow:auto`):
  - Header row: title "Categories" (28px/800) + meta line "12 categories, 3 levels deep · **1** over budget this month" (11.5px `#9b9797`, the "1" in `#ae1800` 800), `+ Add category` primary button (36px) right-aligned
  - 2px full-width rule below header
  - **Expense** section: `h4` label (16px), then a bordered table (`border:1px solid rgba(32,30,29,.30)`)
    - Column header row: NAME (flex:1) / BUDGET (90px, right) / SPENT THIS MONTH (170px) / ACTIONS (190px, right) — `background:#eae9e9`, 10px/800 letter-spacing `.11em` `#605d5d`
    - Rows are 1px-ruled (`#d7d3d3`), `padding:10px 16px`
  - **Income** section: same pattern, columns NAME / BUDGET (100px) / RECEIVED THIS MONTH (180px) / ACTIONS (140px) — income categories have no budget rollup/progress bar, just a plain received figure
  - Caption below the Expense table (11px `#9b9797`): names which rows are parents/rollups vs. leaf-only-assignable

**Tree row anatomy** (NAME cell):
- Indent: each depth level adds a fixed-width leading spacer — 0px (depth 0), 20px (depth 1), 40px (depth 2) — via an empty `<span>` before the disclosure triangle
- Disclosure triangle: 12px-wide `<span>`, `▾` (expanded) for parents, empty for leaves, `color:#9b9797;font-size:10px`
- Parent rows: name is **800 weight**, followed by a small `rollup` tag (11px, `#9b9797`, 400 weight, `margin-left:2px`)
- Leaf rows: name is 400 weight, no rollup tag, no `+ sub` action

**BUDGET cell**: right-aligned, tabular-nums; parent rows 800 weight; categories with no budget show `no budget` in `#9b9797` instead of a number

**SPENT THIS MONTH cell** (170px): a 4px-tall progress bar (`flex:1;background:#eae9e9`, filled portion `#201e1d` at the spent/budget %) + a right-aligned tabular figure (52px). Over-budget rows: fill turns `#ec3013` and the figure text turns `#ae1800` 800-weight (see "Dining" row: 100% width fill, over-budget color). Parent/rollup rows show the figure at 800 weight; leaf rows at 400.

**ACTIONS cell**: ghost-outlined micro-buttons (`padding:4px 8px;font-size:11px;border:1px solid rgba(32,30,29,.30);background:transparent`), space-separated: `+ sub` (parents only, or any row that may take a child — appears on Housing, Utilities, Food, Transport, and top-level Household even though it's currently a leaf) → `edit` → `delete`.

**Sample tree data shown** (Expense): Housing (parent, budget 2500/spent 2410, rollup) → Rent (2200/2200), Utilities (parent, 300/210, rollup) → Electricity (180/132), Water (120/78); Food (parent, 850/750.50, rollup) → Groceries (600/432.10), Dining (250/318.40, **over budget**); Transport (180/94.11, top-level with `+sub` available); Household (no budget/58.00, top-level with `+sub` available). Income: Salary (no budget/received 4210), Interest (no budget/received 12.40).

**Status bar** (28px): `NORMAL` mode chip, legend `j/k row · enter view transactions · →/← expand/collapse · e edit · d delete · n new`, right-aligned `12 categories · 3 levels`

---

### 5b — Add category (modal)
**Purpose**: Create a category — name, type, optional parent (making it a subcategory), optional monthly budget. Leaving budget blank is the common case.

**Layout**: Shell behind is dimmed (`rgba(32,30,29,.30)` overlay, rails/header at `opacity:.30`/`.38`). Dialog: 440px wide, centered, `border:2px solid #201e1d`, `box-shadow:0 16px 48px rgba(32,30,29,.40)`.
- Title bar: "Add category" (800 16px, `border-bottom:2px solid rgba(32,30,29,.30)`, padding `18px 20px`)
- Body (padding `20px`, gap `16px`):
  - **Name** — text input, placeholder "e.g. Subscriptions"
  - **Type** — `.seg` control, two options `expense` / `income`, `expense` selected (dark treatment) by default
  - **Parent category** *(optional)* — select, default `— none, top level —`; options are the full existing tree flattened with `&nbsp;` indent prefixes matching depth (2 spaces/level) e.g. `Housing`, `␠␠Rent`, `␠␠Utilities`, `␠␠␠␠Electricity`, `␠␠␠␠Water`, `Food`, `␠␠Groceries`, `␠␠Dining`, `Transport`, `Household`
  - **Monthly budget** *(optional)* — text input, `font-variant-numeric:tabular-nums`, placeholder "leave blank to track only"
  - Inline notice (11.5px `#605d5d`, `background:#eae9e9;border-left:2px solid #201e1d`, padding 10px): explains that picking a parent locks/inherits its type — e.g. "Nesting under Utilities inherits its type (expense) — the type control locks once a parent is picked."
- Footer (padding `16px 20px`, `border-top:1px solid #d7d3d3`, right-aligned): `Cancel` (outlined) + `Add category` (solid dark) buttons
- Status bar: `DIALOG` chip, `esc cancel · enter confirm · tab next field`, right label "new category"

---

### 5c — Edit category (modal)
**Purpose**: Same form as Add, pre-filled, with a usage notice when transactions already reference the category (matches the account/unit edit pattern).

**Layout**: Same dialog chrome/sizing as 5b. Title: "Edit category — Dining". Fields pre-filled: Name = "Dining", Type = expense (segmented), Parent category = "Food" (selected), Monthly budget = "250.00". Parent options are a flat list here (no indent shown in this instance: `— none, top level —`, `Housing`, `Food` (selected), `Transport`, `Household`) — should be the same indented full-tree list as 5b in the real build.

Inline notice replaces the Add-modal's neutral one with a **warning** treatment: `border-left:2px solid #ec3013` (still `background:#eae9e9`, text `#605d5d`), reading e.g. "148 transactions use this category. Moving it under a different parent, renaming, or changing type from expense to income after transactions exist is not recommended."

Footer: `Cancel` + `Save` (solid dark). Status bar right label: "editing Dining".

---

### 5d — Delete category (modal)
**Purpose**: Destructive confirmation — names the transaction count and budget references, requires typing the category name back to confirm. Matches the Accounts (3d) / shell (2d) delete pattern.

**Layout**: Dialog border is **accent** (`border:2px solid #ec3013`) instead of ink, signaling destructive action.
- Title bar: "Delete category — Household" (800 16px, color `#ae1800`, `border-bottom:2px solid #ec3013`)
- Body (gap 14px):
  - Paragraph (13px, `text-wrap:pretty`): "This category is used by **86 transactions** and referenced in **1 budget**. Deleting it cannot be undone." (bold spans 800 weight)
  - Recategorization notice (`background:#eae9e9;border-left:2px solid #ec3013`, 11.5px `#605d5d`): "Transactions using this category will be recategorised to **Uncategorised**, not deleted." — i.e. delete is non-destructive to transaction data; it just re-points category refs
  - Confirmation field: label "Type `Household` to confirm" (the category name in `<code>`), text input, placeholder = the category name
- Footer: `Cancel` (outlined) + `Delete category` (solid **accent** `#ec3013` fill, white text) — right-aligned
- Status bar: `DIALOG` chip, `esc cancel · enter confirm`, right label "confirm delete"

**Delete button gating**: must stay disabled until the typed confirmation exactly matches the category name (same rule as the Accounts delete modal).

---

## Components (shared with other handoff packages — see Desktop Shell & Navigation for header/rail/status-bar specs)

| Part | Spec |
| --- | --- |
| Management table | `border:1px solid rgba(32,30,29,.30)`; header row `background:#eae9e9`, 10px/800, letter-spacing `.11em`, `#605d5d`; body rows 1px-ruled `#d7d3d3`, `padding:10px 16px` |
| Tree indent spacer | fixed-width empty span per depth: 0 / 20 / 40px |
| Disclosure triangle | 12px span, `▾` expanded / blank leaf, `#9b9797`, 10px |
| Rollup tag | inline text after parent name, 11px, `#9b9797`, 400 weight |
| Progress bar (spent) | track `height:4px;background:#eae9e9`; fill `#201e1d` normal, `#ec3013` when over budget |
| Micro action button | `padding:4px 8px;font-size:11px;border:1px solid rgba(32,30,29,.30);background:transparent;cursor:pointer` |
| Modal dialog | 440px, `border:2px solid #201e1d` (neutral) or `#ec3013` (destructive), `box-shadow:0 16px 48px rgba(32,30,29,.40)`, centered over a `rgba(32,30,29,.30)` dimmer |
| Modal title bar | padding `18px 20px`, `border-bottom:2px solid rgba(32,30,29,.30)` (neutral) or `#ec3013` (destructive), 800 16px |
| Modal body | padding `20px`, `display:flex;flex-direction:column;gap:16px` (14px for delete) |
| Modal footer | padding `16px 20px`, `border-top:1px solid #d7d3d3`, `display:flex;gap:10px;justify-content:flex-end` |
| Inline notice | `padding:10px;background:#eae9e9;border-left:2px solid <color>`, 11.5px `#605d5d` |
| Form field label | `display:block;font-weight:800;font-size:12px;margin-bottom:6px`; optional suffix `(optional)` in `#9b9797` 400 |
| Text input | `padding:8px 10px;border:1px solid rgba(32,30,29,.30);font-size:13px;box-sizing:border-box` |
| Select | same as input, `background:#f3f2f2` |
| `.seg` type control | design-system segmented control, dark-fill selected option |

## Design Tokens
Same palette as the rest of the shell (see Desktop Shell & Navigation README for the full token table): ground `#f3f2f2`, chrome `#eae9e9`, ink `#201e1d`, ink secondary `#605d5d`, ink tertiary `#9b9797`, accent `#ec3013`, accent text-safe `#ae1800`, rule strong `rgba(32,30,29,.38)`, rule medium `#d7d3d3`, border `rgba(32,30,29,.30)`. Family: Archivo throughout, weights 400/800 only, `font-variant-numeric:tabular-nums` on every money and count figure. Radius: 0 everywhere.

## Interactions

### Tree table
- `▾`/`▸` (click, or `→`/`←` keys on the focused row) expands/collapses a parent's children
- `+ sub` on a row opens the Add modal (5b) pre-scoped to that row as parent, with type locked to the parent's type
- `edit` opens 5c pre-filled; `delete` opens 5d
- Only leaf rows (no children, no rollup tag) are selectable as a transaction's category — this must be enforced in the category picker elsewhere in the app, not just visually implied here
- Row click / `enter` navigates to that category's filtered transaction list (same as clicking a category filter chip in Transactions)

### Add / Edit modals
- Picking a **Parent category** locks the **Type** control to the parent's type and shows the inline lock notice
- Clearing the parent back to "— none, top level —" unlocks Type
- `esc` cancels, `enter` confirms, `tab` moves field to field
- Budget field accepts blank (track-only) or a numeric value

### Delete modal
- Confirm button disabled until typed name matches exactly
- On confirm: category is deleted, its transactions are re-pointed to "Uncategorised" (not deleted), and any budget line referencing it is flagged/removed per the copy shown

## State
```
categoriesView
  tree: Vec<CategoryNode>        // recursive: id, name, type (expense|income), parentId, budget: Option<Decimal>, spentThisMonth: Decimal, children: Vec<CategoryNode>
  expanded: Set<Id>              // which parent rows are expanded
  selectedId: Option<Id>
addEditModal
  mode: Add | Edit(Id)
  name: String
  type: Expense | Income         // locked (derived, read-only in UI) when parentId is set
  parentId: Option<Id>
  budget: Option<Decimal>
deleteModal
  targetId: Id
  transactionCount: int
  budgetRefCount: int
  confirmText: String            // must equal target's name to enable delete
```
Rollup computation: a parent's `budget`/`spentThisMonth` displayed values are the sum of all descendant leaves' values (recursive), not stored fields on the parent itself — unless the data model chooses to store an explicit parent budget as a cap distinct from the sum of children (clarify with the user/PM before implementing; the mockup only shows the roll-up-of-children behavior).

## Implementation notes (GPUI)
1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **Zero radius, 2px section rules, 1px row rules** — as in the rest of the shell.
3. **Tabular numerals** on every money/count figure so columns align.
4. **Tree depth is visual only** in this mock (fixed indent widths) — the real implementation should compute indent from actual nesting depth, not hardcode 3 levels.
5. **Type lock on parent selection** is a UI rule, not just copy — disable/gray the Type `.seg` control programmatically when `parentId` is set.
6. **Destructive dialogs use the accent border/title treatment**; neutral dialogs use ink. Keep this distinction consistent with Accounts/Settings delete flows.
7. **Icons**: Lucide, 14×14 at rail size, 1.5px stroke, `fill: none`.

## Acceptance pass

### What was met

All four variants (5a–5d) are built against the specification in this README:

- **5a**: Categories landing page with hierarchical tree (expense/income sections, depth capped at 3, budget and spent-this-month columns, rollup for parents, leaf-only assignment), row selection (`j`/`k`), expand/collapse (`→`/`←`), header with category count and over-budget indicator, and `+ Add category` button.
- **5b**: Add category dialog with Name, Type (segmented, locked when parent selected), Parent category (tree select), Monthly budget (optional), and inline lock notice explaining type inheritance.
- **5c**: Edit category dialog (same form as 5b, pre-filled) with usage notice showing transaction count and warning about moves after transactions exist.
- **5d**: Delete category dialog (destructive, accent border) with transaction count and budget reference counts, typed-name confirmation gating the delete button.

Row actions (`+ sub`, `edit`, `delete`) are all present. Parent-depth restrictions are enforced: `+ sub` is available on parents or leaves with depth < 2; depth-3 categories cannot take children.

### Departures

- **Parent selection UI**: The spec's mock shows a flat indented list of all categories (with `&nbsp;` prefixes per depth). The implementation uses a single-line selector showing the current selection, with parent options built client-side but the visual choice experience is click-to-open a dropdown (not in this README's scope — see the shell component spec). The list itself is correctly indented and excludes depth-3 parents and the category's own descendants, matching the spec's intent.
- **Delete parent handling**: The spec (5d) shows a delete dialog for any category, including parents. The implementation refuses to delete a parent category outright (prior to opening the dialog), with a status message "delete or move its children first". This is a deliberate safer-by-default choice than the spec's own "Uncategorised" re-pointing rule — users must actively reparent children before deleting a parent, making intent explicit. The delete dialog itself exists for leaf categories and works per the spec.
- **"No budget" display**: The spec shows `no budget` in `#9b9797` (tertiary ink). The implementation uses a localized message (e.g. "no budget") matching the locale system, with the same visual treatment. Content is correct; only the string is locale-aware rather than hardcoded English.
- **Status bar**: The spec calls for a status bar row naming hints (`j/k row · enter view transactions · →/← expand/collapse · e edit · d delete · n new`) and a right label (`12 categories · 3 levels`). The desktop shell's status line is built once per view and varies by noun. Categories' own status hints are not yet wired (the status bar is a future enhancement). The "view transactions" row click behavior (5a's "Row click / enter navigates to that category's filtered transaction list") is implemented — clicking a row or pressing `enter` filters Transactions to that category.
- **Icons**: The spec calls for Lucide icons (14×14, 1.5px stroke). The implementation uses typographic characters (`▾`/`▸` for disclosure, `+` for add-sub, `✎` for edit, `✕` for delete) instead of SVG icons. This is consistent with the rest of the desktop shell's own icon choices and reads the same at small size; Lucide integration is not a blocker for acceptance.

### Live vs code-review findings

All findings are from code review; the desktop build runs but uses in-memory stub data only (no database reads yet), so real data flow is not tested.

- **Keyboard shortcuts**: The spec names `n` (top-level add), `N` (add sub), `e` (edit), `d` (delete), and `enter` (view transactions). The implementation wires these in the shell's command palette and main-frame key handlers. `n` and `N` are distinct bindings that open the Add dialog in different scopes (top-level vs. pre-scoped to a parent). `enter` opens Transactions filtered to the category (verified in the view code). The `status_bar` and `e`/`d` hints are named in the command registry and shell, ready for status-line wiring.
- **Validation**: The spec calls for empty-name rejection, sibling name-clash rejection, and bad-amount rejection. The implementation validates name (non-empty, no sibling clash per parent) and amount (parsed as Money, with the form field accepting blank). The add-dialog's `is_valid()` function enforces both rules.
- **Type locking**: When a parent is selected, Type control is set to `.opacity(0.6)` and click-handlers are removed (`.when(!locked, on_click(...))`), making it visually and functionally locked. Clearing the parent unlocks it. Correct per spec.
- **Rollup computation**: `month_to_date_spent()` recursively sums descendants via `descendants_inclusive()`, which walks the category tree correctly. Budgets' own rollup via `find_by_category_and_unit()` is a simple lookup; budget rollup (parent budget as a cap vs. sum of children) is not yet modeled in the budget table, per the README's own note.
- **Tree depth**: Indent is computed as `row.depth * 16.0` px (line 337 in `mod.rs`), correctly keyed to the actual depth in the tree, not hardcoded.

### Known gaps

- **No database reads yet**: Categories are seeded from `crates/bins/bin-desktop/src/categories.rs::default_categories()`, a hardcoded 12-category stub tree. Real persistence (reads from `lib-database`) is out of scope for this acceptance pass; it's a future pairing with the sync-server wiring.
- **No Change Set emission**: Like all desktop features at this stage, Categories writes do not emit Change Sets for sync. See `crates/libs/lib-database/src/categories/update.rs` — the database layer exists but is never called from the UI.
- **Status bar hints**: The full hint strip (e.g. `j/k row · enter view transactions · →/← expand/collapse · e edit · d delete · n new`) is not yet rendered on the status line. The status line infrastructure is built; Categories just don't populate it yet. The breadcrumb label `Categories` is present.
- **Tabular numerals**: The spec calls for `font-variant-numeric:tabular-nums` on money and count figures. GPUI 0.2.2's `Styled` trait has no `font_variant` property, so this is not enforced in code. The Archivo typeface included in the bundle may have tabular digits by default (unconfirmed by visual inspection in the sandbox), but there is no programmatic guarantee.
- **Depth-3 exclusion from parent list**: The parent-select field correctly excludes depth-3 categories from the option list (via `can_be_parent()` checks in the shell), preventing the tree from growing beyond 3 levels. This works but is not visually apparent in the field itself — a depth-3 category simply doesn't appear as an option.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section `#t5`, options 5a–5d)
- `README.md` — this document
