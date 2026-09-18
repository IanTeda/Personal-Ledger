# Handoff: Settings Surface

## Overview
This package contains the design for **Personal Ledger's settings surface** — a two-rail settings destination with a single scrolling body, plus the four modal dialogs that handle entity management (add / edit / delete unit, add institution). Five variants (2a–2e) cover the resting state and each dialog.

The governing idea: **Settings is a destination, not a mode.** The primary rail stays expanded and marks Settings as the active section, so the user never "falls into" a modal settings window. A second rail acts as the page's own index.

A second distinction runs through the content: **Configuration vs Preferences.** Configuration is client-scoped and read from `personal-ledger.conf` at start-up (Display, Sync server, Data & backup, Tracing). Preferences are ledger-scoped and sync as Change Sets (General, Units, Institutions). The UI must make that difference legible — each section header carries a scope note on the right.

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** showing intended look, layout, and interaction model. It is a **design reference, not production code**. Recreate these designs in the target codebase (Rust + GPUI).

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states.

## Frame
All variants drawn at **1280 × 800**. The shell is header (48px) / body (flex:1) / status bar (28px). In 2b–2e the shell behind the modal is drawn at `opacity: .38` in the mockup to focus the dialog — in the real app the shell renders at full opacity behind a dimmer.

---

## Screens

### 2a — Resting state
**Purpose**: The clean landing state. Primary rail expanded with Settings active; the settings index in the second rail; General as the landing pane; no modal.

**Layout**:
- **Header** (48px): breadcrumb `settings › general`
- **Primary rail** (206px): the full app nav (LEDGER / PLAN groups), Settings pinned at the bottom in the active dark treatment
- **Settings index rail** (214px):
  - Filter box: `/ filter`, padding `22px 12px 25.4px`, closed by a 2px rule — this padding is tuned so the rule under the filter **aligns exactly with the 2px rule under the "Settings" page heading**
  - `SETTINGS` label, then eight entries in order: **General, Display, Units, Institutions, Sync server, Data & backup, Tracing (Logs), About**
  - Footer note, above a 1px rule: "preferences sync · configuration local"
- **Body** (flex:1): `padding: 22px 28px; overflow: auto` — one continuous scroll, not a pane switcher
  - Page heading "Settings" (28px/800) + scope note "preferences · synced", then a 2px rule
  - Then each section in order, every one built the same way: `h4` (20px) + right-aligned scope note, a 2px rule (`margin: 14px 0 18px`), then content, closing with **48px** before the next section
- **Status bar** (28px): unchanged from the shell

**Sections, in scroll order** (Display sits directly under General — Configuration is surfaced early, right after ledger identity):

| Section | Scope note | Content |
| --- | --- | --- |
| General | ledger identity | Ledger name, Owner, Financial year starts, Base unit + a THIS LEDGER summary panel |
| Display | client-scoped · never synced | Date format, decimal separator, row density, status glyphs — beside a live PREVIEW table |
| Units | 3 units · synced | One combined section — no separate "Ledger & units" pane. A "UNITS" table title, then the units table, then a **Price Sources** subsection |
| Institutions | 7 institutions · synced | Table (INSTITUTION / ACCOUNT TYPE) with per-row edit/delete, then **+ Add institution** |
| Sync server | synced · every 30 seconds | Server URL, status (green dot + "connected"), last sync, **Sync now** |
| Data & backup | local files · aud · 2.84 mb | Store location, last backup, **Backup now**, **Export ledger (CSV)** |
| Tracing (Logs) | diagnostic · last 1000 entries | Level radios (error/warn/info/debug), a monospace log viewport (120px, scrolling), **Clear logs** |
| About | version info | Version, release date, build stack |

**Units section detail** — there is no longer a standalone "Default unit for new entries" or "Budget period" control here; budget period moved to become a per-budget setting and does not belong in Settings.

