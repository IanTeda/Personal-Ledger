# Handoff: Settings (paged) — v3

## Overview
Personal Ledger's **Settings** destination. Each section of the settings index opens its own page, and modal dialogs handle unit and institution management. Reference data that used to live in the primary rail now lives here: Accounts, Categories, Tags and Payees.

This package supersedes `design_handoff_settings_v2/` (and the older single-scroll `design_handoff_settings/`). Frames are now numbered **15a–15o** (Settings is section 15 of `Ledger Desktop Shell.dc.html`).

### Changes since v2
- **One heading per page.** The two stacked headings (h2 "Settings" plus an h4 page name, each with its own rule and note) are merged into a single h2, **"Settings – <Page>"**, with one combined note and one rule.
- **No index filter.** The `/ filter` box above the Settings index is removed. The index starts directly with the `SETTINGS` label.
- **Display › Sidebar.** A new **Sidebar** field group on Display holds **Hide sidebar** and **Show keyboard navigation hints**.
- **Primary rail** now includes **Notifications** (`g m`, with a red unread badge) in LEDGER. The shell around Settings (header, primary rail, status bar) is specified in `design_handoff_shell_settings/` (frame numbers there predate the renumbering); only the parts Settings depends on are repeated here.

## About the design files
`Settings.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` alongside it.

## Fidelity
**High fidelity.** Colours, typography, spacing, rules and states are final.

## Frame & shell
Every frame is **1280 × 800**: header 48px, body `flex:1`, status bar 28px.

- **Header breadcrumb:** `settings › <page>`.
- **Primary rail (206px):** Settings is active, pinned at the bottom below a 2px rule. Groups:
  - LEDGER: Dashboard, Transactions, Documents, Notifications (red count badge `#ec3013`, 10px 800, `padding:1px 5px`)
  - NET WORTH: Cash, Inventory, Loans, Credit cards, Investments
  - PLAN: Bills, Budgets, Reports
- **Dialog frames:** they dim the shell with opacity in the mockup. In the app, render the shell at full opacity under a `rgba(32,30,29,.30)` dimmer.

## Settings

### Model
- **Settings is a destination, not a mode.** The primary rail stays expanded with Settings active.
- **The second rail (214px) is the index.** Clicking an entry **swaps the pane**. Each page stands alone — there is no long scroll.
- **Configuration vs Preferences stays visible.** Each page header carries a scope note on the right.

### Index (canonical order)
`General · Display · Units · Institutions · Accounts · Categories · Tags · Payees · Sync server · Data & backup · Tracing (Logs) · About`

- **Rail chrome:** 214px wide, `border-right:2px solid rgba(32,30,29,.38)`, `padding:0 0 10px`. There is no filter box.
- **Index rows:** the `SETTINGS` label (`padding:24px 14px 8px; 800 10px/1 Archivo; .11em; #9b9797`) sits directly above the entries. Entries are `padding:8px 14px`; the active one is `background:#201e1d; color:#f3f2f2; 800`.
- **Rail footer:** "preferences sync · configuration local".

