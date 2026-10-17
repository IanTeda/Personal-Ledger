# Handoff: Settings (paged) — v5

> **Package 16 of 20 · Settings** — one deck, `Settings.dc.html`, which opens on the page list linking to each page's frames: pages 16a, 16f, 16g, 16h, 16o, 16i, 16j, 16p, 16q, 16v, 16k, 16l, 16m, 16u, 16n, then dialogs 16c, 16d, 16e, 16r, 16s, 16t, 16w, 16x, 16y, plus the Window Size cards for General, Display and Units. Open the deck through `mise run mockups`. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

## Settings navigation per Window Size

The Chrome around Settings follows `../01-shell/README.md#window-sizes`; these are the parts every Settings page shares.

| Window Size | Primary rail | Settings index | Page heading | Status line hints |
| --- | --- | --- | --- | --- |
| Medium (720 – 1199) | icons, 52px | none docked: `g s` opens a full-page index (DESKTOP-119) | the breadcrumb **Settings › <Page> ▾**: **Settings** goes back to the index, the caret (or `p`) opens the page picker (DESKTOP-120) | `j/k section · [/] pages · p picker · esc settings`, ledger path `…/teda.pldb` |
| Large (1200 – 1599) | expanded, 206px | docked, 232px, the page selected | **<Page>** alone | the form or list hints below |
| X-Large (≥ 1600) | expanded, 206px | docked, 232px, the page selected | **<Page>** alone | the form or list hints below |

- **`[` / `]`** step to the previous and next page in index order at Medium; the status line is their only hint. `[` does nothing on General and `]` nothing on About.
- **Content width:** every page caps its content at 840px at every Window Size. At X-Large the rest of the View stays empty; Settings docks no detail pane.
- **Chrome header:** the wordmark alone, so the page heading is the only trail.
- **Dialogs** stay 420px wide (or their own stated width) at every Window Size.

## Overview
Personal Ledger's **Settings** destination. Each section of the settings index opens its own page, and modal dialogs handle unit and institution management. Reference data that used to live in the primary rail now lives here: Accounts, Categories, Tags and Payees.

This package supersedes `design_handoff_settings_v4/` and older (v3, v2, single-scroll). Frame ids run 16a–16y with gaps (16b is retired); Settings is section 16 of the master `Ledger Desktop Shell.dc.html`.

### Changes in v6 (9 October 2026)
- **New dialogs for AI Connections:** 16w Add provider, 16x Edit provider with a rejected key, 16y Add assistant (connection details shown once).
- **New page: AI Connections (16v)**, above Sync Server in the index. Two blocks: Model providers and Assistant access (MCP server). **Sync server** is now **Sync Server** in the index and the page heading.
- **Data & backup is now Backup (16l)**, with a second block, **Git backup**, that commits a text dump of the Ledger plus its Documents (through Git LFS) to a private remote repository on a schedule ([ADR-0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md)); the mockup's "commits the store" is superseded. The local block shows Store location, Last snapshot, **Back up now** and **Export ledger (CSV)**. **Set up / Edit** opens 16t.
- **Tracing (Logs) is now Logs (16m).** Behaviour is unchanged.
- **New page: History (16u)**, between Backup and Logs. Every change to the ledger, newest first, as one linear undo stack. Undo / Redo (`u` / `ctrl r`, anywhere in the app) step the marker one change; undone changes stay greyed above the marker until a new edit discards them; **Restore to here** jumps several steps; synced changes undo on every device.
- **New dialogs:** 16r Edit sync server (URL and access token, inline **Test connection**), 16s Test failed (Save stays disabled until a test passes), 16t Set up Git backup (remote, SSH key or access token, dry-run **Test**; the first push commits the whole store). Sync server (16k) **Edit** opens 16r.
- **Index** is now: General, Display, Units, Institutions, Accounts, Categories, Tags, Payees, Documents, Inventory, AI Connections, Sync Server, Backup, History, Logs, About.
- **Inventory (16q):** adding a room is explicit in two places: a bold **+ room** on every property row (works collapsed, and expands it) and a dashed **+ Add room** row closing each room list. `r` does the same on the selected property.
- **Header:** the back / forward buttons (`alt ←` / `alt →`) are gone from every frame.
- Layout specs for 16r–16u are in the page table (16u) and the dialog table (16r–16t) below.

### Changes since v4 (4 October 2026)
- **The design hasn't changed.** `Settings.dc.html` was re-checked against the master and matches it frame for frame.
- **This README is corrected:** the frame list and shell path are fixed, every page's sample data and status-bar hints are added, and a **Known inconsistencies** list covers the mockup details that shouldn't be copied literally.

### Changes in v4 (carried forward)
- **Renumbered** from 15x to 16x.
- **New page: Documents (16p)**, after Payees in the index (titled "Settings – Documents"; it was drafted as "Document types"). It defines the document types used by the Documents Type filter.
- **New page: Inventory (16q)**, after Documents. Properties and their rooms, the setup behind the Inventory registers (8a).
- **Tracing (16m):** **Clear logs** moves to the top right, on the level-radio row above the log box. The log box fills the rest of the page (full width and height) and always shows a vertical scroll bar.
- **About (16n):** rebuilt with a tagline, version and date, the build stack, and Author / Repository / Documentation / Report an issue / License, with the copyright pinned to the bottom.
- **Primary rail** now includes **Events** (`g e`) in LEDGER, after Documents.

### Changes in v3 (carried forward)
- **One heading per page.** The two stacked headings (h2 "Settings" plus an h4 page name, each with its own rule and note) are merged into a single h2, **"Settings – <Page>"**, with one combined note and one rule.
- **No index filter.** The `/ filter` box above the Settings index is removed. The index starts directly with the `SETTINGS` label.
- **Display › Sidebar.** A new **Sidebar** field group on Display holds **Hide sidebar** and **Show keyboard navigation hints**.
- **Primary rail** now includes **Notifications** (`g m`, with a red unread badge) in LEDGER. The shell around Settings (header, primary rail, status bar) is specified in `../01-shell/`. Only the parts Settings depends on are repeated here.

