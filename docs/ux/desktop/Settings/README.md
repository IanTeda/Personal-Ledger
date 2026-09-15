# Handoff: Settings Surface & Modals

## Overview
This package contains the detailed design for **Personal Ledger's settings interface** — a comprehensive, scrollable settings surface with ten panes (General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing (Logs), About) and four modal dialogs for managing units and institutions. The design prioritizes clarity, safety in destructive actions, and consistent form patterns.

## About the Design Files
The files in this bundle are **high-fidelity HTML prototypes** showing the intended look, layout, interactions, and modal patterns. They are **design references, not production code**. Your task is to **recreate these designs in your target codebase** using your framework's idioms (React, Vue, GPUI, SwiftUI, or native), informed by the HTML mockups' layouts, typography, colors, spacing, and behavior.

## Fidelity
**High-fidelity (hifi)**: Pixel-perfect mockups with final colors, typography, exact spacing, form validation rules, and modal interactions. Recreate the UI faithfully, translating the HTML structure into your framework's components and patterns.

## Screens / Views

### 2a: Settings Landing — Clean Resting State

**Purpose**: The primary settings view showing General pane with no modal open. This is the entry point to the settings surface.

**Layout**:
- Header: 48px chrome with breadcrumb "settings › general"
- Left rail: 206px primary app nav (expanded), Settings active (dark treatment)
- Center rail: 214px settings index with:
  - Filter input (height 30px, padding 10px 12px 9px, top margin 22px to align with Settings heading)
  - SETTINGS label (10px 'Archivo', 800 weight, letter-spacing .11em)
  - Divider (2px solid rgba(32,30,29,.38), margin-top 8px, margin-bottom 4px)
  - Nine settings items (General active dark, others plain), padding 8px 14px each
- Right pane: scrollable settings body (flex:1, display:block, padding 22px 28px, overflow:auto)
- Footer: 28px bar with mode label, keybindings, status

**Scrollable Body Content** (in order):
- Settings heading: 28px, 800 weight, margin-bottom 24px
- Breadcrumb: "preferences · synced", font-size 11.5px, color #9b9797
- Divider: 2px solid rgba(32,30,29,.38), margin-bottom 24px, flex:none
- General section:
  - Heading (h4): "General", 20px, margin-bottom 14px
  - Divider: 2px, margin 14px 0 18px, flex:none
  - Two-column layout: fields on left (320px, flex:none), info box on right (flex:1)
  - Fields: Ledger name, Owner, Financial year starts (select), Base unit (select)
  - Field spacing: 16px gap vertically, 6px between label and control
  - Info box: "THIS LEDGER" label (10px 'Archivo', 800 weight), table showing accounts (7), transactions (680), units (3), institutions (7)
  - Info box padding: 10px 12px, background #eae9e9, border 1px solid rgba(32,30,29,.30)
  - Explanatory text: 12px, color #605d5d, margin-top 14px
  - Section margin-bottom: 48px
- Ledger & units section:
  - Heading (h4): "Ledger & units"
  - Divider: 2px, margin 14px 0 18px, flex:none
  - Two radio groups: Default unit (aud/btc/vas), Budget period (weekly/monthly/quarterly)
  - Radio group width: 300px, gap 18px
  - Selected radio: aud (Default unit), monthly (Budget period)
  - Section margin-bottom: 48px
- Units section:
  - Heading (h4): "Units", subtitle "3 units · synced"
  - Divider: 2px, margin 14px 0 18px, flex:none
  - Table: CODE (100px), NAME (flex:1), TYPE (80px), ACTIONS (120px right-aligned)
  - Header row: padding 12px 16px, background #eae9e9, font 800 10px 'Archivo', letter-spacing .11em, border-bottom 1px solid rgba(32,30,29,.30)
  - Data rows: 3 rows (aud/Australian Dollar/currency, btc/Bitcoin/crypto, vas/Vanguard Aus Shares/etf)
  - Row padding: 12px 16px, border-bottom 1px solid #d7d3d3 (except last row)
  - Action buttons: edit/delete, padding 4px 10px, font-size 11px, border 1px solid rgba(32,30,29,.30), background transparent
  - Table border: 1px solid rgba(32,30,29,.30), margin-bottom 16px
  - Add unit button: padding 10px 16px, border 1px solid rgba(32,30,29,.30), background #eae9e9, font-weight 800, width fit-content
  - Section margin-bottom: 48px
