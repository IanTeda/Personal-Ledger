# Handoff: Desktop Shell & Navigation

## Overview
This package contains the design for **Personal Ledger's main application shell** — the primary navigation frame, sidebar layout, settings surface, and related screens. The design explores a hybrid vim-inspired keybinding model (: command palette, j/k navigation) paired with a graphical sidebar rail, a modal-driven settings interface, and multi-pane accounts/transactions workflows.

## About the Design Files
The files in this bundle are **high-fidelity HTML prototypes** showing the intended look, layout, and interaction model. They are **design references created in this conversation, not production code**. Your task is to **recreate these designs in your target codebase** (Rust/GPUI, React, Vue, or native) using your established patterns and component libraries, informed by the HTML mockups' layouts, typography, colors, and behavior.

## Fidelity
**High-fidelity (hifi)**: These are pixel-perfect mockups with final colors, typography, exact spacing, interactions, and modal patterns. Recreate the UI faithfully in your codebase, translating the HTML structure into your framework's idioms.

## Screens / Views

### Turn 1: Main Shell Variations (1a, 1b, 1c, 1d)

#### 1a: Grouped Navigation Rail (Accounts Highlighted)
**Purpose**: The primary application shell showing the main nav rail with grouped sections (LEDGER, PLAN, RECORDS) and a highlighted Accounts view.

**Layout**:
- Header: 48px, dark bar with app icon, title "Personal Ledger", breadcrumb, and 3 window controls
- Sidebar: 178px wide, flex column with sections:
  - LEDGER group: Dashboard, Transactions, Accounts (active/highlighted), Categories, Payees, Tags
  - PLAN group: Bills, Budgets, Reports
  - RECORDS group: Categories, Payees, Tags
- Each nav item: 7px vertical padding, 14px horizontal, flex row with icon (14×14), label, keybinding hint
- Active item: background #201e1d, color #f3f2f2, font-weight 800
- Main content area: flex:1, shows Accounts view
- Footer: 28px bar with mode label, keybindings hint, status

**Colors**:
- Primary bg: #eae9e9 (sidebar), #f3f2f2 (content)
- Primary text: #201e1d
- Secondary text: #9b9797
- Active: #201e1d bg with #f3f2f2 text
- Accent: #ec3013 (red)
- Borders: rgba(32,30,29,.30) or #d7d3d3

**Spacing**:
- Nav items: 7px v-padding, 14px h-padding, 9px gap between icon and label
- Section headers: 14px top, 8px bottom
- Content: 18–22px padding

**Interactions**:
- Nav click: highlight item, swap content pane
- Keybindings (vim): j/k up/down, b toggle sidebar, g+letter for direct jump, q return to dashboard

#### 1b: Sidebar with Secondary Rail
**Purpose**: Two-rail navigation — primary (Ledger, Plan, Records) plus secondary filterable rail showing account-specific options.

**Layout**:
- Primary sidebar: 178px (same as 1a)
- Secondary rail: 250px, shows filtered accounts list with search bar
- Both sidebar and secondary rail use #eae9e9 background

**Components**:
- Secondary rail: 250px wide, border-right 2px solid rgba(32,30,29,.38)
- Search box: height 30px, border 1px solid rgba(32,30,29,.30), padding 8px 12px
- Account list: rows with name left, balance right
- Selected account: background #201e1d, color #f3f2f2

#### 1c: Settings View with Settings Rail
**Purpose**: Settings screen with primary nav (Settings active) and secondary settings-index rail.

**Layout**:
- Primary sidebar: 178px, Settings active
- Settings index rail: 214px, contains filter input + settings items (General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing (Logs), About)
- Main content: Settings body scrolling through all panes, starting with Display

**Components**:
- Filter input: height 30px, font-size 12.5px, padding 8px 10px
- Settings items: padding 8px 14px
- Active settings item: background #201e1d, color #f3f2f2, font-weight 800
- Divider lines: height 2px, background rgba(32,30,29,.38), flex:none to prevent shrink

**Typography**:
- Settings heading: 28px, 800 weight
- Section heading (h4): 20px, 800 weight
- Labels: 12px, 800 weight

**Spacing**:
- Content: padding 22px 28px
- Sections: margin-bottom 48px between major blocks
- Dividers: margin 14px 0 18px (or 24px after main heading)

#### 1d: Command Palette (: bud)
**Purpose**: Vim-style command palette triggered by `:`, showing fuzzy-filtered commands.