## About the design files
`Settings.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` alongside it.

## Fidelity
**High fidelity.** Colours, typography, spacing, rules and states are final.

## Frame & shell
The v5 frames are **1280 × 800**, and Window Size cards use their reference frame: header 48px, body `flex:1`, status bar 28px.

- **Header breadcrumb:** `settings › <page>` (lower case). The right of the header reads `Synced 14:22`.
- **Status bar:** the NORMAL chip, then per-page hints on the left and a status on the right.
  - Form pages (General, Display, Units, Institutions, AI Connections, Sync Server, Backup, Logs, About): `j/k section · tab next field · b collapse sidebar · q back to dashboard` · right `saved automatically`.
  - List pages show their own keys (listed in the Pages table) · right `saved automatically`. On Accounts the right side reads `7 accounts`.
- **Primary rail (206px):** Settings is active, pinned at the bottom below a 2px rule. Groups:
  - LEDGER: Dashboard, Transactions, Documents, Events, Notifications (red count badge `#ec3013`, 10px 800, `padding:1px 5px`)
  - NET WORTH: Cash, Inventory, Loans, Credit cards, Investments
  - PLAN: Bills, Budgets, Reports
- **Dialog frames:** they dim the shell with opacity in the mockup. In the app, render the shell at full opacity under a `rgba(32,30,29,.30)` dimmer.

## Settings

### Model
- **Settings is a destination, not a mode.** The primary rail stays expanded with Settings active.
- **The second rail (214px) is the index.** Clicking an entry **swaps the pane**. Each page stands alone — there is no long scroll.
- **Configuration vs Preferences stays visible.** Each page header carries a scope note on the right.

### Index (canonical order)
`General · Display · Units · Institutions · Accounts · Categories · Tags · Payees · Documents · Inventory · AI Connections · Sync Server · Backup · History · Logs · About`

- **Rail chrome:** 214px wide, `border-right:2px solid rgba(32,30,29,.38)`, `padding:0 0 10px`. There is no filter box.
- **Index rows:** the `SETTINGS` label (`padding:24px 14px 8px; 800 10px/1 Archivo; .11em; #9b9797`) sits directly above the entries. Entries are `padding:8px 14px`; the active one is `background:#201e1d; color:#f3f2f2; 800`.
- **Rail footer:** "preferences sync · configuration local".

