# Handoff: Documents

## Overview
**Documents** is Personal Ledger's single library of proof: receipts, statements, policies, warranties, tax papers and identity scans. A document only matters for what it proves, so every file links to one or more records: a Transaction, an Inventory item, an Account (loan / card / investment), a Payee or a Bill.

### Changes in this revision
- **The Documents index rail (190px) is removed.** The list pane takes its width.
- **Filters are dropdowns in the toolbar,** inline with the search box: Type (multi-select), Tax year, Links, Sort.
- **Inbox moves to a header button** with its count badge (4c/4d), or a banner above the list (4e).
- **The storage note** ("1.8 GB beside household.pldb") moves into the list subtitle.
- **Document types are now user-managed** in Settings › Document types (`design_handoff_settings_v4/`, frame 16p). The Type filter lists them in the order set there.
- Frames: **4c** default library · **4d** Type dropdown open with active-filter chips · **4e** alternative with Type as tabs and an Inbox banner. **Pick 4c/4d or 4e**; they're alternatives, not both.

The ledger stores links and metadata. The files themselves stay on disk beside the ledger file (`household.pldb`) and are never renamed.

## About the design files
`Documents.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's existing patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` next to it. Links to other destinations (Inventory, Payees, Bills) point back to `../Ledger Desktop Shell.dc.html`; Document types are specified in `../design_handoff_settings_v4/`.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Documents depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Copy is final except sample data.

