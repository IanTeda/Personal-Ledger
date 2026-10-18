# Handoff: Settings (paged) — v5

> **Package 16 of 20 · Settings** — one deck, `Settings.dc.html`, which opens on the page list linking to each page's frames: pages 16a, 16f, 16g, 16h, 16o, 16i, 16j, 16p, 16q, 16v, 16k, 16l, 16m, 16u, 16n, then dialogs 16c, 16c·src, 16d, 16z, 16z·custom, 16z·edit, 16z·del, 16e, 16r, 16s, 16t, 16w, 16x, 16y, plus the Window Size cards for General, Display and Units. Open the deck through `mise run mockups`. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

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

### Changes since v6 (18 October 2026)
- **Inventory (16q) is Locations and Areas, drawn like Categories (16i).** A Property is now a **Location** (a house, a storage unit, a car, a boat) and a Room an **Area** (Kitchen, Tray, Cabin). The table is one LOCATIONS list, NAME only: each location in 800 with a muted "n areas · n items" note, its areas indented under it. POLICY, SUM INSURED, ITEM LIMIT, ITEMS, VALUE and the address are gone from Settings: address and cover are kept on the register in Inventory (8a), and Add/Edit location asks only for the name and Unit. The rooms sub-table and its header **+** are gone; a location's **Add area…** menu item and `a` (was `r`) add an area. A third sample location, **Hilux**, shows a vehicle.

### Changes in v6 (9 October 2026)
- **New dialogs for AI Connections:** 16w Add provider, 16x Edit provider with a rejected key, 16y Add assistant (connection details shown once).
- **New page: AI Connections (16v)**, above Sync Server in the index. Two blocks: Model providers and Assistant access (MCP server). **Sync server** is now **Sync Server** in the index and the page heading.
- **Data & backup is now Backup (16l)**, with a second block, **Git backup**, that commits a text dump of the Ledger plus its Documents (through Git LFS) to a private remote repository on a schedule ([ADR-0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md)); the mockup's "commits the store" is superseded. The local block shows Store location, Last snapshot, **Back up now** and **Export ledger (CSV)**. **Set up / Edit** opens 16t.
- **Tracing (Logs) is now Logs (16m).** Behaviour is unchanged.
- **New page: History (16u)**, between Backup and Logs. Every change to the ledger, newest first, as one linear undo stack. Undo / Redo (`u` / `ctrl r`, anywhere in the app) step the marker one change; undone changes stay greyed above the marker until a new edit discards them; **Restore to here** jumps several steps; synced changes undo on every device.
- **New dialogs:** 16r Edit sync server (URL and access token, inline **Test connection**), 16s Test failed (Save stays disabled until a test passes), 16t Set up Git backup (remote, SSH key or access token, dry-run **Test**; the first push commits the whole store). Sync server (16k) **Edit** opens 16r.
- **Index** is now: General, Display, Units, Institutions, Accounts, Categories, Tags, Payees, Documents, Inventory, AI Connections, Sync Server, Backup, History, Logs, About.
- **Inventory (16q):** adding a room is explicit in two places: a bold **+ room** on every property row (works collapsed, and expands it) and a dashed **+ Add room** row closing each room list. `r` does the same on the selected property. Superseded: the rooms sub-table header's **+** and the property's context menu now do this (see 16q in the Pages table).
- **Header:** the back / forward buttons (`alt ←` / `alt →`) are gone from every frame.
- Layout specs for 16r–16u are in the page table (16u) and the dialog table (16r–16t) below.

### Changes since v4 (4 October 2026)
- **The design hasn't changed.** `Settings.dc.html` was re-checked against the master and matches it frame for frame.
- **This README is corrected:** the frame list and shell path are fixed, every page's sample data and status-bar hints are added, and a **Known inconsistencies** list covers the mockup details that shouldn't be copied literally.