- Institutions section:
  - Heading (h4): "Institutions", subtitle "7 institutions · synced"
  - Divider: 2px, margin 14px 0 18px, flex:none
  - Table: INSTITUTION (flex:1), ACCOUNT TYPE (140px), ACTIONS (120px right-aligned)
  - Header row: same styling as Units table
  - Data rows: 7 rows (ANZ/savings · offset, Amex/credit card, Vanguard/investment, Westpac/savings, Crypto Exchange/crypto, Superannuation/retirement, plus one more)
  - Row styling: same as Units
  - Add institution button: same styling as Add unit
  - Section margin-bottom: 48px
- Display section:
  - Heading (h4): "Display", subtitle "client-scoped · never synced"
  - Divider: 2px, margin 14px 0 18px, flex:none
  - Two-column layout: controls on left (300px), preview on right (flex:1)
  - Controls: 4 form groups, gap 18px
    - Date format: 3 radio options (12 sep 2026, 12/09/2026, ISO), selected: 12 sep 2026
    - Decimal & thousands separator: 3 radio options (1,234.56, 1.234,56, 1 234.56), selected: 1,234.56
    - Row density: 3 radio options (compact, regular, roomy), selected: regular
    - Status glyphs: 2 radio options (unicode / ascii fallback), selected: unicode
  - Preview: label "PREVIEW" (10px 'Archivo', 800 weight), table with 4 rows showing sample transactions
  - Preview table: border 1px solid rgba(32,30,29,.30), background #eae9e9
  - Preview table header: 7px 12px padding, font 800 10px, letter-spacing .11em, border-bottom 1px solid #d7d3d3
  - Preview rows: 8px 12px padding, font-variant-numeric tabular-nums, border-bottom 1px solid #d7d3d3
  - Explanatory text: 12px, color #605d5d, margin-top 14px, max-width 420px
  - Section margin-bottom: 48px
- Sync server section:
  - Heading: "Sync server"
  - Status info: Server URL (sync.ledger.localhost), Status (green dot + "connected"), Last sync (14 sep 2026 · 09:14)
  - Sync now button: padding 8px 16px, margin-top 8px
  - Section margin-bottom: 48px
- Data & backup section:
  - Heading: "Data & backup"
  - Info: Store location (~/.ledger/personal), Last backup (12 sep 2026 · 23:10)
  - Buttons: Backup now, Export ledger (CSV), padding 8px 16px, margin-top 8px, stacked
  - Section margin-bottom: 48px
- Tracing (Logs) section:
  - Heading: "Tracing (Logs)", subtitle "diagnostic · last 1000 entries"
  - Log level filter: 4 radio options (error, warn, info, debug), selected: error
  - Log viewer: 120px height, border 1px solid rgba(32,30,29,.30), background #eae9e9, padding 10px, overflow-y auto, font-family monospace, font-size 10px, line-height 1.5, color #605d5d
  - Sample logs: [timestamp] context: message (4 sample lines)
  - Clear logs button: padding 8px 16px, margin-top 8px
  - Section margin-bottom: 48px
- About section:
  - Heading: "About"
  - Info: personal-ledger (version 2.1.4), released (08 sep 2026)
  - Tech note: "Built with Rust + SQLite + React"

**Spacing throughout**:
- Sections separated by 48px margin-bottom
- Headers to dividers: 14px top, divider 2px, content starts 18px below
- Post-main-heading divider: margin-bottom 24px
- Form fields: 16px gap vertically, 40px gap horizontally (multi-column)
- All dividers: flex:none to prevent flex shrink

**Body scrolling**: Total height ~2957px in 724px viewport, all sections reachable

### 2b: Full Settings with Add Unit Modal

**Purpose**: Same as 2a but with the "Add unit" modal open over the entire view.

**Modal**:
- Overlay: 100% viewport, background rgba(32,30,29,.30), z-index 10, position absolute inset 0
- Modal box: width 420px, centered (left:50%, top:50%, transform:translateX(-50%))
- Modal styling:
  - Background: #f3f2f2
  - Border: 2px solid #201e1d
  - Box-shadow: 0 16px 48px rgba(32,30,29,.40)
  - Display: flex flex-direction column
- Header: padding 18px 20px, border-bottom 2px solid rgba(32,30,29,.30), font 800 16px 'Archivo', letter-spacing .01em
- Form body: padding 20px, display flex flex-direction column, gap 16px
- Form fields: Code, Name, Type (select)
  - Code: text input, value ""
  - Name: text input, placeholder "e.g. US Dollar"
  - Type: select dropdown, options (currency, crypto, etf, stock), selected: currency
  - Label styling: display block, font-weight 800, font-size 12px, margin-bottom 6px
  - Input styling: width 100%, padding 8px 10px, border 1px solid rgba(32,30,29,.30), font-size 13px, box-sizing border-box
  - Select: same input styling, background #f3f2f2
