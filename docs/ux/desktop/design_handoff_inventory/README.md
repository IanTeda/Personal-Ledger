# Handoff: Inventory

## Overview
**Inventory** (`g o`, under NET WORTH) is a home-contents register built for insurance. It keeps one register per home or policy, and the first thing it answers is **"am I covered?"**: replacement value against sum insured, with any shortfall in red.

Every item carries its proof (receipt, photo, serial). Items above the policy's single-item limit are flagged until they are specified. Items link to the purchase transaction and to documents.

Frames are section 7 of `Ledger Desktop Shell.dc.html`:
- **8a Inventory register:** the selected item is over the limit.
- **8b Export for insurer:** a dialog over 8a.

## About the design files
`Inventory.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder. Links to other sections point to `../Ledger Desktop Shell.dc.html`.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Inventory depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample data is illustrative (AUD; values in whole dollars).

## Frame & shell
- **Frame:** 1280 × 800.
- **Header:** 48px. The mockups show only the wordmark, the `:` command affordance and "synced 14:22". **Align with the rest of the shell:** add the breadcrumb `net worth › inventory`, the sync icon and the three window controls.
- **Primary rail:** 206px. Inventory is active (NET WORTH: Cash `g c` · **Inventory `g o`** · Loans `g n` · Credit cards `g k` · Investments `g i`).
- **Status bar:** the mockups omit it. Add the standard 28px status bar (see Keyboard).
- **Body:** `flex`, with the list pane (`flex:1; min-width:0; padding:18px 24px; flex column; overflow:hidden`) and a **300px** detail pane (`border-left:2px solid rgba(32,30,29,.38); background:#eae9e9`).

---

## 8a · Register