## Frame & shell
- Every frame is **1280 × 800**: a 48px header, then the body (`flex:1`).
- **Header:** `#eae9e9`, `border-bottom:2px solid rgba(32,30,29,.38)`, `padding:0 10px 0 12px`, gap 12. It holds the panel icon (28px box), the wordmark "Personal Ledger" (800 13.5px), a spacer, the `:` "run a command" affordance (`padding:4px 9px; border:1px solid rgba(32,30,29,.30); 12px #605d5d`) and "synced 14:22" (12px #605d5d).
  - **Align with the shell:** add the breadcrumb `ledger › documents` (12px #9b9797), the sync icon and the three 26px window controls, as on every other destination.
- **Primary rail (206px):** Documents is active (`g f`). Groups:
  - LEDGER: Dashboard `g d`, Transactions `g l`, **Documents `g f`**, Events `g e`, Notifications `g m` (red badge)
  - NET WORTH: Cash `g c`, Inventory `g o`, Loans `g n`, Credit cards `g k`, Investments `g i`
  - PLAN: Bills `g w`, Budgets `g b`, Reports `g r`
  - Settings `g s`, pinned below a 2px rule
- **Status bar:** the mockups omit it. Add the standard 28px status bar (see Keyboard) for consistency with other destinations.

## Layout (all frames)
Inside the body, left to right:

| Column | Width | Spec |
| --- | --- | --- |
| Primary rail | 206px | Shell, as above. |
| List pane | `flex:1; min-width:0` | Header, toolbar, (chips / tabs), list. `overflow:hidden`; the list scrolls. |
| Detail pane | 300px | `border-left:2px solid rgba(32,30,29,.38)`, `background:#eae9e9`. |

There is no Documents index rail.

### Detail pane (300px, all frames)
- **Preview:** 200px tall, `#d7d3d3`, 2px bottom rule. It shows a 128×172 page thumbnail centred (`#f3f2f2`, `box-shadow:0 2px 6px rgba(32,30,29,.2)`, 12px padding). In the app, render page 1 of the real file (PDF raster / image).
- **Body:** `padding:12px 16px; gap:10px`.
  - Title 800 15px/1.25, then a meta line 11.5px #605d5d.
  - **Fact rows:** `flex; space-between; padding:5px 0; border-bottom:1px solid rgba(32,30,29,.15); 12px`, label #605d5d, value right-aligned and tabular.
- **Footer:** `border-top:2px solid rgba(32,30,29,.38); padding:12px 16px; gap 8`, with a primary button (`flex:1`) and a secondary button, both 34px tall.

---

## 4c · Library with inline filters
**Purpose:** find a document and see what it proves.

### List header
`padding:16px 20px 12px`, 2px bottom rule.
- **Left:** `h2` "Documents" (28px/800, `margin:0 0 4px`), with the subline "412 files · receipts, statements, policies and tax papers, linked to what they prove · 1.8 GB beside household.pldb" (11.5px #9b9797).
- **Right**, gap 8, all 36px: **Inbox** (`.btn.btn-secondary`, with the red count badge `#ec3013`/`#f3f2f2`, 10px 800, `padding:1px 5px`, gap 8 after the label) · `Import…` (secondary) · `+ Add` (primary).

### Toolbar
`flex; align-items:center; gap:6px; padding:10px 14px; border-bottom:1px solid rgba(32,30,29,.30)`.
- **Search:** `.input`, 30px, 12.5px, `flex:1; min-width:0`, placeholder "/ search names and text inside documents". Search matches file names, metadata and extracted PDF/OCR text.
- **Filter dropdowns**, in order: **Type**, **Tax year**, **Links**, **Sort**. Each is a 30px trigger, `flex:none; padding:0 8px 0 10px; gap:6px; border:1px solid rgba(32,30,29,.30); 12px; white-space:nowrap`, reading label (#605d5d) · value (800) · 10px chevron.
  - **Idle:** `background:#f3f2f2`, value "Any" (Sort: "Newest").
  - **Active** (a value other than Any): `background:#201e1d; color:#f3f2f2`, label `#bab6b6`. The value names the choice ("FY 2025–26") or the count ("2 of 7").
  - **Open:** border becomes `2px solid #201e1d`.
- **Options:**
  - Type: multi-select over the document types from Settings › Document types, each with its count.
  - Tax year: single-select — Any · FY 2026–27 · FY 2025–26 · Earlier (AU financial year, 1 Jul – 30 Jun; start from Settings › General).
  - Links: single-select — Any · Linked · Not linked · by kind (Transaction, Inventory item, Account, Payee, Bill, Event).
  - Sort: Newest · Expiring. Expiring sorts by the nearest renewal / expiry / revalue date; rows without one go last.
- Filters combine with AND; search narrows the filtered set.

### Count line
`padding:8px 14px 4px`, caps label (`800 10px/1 Archivo; .11em; #9b9797`): "412 FILES · 3 NEED REVIEW" with no filter, or "60 OF 412 · 1 NEEDS REVIEW" when filtered. "Need review" counts rows whose date (renewal, expiry, revalue or warranty end) falls within the attention window.

### Document row
`flex; align-items:center; gap:12px; padding:7px 14px; border-bottom:1px solid rgba(32,30,29,.15)`.
- **File glyph:** 30×36, `border:1.5px solid #201e1d`, with the extension (`PDF` / `JPG`) bottom-centred in `800 8px Archivo .05em`.
- **Main column** (`flex:1; min-width:0; gap:3px`):
  - Title 800 12.5px, single line with ellipsis.
  - **Link chips**, gap 10: each is a 10px icon plus a label in 11px #605d5d. They show what the document links to (account, item, signed amount, payee, bill). Unlinked documents show "not linked" in #9b9797.
- **Right column** (112px, right-aligned, gap 3):
  - Document date, 12px #605d5d tabular.
  - Below it, one of:
    - **Type caps**, `10px 800 .08em #9b9797` (STATEMENT, RECEIPT, TAX, BILL).
    - **Date flag**, `10.5px 800 #ae1800`, for any forward-dated obligation: "Renews 14 Jan 2027", "Ends Aug 2027", "Revalue by Mar 2027", "Expires May 2032".
- **Selected row:** `background:#201e1d; color:#f3f2f2`. The glyph border becomes #f3f2f2, chips and date #d7d3d3, and the date flag #f3f2f2.

Sample rows, in order: ANZ Platinum statement — Sep 2026 · Receipt — Camera House (Sony A7 IV, −4,380.00) · **NRMA home contents — PDS & schedule (selected)** · Macquarie home loan — annual statement · CMC Markets — tax statement FY26 · Fridge warranty & manual · Engagement ring valuation (JPG) · Rates notice — Q1 (City Council, Bill) · Passport — scan (not linked).

### Detail (selected: NRMA home contents, all frames)
- **Title and meta:** "NRMA home contents — PDS & schedule", with "PDF · 24 pages · 1.4 MB · page 1 of 24".
- **Facts:** Type "Insurance · policy" · Document date 14 Jan 2026 · Renews **14 Jan 2027** (800 #ae1800) "· reminder set" · Tax year —.
- **LINKED TO** (caps label, gap 6): one link row per record, `padding:6px 8px; border:1px solid rgba(32,30,29,.30); background:#f3f2f2; 12px`. Each row has a kind label (62px, #9b9797) and the record name (800 #201e1d, no underline; it navigates):
  - Inventory → 12 Elm St contents
  - Payee → NRMA Insurance
  - Bill → Home contents · annual
- **Add link:** "+ Link to transaction, account, item…" (11.5px #605d5d) opens the link picker.
- **Footer:** **Open** (primary; opens in the system viewer) · **Show in folder** (secondary).

---

## 4d · Type dropdown open
Same as 4c, with the Type menu open and two types chosen.
- **Trigger:** active and open — "Type · 2 of 7".
- **Menu:** `position:absolute; top:34px; left:0` under the trigger, 244px, `background:#f3f2f2; border:2px solid #201e1d; box-shadow:var(--shadow-md); padding:6px 0; z-index` above the list.
  - Rows `flex; gap:9px; padding:6px 12px; 12.5px`: a 12×12 square checkbox (`border:1.5px solid #201e1d`, checked = `#201e1d` fill with a white tick), the type name (800 when checked) and its count (11px #9b9797, tabular). The hovered / keyboard row is `background:#e2e0df`.
  - Footer: `border-top:1px solid rgba(32,30,29,.30); padding:8px 12px 2px; 11.5px` — "Space toggles · Esc closes" (#605d5d) on the left, **Clear** (800) on the right.
  - Choices apply immediately; the menu stays open until Esc, a click outside, or Tab.
- **Active-filter chips** sit under the toolbar while any filter is set: `flex; gap:6px; padding:8px 14px; border-bottom:1px solid rgba(32,30,29,.15)`. Each chip is `#201e1d`/`#f3f2f2`, 11.5px 800, `padding:3px 7px; gap:6px`, with a 9px ×; clicking × removes that value. **Clear all** (11.5px #605d5d) ends the row.
- Count line: "60 OF 412 · 1 NEEDS REVIEW".
- Note: the mock's rows are unchanged sample data and don't reflect the filter.

## 4e · Alternative: Type as tabs, Inbox banner
Type is the filter used most, so it stays visible as tabs; the other filters stay dropdowns.
- **Header:** no Inbox button — `Import…` · `+ Add` only.
- **Inbox banner** (above the tabs, shown only while the Inbox has files): `flex; gap:10px; padding:8px 14px; #201e1d / #f3f2f2; 12px` — red count badge · "files in Inbox, waiting to be linked" (`flex:1`) · **Review →** (800). Clicking it opens the Inbox.
- **Type tabs:** `flex; gap:18px; padding:0 14px; border-bottom:2px solid rgba(32,30,29,.38)`. Each tab is `padding:9px 0 7px; 12.5px`, label plus count (11px #9b9797, 400). The active tab is 800 #201e1d with a `3px solid #201e1d` bottom border; others are #605d5d with a transparent border. Tabs: All 412 · Receipts · Statements · Tax · Insurance · Warranties · Contracts · Identity. Single-select. If the types overflow, the last tab becomes "More ▾".
- **Toolbar:** search · Tax year (shown active: "FY 2025–26") · Links · Sort.
- Count line: "121 OF 412 · TAX YEAR FY 2025–26".

## Inbox (not redrawn)
The Inbox frame (formerly 4b) isn't drawn in this revision. Build it as specified below, but **without the Documents index rail** — the list pane takes the width, as in 4c. It opens from the header **Inbox** button (4c/4d) or the banner's **Review →** (4e); a **← Documents** back link replaces the header buttons' Inbox entry.

### Inbox spec
**Purpose:** file everything that arrived unfiled (dropped, scanned, emailed or picked up from a watched folder) by accepting or correcting a suggested link.

### List header
- **Left:** `h2` "Inbox", with the subline "7 unfiled · dropped, scanned or imported from a watched folder · each gets a suggested link".
- **Right:** `Watched folder…` (secondary, 36px) and **Accept all strong matches · 2** (primary, 36px). This accepts only 3-of-3 matches.

### Column header
`padding:8px 14px; #eae9e9; 800 10px .11em #605d5d; 1px bottom rule`. Columns are FILE (252px) and SUGGESTED LINK (`flex:1`, `padding-left:26px`).

### Inbox row
`flex; align-items:center; gap:12px; padding:9px 14px; border-bottom:1px solid rgba(32,30,29,.15)`.
- **File glyph:** as in 4c.
- **File column** (210px): file name 800 12.5px with ellipsis, then source and date in 11px #9b9797 ("Scanned 29 Sep · 1 page", "Emailed 14 Sep", "Downloads · 26 Sep").
- **Arrow icon** separating file from suggestion.
- **Suggestion column** (`flex:1`): a bold summary in 12.5px ("Receipt · Woolworths · 212.40"), then a 11px #605d5d line with the **match meter** and the target record.
- **Match meter:** three 6×6 squares, gap 2. Each filled square (`#201e1d`) is one agreeing signal: amount, date, payee. Empty squares are `border:1px solid #9b9797`. 3/3 is a strong match, 2/3 is likely, 1/3 is weak.
- **Action:** the focused row shows **Accept** as a primary button (30px, `padding:0 12px`) with the hint "y". Other rows show a secondary Accept.
- **Focused row:** `background:#f3f2f2; outline:2px solid #201e1d; outline-offset:-2px`. This is outline focus, not the inverted fill used for selection in 4c.
- **Unreadable file:** the suggestion reads "Unreadable — no amount found" (800 #ae1800), with "File it by hand, or skip" below. The action becomes **File…**. Never guess a link for unreadable files.

Sample rows: IMG_4471.jpg → Woolworths −212.40 (3/3) · bunnings-invoice-INV88213.pdf → Bunnings −612.35 (3/3) · Sonos_order_confirmation.pdf → Inventory item Sonos Era 100 and its transaction (2/3) · ATO_NOA_2026.pdf → Tax · FY 2025–26 (1/3) · scan0012.pdf (unreadable).

- **Below the list:** "2 more below · accepted files move to their type and tax year; nothing is renamed on disk" (11px #9b9797).
- **Drop target** at the bottom: `margin:0 14px 14px; border:2px dashed #9b9797; padding:16px; 12.5px #605d5d`, with an icon and the text "Drop files anywhere in the app to add them to the Inbox".

### Detail (focused: IMG_4471.jpg)
- **Title and meta:** "IMG_4471.jpg", with "Read from the image · check before accepting".
- **Extracted facts:** Merchant Woolworths Metro · Date 29 Sep 2026 · Total **212.40** · Type Receipt. Each fact is editable with `e`.
- **Suggested link card:** `border:2px solid #201e1d; background:#f3f2f2; padding:9px 10px`.
  - Caps label SUGGESTED LINK (9.5px).
  - "29 Sep · Woolworths · −212.40" (800).
  - "ANZ Platinum · Groceries · no document yet".
  - The match meter, with "amount, date and payee match".
- **Other candidates:** "Other candidates: none within 7 days" (11.5px #605d5d).
- **Key hint strip:** `padding:8px 16px; 11px; 1px top rule`, reading **y** accept · **e** edit · **x** skip · **j k** move.
- **Footer:** **Accept & next** (primary, `flex:1`) · **Link elsewhere…** (secondary; opens the link picker).

---

## Interactions & behaviour
- **Selecting:** clicking a row or pressing `j`/`k` selects it and fills the detail pane. `enter` / **Open** opens the file in the system viewer.
- **Link navigation:** clicking a LINKED TO name navigates to that record's destination (Inventory, Payees in Settings, Bills, a Transaction in Transactions, an Account, or an Event).
- **Events:** a document can link to an event (a trip's tickets, an accident's claim papers, the statement behind a balance check). The event's detail lists it under DOCUMENTS; deleting the event keeps the document. See `design_handoff_events/`.
- **Date flags** drive Notifications. A renewal or expiry within the user's lead time raises a notification under the rule set in Notifications › Rules.
- **Inbox accept (`y`)** links the file, sets its type and tax year from the suggestion, removes it from the Inbox, decrements the badge and moves focus to the next row. "Accept & next" does the same.
- **Edit (`e`)** makes the extracted facts editable and recomputes candidates on change.
- **Skip (`x`)** leaves the file in the Inbox and moves to the next row.
- **Accept all strong matches** applies to 3/3 rows only. Show a count first and allow undo (`u`).
- **Drop anywhere:** a file dropped on any destination is added to the Inbox, and the Inbox badge increments. While dragging, show the dashed drop target as a full-window overlay.
- **Watched folder…** chooses a folder that is polled for new files. Files are referenced, not moved.
- **Search** is debounced at about 150ms. Matched text inside documents should be highlighted in the detail preview.
- **Empty states:**
  - Inbox empty: "Nothing to file", with the drop target still shown.
  - Facet with 0 documents: the list is replaced by a one-line note and `+ Add`.
- **Loading:** text extraction / OCR runs in the background. A row being read shows "Reading…" in the suggestion column, with no meter.
- **Errors:**
  - A missing file on disk shows the row title in #ae1800 with "File missing — locate…" and keeps its links.

## Keyboard
| Key | Action |
| --- | --- |
| `g f` | Open Documents (last mode; the Library on first visit). |
| `j` / `k` | Next / previous row. |
| `/` | Focus search. |
| `enter` | Open the file. |
| `i` | Toggle Inbox / Library. |
| `f` | Open the first filter dropdown; `←`/`→` move between dropdowns, `space` toggles, `esc` closes. |
| `[` / `]` | Previous / next Type tab (4e). |
| `F` | Clear all filters. |
| `y` | Accept the suggestion (Inbox). |
| `e` | Edit extracted facts / metadata. |
| `x` | Skip (Inbox). |
| `l` | Link to… (opens the picker). |
| `o` | Show in folder. |
| `s` | Toggle sort Newest / Expiring. |
| `esc` | Clear search / close the picker. |

**Status bar (to add):**
- **Library:** `NORMAL` · `j/k row · enter open · / search · l link · o show in folder · i inbox`
- **Inbox:** `j/k row · y accept · e edit · x skip · l link elsewhere`

The right side of the status bar shows "1.8 GB beside household.pldb".

Palette: `:documents`, `:documents inbox`, `:attach` (on a selected transaction or item, opens the file picker and links the result).

## State
```
documents
  mode: Library | Inbox
  filters: { types: Set<DocTypeId>, taxYear: Any | FY(year) | Earlier,
             links: Any | Linked | NotLinked | Kind(LinkKind) }
  openMenu: None | Type | TaxYear | Links | Sort
  query: String
  sort: Newest | Expiring
  selectedId: Option<DocumentId>
  inbox: { focusedId, pending: Vec<InboxItem> }
  picker: None | LinkPicker { forDoc, query }

Document  { id, path, mime, pages, bytes, title, docType, documentDate,
            taxYear?, keyDate?: { kind: Renews|Ends|Expires|Revalue, date, reminder: bool },
            links: Vec<Link>, extractedText?: String }
Link      = Transaction(id) | InventoryItem(id) | Account(id) | Payee(id) | Bill(id) | Event(id)
InboxItem { docId, source: Scanned|Emailed|Downloads|WatchedFolder|Dropped, receivedAt,
            extracted: { merchant?, date?, total?, docType? },
            suggestion?: { link: Link, signals: { amount: bool, date: bool, payee: bool } },
            readable: bool }
DocType   { id, name, order, tracksDate?: Renews|Ends|Expires, remindDays?, taxYear: bool }  // user-managed, Settings › Document types (16p)
```
- `signals` drives the meter; strength = count of true.
- `taxYear` derives from `documentDate`, using the Financial year start set in Settings › General.
- The Library counts and the Inbox badge are derived queries; don't store them.
- Documents sync as metadata and links. File bytes are local only unless the sync server stores attachments.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · chrome / panel `#eae9e9` · preview well `#d7d3d3`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6` / `#d7d3d3`.
  - Accent `#ec3013` (badges) · text-safe accent `#ae1800` (date flags, errors).
  - Rules: strong `rgba(32,30,29,.38)` (2px) · row `rgba(32,30,29,.15)` (1px) · border `rgba(32,30,29,.30)` · dashed drop `#9b9797`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 8 (glyph) / 9.5 / 10 / 10.5 / 11 / 11.5 / 12 / 12.5 / 13.5 / 15 / 28px.
  - Caps labels `.11em`. Tabular numerals on every date, count and amount.
- **Spacing:** 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 20px.
- **Radius:** 0 everywhere.
- **Shadows:** only the preview page, `0 2px 6px rgba(32,30,29,.2)`.
- **Focus:**
  - Inbox row: `outline:2px solid #201e1d; outline-offset:-2px`.
  - Controls: `outline:2px solid #ec3013; outline-offset:2px`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary`, `.input` from Modernist (`styles.css`).

## Assets
- **Icons:** Lucide at 14px (rail) and 10px (link chips), 1.5 stroke. Chip icons by link kind:
  - credit card → account
  - box → inventory item
  - arrows → transaction amount
  - user → payee
  - calendar → bill
  - arrow → Inbox suggestion
- **File glyph and preview thumbnail:** drawn with CSS boxes in the mock. Replace them with real page renders.
- No images are bundled.

## Files
- `Documents.dc.html` — design reference: 4c library with inline filters, 4d Type menu open, 4e tabs + Inbox banner alternative.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