### Changes in v4 (carried forward)
- **Renumbered** from 15x to 16x.
- **New page: Documents (16p)**, after Payees in the index (titled "Settings – Documents"; it was drafted as "Document types"). It defines the document types used by the Documents Type filter.
- **New page: Inventory (16q)**, after Documents. Properties and their rooms, the setup behind the Inventory registers (8a). Since v6 these are Locations and Areas.
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
- **List pages:** each table follows the Units table principles (16g):
  - **Title row:** the caps title or kicker on the left (`800 10px/1, .11em, #9b9797`) and a square 26px **+** button at the table's right edge (`border:1px solid rgba(32,30,29,.30); background:#eae9e9; 800 15px`, `title` and `aria-label` naming what it adds, e.g. Add institution), `margin-bottom:8px`. It replaces every `+ Add …` button above or under a table.
  - **No ACTIONS column** and no buttons or links inside rows, at any Window Size.
  - **Double-click** a row opens its edit modal; **right-click** opens a context menu at the pointer listing that row's actions, the destructive one last in red (`#ec3013`) below a 1px `#d7d3d3` rule. Menu: 190px (210px on Tags), `border:2px solid #201e1d`, the dialog shadow, items `padding:7px 12px; 12.5px`, the hovered item `rgba(32,30,29,.10)` and 800. The row right-clicked takes the selection treatment (`#201e1d` / `#f3f2f2`, muted cells `#d7d3d3`) while its menu is open.
  - Each page's frame draws one menu; the menus for the other rows are listed in the Pages table.
- **Breadcrumb:** the header reads `settings › <page>`.