**Layout**:
- Overlay: rgba(32,30,29,.30) dimmed background
- Modal: centered, ~820px wide, containing:
  - Input row: > prompt, input field, result count
  - Divider: 2px solid #201e1d
  - Command rows: highlighted row dark, others normal
  - Argument help row: current value, limit, variance
  - Footer hint: navigation and action keys

**Components**:
- Modal: background #f3f2f2, border 2px solid #201e1d, box-shadow 0 12px 32px rgba(45,43,43,.30)
- Selected row: background #201e1d, color #f3f2f2
- Input cursor: 8px wide, 17px tall, background #ec3013
- Description: color #d7d3d3 (selected) or #605d5d (normal)
- Keybinding hint: color #bab6b6 (selected) or #9b9797 (normal)

**Spacing**:
- Modal padding: 12–16px for rows
- Dimmer: full screen, z-index 10
- Modal: positioned center (transform translateX(-50%))

### Turn 2: Settings Surface (2a–2e)

#### 2a: Settings Landing (General Pane, No Modal)
**Purpose**: Clean resting state — expanded primary nav, Settings active, General pane visible, no modal.

**Layout**:
- Header: 48px, breadcrumb "settings › general"
- Primary nav: 206px, expanded with all app sections, Settings active
- Settings index: 214px, with filter + 9 items (General active)
- Main body: scrollable, padding 22px 28px

**Sections** (in scroll order):
- Settings heading (28px), margin-bottom 24px
- General: Ledger name, Owner, Financial year, Base unit inputs, THIS LEDGER info box
- 48px gap
- Ledger & units: Default unit, Budget period radios
- 48px gap
- Units: Table (CODE, NAME, TYPE) with 3 rows, edit/delete buttons, + Add unit
- 48px gap
- Institutions: Table (INSTITUTION, ACCOUNT TYPE) with 7 rows, edit/delete buttons, + Add institution
- 48px gap
- Display: Date format, Decimal separator, Row density, Status glyphs, PREVIEW table
- 48px gap
- Sync server, Data & backup, Tracing (Logs), About sections follow

**Components**:
- Input fields: padding 8px 10px, border 1px solid rgba(32,30,29,.30), font-size 13px
- Buttons: padding 8px 16px, border 1px solid rgba(32,30,29,.30), background #eae9e9, font-weight 800
- Table header: padding 12px 16px, background #eae9e9, font 800 10px 'Archivo', letter-spacing .11em
- Table rows: padding 12px 16px, border-bottom 1px solid #d7d3d3
- Action buttons: padding 4px 10px, font-size 11px

**Typography**:
- Settings heading: 800 28px 'Archivo'
- Section headings: 800 20px
- Labels: 800 12px
- Content: 13px

**Spacing**:
- Content: padding 22px 28px
- Section wrappers: margin-bottom 48px
- Dividers: height 2px, flex:none, margin 14px 0 18px (or 24px post-header)
- Form gap: 16px vertically, 40px horizontally for multi-column

**Scrolling**: Body is 2957px tall in 724px viewport, all sections reachable by scroll

#### 2b: Full Settings with Add Unit Modal
**Purpose**: Same as 2a with "Add unit" modal open.

**Modal**:
- Overlay: rgba(32,30,29,.30), z-index 10
- Modal box: background #f3f2f2, border 2px solid #201e1d, box-shadow 0 16px 48px rgba(32,30,29,.40), width 420px
- Header: padding 18px 20px, border-bottom 2px solid rgba(32,30,29,.30), font 800 16px 'Archivo'
- Form fields: Code, Name, Type (select)
- Button row: padding 16px 20px, gap 10px, Cancel (transparent) and Add (dark)

**Spacing**:
- Form padding: 20px
- Fields gap: 16px vertically

#### 2c: Edit Unit Modal
**Purpose**: "Edit unit — aud" form with usage notice.

**Modal**:
- Same structure as 2b
- Fields pre-filled: Code=aud, Name=Australian Dollar, Type=currency
- Usage notice: padding 10px, background #eae9e9, border-left 2px solid #ec3013, font-size 11.5px
  Text: "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references."
- Buttons: Cancel, Save

#### 2d: Delete Unit Confirmation Modal
**Purpose**: Destructive action confirmation with typed verification.