### Page anatomy (all pages)
- **Body:** `flex:1; min-width:0; padding:22px 28px; overflow:auto`.
- **Page heading (one per page):** a flex row, `align-items:baseline; justify-content:space-between; gap:16px; margin-bottom:24px` (16px on Categories).
  - Left: `h2` **"Settings – <Page>"** (28px/800, en dash with spaces), e.g. "Settings – General", "Settings – Backup".
  - Right: a single combined note (11.5px #9b9797, right-aligned). It is the scope note followed by the page meta, joined with " · ", duplicates dropped (see the Pages table).
  - Then one 2px rule `rgba(32,30,29,.38)`, `flex:none`, `margin-bottom:24px` (16px on Categories).
  - There is no separate h4 section heading and no second rule.
- **List pages:** a kicker row comes next — a label on the left (`800 10px/1, .11em, #9b9797`) and a primary `+ Add …` button (`.btn.btn-primary`, 32px) on the right.
- **Breadcrumb:** the header reads `settings › <page>`.

### Pages
| Frame | Page | Heading note | Content |
| --- | --- | --- | --- |
| **16a** | General | preferences · synced · ledger identity | Ledger name, Owner, Financial year starts (july / january / april), Base unit (aud — Australian Dollar / btc — Bitcoin) in a 320px field column, beside a THIS LEDGER summary (accounts 7 · transactions 680 · units 3 · institutions 7). Footnote: General settings are ledger-scoped Preferences that travel with the ledger as Change Sets; client-only Configuration is under Display. |
| **16f** | Display | configuration · local · never synced | Segmented controls: Date format (12 sep 2026 / 12/09/2026 / ISO), Decimal & thousands separator (1,234.56 / 1.234,56 / 1 234.56) and Row density (compact / regular / roomy). Radios: Status glyphs (unicode — ○ ◐ ● ⚑ / ascii fallback — o / x !). Beside them, a live PREVIEW (DATE · PAYEE · AMOUNT): ● 12 sep 2026 Woolworths Metro −86.40 · ◐ 10 sep 2026 Telstra −99.00 · ⚑ 09 sep 2026 Dept of Education +4,210.00. A footnote says these values are read from `personal-ledger.conf` at start-up and written back here. Below the field column, a **Sidebar** field group (same `.field` label style as Status glyphs, `margin-top:18px`, 300px max) with two checkboxes (14×14, `accent-color:#201e1d`, square; bold 12.5px label over an 11.5px #605d5d hint): **Hide sidebar** (off) — "Start with the primary rail hidden. **b** or the header toggle still shows it for the session." · **Show keyboard navigation hints** (on) — "Show each destination's **g**-jump, e.g. **g d**, at the right of its row. The jumps work either way." |
| **16g** | Units | ledger data · synced · 3 units | UNITS table: CODE 100 · NAME 180 · FLAGS flex · SOURCE 130 · TYPE 80 · ACTIONS 120. Rows: aud Australian Dollar · USD · currency (carries the `base` `.tag-accent` and `default` `.tag-outline` flags) · btc Bitcoin · CoinGecko · crypto · vas Vanguard Aus Shares · Manual entry · etf. Then **+ Add unit**. PRICE SOURCES table: NAME 130 · SOURCE flex · LAST UPDATED 130 · ACTIONS 170 (test · edit · delete). Rows: Bitcoin · CoinGecko — auto, every 15 min · 5 minutes ago, and Vanguard Aus Shares · Manual entry · —. Then **+ Add price source**. |
| **16h** | Institutions | ledger data · synced · 7 institutions | INSTITUTION flex · ACCOUNT TYPE 140 · ACTIONS 120. Rows: ANZ Banking Group (savings · offset), American Express (credit card), Vanguard Investments (investment), Westpac Banking (savings), Cryptocurrency Exchange (crypto), Superannuation Fund (retirement). Then **+ Add institution** (opens 16e). |
| **16o** | Accounts | ledger data · synced · 7 accounts · net worth **83,995.87 aud** | Kicker BY TYPE + **+ Add account**. Grouped tables (BANK, CREDIT CARD, LOAN, INVESTMENT), each with NAME flex · INSTITUTION 130 · UNIT 90 · BALANCE 120 (right, tabular) · ACTIONS 150 (edit · delete). Rows are `padding:8px 16px`. BANK: ANZ Everyday 4,182.55 · ANZ Offset 61,204.10 (ANZ, aud). CREDIT CARD: Amex Platinum −2,318.44. LOAN: Home Loan (ANZ) −381,311.34. INVESTMENT: Vanguard VAS 1,240 u (vas) · Bitcoin (Crypto.com) 0.4120 u (btc). Add and edit reuse the account form from section 17 (17b/17c). Status bar: `j/k row · enter open ledger · e edit · d delete · n new`. |
| **16i** | Categories | ledger data · synced · 12 categories · 3 levels deep | Kicker EXPENSE · 10 + **+ Add category**. A nested tree: ▾ disclosure, parent rows in 800 with an "n subcategories" note, row actions `+ sub · edit · delete` (leaf rows: edit · delete). EXPENSE: ▾ Housing › Rent · ▾ Utilities › Electricity, Water · ▾ Food › Groceries, Dining · Transport · Household. Then INCOME · 2: Salary, Interest. No budget or spend columns. Footnote: categories nest to any depth, and only leaf categories can be picked on a transaction. Keys: `j/k`, `→/←` expand/collapse, `e`, `d`, `n`. |
| **16j** | Tags | ledger data · synced · 18 tags · **2** likely duplicates (#ae1800) | Kicker A–Z + **+ Add tag**. Each row has a colour swatch (10×10) and the tag name in 800, with edit · remove. Rows: shared, reimbursable, tax-deductible, Shared (looks like a duplicate of "shared"), work-trip, gift, one-off. Likely-duplicate rows carry a warning and a direct **merge**; the merge flow is unchanged from 19e. No transaction, total or last-used columns. Keys: `j/k`, `e`, `x`, `m`, `n`. |
| — | Payees | ledger data · synced | **Indexed but not yet drawn.** Build it to the same pattern as 16j: a kicker A–Z + **+ Add payee**, then name · default category · match rules · actions (edit · delete), reusing the Payees management view in section 20 (20a) with its count and spend columns dropped. |
| **16p** | Documents | ledger data · synced · 7 types · 392 of 412 files typed | Kicker IN TYPE-FILTER ORDER + **+ Add type**. Table: TYPE flex (800) · TRACKS DATE 150 (Renews / Ends / Expires, or — in #9b9797) · REMIND 130 ("30 days before", or —) · TAX YEAR 90 (Yes / No) · FILES 60 (right, tabular) · ACTIONS 118 (edit · remove). Rows `padding:6px 14px; gap:14px; 12.5px`; selected row inverted (`#201e1d`/`#f3f2f2`, light-bordered buttons). Rows (TYPE · TRACKS DATE · REMIND · TAX YEAR · FILES): Receipts — — Yes 186 · Statements — — Yes 94 · Tax — — Yes 31 · **Insurance Renews 30 days before No 18 (selected)** · Warranties & manuals Ends 30 days before No 42 · Contracts Ends 60 days before No 9 · Identity Expires 6 months before No 6. Below, three 12px #605d5d notes (max 640px) explaining Tracks date, Tax year and removal (20 untyped files stay in the Inbox). Row order is the order of the Documents Type filter; reorder with `J`/`K`. Removing a type that still has files asks where to move them. Keys: `j/k`, `e`, `x`, `n`. |
| **16q** | Inventory | ledger data · synced · 2 properties · 11 rooms · 232 items | Kicker PROPERTIES + **+ Add property**. A collapsible properties table: disclosure 14 · PROPERTY flex (name 800 over an 11px #605d5d address) · POLICY 170 (insurer · policy no.) · SUM INSURED 90 · ITEM LIMIT 80 · ITEMS 46 (all right-aligned, tabular) · ACTIONS 150 (+ room · edit · remove). Header row `padding:6px 14px`; property rows `padding:8px 14px; 12.5px`. An expanded property (▾) shows an indented rooms sub-table — ROOM flex · ITEMS · VALUE · ACTIONS (edit · remove) — in room-tab order, ending with **+ Add room**. Sample: **12 Elm St contents** (12 Elm St, Ainslie ACT · NRMA HC-4471902 · 150,000 · 2,000 · 214 items), expanded with Living 38 · 24,150 / Kitchen 46 · 22,310 / Bedroom 1 24 · 18,420 / Bedroom 2 16 · 8,060 / Bedroom 3 12 · 5,540 / **Office 19 · 15,240 (selected, inverted)** / Garage 27 · 15,060 / Laundry 11 · 3,120 / Bathroom 9 · 1,880 / Outdoor & shed 12 · 49,700 — values sum to **163,480**, the 8a total. **Storage unit** (Kennards, Unit 114 · Mitchell ACT · NRMA HC-4471902 away cover · 20,000 · 1,000 · 18 items) is shown collapsed (▸). Below, 12px #605d5d notes: each property is one Inventory register (switch with `o`) and its rooms are the room tabs, in this order; Sum insured and Item limit drive the cover check and over-limit flag; removing a room with items asks where to move them; removing a property keeps its items' transactions and documents. Keys: `j/k`, `→/←`, `e`, `x`, `n` new property, `r` new room. |
| **16v** | AI Connections | this device · 3 providers · 2 assistants | `max-width:640px` column, `gap:32px`, two blocks, each with a caps header (`800 10px/1`, `.11em`, #605d5d) over a 2px rule and a right-hand underlined action. **MODEL PROVIDERS** (+ Add provider): rows `padding:9px 0`, 1px `#d7d3d3` rule — name 800 over a 11px #605d5d line (model · key location or URL), a 150px status (6px dot + 800 12px: connected `#2ecc71`, key rejected `#ae1800`), and an underlined `Edit · Remove`. The default provider carries a dark **DEFAULT** chip. Sample: Claude `claude-sonnet-5-5` (default), Ollama (local) `llama3.1:8b` at `http://localhost:11434`, OpenAI `gpt-5` with a rejected key. Note: keys are stored in the OS keychain and never synced; only selected transactions are sent; **Test** checks key and model. **ASSISTANT ACCESS · MCP SERVER** (+ Add assistant): label / value rows — Server (dot + `on · 127.0.0.1:7421`), Access (two-segment toggle **Read only** active / **Read & write**, 260px), Approval (`ask before each change`) — then a caps `CONNECTED ASSISTANTS` list in the same row treatment: Claude Desktop (read only · last used today 14:02, connected) and Claude Code (read only · last used 12 sep 2026, idle), each with **Revoke**. Note: the server listens on this device only; revoking ends the session at once. Add provider opens 16w, a rejected provider's Edit opens 16x, Add assistant opens 16y. Decided in [ADR-0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md): everything here is per-Client Configuration, never synced, and Assistant access ships **read only**, so build the Access toggle with Read & write disabled until a follow-up ADR. |
| **16k** | Sync Server | this device · synced · every 30 seconds | Server URL `sync.ledger.localhost`, Status (green dot `#2ecc71` + "connected"), Last sync `14 sep 2026 · 09:14`, **Sync now**. **Edit** opens 16r; a failed **Test connection** shows 16s. |
| **16l** | Backup (was Data & backup) | this device · local files · aud · 2.84 mb | LOCAL block: Store location `~/.ledger/personal`, Last snapshot `12 sep 2026 · 23:10`, **Back up now**, **Export ledger (CSV)**. GIT BACKUP block under it, same row treatment (label left #605d5d, value right 800 12px; `padding:8px 0`, 1px `#d7d3d3` rules), caps header `GIT BACKUP` with an underlined **Edit** at the right over a 2px rule: Remote `git@github.com:teda/ledger-backup.git` (11px), Branch `main`, Visibility (green dot + `private · checked 14:00`), Authentication `SSH key · ~/.ssh/id_ed25519`, Schedule `after changes · at most hourly`, and Commit message as a read-only prefix chip `2026-09-14 14:00 ·` (#eae9e9, tabular) beside the editable text `ledger backup`. **Set up / Edit** opens 16t. |
| **16m** | Logs (was Tracing) | this device · diagnostic · last 1000 entries | The page body is a flex column (`overflow:hidden`). A `flex:none` row (`space-between; gap:12px; margin-bottom:10px`) holds the level radios (error / warn / info / debug) on the left and **Clear logs** (`.btn.btn-secondary`, 32px) on the right. Below it the log box fills the remaining width and height (`flex:1; min-height:0`): `border:1px solid rgba(32,30,29,.30); #eae9e9; padding:10px 12px`, monospace 11px/1.6 #605d5d, newest first, `overflow-y:scroll` with an always-visible scroll bar (`scrollbar-color:#605d5d #d7d3d3`). Entries are `[hh:mm:ss] <subsystem>: <message>`, using the subsystems sync, txn, budget, import, docs, price, bills, events, notify, store, loans and cards. In GPUI, use a virtualised list with a persistent scroll bar. |
| **16u** | History | every change · newest first | One linear undo stack, newest first, with a marker. **Undo / Redo** move the marker one change; undone changes are greyed above it until a new edit discards them; **Restore to here** jumps several steps. Shared with `u` / `ctrl r` app-wide; synced changes undo on every device. See the frame for layout. |
| **16u** | History | all devices · last 30 days · 214 changes | Flex column (`overflow:hidden`). A `flex:none` toolbar (`gap:8px; margin-bottom:12px`): **Undo** `u` and **Redo** `ctrl r` buttons (`padding:8px 14px; #eae9e9; border:1px solid rgba(32,30,29,.30)`, 800, key hint 400 11px #605d5d), then `Next undo: <b>Added Camera House −4,380.00</b>` (12px #605d5d), then a spacer and an area filter select (All areas / Transactions / Budgets / Settings, 12px). Below it a bordered list fills the rest (`flex:1; min-height:0; border:1px solid rgba(32,30,29,.30)`). Header row `#eae9e9`, `800 10px/1`, `.11em`, #605d5d: TIME 56 · AREA 110 · CHANGE flex · DEVICE 96 · action 120. Rows `padding:9px 12px; gap:12px`, 1px `#d7d3d3` rule, time tabular, area #605d5d, device #605d5d 12px (`this device`, `laptop`), newest first. **Undone rows** sit above the marker: all text `#9b9797`, the change struck through, action **Redo to here** (underlined, 12px). The **marker** is a dark band between them (`#201e1d`, `#f3f2f2`, `800 10px`, `.11em`): `▸ CURRENT STATE`, a hairline, then `2 changes can be redone · a new edit clears them` (11px #bab6b6). Applied rows below the marker carry the action **Restore to here**. Sample: 14:21 Transactions *Recategorised Woolworths −86.40 → Groceries* and 14:19 Budgets *Dining limit 400.00 → 350.00* are undone; 14:12 *Added Camera House −4,380.00 · Everyday*, 14:05 *Renamed tag holiday → travel*, 13:58 Bills *Marked Rates Q1 paid · 612.00* (laptop), 13:51 *Split Bunnings −142.75 three ways*, 13:44 *Added room Garage · 12 Elm St*, 13:30 *Imported 6 rows · anz-sep.qif*, 12:02 Accounts *Created ANZ Offset · aud* (laptop), 11:47 *Deleted duplicate Netflix −22.99*, 11:40 Budgets *Rollover applied · September*. Undo / Redo step the marker one change; **Redo to here** and **Restore to here** move it several. A new edit discards the greyed rows. Synced changes undo on every device. Keys: `u` undo, `ctrl r` redo, anywhere in the app. |
| **16n** | About | version info | Flex column. Tagline (13px #605d5d, max 720px), 1px rule `rgba(32,30,29,.30)`, then **Personal Ledger v0.1.0** (800 13px) "(08 September 2026)", "Built with Rust + GPUI + SQLite" (12.5px #9b9797). Then labelled entries — caps label (`800 10px/1, .11em, #605d5d; margin:18px 0 8px`) over a 13px link in `#ae1800` (no underline; opens in the browser): AUTHOR Ian Teda · REPOSITORY github.com/IanTeda/personal-ledger · DOCUMENTATION ianteda.github.io/personal-ledger · REPORT AN ISSUE …/issues · LICENSE "Distributed under the **GPL-3.0 License**". A spacer pushes a 1px rule and "© 2025–2026 Ian Teda. All rights reserved." (11.5px #9b9797) to the bottom. Version, date and links come from build metadata. |

**16a at each Window Size.** Three cards after 16a draw the General page in the Window Size Chrome from `../01-shell/README.md#window-sizes`. The fields, the THIS LEDGER summary and the footnote are 16a's, capped at 840px; only the Chrome and the heading change.

- **DESKTOP-126 · Medium (960 × 1040):** icon rail (52px), no docked index. The heading is the breadcrumb **Settings › General ▾** with the page picker caret; `]` steps to Display, and `[` does nothing on the first page. The 320px fields and the 400px summary still sit side by side. Status line `j/k section · [/] pages · p picker · esc settings` and `…/teda.pldb`.
- **DESKTOP-127 · Large (1280 × 800):** expanded rail (206px) and the docked 232px Settings index with General selected. Heading **General** alone. The two columns take 760px of the 782px View.
- **DESKTOP-128 · X-Large (1920 × 1080):** the same Chrome as Large. The content stops at 840px and the rest of the View stays empty.

**16f at each Window Size.** Five cards after 16f draw the Display page in the Window Size Chrome from `../01-shell/README.md#window-sizes`, and add the Toasts field and the Colour Theme group. The code lives in `crates/bins/bin-desktop/src/settings/display/`. Display has no Dialogs: every field saves on change.

Rows run Medium, Large and X-Large, the order the Chrome is designed in ([ADR-0042](../../../adr/0042-window-size-from-window-width-with-a-medium-floor.md)). The fields, PREVIEW and Sidebar group are 16f's; the Toasts field and the Colour Theme group below them are new to these cards. Everything is capped at 840px; only the Chrome, the heading, the space beside the PREVIEW and how far the page scrolls change.

| Card | Window Size | Frame | Chrome | Heading | Content |
| --- | --- | --- | --- | --- | --- |
| **DESKTOP-129** | Medium | 960 × 1040 | icon rail (52px), no docked index | breadcrumb **Settings › Display ▾** with the page picker caret | The PREVIEW takes the 500px beside the 300px field column; the Sidebar group sits below the fields, then the Toasts field and the Colour Theme group, four cards to a row. The page runs about 120px past the View, so it scrolls. |
| **DESKTOP-130** | Large | 1280 × 800 | expanded rail (206px), docked 232px Settings index with Display selected | **Display** alone | The PREVIEW flexes to the View's edge. The page is taller than the View: a vertical scroll bar shows and the Toasts field and Colour Theme group start below the fold. |
| **DESKTOP-132** | Large | 1280 × 800 | as DESKTOP-130 | **Display** alone, scrolled with the page | DESKTOP-130 scrolled to the end, so the Toasts field and the Colour Theme group show in full. |
| **DESKTOP-133** | Large | 1280 × 800 | as DESKTOP-130, in Modernist's Dark Colour Variant | **Display** alone, scrolled with the page | DESKTOP-132 with Appearance set to Dark and a `[theme.dark] positive = "#2a6340"` override: every card draws its Dark roles with that `positive`, and the override note and contrast line sit under the grid. |
| **DESKTOP-131** | X-Large | 1920 × 1080 | the same Chrome as Large | **Display** alone | The PREVIEW stops with the page at 840px and the rest of the View stays empty. Even at 1080px the page runs about 80px past the View, so a short scroll bar shows. Settings docks no detail pane. |
| DESKTOP-064 | — (v5) | 1280 × 800 | the v5 Chrome: expanded rail, 214px index, header breadcrumb | **Settings – Display** | Kept as the reference for the field values below. Where it differs from DESKTOP-130, DESKTOP-130 is correct. |

- **Status line at Medium:** `j/k section · [/] pages · p picker · esc settings`, with the ledger path shortened to `…/teda.pldb`. At Large and X-Large the form-page hints under Frame & shell apply.
- **Stacking:** at 960 the two columns sit side by side. Below about 700px of page width, as at the 720px floor with the icon rail, the PREVIEW wraps under the field column instead of crushing the payee.

#### 16f content

- **Heading note:** `configuration · local · never synced`.
- **Segmented controls:** Date format (12 sep 2026 / 12/09/2026 / ISO), Decimal & thousands separator (1,234.56 / 1.234,56 / 1 234.56) and Row density (compact / regular / roomy).
- **Radios:** Status glyphs (unicode — ○ ◐ ● ⚑ / ascii fallback — o / x !).
- **PREVIEW (DATE · PAYEE · AMOUNT):** ● 12 sep 2026 Woolworths Metro −86.40 · ◐ 10 sep 2026 Telstra −99.00 · ⚑ 09 sep 2026 Dept of Education +4,210.00. It redraws as the fields change.
- **Sidebar group** (`margin-top:18px`, 300px max), two checkboxes (14×14, square, `accent-color:#201e1d`; bold 12.5px label over an 11.5px #605d5d hint):
  - **Hide sidebar** (off): "Start with the primary rail hidden. **b** or the header toggle still shows it for the session." It applies at Large and X-Large only; Medium always starts with the icon rail.
  - **Show keyboard navigation hints** (on): "Show each destination's **g**-jump, e.g. **g d**, at the right of its row. The jumps work either way."
- **Toasts** (`margin-top:24px`, 300px, below the Sidebar group; `toasts_field` in `render.rs`, [ADR-0027](../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)): an On / Off segmented control with 88px options, On in the sample, over the 12px #605d5d note "Errors still show". Off sends Toasts to the status line instead; Errors still toast.
- **Colour Theme group** (full width, `margin-top:28px`, below the Toasts field), per `docs/colour-themes-design.md` "Settings":
  - **Appearance:** a Light / Dark / System segmented control, the same as the other segmented fields. System is chosen in the sample, with the 12px #605d5d note "System (currently Light)" under it; if the OS reports nothing it reads "System (not detected, using Dark)".
  - **Colour theme:** a wrapping grid (`gap:12px`) of 160px cards, one per built-in Colour Theme in order: Modernist, High Contrast, Catppuccin, Gruvbox, Nord. Each card is a miniature ledger 84px tall in that Colour Theme's Colour Variant in effect (Light in most frames, Dark in DESKTOP-133), run through `resolve` with this Client's `[theme]` overrides: an 18px chrome band with a muted bar and an accent chip, then three 14px rows, the middle one selected (the `foreground`/`background` swap), carrying a positive, a negative and a muted amount. The name sits 6px under it in 12px.
  - **Chosen card:** a 2px border in the current Colour Theme's `accent` and a bold name; the rest have a 1px border. Keyboard focus draws a 1px `foreground` ring 2px outside the card, so focus and choice never hide each other.
  - **Override and contrast notes** (DESKTOP-133): while `[theme]` overrides any role in the Colour Variant drawn, a 12px `muted` note 6px under the grid reads "positive overridden by [theme] in your configuration file. Restart to change them."; each contrast failure adds a 12px line 4px below in the negative text shade, "positive on background is 2.6:1, needs 3:1". The override is still used.
- **Footnote:** these values are read from `personal-ledger.conf` at start-up and written back here; ledger-scoped Preferences are under **General**. DESKTOP-064 still says "Ledger & units".

#### 16f keyboard

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous field. |
| `tab` | Next field. |
| `tab` onto the grid, then arrows or `h`/`j`/`k`/`l` | Move focus between Colour Theme cards; `enter` chooses. Focus and hover never preview. |
| `[` / `]` | Medium only: previous / next page, General and Units. |
| `p` | Medium only: open the page picker. |
| `esc` | Medium only: back to the Settings index. |

#### 16f known gaps

Two differences from the code in `settings/display/render.rs` remain, both inherited from 16f:

- **Locale isn't drawn.** The code opens the field column with a read-only Locale field (`locale_field`: the effective Locale, where it came from, and a hint that it changes through the `locale` setting or `--locale` and a restart, per [ADR-0021](../../../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md)). No frame shows it.
- **Decimal & thousands separator isn't in the code.** The frames draw it from 16f, but the Desktop has no such field: number formatting follows the Locale (ADR-0021).

**16g at each Window Size.** Three cards after 16g draw the Units page in the Window Size Chrome from `../01-shell/README.md#window-sizes`. The page content, both tables and their columns are 16g's, capped at 840px; the Chrome and the heading change, and X-Large adds a detail pane.

- **DESKTOP-123 · Medium (960 × 1040):** icon rail (52px), no docked index. The heading is the breadcrumb **Settings › Units ▾** with the page picker caret. Status line `j/k section · [/] pages · p picker · esc settings` and `…/teda.pldb`.
- **DESKTOP-124 · Large (1280 × 800):** expanded rail (206px) and the docked 232px Settings index with Units selected. Heading **Units** alone. The View is 782px, so the content runs just under the cap.
- **DESKTOP-125 · X-Large (1920 × 1080):** the same Chrome as Large. The tables stop at 840px and Units docks a 400px detail pane at the View's right edge, the Documents and Inventory detail pane treatment (`border-left:2px solid rgba(32,30,29,.38); background:#eae9e9`). The selected row in the UNITS table takes the selection treatment (`background:#201e1d; color:#f3f2f2`, muted cells #d7d3d3); btc in the sample. Both tables drop their ACTIONS columns at X-Large: the pane carries every action, for the selected unit and for its price source.
  - **Groups:** UNIT, PRICE SOURCE and ACTIVITY, each a caps label (`800 10px/1`, `.11em`, #201e1d) over a 2px rule, body `padding:12px 16px 16px`.
  - **Fact rows** (`padding:5px 0; border-bottom:1px solid rgba(32,30,29,.15); 12px`, label #605d5d, value right-aligned and tabular).
  - **UNIT:** the title `btc · Bitcoin` (800 15px/1.25) and the meta line `crypto · valued in aud` (11.5px #605d5d), fact rows Flags — · Used by 1 account · 38 transactions, then **Edit unit** and **Delete unit**.
  - **PRICE SOURCE:** the selected unit's price source, so a source is reached by selecting the unit it values. Fact rows Source CoinGecko · Updates auto, every 15 min · Last price 148,212.40 aud · Last updated 5 minutes ago, then **Test**, **Edit source** and **Delete source**. A unit without a source would show an **Add price source** button here instead.
  - **Action buttons:** every pane button is the same size, 112 × 34px (three fill the 368px row with 8px gaps), `border:1px solid rgba(32,30,29,.30); background:#f3f2f2; 800`, left-aligned under their group's fact rows. Edit and Delete open the 16c–16d modals; the pane shows, it doesn't edit.
  - **ACTIVITY:** the page's outcomes, newest first, one 12px row each with a 38px tabular time, the outcome, and an 11px #9b9797 sub-line. Sample: 14:20 *Tested Bitcoin price source* (CoinGecko · 312 ms · 148,212.40 aud), 14:12 *Deleted unit eth*, 13:58 *Saved unit vas*. A closing 11px #9b9797 note reads "This session's Units outcomes, newest first. Errors still toast."
  - **Instead of Toasts:** while the pane is docked, a Units Success or Info outcome (`toast-unit-deleted`, a saved unit, a price-source test) lands in ACTIVITY and raises no Toast, since the pane is already in view beside the row it concerns. Errors still toast ([ADR-0027](../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)). Below X-Large there is no pane and the outcomes toast as before. ACTIVITY lists the whole page's outcomes, not only the selected unit's, so a delete stays visible after its row has gone.
  - **Status line:** `j/k unit · e edit · d delete · n new · t test source · tab next section · b collapse sidebar`.

### Dialogs
All dialogs are 420px, `border:2px solid #201e1d`, with the shadow `0 16px 48px rgba(32,30,29,.40)`. The shell behind is dimmed.

| Frame | Dialog | Notes |
| --- | --- | --- |
| — | Add unit | Not drawn separately. It is 16c's form, empty, with **Add** as the confirm. Fields: Code, Name, Type (currency / cryptocurrency / custom). |
| **16c** | Edit unit | Pre-filled `aud`. Usage notice (`#eae9e9`, 2px left rule `#ec3013`, 11.5px): "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references." Cancel / Save. |
| **16d** | Delete unit | Red treatment: border and header rule `#ec3013`, title `#ae1800`. Names every reference. Has a "Type btc to confirm" field; **Delete unit** stays disabled until the text matches exactly. |
| **16e** | Add institution | Name, Account types as **multi-select chips** (savings / credit card / offset / loan / investment), Default unit. Cancel / Add institution. |

- Opening a dialog moves focus to its first field and traps focus while open. Closing returns focus to the trigger.
- `esc` cancels. `enter` submits when the confirm button is enabled.
- Plain fields **save on change** — there is no page Save button. Creating and deleting always go through a dialog.

---



### Dialogs added in v6 (16r–16t, 16w–16y)
All three share the dialog treatment of 16c–16e: dimmer `rgba(32,30,29,.30)` over the Settings page, panel `#f3f2f2; border:2px solid #201e1d; box-shadow:0 16px 48px rgba(32,30,29,.40)`, column flex. Title bar `padding:18px 20px; 800 16px Archivo; border-bottom:2px solid rgba(32,30,29,.30)`. Body `padding:20px` (18px on 16t), field `gap:16px` (14px on 16t). Labels `800 12px; margin-bottom:6px`; inputs `padding:8px 10px; border:1px solid rgba(32,30,29,.30); 13px; #f3f2f2`; help text 11.5px #605d5d, `margin-top:6px`. Footer `padding:16px 20px; border-top:1px solid #d7d3d3`: a 11.5px #605d5d note (flex 1), **Cancel** (transparent, 1px border), primary (`#201e1d` fill, `#f3f2f2`). Buttons `padding:8px 16px; 800`. The status bar shows the DIALOG chip, key hints and a right-hand state.

| Frame | Dialog | Content |
| --- | --- | --- |
| **16r** | Edit sync server (480px) | **Server URL** `https://sync.ledger.localhost` (help: https:// is required; include the port if it isn't 443). **Access token**, masked, `letter-spacing:.15em` (help: from the server's admin page, stored in the OS keychain, never synced). **Test again** button (`#eae9e9`, 1px border), then a result box (`padding:10px 12px; #eae9e9; 12px`): green dot `#2ecc71`, **Connected · 42 ms**, then `server v1.4.2 · token accepted · tested 14:23` in #605d5d. Footer note "Saving reconnects and syncs this device."; **Save** primary. Status bar: `t test · esc cancel · enter save`, right `test passed`. Opened from 16k **Edit**. |
| **16s** | Edit sync server, test failed | Same dialog with URL `https://sync.ledger.home:8443`, button label **Test connection**, and the result box in the error treatment: border and square (not round) marker in `#ae1800`, headline `#ae1800` **Couldn't connect · connection refused**, then `sync.ledger.home:8443 didn't answer. Check the address and port, and that the server is running.` The URL stays editable. Footer note "Save unlocks after a passing test."; **Save** is disabled (`disabled`, fill `#bab6b6`, no pointer cursor). Status bar right `test failed`. Name the cause in plain words for each failure (refused, timed out, bad certificate, token rejected). |
| **16t** | Set up Git backup (540px) | **Remote URL** `git@github.com:teda/ledger-backup.git` (help: use a private repository, SSH `git@…` or HTTPS). A two-column row (`minmax(0,1fr) minmax(0,1.6fr)`, `gap:12px`): **Branch** `main`, and **Authentication** as a two-segment toggle, **SSH key** (active, inverted) / **Access token**. Below, the credential field follows the toggle: for SSH key a select (`~/.ssh/id_ed25519`, `~/.ssh/id_rsa`, `Generate a new key…`). **Push schedule**: four-segment toggle After changes (active) / Hourly / Daily / Manual, help "After changes waits for 5 minutes of quiet, then pushes at most hourly." **Commit message**: fixed prefix chip `yyyy-mm-dd hh:mm ·` (#eae9e9, tabular, not editable) beside an input `ledger backup`; help "The push's date and time is added in front automatically, e.g. 2026-09-14 14:00 · ledger backup." **Test connection** does a dry-run push; the result box shows green dot, **Ready · repository is empty**, `authenticated as teda · repository is private · dry-run push to main succeeded`. Footer note "Saving makes the first push (2.84 mb)."; primary **Save & push**. Opened from 16l **Set up / Edit**. |
| **16w** | Add provider (520px) | **Provider** four-segment toggle Claude (active) / OpenAI / Ollama / Custom. **API key** masked (`letter-spacing:.15em`; help: stored in the OS keychain, never synced). **Model** select (`claude-sonnet-5-5`, `claude-opus-5-5`, `claude-haiku-4-5-20251001`). A checkbox (14px square, 1px `#201e1d`, filled with ✓ when on) **Use as the default provider**. **Test connection**, then the result box: green dot, **Connected · 310 ms**, `key accepted · claude-sonnet-5-5 available · tested 14:31`. Footer note "Saving stores the key in the OS keychain."; primary **Save**. Status bar: `t test · esc cancel · enter save`, right `test passed`. Ollama replaces API key with a URL field (`http://localhost:11434`); Custom adds a base URL and an optional key. |
| **16x** | Edit provider, key rejected (520px) | Same dialog for OpenAI (`gpt-5`), default checkbox off. Result box in the error treatment: border and square marker `#ae1800`, headline **Key rejected · 401**, `OpenAI didn't accept this key. Paste a new key from your OpenAI account, then test again.` Footer "Save unlocks after a passing test."; **Save** disabled (`#bab6b6`). Status bar `t test · esc cancel`, right `test failed`. Opened from the red row in 16v. |
| **16y** | Add assistant, after Create (520px) | **Name** `Claude Desktop` (help: shown in Connected assistants). **Access** two-segment toggle (260px) **Read only** (active) / **Read & write**, help: read only can see accounts, transactions and reports; read & write can also add and edit. **Connection details**: two copy rows (`Address  http://127.0.0.1:7421/mcp`, `Token  plk_9f3a…c71e`; a 62px #eae9e9 label cell, 12px value, underlined **Copy**), and a `#ae1800` 11.5px line "Shown once. Copy the token now." Footer "The assistant now appears under Connected assistants."; secondary **Copy token**, primary **Done**. Status bar `c copy token · esc close`, right `token not shown again`. The step before Create (name and access only) is not drawn; Create generates the token. Closing hides it for good, so a new token needs Revoke then Add. |

## Keyboard
| Key | Action |
| --- | --- |
| `g s` | Open Settings on the last-visited page (General on first visit). |
| `j` / `k` | Move to the next / previous row on list pages or field on form pages. |
| `e` / `d` / `n` | Edit / delete / new on list pages. |
| `x` | Remove (Tags). |
| `m` | Merge (Tags). |
| `J` / `K` | Move the row down / up (Documents, Inventory rooms). |
| `→` / `←` | Expand / collapse (Categories, Inventory properties). |
| `r` | New room under the selected property (Inventory). |
| `esc` | Close a dialog. |
| `enter` | Submit a dialog when its confirm is enabled. On Accounts, opens the account's ledger. |

Palette commands: `:settings <page>` jumps straight to a page, e.g. `:settings accounts`.

## Components
| Part | Spec |
| --- | --- |
| Header | 48px, `#eae9e9`, `border-bottom:2px solid rgba(32,30,29,.38)`, `padding:0 10px 0 12px`, gap 12. Wordmark 800 13.5px; breadcrumb 12px #9b9797; three 26px window controls `rgba(32,30,29,.08)`. |
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
  page: General | Display | Units | Institutions | Accounts | Categories | Tags | Payees | Documents | Inventory
      | AiConnections | SyncServer | Backup | History | Logs | About
  dialog: None | AddUnit | EditUnit(id) | DeleteUnit(id) | AddInstitution
        | AddAccount | EditAccount(id) | AddCategory(parent?) | AddTag | MergeTags(a,b) | AddPayee
        | AddDocType | EditDocType(id) | RemoveDocType(id, moveTo?)
        | AddProperty | EditProperty(id) | RemoveProperty(id)
        | EditSyncServer | SetupGitBackup
        | AddProvider | EditProvider(id) | AddAssistant
        | AddRoom(propertyId) | EditRoom(id) | RemoveRoom(id, moveTo?)
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
  documentTypes[] { id, name, order, tracksDate?: Renews|Ends|Expires, remindDays?, taxYear: bool }
  properties[] { id, name, address, insurer?, policyNo?, sumInsured?, itemLimit?, order }
  rooms[] { id, propertyId, name, order }   // item count and value are derived from inventory items
ui
  inventoryExpanded: Set<PropertyId>       // session only
configuration // client-scoped, read from personal-ledger.conf
  dateFormat, separators, rowDensity, statusGlyphs
  sidebarHidden: bool             // start with the primary rail hidden; `b` toggles per session
  showNavHints: bool              // show the g-jump column in the primary rail
  syncServerUrl, storePath, traceLevel
```

## Known inconsistencies in the mockup
Build to the intent, not the literal mockup:
1. **Counts and visible rows don't match.**
   - Institutions says 7 but shows 6 rows.
   - Tags says 18 tags and 2 likely duplicates, but shows 7 rows and 1 duplicate.
   - Housing says "2 subcategories" but shows only Rent.
   Always derive counts from the data.
2. **Account institution names don't match the Institutions page.** Accounts uses ANZ, Vanguard and Crypto.com; Institutions uses ANZ Banking Group, Vanguard Investments and Cryptocurrency Exchange. Show the account's linked institution by its stored name.
3. **Account types don't match.** Institutions rows use crypto and retirement, but the Add institution chips (16e) are savings / credit card / offset / loan / investment. Use one account-type enum for both.
4. **Stale footnote links.**
   - The Display footnote points to "Ledger & units", which is now **General** and **Units**.
   - The Categories footnote says Reports is "14a"; Reports is now **15a** (package 15).
5. **Sync times disagree.** The header says `synced 14:22` but Sync server says Last sync 09:14. Both should read the same sync state.
6. **Two gaps remain.** The Payees page isn't drawn (see the Pages table). The Inbox mentioned on Documents (16p) isn't designed; it's tracked in `../README.md`.

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
13. **Inventory counts are derived.** ITEMS and VALUE come from the inventory items; never store them. Rooms order = the room tabs in Inventory (8a).

## Files
- `Settings.dc.html` — section 16: the page list, then the frames of every page, each followed by its Window Size cards: 16a, 16f, 16g, 16h, 16o, 16i, 16j, 16p, 16q, 16k–16n, then 16u, then dialogs 16c–16e and 16r–16t. Links to other sections point to the sibling package folders.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