- Button row: padding 16px 20px, border-top 1px solid #d7d3d3, display flex, gap 10px, justify-content flex-end
  - Cancel button: padding 8px 16px, border 1px solid rgba(32,30,29,.30), background transparent, font-weight 800, cursor pointer
  - Add button: padding 8px 16px, border none, background #201e1d, color #f3f2f2, font-weight 800, cursor pointer

### 2c: Edit Unit Modal

**Purpose**: Modal for editing an existing unit with usage information.

**Modal**:
- Same structure and styling as 2b
- Header: "Edit unit — aud"
- Form fields:
  - Code: value "aud" (pre-filled)
  - Name: value "Australian Dollar"
  - Type: value "currency" (select)
- Usage notice (between form and buttons):
  - Padding: 10px
  - Background: #eae9e9
  - Border-left: 2px solid #ec3013
  - Font-size: 11.5px
  - Color: #605d5d
  - Text: "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references."
- Buttons: Cancel, Save (dark background #201e1d)

### 2d: Delete Unit Confirmation Modal

**Purpose**: Destructive action confirmation requiring typed verification.

**Modal**:
- Same structure as 2b/2c but styled as danger/destructive
- Border: 2px solid #ec3013 (red, not dark)
- Header: border-bottom 2px solid #ec3013, color #ae1800, text "Delete unit — btc"
- Warning paragraph: font-size 13px, text-wrap pretty
  - Text: "This unit is referenced by 1 account and 9 transactions. Deleting it cannot be undone."
- Reference box: padding 12px, background #eae9e9, border-left 2px solid #ec3013, font-size 11.5px, color #605d5d
  - Content: "Bitcoin · 0.4120 u held in Bitcoin account"
- Confirmation input:
  - Label: "Type btc to confirm", display block, font-weight 800, font-size 12px, margin-bottom 6px
  - Input: width 100%, padding 8px 10px, border 1px solid rgba(32,30,29,.30), font-size 13px, placeholder "btc"
- Buttons: Cancel, Delete unit (background #ec3013, color #f3f2f2, text "Delete unit")

### 2e: Add Institution Modal

**Purpose**: Modal for adding a new institution with multi-select account types.

**Modal**:
- Same structure and styling as 2b
- Header: "Add institution"
- Form fields:
  - Institution name: text input, placeholder "e.g. Commonwealth Bank"
  - Account types: multi-select chips (5 checkboxes for savings, credit card, offset, loan, investment)
    - Chip styling: display flex, align-items center, gap 6px, padding 6px 10px, border 1px solid rgba(32,30,29,.30), font-size 12px, cursor pointer
    - Wraps in flex row with flex-wrap wrap, gap 8px, margin-top 4px
    - Checked state: depends on design system, but visually distinct
  - Default unit: dropdown (aud, btc, vas options)
- Label styling: display block, font-weight 800, font-size 12px, margin-bottom 6px
- Input/select styling: same as 2b
- Buttons: Cancel, Add institution

## Interactions & Behavior

### Navigation Within Settings
- **Settings rail click**: Click any item (General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing, About) to scroll the body to that section's heading
- **Filter input**: Type to filter visible settings items in the rail (real-time fuzzy match, e.g. typing "unit" highlights Units)
- **Tab key**: Move focus between fields in the scrollable body
- **Keybindings** (inherited from main app):
  - j/k: scroll through sections (up/down)
  - q: back to dashboard
  - b: collapse sidebar (not shown in mocks)

### Form Interactions
- **Text inputs**: Type to edit, live-save after each keystroke (footer shows "saved automatically")
- **Select dropdowns**: Click to open menu, select option, auto-save
- **Radio groups**: Click option to select (mutually exclusive per group), auto-save
- **Checkboxes** (Add Institution): Click to toggle, support multi-select, no auto-save until modal Submit
- **Table buttons** (edit/delete): Click to open corresponding modal
- **Modal Cancel**: Discard changes, close modal
- **Modal Submit** (Add/Save): Validate form, persist changes, close modal, refresh affected table/content
- **Delete modal confirmation**: Delete button disabled until user types the required code, then enable and allow submit

### Hover States
- Nav items (rail): subtle bg darkening on hover
- Table rows: light bg change
- Buttons: lighter border or shadow on hover

### Focus States
- Inputs/selects: 2px outline (browser default or custom)
- Active nav item: already dark, so focus is less critical

## State Management

### Settings Page State
- `activeSetting`: current pane (General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing, About)
- `scrollPosition`: to restore on nav back
- `filterQuery`: settings rail filter string
- `filteredItems`: computed from all items filtered by filterQuery

### Form State (General Pane)
- `ledgerName`: string
- `owner`: string
- `financialYearStart`: enum (july, january, april)
- `baseUnit`: enum (aud, btc, vas)

### Form State (Ledger & Units Pane)
- `defaultUnit`: enum (aud, btc, vas)
- `budgetPeriod`: enum (weekly, monthly, quarterly)

### Form State (Display Pane)
- `dateFormat`: enum (12 sep 2026, 12/09/2026, ISO)
- `decimalSeparator`: enum (1,234.56, 1.234,56, 1 234.56)
- `rowDensity`: enum (compact, regular, roomy)
- `statusGlyphs`: enum (unicode, ascii)

### Form State (Sync Server Pane)
- `serverUrl`: string (read-only display)
- `syncStatus`: enum (connected, disconnected, syncing)
- `lastSync`: datetime

### Form State (Tracing Pane)
- `logLevel`: enum (error, warn, info, debug)

### Modal State
- `modalOpen`: null | "add-unit" | "edit-unit" | "delete-unit" | "add-institution"
- `editingUnit`: Unit object or null (for edit/delete modals)
- `formData`: form fields specific to open modal

### Units/Institutions Lists
- `units`: array of {code, name, type, usageCount, transactionCount}
- `institutions`: array of {name, accountTypes: [], defaultUnit}

## Design Tokens

### Colors
- Primary text: #201e1d
- Secondary text: #9b9797
- Tertiary text: #605d5d
- Light bg: #f3f2f2
- Dark bg: #eae9e9
- Sidebar bg: #eae9e9
- Active item: #201e1d bg with #f3f2f2 text
- Accent (error/warning): #ec3013
- Dividers/borders: rgba(32,30,29,.30) or #d7d3d3 or #201e1d
- Overlay: rgba(32,30,29,.30)

### Spacing Scale
4px (implicit), 6px, 7px, 8px, 9px, 10px, 12px, 14px, 16px, 18px, 20px, 22px, 24px, 28px, 40px, 48px

### Typography
- Font family: 'Archivo' (headers, labels), system sans-serif (body)
- Font weights: 400 (regular), 800 (bold)
- Sizes:
  - 10px: labels, table headers
  - 11px: small text, keybindings
  - 11.5px: subtitles, hints
  - 12px: body, labels, small inputs
  - 13px: content, input text
  - 13.5px: breadcrumb
  - 16px: modal headers
  - 20px: section headings (h4)
  - 28px: page heading (h2)
- Line-height: 1 (labels), 1.3 (body), 1.5 (prose), 1.6 (hints)
- Letter-spacing: .11em (labels), .01em (modal header), -.01em (tight), 0 (normal)

### Borders & Shadows
- Border widths: 1px (inputs, table borders), 2px (dividers, modal borders)
- Modal shadow: 0 16px 48px rgba(32,30,29,.40)
- No border-radius (all sharp corners)

## Assets
- SVG icons (14×14, inline, stroke): none in this settings surface (sidebar nav icons are part of main shell)
- No external images or raster assets

## Files
- Ledger Desktop Shell.dc.html — Contains all five settings variation cards (2a–2e)
- support.js, styles.css — shared Claude Design canvas runtime and design-token source, copied from the `../Shell & Navigation/` bundle so this handoff renders standalone

## Companion bundles
- `docs/ux/desktop/Shell & Navigation/` — the shell, primary/context rails, command palette and design tokens this settings surface renders inside of; read it first.
- `docs/ux/desktop/README.md` — the `gpui`-facing living spec this shell is actually built against; it names this bundle as `Noun::Settings`'s handoff but doesn't yet carry a translated spec for it.

## Notes for Implementation

1. **HTML is a reference** — Translate layouts and styles into your target framework. Use your established component library and patterns.
2. **Scrollable body** — Settings body container uses `display: block; overflow: auto` (NOT column flex) so divider lines render at full 2px height.
3. **Divider safety** — All divider rules carry `flex: none` (or equivalent) to prevent flex shrink in containers.
4. **Radio namespacing** — Radio inputs use namespaced `name` attributes per card (e.g., `df-1c`, `df-2a`) to avoid browser collisions when multiple cards render.
5. **Modal centering** — Modals use `position: absolute; left: 50%; transform: translateX(-50%)` to center horizontally. Adjust top position as needed for your layout.
6. **Form validation** — Delete confirmation requires exact typed match before enabling Delete button. Consider debouncing the confirm input.
7. **Auto-save** — Inputs and selects should persist immediately on change (footer hints "saved automatically"). Consider debouncing if backend latency is an issue.
8. **Table density** — Rows are compact (12px padding v). Ensure table content is readable at this density.
9. **Spacing consistency** — Sections are separated by 48px `margin-bottom`. Maintain this rhythm across all settings panes.
10. **Responsive behavior** — Not specified in these mocks (desktop-only), but consider how multi-column layouts (General, Display) reflow on narrower screens.