**Modal**:
- Border: 2px solid #ec3013 (red)
- Header: border-bottom 2px solid #ec3013, color #ae1800, text "Delete unit — btc"
- Warning text: lists affected accounts/transactions
- Reference box: padding 12px, background #eae9e9, border-left 2px solid #ec3013
- Confirmation input: label "Type btc to confirm", text input
- Buttons: Cancel, Delete unit (red background #ec3013, color #f3f2f2)

#### 2e: Add Institution Modal
**Purpose**: "Add institution" form with multi-select account types.

**Modal**:
- Header: "Add institution"
- Fields:
  - Institution name (text input)
  - Account types: 5 checkboxes (savings, credit card, offset, loan, investment) — multi-select chips
  - Default unit (dropdown: aud, btc, vas)
- Checkbox chip style: display flex, align-items center, gap 6px, padding 6px 10px, border 1px solid rgba(32,30,29,.30), font-size 12px
- Buttons: Cancel, Add institution

## Interactions & Behavior

### Navigation
- Sidebar click: switch sections, highlight active item
- Keybindings: j/k navigate, g+letter jump, b toggle sidebar, q return to Dashboard, : command palette, / search

### Settings
- Rail navigation: click settings item to jump to that section
- Form inputs: live-save (footer: "saved automatically")
- Modals:
  - Add unit: fill form, click Add, refresh Units table
  - Edit unit: modify fields, click Save
  - Delete unit: type unit code to unlock Delete, confirm
  - Add institution: select account types, choose default unit, click Add

### Hover States
- Nav items: subtle bg darkening
- Buttons: lighter border or shadow
- Table rows: bg change

### Focus States
- Inputs/selects: visible focus ring (2px outline)

## State Management

### Navigation
- activeSection: current nav section
- sidebarCollapsed: boolean (future)

### Settings
- activeSetting: current pane (General, Ledger & units, Units, Institutions, Display, Sync server, Data & backup, Tracing, About)
- Modal state: which modal is open (null, "add-unit", "edit-unit", "delete-unit", "add-institution")

### Command Palette
- isOpen: boolean
- query: search string
- results: filtered command list
- selectedIndex: highlighted result

## Design Tokens

### Colors
- Primary text: #201e1d
- Secondary text: #9b9797
- Tertiary text: #605d5d
- Light bg: #f3f2f2
- Dark bg: #eae9e9
- Active: #201e1d
- Active light: #f3f2f2
- Accent (error): #ec3013
- Muted: rgba(32,30,29,.30) or rgba(32,30,29,.38)

### Spacing Scale
6px, 7px, 8px, 9px, 10px, 12px, 14px, 16px, 18px, 20px, 22px, 24px, 28px, 40px, 48px

### Typography
- Font family: 'Archivo' (headers), system sans-serif (body)
- Weights: 400 (regular), 800 (bold)
- Sizes: 10px, 11px, 12px, 13px, 13.5px, 15px, 20px, 28px
- Letter-spacing: .11em (labels), .01em (titles), -.01em (tight)
- Line-height: 1, 1.3, 1.5, 1.6

### Borders & Shadows
- Dividers: 2px
- Input borders: 1px
- Modal borders: 2px
- Palette shadow: 0 12px 32px rgba(45,43,43,.30)
- Modal shadow: 0 16px 48px rgba(32,30,29,.40)

## Assets
- SVG icons (14×14, inline): Dashboard, Transactions, Accounts, Categories, Payees, Tags, Bills, Budgets, Reports, Settings
- No external images or raster assets

## Files
- Ledger Desktop Shell.dc.html — Main shell with all variations (1a–1d, 2a–2e)
- docs/ux/desktop/Ledger Desktop Shell.dc.html — Reference copy
- design_handoff_shell_navigation/Ledger Desktop Shell.dc.html — Handoff reference

## Companion bundles
- `docs/ux/desktop/Settings/` — the `Noun::Settings` view's own ten-pane surface and modals, re-hosted inside this shell; shares this bundle's `support.js` runtime and design tokens.
- `docs/ux/desktop/README.md` — the `gpui`-facing living spec this shell is actually built against.

## Notes for Implementation

1. **HTML is a reference** — Translate layouts and styles into your target framework using its idioms (React, Vue, GPUI, SwiftUI).
2. **Vim keybindings** — Implement j/k, g+letter, :, / as documented. Consider using a keybinding library.
3. **Modal z-index** — All modals at z-index 10, dimmed overlay at same level. Use portals or stacking context.
4. **Scrolling body** — Settings body is block-flow flex (not column flex) with overflow:auto, so dividers render at full 2px.
5. **Radio namespacing** — Namespace radio name attributes per instance to avoid browser collisions (e.g., df-1c, df-2a).
6. **Typography scale** — Stick to listed sizes and weights; avoid intermediate values.
7. **Spacing consistency** — Most section boundaries use 48px margin-bottom; apply uniformly.