### Pages
| Frame | Page | Heading note | Content |
| --- | --- | --- | --- |
| **16a** | General | preferences · synced · ledger identity | Ledger name, Owner, Financial year starts (july / january / april), Base unit (aud — Australian Dollar / btc — Bitcoin) in a 320px field column, beside a THIS LEDGER summary (accounts 7 · transactions 680 · units 3 · institutions 7). Footnote: General settings are ledger-scoped Preferences that travel with the ledger as Change Sets; client-only Configuration is under Display. |
| **16f** | Display | configuration · local · never synced | Segmented controls: Date format (12 sep 2026 / 12/09/2026 / ISO), Decimal & thousands separator (1,234.56 / 1.234,56 / 1 234.56) and Row density (compact / regular / roomy). Radios: Status glyphs (unicode — ○ ◐ ● ⚑ / ascii fallback — o / x !). Beside them, a live PREVIEW (DATE · PAYEE · AMOUNT): ● 12 sep 2026 Woolworths Metro −86.40 · ◐ 10 sep 2026 Telstra −99.00 · ⚑ 09 sep 2026 Dept of Education +4,210.00. A footnote says these values are read from `personal-ledger.conf` at start-up and written back here. Below the field column, a **Sidebar** field group (same `.field` label style as Status glyphs, `margin-top:18px`, 300px max) with two checkboxes (14×14, `accent-color:#201e1d`, square; bold 12.5px label over an 11.5px #605d5d hint): **Hide sidebar** (off) — "Start with the primary rail hidden. **b** or the header toggle still shows it for the session." · **Show keyboard navigation hints** (on) — "Show each destination's **g**-jump, e.g. **g d**, at the right of its row. The jumps work either way." |
| **16g** | Units | ledger data · synced · 3 units | UNITS table: an unheaded 24px glyph column · CODE 100 · NAME flex · TYPE 90 (left-aligned) · PRICE SOURCE 130 (left-aligned), with no FLAGS or ACTIONS column. The glyph column, at the far left, marks the base unit with ◆ (#ec3013); the base unit is the default unit, so there is no separate default flag. Rows: aud Australian Dollar · currency · USD (◆) · btc Bitcoin · crypto · CoinGecko · vas Vanguard Aus Shares · etf · Manual entry. Under the table, aligned right, the legend `◆ base unit` (11.5px #605d5d). Each table's caps title shares its row with a square 26px **+** button (`title` Add unit / Add price source) at the table's right edge, in place of the old **+ Add unit** and **+ Add price source** buttons under the tables. PRICE SOURCES table: NAME 130 · SOURCE flex · LAST UPDATED 130, with no ACTIONS column. NAME is the price source's own name, not the unit it values, and SOURCE is the URL its prices are fetched from (a #9b9797 — for a source with none). Rows: CoinGecko · https://api.coingecko.com/api/v3 · 5 minutes ago, and Manual entry · — · —. Row actions: **double-click** a row opens its edit modal; **right-click** opens a context menu at the pointer listing that row's actions, the destructive one last in red below a rule (unit: Edit unit… · Delete unit…; price source: Test price source · Edit price source… · Delete price source…). The row right-clicked takes the selection treatment while its menu is open. 16g draws the unit menu on btc, DESKTOP-124 the price source menu on CoinGecko, and **DESKTOP-134**, the card after 16g, draws double-click as a before and after: vas selected under the pointer, then **Edit unit — vas** (the 16c modal) over the dimmed table. |
| **16h** | Institutions | ledger data · synced · 7 institutions | Title INSTITUTIONS with a **+** (Add institution, opens 16e). INSTITUTION flex · ACCOUNT TYPE 140, no ACTIONS column. Rows: ANZ Banking Group (savings · offset), American Express (credit card), Vanguard Investments (investment), Westpac Banking (savings), Cryptocurrency Exchange (crypto), Superannuation Fund (retirement). Double-click opens 16e pre-filled as Edit institution. Menu: Edit institution… · Delete institution…, drawn on American Express. |
| **16o** | Accounts | ledger data · synced · 7 accounts · net worth **83,995.87 aud** | Kicker BY TYPE with a **+** (Add account). Grouped tables (BANK, CREDIT CARD, LOAN, INVESTMENT), each with NAME flex · INSTITUTION 130 · UNIT 90 · BALANCE 120 (right, tabular), no ACTIONS column. Double-click opens Edit account (17c). Menu: Open ledger · Edit account… · Delete account…, drawn on ANZ Offset. Rows are `padding:8px 16px`. BANK: ANZ Everyday 4,182.55 · ANZ Offset 61,204.10 (ANZ, aud). CREDIT CARD: Amex Platinum −2,318.44. LOAN: Home Loan (ANZ) −381,311.34. INVESTMENT: Vanguard VAS 1,240 u (vas) · Bitcoin (Crypto.com) 0.4120 u (btc). Add and edit reuse the account form from section 17 (17b/17c). Status bar: `j/k row · enter open ledger · e edit · d delete · n new`. |
| **16i** | Categories | ledger data · synced · 12 categories · 3 levels deep | Kicker EXPENSE · 10 with a **+** (Add expense category), and INCOME · 2 with its own **+** (Add income category); each adds a top-level category. A nested tree, NAME only, rows `padding:6px 14px`: ▾ disclosure, parent rows in 800 with an "n subcategories" note, no row buttons. Double-click opens Edit category. Menu on a parent: Add subcategory… · Edit category… · Delete category… (drawn on Food, which takes the selection); on a leaf: Edit category… · Delete category…. EXPENSE: ▾ Housing › Rent · ▾ Utilities › Electricity, Water · ▾ Food › Groceries, Dining · Transport · Household. Then INCOME · 2: Salary, Interest. No budget or spend columns. Footnote: categories nest to any depth, and only leaf categories can be picked on a transaction. Keys: `j/k`, `→/←` expand/collapse, `e`, `d`, `n`. |
| **16j** | Tags | ledger data · synced · 18 tags · **2** likely duplicates (#ae1800) | Kicker A–Z with a **+** (Add tag). Each row has a colour swatch (10×10) and the tag name in 800, no ACTIONS column. Rows: shared, reimbursable, tax-deductible, Shared (looks like a duplicate of "shared"), work-trip, gift, one-off. Likely-duplicate rows keep their warning chip; the merge moves into the menu. Double-click opens Edit tag. Menu: Edit tag… · Remove tag…, led on a likely duplicate by **Merge into “shared”…** (drawn on Shared); the merge flow is unchanged from 19e. No transaction, total or last-used columns. Keys: `j/k`, `e`, `x`, `m`, `n`. |
| — | Payees | ledger data · synced | **Indexed but not yet drawn.** Build it to the same pattern as 16j: a kicker A–Z with a **+** (Add payee), then name · default category · match rules with no ACTIONS column, double-click to edit and a menu of Edit payee… · Delete payee…, reusing the Payees management view in section 20 (20a) with its count and spend columns dropped. |
| **16p** | Documents | ledger data · synced · 7 types · 392 of 412 files typed | Kicker IN TYPE-FILTER ORDER with a **+** (Add document type). Table: TYPE flex (800) · TRACKS DATE 150 (Renews / Ends / Expires, or — in #9b9797) · REMIND 130 ("30 days before", or —) · TAX YEAR 90 (Yes / No) · FILES 60 (right, tabular), no ACTIONS column. Rows `padding:6px 14px; gap:14px; 12.5px`; selected row inverted (`#201e1d`/`#f3f2f2`). Double-click opens Edit type. Menu: Edit type… · Move up · Move down · Remove type…, drawn on Insurance. Rows (TYPE · TRACKS DATE · REMIND · TAX YEAR · FILES): Receipts — — Yes 186 · Statements — — Yes 94 · Tax — — Yes 31 · **Insurance Renews 30 days before No 18 (selected)** · Warranties & manuals Ends 30 days before No 42 · Contracts Ends 60 days before No 9 · Identity Expires 6 months before No 6. Below, three 12px #605d5d notes (max 640px) explaining Tracks date, Tax year and removal (20 untyped files stay in the Inbox). Row order is the order of the Documents Type filter; reorder with `J`/`K`. Removing a type that still has files asks where to move them. Keys: `j/k`, `e`, `x`, `n`. |
| **16q** | Inventory | ledger data · synced · 3 locations · 14 areas · 241 items | Drawn like Categories (16i). Kicker LOCATIONS · 3 with a **+** (Add location). One table, header NAME only, rows `padding:6px 14px; 12.5px`, 1px `rgba(32,30,29,.15)` rules: a location row has a ▾/▸ disclosure, its name in 800 and a 10.5px #9b9797 note "n areas · n items"; its areas follow indented 20px, name only, in area-tab order. No address, POLICY, SUM INSURED, ITEM LIMIT, ITEMS or VALUE columns, and no rooms sub-table: address and cover are kept on the register in Inventory (8a). Double-click opens Edit location (name and Unit) or Edit area; the disclosure alone expands and collapses. Menus: a location's Add area… (works collapsed, and expands it) · Edit location… · Remove location…; an area's Edit area… · Move up · Move down · Remove area…, drawn on Office. Sample: **12 Elm St contents** (10 areas · 214 items), expanded with Living · Kitchen · Bedroom 1 · Bedroom 2 · Bedroom 3 · **Office (selected, inverted)** · Garage · Laundry · Bathroom · Outdoor & shed; **Storage unit** (1 area · 18 items) and **Hilux** (3 areas · 9 items: Cab, Tray, Toolbox) collapsed (▸). Footnote (11.5px #605d5d): each location is one Inventory register (switch with `o`) and its areas are the area tabs, in this order; address and cover are kept on the register in Inventory; removing an area with items asks where to move them; removing a location keeps its items' transactions and documents. Keys: `j/k`, `→/←`, `e`, `x`, `n` new location, `a` new area. |
| **16v** | AI Connections | this device · 3 providers · 2 assistants | `max-width:640px` column, `gap:32px`, two blocks, each with a caps header (`800 10px/1`, `.11em`, #605d5d) over a 2px rule and a square **+** at the right. Each caps header ends with a square **+** in place of the underlined `+ Add …` link, and rows carry no action links (the Units table principles). **MODEL PROVIDERS** (+ Add provider): rows `padding:9px 10px`, 1px `#d7d3d3` rule — name 800 over a 11px #605d5d line (model · key location or URL) and a 150px status (6px dot + 800 12px: connected `#2ecc71`, key rejected `#ae1800`). Double-click opens Edit provider (16x for a rejected key). Menu: Test connection · Edit provider… · Make default · Remove provider…, drawn on OpenAI. The default provider carries a dark **DEFAULT** chip. Sample: Claude `claude-sonnet-5-5` (default), Ollama (local) `llama3.1:8b` at `http://localhost:11434`, OpenAI `gpt-5` with a rejected key. Note: keys are stored in the OS keychain and never synced; only selected transactions are sent; **Test** checks key and model. **ASSISTANT ACCESS · MCP SERVER** (+ Add assistant): label / value rows — Server (dot + `on · 127.0.0.1:7421`), Access (two-segment toggle **Read only** active / **Read & write**, 260px), Approval (`ask before each change`) — then a caps `CONNECTED ASSISTANTS` list in the same row treatment: Claude Desktop (read only · last used today 14:02, connected) and Claude Code (read only · last used 12 sep 2026, idle). An assistant's menu holds Revoke assistant… alone; double-click does nothing, since an assistant has nothing to edit. Note: the server listens on this device only; revoking ends the session at once. Add provider opens 16w, a rejected provider's Edit opens 16x, Add assistant opens 16y. Decided in [ADR-0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md): everything here is per-Client Configuration, never synced, and Assistant access ships **read only**, so build the Access toggle with Read & write disabled until a follow-up ADR. |
| **16k** | Sync Server | this device · synced · every 30 seconds | Server URL `sync.ledger.localhost`, Status (green dot `#2ecc71` + "connected"), Last sync `14 sep 2026 · 09:14`, **Sync now**. **Edit** opens 16r; a failed **Test connection** shows 16s. |
| **16l** | Backup (was Data & backup) | this device · local files · aud · 2.84 mb | LOCAL block: Store location `~/.ledger/personal`, Last snapshot `12 sep 2026 · 23:10`, **Back up now**, **Export ledger (CSV)**. GIT BACKUP block under it, same row treatment (label left #605d5d, value right 800 12px; `padding:8px 0`, 1px `#d7d3d3` rules), caps header `GIT BACKUP` with an underlined **Edit** at the right over a 2px rule: Remote `git@github.com:teda/ledger-backup.git` (11px), Branch `main`, Visibility (green dot + `private · checked 14:00`), Authentication `SSH key · ~/.ssh/id_ed25519`, Schedule `after changes · at most hourly`, and Commit message as a read-only prefix chip `2026-09-14 14:00 ·` (#eae9e9, tabular) beside the editable text `ledger backup`. **Set up / Edit** opens 16t. |
| **16m** | Logs (was Tracing) | this device · diagnostic · last 1000 entries | The page body is a flex column (`overflow:hidden`). A `flex:none` row (`space-between; gap:12px; margin-bottom:10px`) holds the level radios (error / warn / info / debug) on the left and **Clear logs** (`.btn.btn-secondary`, 32px) on the right. Below it the log box fills the remaining width and height (`flex:1; min-height:0`): `border:1px solid rgba(32,30,29,.30); #eae9e9; padding:10px 12px`, monospace 11px/1.6 #605d5d, newest first, `overflow-y:scroll` with an always-visible scroll bar (`scrollbar-color:#605d5d #d7d3d3`). Entries are `[hh:mm:ss] <subsystem>: <message>`, using the subsystems sync, txn, budget, import, docs, price, bills, events, notify, store, loans and cards. In GPUI, use a virtualised list with a persistent scroll bar. |
| **16u** | History | every change · newest first | One linear undo stack, newest first, with a marker. **Undo / Redo** move the marker one change; undone changes are greyed above it until a new edit discards them; **Restore to here** jumps several steps. Shared with `u` / `ctrl r` app-wide; synced changes undo on every device. See the frame for layout. |
| **16u** | History | all devices · last 30 days · 214 changes | Flex column (`overflow:hidden`). A `flex:none` toolbar (`gap:8px; margin-bottom:12px`): **Undo** `u` and **Redo** `ctrl r` buttons (`padding:8px 14px; #eae9e9; border:1px solid rgba(32,30,29,.30)`, 800, key hint 400 11px #605d5d), then `Next undo: <b>Added Camera House −4,380.00</b>` (12px #605d5d), then a spacer and an area filter select (All areas / Transactions / Budgets / Settings, 12px). Below it a bordered list fills the rest (`flex:1; min-height:0; border:1px solid rgba(32,30,29,.30)`). Header row `#eae9e9`, `800 10px/1`, `.11em`, #605d5d: TIME 56 · AREA 110 · CHANGE flex · DEVICE 96 · action 120. Rows `padding:9px 12px; gap:12px`, 1px `#d7d3d3` rule, time tabular, area #605d5d, device #605d5d 12px (`this device`, `laptop`), newest first. **Undone rows** sit above the marker: all text `#9b9797`, the change struck through, action **Redo to here** (underlined, 12px). The **marker** is a dark band between them (`#201e1d`, `#f3f2f2`, `800 10px`, `.11em`): `▸ CURRENT STATE`, a hairline, then `2 changes can be redone · a new edit clears them` (11px #bab6b6). Applied rows below the marker carry the action **Restore to here**. Sample: 14:21 Transactions *Recategorised Woolworths −86.40 → Groceries* and 14:19 Budgets *Dining limit 400.00 → 350.00* are undone; 14:12 *Added Camera House −4,380.00 · Everyday*, 14:05 *Renamed tag holiday → travel*, 13:58 Bills *Marked Rates Q1 paid · 612.00* (laptop), 13:51 *Split Bunnings −142.75 three ways*, 13:44 *Added area Garage · 12 Elm St*, 13:30 *Imported 6 rows · anz-sep.qif*, 12:02 Accounts *Created ANZ Offset · aud* (laptop), 11:47 *Deleted duplicate Netflix −22.99*, 11:40 Budgets *Rollover applied · September*. Undo / Redo step the marker one change; **Redo to here** and **Restore to here** move it several. A new edit discards the greyed rows. Synced changes undo on every device. Keys: `u` undo, `ctrl r` redo, anywhere in the app. |
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

**16g at each Window Size.** Four cards after 16g and DESKTOP-134 draw the Units page in the Window Size Chrome from `../01-shell/README.md#window-sizes`. The page content, both tables and their columns are 16g's, capped at 840px; the Chrome and the heading change, and X-Large adds a detail pane.

- **DESKTOP-123 · Medium (960 × 1040):** icon rail (52px), no docked index. The heading is the breadcrumb **Settings › Units ▾** with the page picker caret. Status line `j/k section · [/] pages · p picker · esc settings` and `…/teda.pldb`.
- **DESKTOP-124 · Large (1280 × 800):** expanded rail (206px) and the docked 232px Settings index with Units selected. Heading **Units** alone. The View is 782px, so the content runs just under the cap.
- **DESKTOP-125 · X-Large (1920 × 1080):** the same Chrome as Large. The tables stop at 840px and Units docks a 400px detail pane at the View's right edge, the Documents and Inventory detail pane treatment (`border-left:2px solid rgba(32,30,29,.38); background:#eae9e9`). The selected row in the UNITS table takes the selection treatment (`background:#201e1d; color:#f3f2f2`, muted cells #d7d3d3); btc in the sample. Neither table has an ACTIONS column at any Window Size; at X-Large the pane also carries every action, for the selected unit and for its price source.
  - **Groups:** UNIT, PRICE SOURCE, ACCOUNTS and ACTIVITY, each a caps label (`800 10px/1`, `.11em`, #201e1d) over a 2px rule, body `padding:12px 16px 16px`.
  - **Fact rows** (`padding:5px 0; border-bottom:1px solid rgba(32,30,29,.15); 12px`, label #605d5d, value right-aligned and tabular).
  - **UNIT:** the title `btc · Bitcoin` (800 15px/1.25) and the meta line `crypto · valued in aud` (11.5px #605d5d), fact rows Flags — · Used by 1 account · 38 transactions, then **Edit unit** and **Delete unit**.
  - **PRICE SOURCE:** the selected unit's price source. Selecting the source's own row in PRICE SOURCES shows it in full instead (DESKTOP-140). Fact rows Source CoinGecko · URL https://api.coingecko.com/api/v3 · Updates auto, every 15 min · Last price 148,212.40 aud · Last updated 5 minutes ago, then **Test**, **Edit source** and **Delete source**. A unit without a source would show an **Add price source** button here instead.
  - **ACCOUNTS:** the accounts held in the selected unit, one 12px row each: the account name (800) over its type (11px #9b9797) on the left, and on the right the balance in the unit's own precision over its value in the base unit (11px #9b9797), both tabular. A bold **Total** row with no rule closes the list. Sample, from the Accounts mockup: Bitcoin · Investment · 0.41200000 btc · 61,063.51 aud, then Total 0.41200000 btc · 61,063.51 aud. The base-unit value is the balance times the PRICE SOURCE's last price. A unit that no account uses would show "No accounts use this unit." in 11px #9b9797 instead.
  - **Action buttons:** every pane button is the same size, 112 × 34px (three fill the 368px row with 8px gaps), `border:1px solid rgba(32,30,29,.30); background:#f3f2f2; 800`, left-aligned under their group's fact rows. Edit and Delete open the 16c–16d modals; the pane shows, it doesn't edit.
  - **ACTIVITY:** the page's outcomes, newest first, one 12px row each with a 38px tabular time, the outcome, and an 11px #9b9797 sub-line. Sample: 14:20 *Tested CoinGecko price source* (btc · 312 ms · 148,212.40 aud), 14:12 *Deleted unit eth*, 13:58 *Saved unit vas*. A closing 11px #9b9797 note reads "This session's Units outcomes, newest first. Errors still toast."
  - **Instead of Toasts:** while the pane is docked, a Units Success or Info outcome (`toast-unit-deleted`, a saved unit, a price-source test) lands in ACTIVITY and raises no Toast, since the pane is already in view beside the row it concerns. Errors still toast ([ADR-0027](../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)). Below X-Large there is no pane and the outcomes toast as before. ACTIVITY lists the whole page's outcomes, not only the selected unit's, so a delete stays visible after its row has gone.
  - **Status line:** `j/k unit · e edit · d delete · n new · t test source · tab next section · b collapse sidebar`.
- **DESKTOP-140 · X-Large, a price source selected (1920 × 1080):** DESKTOP-125 with the CoinGecko row in the PRICE SOURCES table selected instead of a unit; btc drops back to an ordinary row. Selection moves between the two tables, so only one row is selected at a time and the pane follows it. The pane shows read-only what the 16z·edit modal shows, so nothing in the modal is hidden at X-Large.
  - **Groups:** PRICE SOURCE, VALUES and ACTIVITY, in DESKTOP-125's treatment. UNIT and ACCOUNTS belong to a selected unit and are not drawn.
  - **PRICE SOURCE:** the title `CoinGecko` (the source's name, 800 15px/1.25) and the meta line `built-in · prices crypto · key not needed`. Fact rows Source CoinGecko · built-in · Address `https://api.coingecko.com/api/v3` · Request `/simple/price?ids={symbol}&vs_currencies={quote}` · Price path `/{symbol}/{quote}` · Updates every 15 min · Last updated 5 minutes ago; the three URL parts are monospace and wrap anywhere. An 11px #9b9797 note: "Built-in sources update with the app, so their address isn't edited here." Then **Test**, **Edit source** and **Delete source**, and below them the last test in 16r's result box (`#f3f2f2` here, on the pane's grey): **Price found · 312 ms**, `btc · 148,212.40 aud · tested 14:20`. A custom source adds Authentication and Key parameter fact rows (the key itself is never shown) and drops the built-in note.
  - **VALUES:** the units this source prices, 16z·edit's usage notice as a list. One 12px row each: `btc · Bitcoin` over `symbol bitcoin · quoted in aud` (11px #9b9797, the symbol monospace) on the left, and the last price over its age on the right, tabular. A closing 11px #9b9797 note: "Each unit sets its own symbol in Edit unit. Deleting this source moves these units to Manual entry." A source no unit uses would show "No units use this source." instead.
  - **Manual entry:** selecting it shows the title and "Prices you enter by hand." with no fact rows and no buttons, since it is neither edited nor deleted; VALUES lists the units priced by hand.
  - **Status line:** `j/k price source · e edit · d delete · n new · t test · tab next section · b collapse sidebar`.

### Dialogs
All dialogs are 420px, `border:2px solid #201e1d`, with the shadow `0 16px 48px rgba(32,30,29,.40)`. The shell behind is dimmed.

| Frame | Dialog | Notes |
| --- | --- | --- |
| — | Add unit | Not drawn separately. It is 16c·src's form, empty, with **Add** as the confirm. Fields: Code, Type (currency / cryptocurrency / custom), Name, then the PRICE SOURCE group with Source set to Manual entry. |
| **16c** | Edit unit | Pre-filled `aud`. Usage notice (`#eae9e9`, 2px left rule `#ec3013`, 11.5px): "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references." Cancel / Save. |
| **16c·src** (DESKTOP-139) | Edit unit — btc, with its price source | Code and Type share a row (`minmax(0,1fr) minmax(0,1fr)`, `gap:12px`), then Name. A PRICE SOURCE group label (caps 10px, 2px rule) heads **Source**, a select of the Ledger's Price Sources (help: "From the Price Sources table. **+** on that table adds another."), then a row (`minmax(0,1.4fr) minmax(0,1fr)`) of **Symbol** `bitcoin` (monospace, help "CoinGecko's id for this unit.") and **Quoted in**, defaulting to `aud (base unit)`. **Test price**, then the 16r result box: **Price found · 298 ms**, `148,212.40 aud · tested 14:22`. 16c's usage notice closes the body. Choosing Manual entry hides Symbol, Quoted in and Test. The symbol belongs to this link, not to the source. Body gap 12px. |
| **16d** | Delete unit | Red treatment: border and header rule `#ec3013`, title `#ae1800`. Names every reference. Has a "Type btc to confirm" field; **Delete unit** stays disabled until the text matches exactly. |
| **16z** (DESKTOP-135, 480px) | Add price source, built-in | **Source** four-segment toggle CoinGecko (active) / Yahoo Finance / Frankfurter / Custom…. A read-only fact block (`#eae9e9`, fact rows as the X-Large pane's): Address `https://api.coingecko.com/api/v3` · Request `/simple/price?ids={symbol}&vs_currencies={quote}` · Prices crypto · Key not needed, both URLs monospace; help "Built-in sources update with the app, so their address isn't edited here." **Name** `CoinGecko` (help: shown in the Price Sources table and the unit picker). **Updates** select Every 15 min / Hourly / Daily / Manual only. Footer note "Units pick this source in Edit unit."; primary **Add price source**. A built-in source is stored by its id, so only its name and update interval belong to the Ledger. |
| **16z·custom** (DESKTOP-136, 620px) | Add price source, Custom… | Toggle on Custom…. **Name** `Alpha Vantage` and **Updates** Daily share a row. **Address** `https://www.alphavantage.co`, **Request** `/query?function=GLOBAL_QUOTE&symbol={symbol}` (help names the `{symbol}` and `{quote}` placeholders) and **Price path** `/Global Quote/05. price` (help: a JSON pointer to the price in the response), all monospace. One three-column row: **Authentication** toggle None / API key (active), **Key parameter** `apikey`, **API key** masked; help "The key is stored in the OS keychain on this device and never synced." A symbol field `VAS.AX` beside **Test with symbol**, then the result box **Price found · 412 ms**, `VAS.AX · 101.84 · tested 14:36`. Footer note "Saving stores the key in the OS keychain."; primary **Add price source**. Body gap 12px so the dialog fits the 800px frame. |
| **16z·edit** (DESKTOP-137, 480px) | Edit price source — CoinGecko | Opened by double-click or **Edit price source…**. **Name** editable; fact block Source CoinGecko · built-in · Address · Request · Price path `/{symbol}/{quote}`, read-only; **Updates** editable. 16c's usage notice: "Values 1 unit: **btc**. Last price 148,212.40 aud, 5 minutes ago." **Test price source** prices the first unit it values: **Price found · 312 ms**, `btc · 148,212.40 aud · tested 14:20`. Cancel / Save. A custom source opens 16z·custom's fields pre-filled instead. |
| **16z·del** (DESKTOP-138, 420px) | Delete price source — CoinGecko | 16d's red treatment. "This price source values **1 unit**. Deleting it cannot be undone." A reference box: "**btc** falls back to Manual entry and keeps its last price, 148,212.40 aud." "Type `CoinGecko` to confirm"; **Delete price source** stays disabled until the text matches. Manual entry is the fallback, so its row offers neither Edit nor Delete. |
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
| `J` / `K` | Move the row down / up (Documents, Inventory areas). |
| `→` / `←` | Expand / collapse (Categories, Inventory locations). |
| `a` | New area under the selected location (Inventory). |
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
| Add button | 26 × 26px square **+** at the right of a table's title row, `border:1px solid rgba(32,30,29,.30); #eae9e9; 800 15px`. |
| Row context menu | Opened by right-click at the pointer; see **List pages** under Page anatomy. Rows carry no action buttons. |
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
        | AddLocation | EditLocation(id) | RemoveLocation(id)
        | EditSyncServer | SetupGitBackup
        | AddProvider | EditProvider(id) | AddAssistant
        | AddArea(locationId) | EditArea(id) | RemoveArea(id, moveTo?)
  confirmInput: String
preferences   // ledger-scoped, sync as Change Sets
  ledgerName, owner, fyStart, baseUnit
  units[] { code, name, type, isBase }
  priceSources[] { unitCode, source, lastUpdatedAt }
  institutions[] { name, accountTypes[], defaultUnit }
  accounts[] { name, institution, type, unit, openingBalance, number? }
  categories[] { id, name, parentId?, kind }
  tags[] { name, colour }
  payees[] { name, defaultCategory?, matchRules[] }
  documentTypes[] { id, name, order, tracksDate?: Renews|Ends|Expires, remindDays?, taxYear: bool }
  locations[] { id, name, unit, order }    // address and cover are edited on the register in Inventory (8a)
  areas[] { id, locationId, name, order }  // item count and value are derived from inventory items
ui
  inventoryExpanded: Set<LocationId>       // session only
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
5. **Data-driven flags.** The ◆ base glyph renders only on the base unit; the legend under the table always shows it.
6. **Price sources are named for the source, not the unit.** NAME is the provider (CoinGecko) and SOURCE its URL; the unit a source values shows in the UNITS table's PRICE SOURCE column and the X-Large detail pane.
7. **A price source is fields, not one URL.** Address, Request template and Price path are stored apart; the unit's symbol lives on the unit's link to the source, and an API key lives in the OS keychain on each device, never in the synced Ledger.
8. **2px rules need `flex:none`.** Without it they collapse to hairlines.
9. **One heading per page.** Build the "Settings – <Page>" title from the index entry name so they never drift; the right-hand note is scope + meta.
10. **Scope radio-group names per instance**, so two rendered copies of a section don't share selection.
11. **No keybinding editor.** Bindings are static configuration.
12. **Zero radius, flush-left labels.** Labels stay flush left even inside wide buttons. Icons are Lucide at 14px with a 1.5px stroke.
13. **Sidebar settings apply live.** Toggling Hide sidebar collapses the rail immediately and persists for next launch; Show keyboard navigation hints hides only the binding column — the `g` jumps still work.
14. **Inventory counts are derived.** The "n areas · n items" note comes from the areas and inventory items; never store it. Areas order = the area tabs in Inventory (8a). Settings sets up locations and areas only; address and cover belong to Inventory.

## Files
- `Settings.dc.html` — section 16: the page list, then the frames of every page, each followed by its Window Size cards: 16a, 16f, 16g, 16h, 16o, 16i, 16j, 16p, 16q, 16k–16n, then 16u, then dialogs 16c, 16c·src, 16d, 16z–16z·del, 16e and 16r–16t. Links to other sections point to the sibling package folders.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