- Table title label: "UNITS" (`font:800 10px/1 'Archivo'; letter-spacing:.11em; color:#9b9797`), same treatment as "PRICE SOURCES" and "PREVIEW" below it
- Units table columns, in order: **CODE (100px) → NAME (180px) → FLAGS (flex:1, left-aligned) → SOURCE (130px) → TYPE (80px) → ACTIONS (120px)**
  - FLAGS: tag pills, `class="tag tag-accent"` for **base**, `class="tag tag-outline"` for **default** — only aud carries them (it is both the ledger's base unit and the default unit for new entries); other rows leave the cell empty. Tags sit immediately left-aligned next to NAME, since NAME is a fixed 180px column (not flex) — flags never drift into empty middle space.
  - SOURCE: the price source shorthand — aud shows **USD** (its rate is sourced against USD as the reference currency), btc shows **CoinGecko**, vas shows **Manual entry**
  - Rows: aud / Australian Dollar / currency, btc / Bitcoin / crypto, vas / Vanguard Aus Shares / etf
  - Row action buttons: edit, delete
- **+ Add unit** button below the table
- **Price Sources** subsection (own "PRICE SOURCES" label, `margin-top:32px`):
  - Table columns: **NAME (130px) → SOURCE (flex:1) → LAST UPDATED (130px) → ACTIONS (170px)**
  - The NAME column shows the unit's name, not its code (Bitcoin, Vanguard Aus Shares) — this is the one place the unit is referenced by name rather than code, since the table is about the source relationship, not the unit record itself
  - Rows: Bitcoin / "CoinGecko — auto, every 15 min" / "5 minutes ago"; Vanguard Aus Shares / "Manual entry" / "—"
  - Row action buttons: **test**, edit, delete — `test` checks the source connection/response without editing it, and sits first (left of edit/delete) in the action group
  - **+ Add price source** button below the table

**Keybindings are not in Settings** — they are statically set in the configuration file and deliberately absent from this surface.

**Scroll**: the body scrolls; sections stay in the DOM. The index rail scrolls to them.

### 2b — Add unit
Same surface, with the **Add unit** dialog open over it. Fields: Code, Name, Type (select: currency / cryptocurrency / custom). Actions: Cancel, Add.

### 2c — Edit unit
The same form as add, **pre-filled** (`aud` / Australian Dollar / currency), plus a usage notice so the cost of a code change is visible before committing:

> Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references.

Notice style: `padding: 10px; background: #eae9e9; border-left: 2px solid #ec3013; font-size: 11.5px; color: #605d5d`. Actions: Cancel, Save.

### 2d — Delete unit (destructive confirm)
References are **named**, and the code must be **typed back** — so deletion cannot happen by muscle memory.

- Dialog border and header rule: `2px solid #ec3013`; title in `#ae1800` — "Delete unit — btc"
- Warning copy, then a reference panel (`background:#eae9e9; border-left:2px solid #ec3013`) naming each affected record, e.g. "Bitcoin · 0.4120 u held in Bitcoin account"
- Confirmation field: label "Type btc to confirm", empty text input
- Actions: Cancel, **Delete unit** (`background:#ec3013; color:#f3f2f2`) — **disabled until the typed value matches the code exactly**

### 2e — Add institution
Account types are **multi-select chips**, because one institution usually carries several. Fields: Institution name, Account types (savings / credit card / offset / loan / investment), Default unit (select). Actions: Cancel, Add institution.

---

## Components

| Part | Spec |
| --- | --- |
| Settings index rail | 214px, `border-right: 2px solid rgba(32,30,29,.38)`, `padding: 0 0 10px` |
| Filter box | `height: 30px; min-height: 30px; font-size: 12.5px`, wrapper `padding: 22px 12px 25.4px` + 2px bottom rule |
| Index entry | `padding: 8px 14px` |
| Index entry (active) | `background:#201e1d; color:#f3f2f2; font-weight:800` |
| Rail label | `font: 800 10px/1 'Archivo'; letter-spacing: .11em; color: #9b9797; padding: 10px 12px 4px` |
| Body | `flex:1; min-width:0; padding: 22px 28px; overflow: auto` |
| Page heading | `h2`, 28px/800, with an 11.5px `#9b9797` scope note baseline-aligned right |
| Section heading | `h4`, 20px, same right-aligned scope note |
| Section rule | `height:2px; flex:none; background:rgba(32,30,29,.38); margin:14px 0 18px` |
| Section gap | **48px** bottom margin — every section, no exceptions |
| Subsection label | `font:800 10px/1 'Archivo'; letter-spacing:.11em; color:#9b9797; margin-bottom:10px` — used for "UNITS", "PRICE SOURCES", "THIS LEDGER", "PREVIEW" |
| Field label | `display:block; font-weight:800; font-size:12px; margin-bottom:6px` |
| Input / select | `width:100%; padding:8px 10px; border:1px solid rgba(32,30,29,.30); font-size:13px; box-sizing:border-box`; selects add `background:#f3f2f2` |
| Field column | `width:320px; flex:none; gap:16px`; two columns sit `gap:40px` apart |
| Tag (flags) | `.tag.tag-accent` (filled accent) for **base**; `.tag.tag-outline` for **default** — from the Modernist bundle, not hand-rolled |
| Table header | `padding:12px 16px; background:#eae9e9; font:800 10px/1 'Archivo'; letter-spacing:.11em; color:#605d5d`, 1px bottom rule |
| Table row | `padding:12px 16px; border-bottom:1px solid #d7d3d3` (last row omits it); primary cell 800 weight |
| Row action button | `padding:4px 10px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent` |
| Section button | `padding:8px–10px 16px`, `border:1px solid rgba(32,30,29,.30); background:#eae9e9; font-weight:800; width:fit-content` |
| Info panel | `padding:10–12px; background:#eae9e9; border-left:2px solid #ec3013; font-size:11.5px` |
| Log viewport | `border:1px solid rgba(32,30,29,.30); background:#eae9e9; padding:10px; height:120px; overflow-y:auto; font-family:monospace; font-size:10px; line-height:1.5` |

### Dialog

| Part | Spec |
| --- | --- |
| Dimmer | `position:absolute; inset:0; background:rgba(32,30,29,.30); display:flex; align-items:center; justify-content:center; z-index:10` |
| Dialog | `width:420px; background:#f3f2f2; border:2px solid #201e1d; box-shadow:0 16px 48px rgba(32,30,29,.40)` |
| Destructive dialog | same, `border:2px solid #ec3013` |
| Header | `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30)`; title `font:800 16px 'Archivo'; letter-spacing:.01em` |
| Destructive header | `border-bottom:2px solid #ec3013`; title `color:#ae1800` |
| Body | `padding:20px`, fields `gap:16px` |
| Action row | `padding:16px 20px; border-top:1px solid #d7d3d3; display:flex; gap:10px; justify-content:flex-end` |
| Cancel | `padding:8px 16px; border:1px solid rgba(32,30,29,.30); background:transparent; font-weight:800` |
| Confirm | `padding:8px 16px; background:#201e1d; color:#f3f2f2; border:none; font-weight:800` |
| Destructive confirm | as above, `background:#ec3013` |
| Chip (multi-select) | `display:flex; align-items:center; gap:6px; padding:6px 10px; border:1px solid rgba(32,30,29,.30); font-size:12px; cursor:pointer`; selected takes the dark treatment |

---

## Design Tokens

### Color
| Role | Value |
| --- | --- |
| Ground | `#f3f2f2` |
| Chrome / rail / panel | `#eae9e9` |
| Ink | `#201e1d` |
| Ink secondary | `#605d5d` |
| Ink tertiary | `#9b9797` |
| Ink on dark | `#f3f2f2` / `#bab6b6` (dimmed) |
| Accent | `#ec3013` |
| Accent (text-safe) | `#ae1800` |
| Positive (sync status) | `#2ecc71` |
| Rule (strong) | `rgba(32,30,29,.38)` |
| Rule (medium) | `#d7d3d3` |
| Border | `rgba(32,30,29,.30)` |
| Dimmer | `rgba(32,30,29,.30)` |

### Type
- Family: **Archivo** throughout; monospace only for log output and file paths
- Weights: 400, 800 — nothing between
- Sizes: 10px (labels, log), 11px (row actions, meta), 11.5px (scope notes, info panels), 12px (field labels), 12.5px (filter), 13px (body, inputs), 16px (dialog title), 20px (section heading), 28px (page heading)
- Letter-spacing: `.11em` (all-caps labels), `.01em` (dialog title)
- Numbers: `font-variant-numeric: tabular-nums` on every figure

### Spacing
6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 40, 48px. Section gap is always 48px; field gap 16px; column gap 40px; subsection gap (e.g. Units → Price Sources) 32px.

### Radius & elevation
- **Radius 0 everywhere.**
- Dialog shadow: `0 16px 48px rgba(32,30,29,.40)`

---

## Interactions

### Navigation
- Index entry click → scroll the body to that section's heading; the entry takes the active dark treatment
- `/ filter` → live substring filter over the index entries only; body sections are untouched and stay in the DOM
- The primary rail stays interactive throughout — Settings is never a trap

### Forms
- Text inputs, selects, radios, and segmented controls **save on change**; there is no page-level Save button
- Destructive and entity-creating actions are the exception — they go through a dialog with an explicit confirm

### Dialog lifecycle
| Trigger | Flow |
| --- | --- |
| **+ Add unit** | open 2b → fill Code / Name / Type (all required) → Add → validate → append to Units table → close |
| Row **edit** (unit) | open 2c pre-filled → modify → Save → if the code changed, rewrite references → close |
| Row **delete** (unit) | open 2d → type the code → confirm button enables on exact match → Delete → remove + close |
| **+ Add institution** | open 2e → name + at least one account type + default unit → Add → append → close |
| Price source **test** | pings/validates the source's connection and shows a transient success/failure state on the button — does not open a dialog |

- `esc` closes any dialog without saving; `enter` submits when the confirm button is enabled
- Focus moves into the dialog's first field on open and returns to the trigger on close
- The dialog traps focus while open

### States
- Hover: index entries and rows take a subtle ground tint; buttons shift border or ground
- Focus: `outline: 2px solid #ec3013; outline-offset: 2px` — never the browser default
- Disabled: 45% opacity (the delete confirm before the code matches)

---

## State

```
settings
  activeSection: SettingsSection   // General | Display | Units | Institutions | …
  filterQuery: String              // filters the index rail only
  dialog: Option<Dialog>           // AddUnit | EditUnit(id) | DeleteUnit(id) | AddInstitution
  confirmInput: String             // typed-back code for DeleteUnit
preferences   // ledger-scoped, syncs as Change Sets
  units[] { code, name, type, isBase, isDefault }
  institutions[]
  priceSources[] { unitCode, sourceLabel, lastUpdatedAt }
configuration // client-scoped, read from personal-ledger.conf
  dateFormat, decimalSeparator, rowDensity, statusGlyphs
  syncServerUrl, backupPath, traceLevel
```

---

## Implementation notes (GPUI)

1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **One scrolling body, not a pane switcher.** The index rail scrolls to sections — it does not swap views. This is what makes the surface feel like a document.
3. **Rules need `flex: none`** inside a flex column, or a 2px rule gets compressed to a hairline.
4. **Section gap is 48px, uniformly.** Carry it on a consistent edge (bottom of each section wrapper) rather than mixing top and bottom margins — mixed ownership is how gaps go missing.
5. **The filter rule and the page-heading rule must align.** The filter wrapper's `25.4px` bottom padding is what buys that; recompute it if the heading block's metrics change.
6. **Configuration vs Preferences must be visible.** Keep each section's scope note — it is the only thing telling the user whether a change syncs. Display is placed directly after General specifically so Configuration is visible early, not buried.
7. **Destructive confirm is typed, not clicked.** Keep the confirm disabled until the typed value matches exactly; do not substitute a plain "are you sure".
8. **Units and "Ledger & units" are one section, not two.** There is no separate pane for default-unit/budget-period controls — budget period is a per-budget setting and lives outside Settings entirely. Don't reintroduce it here.
9. **Base/default flags are data-driven tags, not free text.** Only render `base` / `default` pills on the unit(s) that actually hold those flags; most rows render an empty flags cell.
10. **Price Sources references units by name, not code** — this is deliberate and different from every other table in Settings (which key on code). Keep the distinction; it reflects that this table is about the relationship to an external source, read by a human, not a machine key.
11. **Namespace radio groups per instance** (`df-2a`, `df-2b`, …) so two rendered copies of a section don't share selection.
12. **No keybinding editor.** Bindings are static configuration; if a request to expose them arrives, it is a scope change.
13. **Zero radius, flush-left labels** — including labels inside wide buttons.
14. **Icons**: Lucide, 14×14 at rail size, 1.5px stroke.

## Acceptance pass

Issue #188's own closing walk — every "2a" section and "2b–2e" dialog checked against this document, now that all eight sections and all four dialogs have landed against `crates/bins/bin-desktop/src/view/settings/`, the same way issue #154 walked the Shell & Navigation map's own 10 acceptance criteria (`docs/ux/desktop/README.md`'s "Acceptance criteria walk").

- **All eight sections render in scroll order** (General, Display, Units, Institutions, Sync server, Data & backup, Tracing (Logs), About) **with the uniform section pattern** (heading + scope note + 2px rule + 48px gap, no exceptions) **and correct scope notes.** Met — `SettingsSection::ALL`/`label`/`scope_note` (`crate::settings`) match this document's own "Sections, in scroll order" table verbatim, and `view::settings::section_block` is the sole owner of the 48px gap and 2px rule, so no individual section's own render function can drift from it.
- **Index rail click scrolls to the right section and takes the active treatment; `/ filter` live-filters the index only.** Met — `Shell::handle_settings_index_click` sets the active section and calls `ScrollHandle::scroll_to_top_of_item` with `SettingsSection::body_child_index`; `Shell::handle_search_key` only ever mutates `settings_filter`, which `rail::settings_index::SettingsIndexRail` reads through `SettingsSection::matches_filter` — body sections stay mounted regardless of the filter text.
- **All four dialogs open/close correctly, including the Delete-unit typed-confirmation gating.** Met by code review and this crate's own test coverage, not a live click-through — this sandbox has no working synthetic mouse-click path for `bin-desktop` (the same limitation this map's own Notes, and issue #144's own #154, both flag). `DeleteUnitForm::matches` is a plain case-sensitive `==` against the row's own code; `dialog::confirm_button` only attaches an `on_click` handler when `enabled` is true, so a mismatched confirmation field is genuinely inert, not merely dimmed. `Esc` discards `settings_dialog` unconditionally, regardless of which variant is open.
- **Design tokens match this document's own tables.** Met — every named colour in `theme::color` was checked numerically against the Design Tokens table above (Ground, Chrome, Ink/secondary/tertiary, Accent/text-safe, Positive, both rule weights, Border, Dimmer); no mismatches found.
- **What's verified live vs by code review.** General and Display were confirmed with a real, resized floating window and screenshot (this sandbox's own `hyprctl`/`grim` path) — both render pixel-for-pixel against the mockup, including Display's own live PREVIEW table re-rendering correctly under every Date format / Decimal separator / Row density / Status glyphs combination. Units, Institutions, Sync server, Data & backup, Tracing, About, and all four dialogs are verified by code review and their own test coverage only, for the same synthetic-input reason noted above.

No drift found in this document itself against the shipped code. One stale rustdoc reference was fixed in passing: `theme::color::DIVIDER`'s own doc comment still pointed at the now-deleted `ledger_units` module (issue #189) rather than `add_unit_dialog::segmented_control`, its real home since that ticket (and now also reused by `display`).

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section 2 = 2a–2e; section 1 = the shell variants; section 3 = Accounts)
- `README.md` — this document