### 1. Heading
`flex; align-items:flex-end; justify-content:space-between; margin-bottom:14px`.
- **Left: inventory switcher.**
  - `h2` "12 Elm St contents" (28px/800) followed by a chevron-down icon, gap 8, `cursor:pointer`, `margin-bottom:4px`.
  - Clicking it or pressing `o` opens a menu of inventories (one per home or policy) plus **+ New inventory…**.
  - Subline (11.5px #9b9797): "Insured with NRMA · policy HC-4471902 · renews 14 Jan 2027".
- **Right:** **Export for insurer…** (`.btn.btn-secondary`, 36px) and **+ Add item** (`.btn.btn-primary`, 36px), gap 8.

### 2. Cover strip
`flex; border:1px solid rgba(32,30,29,.30); margin-bottom:14px`. Four equal cells, `flex:1; padding:10px 14px; border-right:1px solid rgba(32,30,29,.30)`.
- **Cell content:**
  - Caps label: `800 10px/1 Archivo .11em #9b9797; margin-bottom:6px`.
  - Value: 19px/800, tabular.
  - Note: 11px #9b9797, `margin-top:2px`.

| Label | Value | Note | Logic |
| --- | --- | --- | --- |
| REPLACEMENT VALUE | 163,480 | 214 items · new-for-old | Sum of item replacement values. |
| SUM INSURED | 150,000 | **13,480 under-insured** (800 `#ae1800`) | From the policy. When covered, the note reads "13,480 headroom" in #605d5d. |
| MISSING PROOF | 41 | no receipt or photo | Items with neither R nor P. |
| WARRANTIES | 2 end soon | Sony camera · Dec 2026 | Warranties ending within 90 days; the note names the soonest. |

- **Cover bar** (inside the SUM INSURED cell, `margin-top:6px`): a 4px track in `#d7d3d3`.
  - Fill `#201e1d` = min(insured, replacement) ÷ replacement.
  - A `#ec3013` segment, right-aligned, = shortfall ÷ replacement (8.2% here).

### 3. Room tabs
`flex; gap:2px; align-items:stretch`.
- **Tab:** `padding:7px 12px; flex column; gap:2px`. Name 800 12.5px, then "count · value" (10.5px #9b9797 tabular, value in `k`).
- **Active tab:** `background:#201e1d; color:#f3f2f2`, with the meta in `#bab6b6`.
- **Tabs:** All 214 · 163k · Living 38 · 24k · Kitchen 46 · 22k · Bedrooms 52 · 32k · Office 19 · 15k · Garage 27 · 15k · Other 32 · 56k.
- **Below the tabs:** a 2px rule `rgba(32,30,29,.38)`, `margin-bottom:10px`.

### 4. Toolbar
`flex; align-items:center; gap:8px; margin-bottom:10px`.
- **Search:** `.input`, 30px, 220px, placeholder "/ find item, serial, brand".
- **Filter segmented control:** `border:1px solid rgba(32,30,29,.30); 12px`, segments `padding:6px 10px`. Options are **All** (active, `#201e1d` / `#f3f2f2` 800) · **Missing proof 41** · **Over limit 3** (800 `#ae1800`).
- **Proof legend** (right-aligned, 11px #9b9797, gap 8): R receipt · P photo · S serial.

### 5. Item table
`border:1px solid rgba(32,30,29,.30)`.
- **Header row:** `flex; gap:10px; padding:6px 12px; #eae9e9; 800 10px .11em #605d5d; border-bottom:1px solid rgba(32,30,29,.20)`.
- **Columns:**

  | Column | Width | Notes |
  | --- | --- | --- |
  | thumb | 26px | |
  | ITEM | flex 2.4 | |
  | ROOM | flex .9 | |
  | BOUGHT | flex 1 | #605d5d |
  | REPLACE | flex .9 | right, 800 |
  | PROOF | flex .8 | `padding-left:14px` |
  | WARRANTY | flex 1 | |
- **Row:** `flex; align-items:center; gap:10px; padding:4px 12px; border-bottom:1px solid rgba(32,30,29,.15); 12.5px`, tabular.
- **Thumb:** 26×26, `#d7d3d3`, holding a photo or a category icon. With no photo it becomes `border:1px dashed #9b9797`, transparent.
- **Item cell:** name 800, then a sub-line (10.5px #9b9797) with the serial ("SN C02FK3…Q6LR", middle-ellipsised), the model, the brand or the source.
- **Flags** sit inline after the name, `margin-left:6px; 800 9px/1 Archivo .1em; padding:3px 5px`:
  - **OVER LIMIT:** `#ec3013` / `#f3f2f2`. The replacement value exceeds the policy's single-item limit and the item is not specified.
  - **SPECIFIED:** `border:1px solid rgba(32,30,29,.35)`, ink text. The item is listed individually on the policy.
  - **NEW:** `#201e1d` / `#f3f2f2`. Created from a transaction and not yet reviewed (still missing proof).
- **Proof chips:** 17×17, `800 9.5px Archivo`, centred, gap 3.
  - Present: `#201e1d` fill with `#f3f2f2` text.
  - Missing: `border:1px dashed #9b9797`, `#9b9797` text.
- **Warranty:** "Mar 2027", "expired" (#9b9797), or "—".
- **Selected row:** `background:#201e1d; color:#f3f2f2`. The sub-line becomes #bab6b6 and the date #d7d3d3. Present chips invert to `#f3f2f2` fill with `#201e1d` text.
- **Footer note** (11px #9b9797, `margin-top:8px`): "Showing 8 of 214 · sorted by room · purchases in Transactions can be added as items from the transaction".

**Sample rows:**

| Item | Room | Bought | Replace | Proof | Warranty |
| --- | --- | --- | --- | --- | --- |
| MacBook Pro 14" | Office | 12 Mar 2024 | 3,999 | R P S | Mar 2027 |
| Samsung 75" QLED TV | Living | 18 Nov 2023 | 2,299 | R P S | expired |
| Engagement ring `SPECIFIED` | Bedroom 1 | 2 Jun 2019 | 9,200 | R P – | — |
| Fisher & Paykel fridge | Kitchen | 9 Aug 2022 | 3,399 | R P S | Aug 2027 |
| **Sony A7 IV + 24–70 lens** `OVER LIMIT` *(selected)* | Office | 4 Dec 2023 | 4,590 | R P S | Dec 2026 |
| Trek Domane SL5 `OVER LIMIT` | Garage | 14 Oct 2021 | 4,650 | R P S | — |
| Sofa, 3-seat | Living | 20 Feb 2020 | 3,200 | – P – | — |
| Sonos Era 100 `NEW` | Living | 26 Sep 2026 | 389 | R – – | Sep 2027 |

### 6. Detail pane (300px)
- **Photo well:** 150px, `#d7d3d3`, 2px bottom rule, with a centred icon and "3 photos · camera, lens, serial plate" (11px #605d5d). In the app, show the first photo with a filmstrip and arrows.
- **Body:** `padding:12px 16px; flex column; gap:10px`.
  - **Title:** "Sony A7 IV + 24–70 lens" (16px/800), then "Office · Electronics › Camera" (11.5px #605d5d) for room and category.
  - **Over-limit callout:** `border:2px solid #ec3013; background:#f3f2f2; padding:8px 10px; 11.5px/1.4`. "**Over the 2,000 single-item limit.**" in `#ae1800`, then "Unspecified, this pays at most 2,000 in a claim. Specify it on the policy to cover the full 4,590." Shown only for OVER LIMIT items.
  - **Fact rows:** `flex; space-between; padding:5px 0; border-bottom:1px solid rgba(32,30,29,.15); 12px`, label #605d5d, value tabular.
    - Bought: 4 Dec 2023 · Camera House
    - Paid: 4,380.00
    - Replacement: **4,590.00** · checked Sep 2026
    - Serial: 4410…2291
    - Warranty: to Dec 2026
    - Transaction: "−4,380.00 · ANZ Platinum", a link that opens the transaction in Transactions.
  - **DOCUMENTS** (caps label, gap 5): one row per linked file, `padding:6px 8px; border:1px solid rgba(32,30,29,.30); 12px`, with an icon, the file name (`flex:1`) and the size (10.5px #9b9797). Sample: "Receipt · Camera House.pdf" 212 KB · "Warranty card.jpg" 1.1 MB. These are the same records as Documents (section 4).
- **Footer:** `border-top:2px solid rgba(32,30,29,.38); padding:12px 16px; gap:8`. **Mark as specified** (primary, `flex:1`, 34px; shown only when OVER LIMIT; otherwise the primary is **Add proof…**) and **Edit** (secondary).

---

## 8b · Export for insurer
- **Overlay:** a dimmer covers the content area (`inset:48px 0 0 206px; background:rgba(32,30,29,.18)`).
- **Dialog:** 480px, placed at `left:420px; top:110px`, `#f3f2f2; border:2px solid #201e1d; box-shadow:6px 6px 0 rgba(32,30,29,.18); 12.5px`.
  - The mock differs from the shared dialog shell. **Align with it:** dimmer `rgba(32,30,29,.30)` over the whole window, centred, shadow `0 16px 48px rgba(32,30,29,.40)`.
- **Header:** `padding:14px 18px 10px; border-bottom:2px solid rgba(32,30,29,.38)`. Title "Export for insurer" (800 17px), subline "For a claim, a renewal or a copy kept off-site." (11.5px #605d5d).
- **Body:** `padding:14px 18px; flex column; gap:14px`. Field labels are 11.5px #605d5d, `margin-bottom:5px`.
  - **Purpose:** a full-width segmented control (`border:1px solid rgba(32,30,29,.30)`, segments `padding:7px 10px; 12px`, active `#201e1d` / `#f3f2f2` 800). Options are **Claim** · Renewal review · Off-site copy.
  - **Items / Event date:** two columns, gap 10.
    - **Items:** a select (32px, focused here with `border:2px solid #201e1d`) showing "**Living**" with "38 items ▾" (11px #9b9797). Options: All, each room, Over limit, Missing proof, a custom selection.
    - **Event date:** 32px, "28 Sep 2026". Shown only for **Claim**.
  - **Include:** checkboxes (14×14, `border:2px solid #201e1d`, checked = filled with a white check; `padding:4px 0; gap:9px`), each with an 11px #605d5d hint:
    - **Photos** ✓, "first photo per item, full set in the zip"
    - **Receipts, valuations & warranties** ✓, "attached as originals"
    - **Serial numbers** ✓
    - **Price paid** ☐, "insurers ask for replacement value"
  - **Summary strip:** `flex; border:1px solid rgba(32,30,29,.30)`. Three cells with `padding:8px 12px`, a caps label (9.5px) and a 16px/800 value:
    - ITEMS 38
    - REPLACEMENT 24,310
    - NO PROOF **6** (in `#ae1800`)
  - **Note** (11.5px/1.45 #605d5d): "6 items have no receipt or photo. They're listed with a note so the insurer can ask for other evidence."
- **Footer:** `padding:12px 18px; border-top:2px solid rgba(32,30,29,.38); gap:8`. **Export PDF + zip · 38 items** (primary, `flex:1`, 36px) and **CSV only** (secondary).

**Export output:**
- **PDF schedule:**
  - Cover: policy, purpose, event date, totals.
  - One row per item: photo, name, room, bought, replacement, serial, proof status.
  - Items without proof are listed with "No receipt or photo on file", never dropped.
- **Zip:** the PDF plus the original documents and photos, in folders by room. File names are `<room>/<item>/<original name>`.
- **CSV:** item, room, category, bought, replacement, paid (if ticked), serial, specified, proof flags.

---

## Interactions & behaviour
- **Coverage recomputes** whenever an item's replacement value, the sum insured or the specified state changes.
- **Over limit** = replacement > the policy's single-item limit AND not specified. **Mark as specified** records it as listed on the policy, clears the flag, and keeps a note on the policy record.
- **Proof flags derive from links:**
  - **R:** a linked receipt or valuation document.
  - **P:** at least one photo.
  - **S:** a serial or model field.
  - Clicking a dashed chip opens the matching add flow (attach a document, add a photo, enter a serial).
- **Add from transaction:** in Transactions, a purchase can be turned into an item (**Add to inventory**). It pre-fills the name, date, paid amount and the transaction link, and the item lands with **NEW** until it is reviewed. Matching documents from the Documents Inbox can attach automatically.
- **Replacement value** defaults to the price paid. "checked Sep 2026" records when it was last reviewed. A renewal review export prompts for values unchecked for more than 12 months.
- **Notifications:** the "Item missing proof" and "Warranty expiring" rules (section 5) read from here.
- **Net worth:** Inventory counts at depreciated value, not replacement value (see Dashboard 2c).
- **Empty inventory:** "No items yet" with **+ Add item** and "Add from a transaction" guidance.
- **Search** matches name, brand, model, serial and room.

## Keyboard
| Key | Action |
| --- | --- |
| `g o` | Open Inventory (last inventory used). |
| `o` | Switch inventory. |
| `j` / `k` | Next / previous item. |
| `[` / `]` | Previous / next room tab. |
| `/` | Focus search. |
| `n` | Add item. |
| `e` | Edit item. |
| `p` | Add proof (opens the attach menu: receipt / photo / serial). |
| `s` | Mark as specified. |
| `x` | Export for insurer… |
| `enter` | Open item editor. |
| `esc` | Close a dialog or menu. |

**Status bar (to add):**
- **Left:** `NORMAL` · `j/k item · [ ] room · n add · p proof · s specify · x export · o switch`
- **Right:** "NRMA · single-item limit 2,000"

## State
```
inventory
  inventoryId: InventoryId
  room: All | RoomId
  filter: All | MissingProof | OverLimit
  query: String
  selectedItemId: Option<ItemId>
  dialog: None | Export(ExportDraft) | ItemEditor(ItemDraft) | SwitchInventory

Inventory { id, name, address?, policy?: { insurer, number, renewsOn, sumInsured, singleItemLimit } }
Item      { id, inventoryId, name, roomId, category, brand?, model?, serial?,
            boughtOn?, paid?, replacement, replacementCheckedOn?, warrantyEnds?,
            specified: bool, reviewed: bool, photos: Vec<FileRef>,
            documents: Vec<DocumentId>, transactionId?: TransactionId }
Room      { id, inventoryId, name, order }
ExportDraft { purpose: Claim|Renewal|OffSite, scope: All|Room(id)|OverLimit|MissingProof|Selection(ids),
              eventDate?, include: { photos, documents, serials, pricePaid } }
```
- The cover strip, room totals, proof flags and OVER LIMIT are all derived.
- `NEW` is shown when `reviewed == false`.
- Photos and documents are files kept beside the ledger, as in Documents.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · chrome / detail `#eae9e9` · well / thumb / track `#d7d3d3`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6` / `#d7d3d3`.
  - Accent `#ec3013` (OVER LIMIT, shortfall bar, callout border) · text-safe accent `#ae1800`.
  - Rules: strong `rgba(32,30,29,.38)` · border `rgba(32,30,29,.30)` · header rule `rgba(32,30,29,.20)` · row `rgba(32,30,29,.15)` · missing `1px dashed #9b9797`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 9 / 9.5 / 10 / 10.5 / 11 / 11.5 / 12 / 12.5 / 16 / 17 / 19 / 28px.
  - Caps labels `.11em` (flags `.1em`). Tabular numerals on every figure.
- **Spacing:** 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 24px.
- **Radius:** 0 everywhere. **Shadows:** the dialog only (see 8b).
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Focused inputs use `border:2px solid #201e1d`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary`, `.input` from Modernist (`styles.css`).

## Assets
- **Icons:** Lucide-style, 1.5 stroke. The rail uses a 14px box/package. The mock also uses a chevron-down (switcher), category glyphs in the thumbs, camera/photo in the well, and file icons in DOCUMENTS.
- **Item photos** are user files. The mock uses grey placeholders.
- No bundled images.

## Files
- `Inventory.dc.html` — design reference: 8a Register, 8b Export for insurer.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