### Page anatomy (all pages)
- **Body:** `flex:1; min-width:0; padding:22px 28px; overflow:auto`.
- **Page heading (one per page):** a flex row, `align-items:baseline; justify-content:space-between; gap:16px; margin-bottom:24px` (16px on Categories).
  - Left: `h2` **"Settings – <Page>"** (28px/800, en dash with spaces), e.g. "Settings – General", "Settings – Data & backup".
  - Right: a single combined note (11.5px #9b9797, right-aligned). It is the scope note followed by the page meta, joined with " · ", duplicates dropped (see the Pages table).
  - Then one 2px rule `rgba(32,30,29,.38)`, `flex:none`, `margin-bottom:24px` (16px on Categories).
  - There is no separate h4 section heading and no second rule.
- **List pages:** a kicker row comes next — a label on the left (`800 10px/1, .11em, #9b9797`) and a primary `+ Add …` button (`.btn.btn-primary`, 32px) on the right.
- **Breadcrumb:** the header reads `settings › <page>`.

### Pages
| Frame | Page | Heading note | Content |
| --- | --- | --- | --- |
| **15a** | General | preferences · synced · ledger identity | Ledger name, Owner, Financial year starts, Base unit (320px field column) beside a THIS LEDGER summary (accounts 7 · transactions 680 · units 3 · institutions 7). |
| **15f** | Display | configuration · local · never synced | Date format, separators and row density (segmented), plus status glyphs (radio), beside a live PREVIEW of three ledger rows. Below the field column, a **Sidebar** field group (same `.field` label style as Status glyphs, `margin-top:18px`, 300px max) with two checkboxes (14×14, `accent-color:#201e1d`, square; bold 12.5px label over an 11.5px #605d5d hint): **Hide sidebar** (off) — "Start with the primary rail hidden. **b** or the header toggle still shows it for the session." · **Show keyboard navigation hints** (on) — "Show each destination's **g**-jump, e.g. **g d**, at the right of its row. The jumps work either way." |
| **15g** | Units | ledger data · synced · 3 units | UNITS table: CODE 100 · NAME 180 · FLAGS flex · SOURCE 130 · TYPE 80 · ACTIONS 120. `aud` carries the `base` (`.tag-accent`) and `default` (`.tag-outline`) flags. Then **+ Add unit**. PRICE SOURCES table: NAME 130 · SOURCE flex · LAST UPDATED 130 · ACTIONS 170 (test · edit · delete), then **+ Add price source**. |
| **15h** | Institutions | ledger data · synced · 7 institutions | INSTITUTION flex · ACCOUNT TYPE 140 · ACTIONS 120, then **+ Add institution** (opens 15e). |
| **15o** | Accounts | ledger data · synced · 7 accounts · net worth **83,995.87 aud** | Kicker BY TYPE + **+ Add account**. Grouped tables (BANK, CREDIT CARD, LOAN, INVESTMENT), each with NAME flex · INSTITUTION 130 · UNIT 90 · BALANCE 120 (right, tabular) · ACTIONS 150 (edit · delete). Rows are `padding:8px 16px`. Add and edit reuse the account form from section 16 (16b/16c). Status bar: `j/k row · enter open ledger · e edit · d delete · n new`. |
| **15i** | Categories | ledger data · synced · 12 categories · 3 levels deep | Kicker EXPENSE · 10 + **+ Add category**. A nested tree: ▾ disclosure, parent rows in 800 with an "n subcategories" note, row actions `+ sub · edit · delete`. No budget or spend columns. Keys: `j/k`, `→/←` expand/collapse, `e`, `d`, `n`. |
| **15j** | Tags | ledger data · synced · 18 tags · **2** likely duplicates (#ae1800) | Kicker A–Z + **+ Add tag**. Each row has a colour swatch (10×10) and the tag name in 800, with edit · remove. Likely-duplicate rows carry a warning and a direct **merge**; the merge flow is unchanged from 18e. No transaction, total or last-used columns. Keys: `j/k`, `e`, `x`, `m`, `n`. |
| — | Payees | ledger data · synced | **Indexed but not yet drawn.** Build it to the same pattern as 15j: a kicker A–Z + **+ Add payee**, then name · default category · match rules · actions (edit · delete), reusing the Payees management view in section 19 (19a) with its count and spend columns dropped. |
| **15k** | Sync server | this device · synced · every 30 seconds | Server URL, Status (green dot `#2ecc71` + "connected"), Last sync, **Sync now**. |
| **15l** | Data & backup | this device · local files · aud · 2.84 mb | Store location, Last backup, **Backup now**, **Export ledger (CSV)**. |
| **15m** | Tracing (Logs) | this device · diagnostic · last 1000 entries | Level radios (error / warn / info / debug), a monospace log viewport (120px, scrolls), **Clear logs**. |
| **15n** | About | this device · version info | Version 2.1.4, released 08 sep 2026, build stack. |

### Dialogs
All dialogs are 420px, `border:2px solid #201e1d`, with the shadow `0 16px 48px rgba(32,30,29,.40)`. The shell behind is dimmed.

| Frame | Dialog | Notes |
| --- | --- | --- |
| — | Add unit | Not drawn separately. It is 15c's form, empty, with **Add** as the confirm. Fields: Code, Name, Type (currency / cryptocurrency / custom). |
| **15c** | Edit unit | Pre-filled `aud`. Usage notice (`#eae9e9`, 2px left rule `#ec3013`, 11.5px): "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references." Cancel / Save. |
| **15d** | Delete unit | Red treatment: border and header rule `#ec3013`, title `#ae1800`. Names every reference. Has a "Type btc to confirm" field; **Delete unit** stays disabled until the text matches exactly. |
| **15e** | Add institution | Name, Account types as **multi-select chips** (savings / credit card / offset / loan / investment), Default unit. Cancel / Add institution. |

- Opening a dialog moves focus to its first field and traps focus while open. Closing returns focus to the trigger.
- `esc` cancels. `enter` submits when the confirm button is enabled.
- Plain fields **save on change** — there is no page Save button. Creating and deleting always go through a dialog.

---


## Keyboard
| Key | Action |
| --- | --- |
| `g s` | Open Settings on the last-visited page (General on first visit). |
| `j` / `k` | Move to the next / previous row on list pages or field on form pages. |
| `e` / `d` / `n` | Edit / delete / new on list pages. |
| `x` | Remove (Tags). |
| `m` | Merge (Tags). |
| `→` / `←` | Expand / collapse (Categories). |
| `esc` | Close a dialog. |
| `enter` | Submit a dialog when its confirm is enabled. On Accounts, opens the account's ledger. |

Palette commands: `:settings <page>` jumps straight to a page, e.g. `:settings accounts`.

## Components
| Part | Spec |
| --- | --- |
| Header | 48px, `#eae9e9`, `border-bottom:2px solid rgba(32,30,29,.38)`, `padding:0 10px 0 12px`, gap 12. Wordmark 800 13.5px; breadcrumb 12px #9b9797; `:` command affordance `padding:4px 9px; border:1px solid rgba(32,30,29,.30)`; three 26px window controls `rgba(32,30,29,.08)`. |
| Rail group label | `800 10px/1 Archivo; letter-spacing:.11em; #9b9797; padding:0 14px 8px` (16px top padding for later groups). |
| Nav item | `flex; gap:9px; padding:7px 14px`; 14×14 Lucide icon (1.5 stroke); label `flex:1`; binding 11px #9b9797. Active: `#201e1d` ground, `#f3f2f2` text and icon, label 800, binding `#bab6b6`. |
| Status bar | 28px, `#eae9e9`, 2px top rule, 11.5px #605d5d. Mode chip `#201e1d/#f3f2f2 800 10px .1em padding:2px 7px`; COMMAND mode uses `#ec3013`. |
| Table header | `padding:12px 16px` (6–10px on dense lists), `#eae9e9`, `800 10px .11em #605d5d`, 1px bottom rule. |
| Table row | `padding:12px 16px` (8px on Accounts); `border-bottom:1px solid #d7d3d3`, omitted on the last row; primary cell 800. |
| Row action | `padding:4px 10px; 11px; border:1px solid rgba(32,30,29,.30); transparent`. |
| Section rule | `height:2px; flex:none; rgba(32,30,29,.38)` — `flex:none` is mandatory. |
| Info panel | `#eae9e9`, `border-left:2px solid #ec3013`, 11.5px #605d5d. |

## Design tokens
- **Colour:**
  - Ground `#f3f2f2`. Chrome / panel `#eae9e9`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6`.
  - Accent `#ec3013`; text-safe accent `#ae1800`. Positive `#2ecc71`.
  - Rules: strong `rgba(32,30,29,.38)`, medium `#d7d3d3`. Border `rgba(32,30,29,.30)`. Dimmer `rgba(32,30,29,.30)`.
  - The full ramps are in `styles.css` (Modernist).
- **Type:**
  - Archivo only (monospace for logs and paths), weights 400 and 800.
  - Sizes 10 / 11 / 11.5 / 12 / 12.5 / 13 / 13.5 / 16 / 20 / 26 / 28px.
  - Caps labels `.11em`. Tabular numerals on every figure.
- **Spacing:** 6, 7, 8, 9, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 40, 48px.
- **Radius:** **0 everywhere.**
- **Shadows:**
  - Dialog `0 16px 48px rgba(32,30,29,.40)`.
  - Palette `0 12px 32px rgba(45,43,43,.30)`.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Disabled controls render at 45% opacity.


## State
```
settings
  page: General | Display | Units | Institutions | Accounts | Categories | Tags | Payees
      | SyncServer | DataBackup | Tracing | About
  dialog: None | AddUnit | EditUnit(id) | DeleteUnit(id) | AddInstitution
        | AddAccount | EditAccount(id) | AddCategory(parent?) | AddTag | MergeTags(a,b) | AddPayee
  confirmInput: String
preferences   // ledger-scoped, sync as Change Sets
  ledgerName, owner, fyStart, baseUnit
  units[] { code, name, type, isBase, isDefault }
  priceSources[] { unitCode, source, lastUpdatedAt }
  institutions[] { name, accountTypes[], defaultUnit }
  accounts[] { name, institution, type, unit, openingBalance, number? }
  categories[] { id, name, parentId?, kind }
  tags[] { name, colour }
  payees[] { name, defaultCategory?, matchRules[] }
configuration // client-scoped, read from personal-ledger.conf
  dateFormat, separators, rowDensity, statusGlyphs
  sidebarHidden: bool             // start with the primary rail hidden; `b` toggles per session
  showNavHints: bool              // show the g-jump column in the primary rail
  syncServerUrl, storePath, traceLevel
```

## Implementation notes (GPUI)
1. **Pages swap, they don't scroll to anchors.** Remember the last-visited page per session.
2. **Keep the scope note on every page.** It is the only signal of whether a change syncs (Preferences) or stays on this device (Configuration).
3. **Fields save on change.** There is no Save button on pages. Creating and deleting always go through a dialog.
4. **Typed delete confirms.** The confirm enables only on an exact match. Never substitute "are you sure?".
5. **Data-driven flags.** `base` / `default` tags render only on the units that hold them.
6. **Price sources name units by name, not code.** This is deliberate; every other table keys on code.
7. **2px rules need `flex:none`.** Without it they collapse to hairlines.
8. **One heading per page.** Build the "Settings – <Page>" title from the index entry name so they never drift; the right-hand note is scope + meta.
9. **Scope radio-group names per instance**, so two rendered copies of a section don't share selection.
10. **No keybinding editor.** Bindings are static configuration.
11. **Zero radius, flush-left labels.** Labels stay flush left even inside wide buttons. Icons are Lucide at 14px with a 1.5px stroke.
12. **Sidebar settings apply live.** Toggling Hide sidebar collapses the rail immediately and persists for next launch; Show keyboard navigation hints hides only the binding column — the `g` jumps still work.

## Files
- `Settings.dc.html` — design reference (section 15): 15a, 15f, 15g, 15h, 15o, 15i, 15j, 15k–15n, then dialogs 15c–15e. Links to other sections point to `../Ledger Desktop Shell.dc.html`.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
