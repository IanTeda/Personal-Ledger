# Handoff: Accounts Surface

> **Package 17 of 20 · Accounts** — frames 17a–17d (17a, 17b, 17c, 17d). Open `Accounts.dc.html` in a browser from this folder. Frame numbers match the master file `Ledger Desktop Shell.dc.html`; links to other frames open the sibling package. Shared shell, rail, tokens and the package index are in `../README.md`.

## Overview
This package contains the design for **Personal Ledger's Accounts management surface** — the full-width landing page for the Accounts section, and the three dialogs that create, edit, and retire accounts. Four variants (17a–17d) cover the resting state and each dialog.

Accounts here is a **management page**, not a scoped ledger — it is the destination reached from the primary rail's Accounts item when no single account is selected. Selecting a row opens the per-account transaction ledger (documented separately, see the Shell & Navigation handoff, screen 1b); this page is where accounts are created, renamed, and deleted.

## About the Design Files
`Accounts.dc.html` is a **high-fidelity HTML prototype** showing intended look, layout, and interaction model. It is a **design reference, not production code**. Recreate these designs in the target codebase (Rust + GPUI).

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states.

## Frame
All variants drawn at **1280 × 800**. Shell is header (48px) / body (flex:1) / status bar (28px). In 17b–17d the shell behind the modal is drawn dimmed in the mockup (`opacity:.30`/`.38`) to focus the dialog — in the real app the shell renders at full opacity behind a semi-transparent dimmer layer, per the same pattern used in the command palette and Settings dialogs.

---

## Screens

### 17a — Accounts (resting state)
**Purpose**: The Accounts landing page. Primary rail expanded with Accounts active; main pane is a full-width table grouped by account type, mirroring the Units/Institutions table pattern from Settings.

**Layout**:
- **Header** (48px): breadcrumb `accounts`
- **Primary rail** (206px): full app nav, Accounts active (dark treatment, count badge `7` in the inverted color)
- **Body** (flex:1, `padding:22px 28px; overflow:auto`):
  - Page header row: "Accounts" (28px/800) + net worth summary ("7 accounts · net worth **83,995.87 aud**", 11.5px `#9b9797`) on the left; **+ Add account** button (`background:#201e1d; color:#f3f2f2; padding:10px 16px; font-weight:800`) on the right
  - 2px rule (`margin-bottom:24px`)
  - One group block per account type, in order: **Bank, Credit card, Loan, Investment** — `margin-bottom:28px` between groups
    - Group heading: `h4`, 16px, `margin-bottom:14px`
    - Table: `border:1px solid rgba(32,30,29,.30)`
      - Header row: `padding:10px 16px; background:#eae9e9; font:800 10px 'Archivo'; letter-spacing:.11em; color:#605d5d`, columns NAME / INSTITUTION / UNIT / BALANCE / ACTIONS
      - Data row: `padding:10px 16px`, 1px bottom rule between rows (last row in each group omits it); name column 800 weight for the account, `flex:1`; institution and unit columns `color:#605d5d`, fixed widths 130px/90px; balance right-aligned, `tabular-nums`, negative balances in `#ae1800`; actions column right-aligned, 150px, holds **edit** and **delete** buttons
      - Row action buttons: `padding:4px 10px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent`
- **Status bar** (28px): `NORMAL` mode chip; legend `j/k row · enter open ledger · e edit · d delete · n new`; right-aligned "7 accounts"

**Data shown**: Bank — ANZ Everyday (aud, 4,182.55), ANZ Offset (aud, 61,204.10). Credit card — Amex Platinum (aud, −2,318.44). Loan — Home Loan (aud, −381,311.34). Investment — Vanguard VAS (vas, 1,240 u), Bitcoin (btc, 0.4120 u).

### 17b — Add account
**Purpose**: The dialog behind **+ Add account**. Four required fields, one optional.

**Fields** (dialog width 440px, body `padding:20px`, field `gap:16px`):
- Name — text input, full width, placeholder "e.g. Everyday Account"
- Institution / Type — two selects side by side (`gap:16px`, each `flex:1`). Institution options: ANZ, Westpac, Commonwealth Bank, NAB, Vanguard, Crypto.com. Type options: savings, credit card, offset, loan, investment, cryptocurrency
- Unit / Opening balance — two fields side by side. Unit select: aud, btc, vas. Opening balance: text input, `tabular-nums`, placeholder "0.00"
- Account number — text input, label suffixed "(optional)" in `#9b9797`, placeholder "•••• •••• 1234"

Actions: **Cancel** (secondary, transparent) / **Add account** (primary, `background:#201e1d`).

### 17c — Edit account
**Purpose**: Same form as 17b, pre-filled for "ANZ Everyday", plus a usage notice.

**Difference from 17b**: fields carry `value=` instead of `placeholder=`; Account number replaces Opening balance in the second row (balance isn't editable once an account has history); an info panel closes the form:

> Opened Mar 2019 · 312 transactions. Renaming is safe; changing the unit after transactions exist is not recommended.

Panel style: `padding:10px; background:#eae9e9; border-left:2px solid #ec3013; font-size:11.5px; color:#605d5d` — same as the Units edit notice (16c). Actions: Cancel / **Save**.

### 17d — Delete account
**Purpose**: Destructive confirm for "Amex Platinum", following the exact pattern used for unit deletion (16d) — named consequences, typed-back confirmation.

**Layout**:
- Dialog border and header rule: `2px solid #ec3013`; title `color:#ae1800` — "Delete account — Amex Platinum"
- Warning paragraph: names the balance and transaction count — "This account has a balance of **−2,318.44 aud** and **204 transactions**. Deleting it cannot be undone."
- Reference panel (`background:#eae9e9; border-left:2px solid #ec3013`): "204 transactions will be permanently deleted · referenced in **2 budgets**"
- Confirmation field: label "Type `Amex Platinum` to confirm", empty text input
- Actions: Cancel / **Delete account** (`background:#ec3013; color:#f3f2f2`) — **disabled until the typed value matches the account name exactly**

---

## Components

| Part | Spec |
| --- | --- |
| Page header | flex row, `align-items:flex-end; justify-content:space-between; margin-bottom:24px` |
| Net worth line | 11.5px, `#9b9797`; the figure itself `color:#201e1d; font-weight:800; font-variant-numeric:tabular-nums` |
| Add button (page) | `padding:10px 16px; background:#201e1d; color:#f3f2f2; font-weight:800; border:none` |
| Group heading | `h4`, 16px, `margin:0 0 14px` |
| Group table | `border:1px solid rgba(32,30,29,.30)` |
| Table header row | `padding:10px 16px; background:#eae9e9; font:800 10px 'Archivo'; letter-spacing:.11em; color:#605d5d` |
| Table data row | `padding:10px 16px`, `border-bottom:1px solid #d7d3d3` except last row in group |
| Row action button | `padding:4px 10px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent` |
| Dialog | 440px, `background:#f3f2f2; border:2px solid #201e1d; box-shadow:0 16px 48px rgba(32,30,29,.40)` |
| Destructive dialog | same, `border:2px solid #ec3013` |
| Dialog header | `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30)`; title `font:800 16px 'Archivo'; letter-spacing:.01em` |
| Destructive header | `border-bottom:2px solid #ec3013`; title `color:#ae1800` |
| Dialog body | `padding:20px`, fields `gap:16px`; two-up field rows `gap:16px` with each field `flex:1` |
| Field label | `display:block; font-weight:800; font-size:12px; margin-bottom:6px` |
| Input / select | `width:100%; padding:8px 10px; border:1px solid rgba(32,30,29,.30); font-size:13px; box-sizing:border-box`; selects add `background:#f3f2f2` |
| Info / warning panel | `padding:10–12px; background:#eae9e9; border-left:2px solid #ec3013; font-size:11.5px; color:#605d5d` |
| Dialog action row | `padding:16px 20px; border-top:1px solid #d7d3d3; display:flex; gap:10px; justify-content:flex-end` |
| Cancel button | `padding:8px 16px; border:1px solid rgba(32,30,29,.30); background:transparent; font-weight:800` |
| Confirm button | `padding:8px 16px; background:#201e1d; color:#f3f2f2; border:none; font-weight:800` |
| Destructive confirm | same, `background:#ec3013` |

---

## Design Tokens

### Color
| Role | Value |
| --- | --- |
| Ground | `#f3f2f2` |
| Chrome / panel | `#eae9e9` |
| Ink | `#201e1d` |
| Ink secondary | `#605d5d` |
| Ink tertiary | `#9b9797` |
| Ink on dark | `#f3f2f2` / `#bab6b6` (dimmed) |
| Accent | `#ec3013` |
| Accent (text-safe) | `#ae1800` |
| Negative balance | `#ae1800` |
| Rule (strong) | `rgba(32,30,29,.38)` |
| Rule (medium) | `#d7d3d3` |
| Border | `rgba(32,30,29,.30)` |
| Dimmer | `rgba(32,30,29,.30)` |

### Type
- Family: **Archivo** throughout
- Weights: 400, 800 — nothing between
- Sizes: 10px (table headers), 11px (row actions), 11.5px (meta, panels), 12px (field labels, breadcrumb), 13px (body, inputs), 16px (dialog title), 16px (group heading), 28px (page heading)
- Numbers: `font-variant-numeric: tabular-nums` on every balance figure

### Spacing
6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28px. Field gap 16px inside dialogs; group gap 28px on the page; row padding 10px 16px in tables.

### Radius & elevation
- **Radius 0 everywhere.**
- Dialog shadow: `0 16px 48px rgba(32,30,29,.40)`

---

## Interactions

### Page (17a)
- Row click / `enter` → navigate to that account's transaction ledger (screen 1b)
- `e` or the row's edit button → open 17c pre-filled for that row
- `d` or the row's delete button → open 17d for that row
- `n` or **+ Add account** → open 17b
- `j`/`k` move the row selection

### Dialog lifecycle
| Trigger | Flow |
| --- | --- |
| **+ Add account** | open 17b → fill Name, Institution, Type, Unit, Opening balance (Account number optional) → Add account → validate all required fields → append to the matching type group → close |
| Row **edit** | open 17c pre-filled → modify Name / Institution / Type / Unit / Account number → Save → update row in place → close |
| Row **delete** | open 17d → type the account name → Delete account enables on exact match → Delete account → remove row + close |

- `esc` closes any dialog without saving; `enter` submits when the confirm action is enabled
- Focus moves into the dialog's first field on open, returns to the trigger on close
- Dialog traps focus while open

### States
- Hover: rows and buttons take a subtle ground tint or border shift
- Focus: `outline: 2px solid #ec3013; outline-offset: 2px`
- Disabled: 45% opacity (the delete confirm before the typed name matches)

---

## State

```
accounts
  list: Vec<Account>            // grouped by `type` for display
  dialog: Option<Dialog>        // AddAccount | EditAccount(id) | DeleteAccount(id)
  confirmInput: String          // typed-back name for DeleteAccount

Account {
  id, name, institution, type,  // savings | credit_card | offset | loan | investment | cryptocurrency
  unit, balance, accountNumber?, openedAt, transactionCount
}
```

---

## Implementation notes (GPUI)

1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **This page is a management surface, not the ledger.** Keep it separate from the per-account transaction view (1b) — this page lists and administers accounts; 1b is where you post and reconcile transactions.
3. **Group ordering is fixed**: Bank, Credit card, Loan, Investment — do not alphabetize or reorder by balance.
4. **Balance sign color is a rule, not a per-row choice**: negative balances always render `#ae1800`, regardless of account type (a loan's negative balance and a credit card's negative balance use the same rule).
5. **Delete confirm is typed, not clicked** — keep the confirm button disabled until the typed value matches the account name exactly, same as the Units and Institutions destructive dialogs.
6. **Opening balance is add-only.** The edit form deliberately drops the Opening balance field and adds Account number in its place — balance isn't an editable field once transactions exist.
7. **Zero radius, flush-left labels** — including labels inside wide buttons.
8. **Reuse the existing dialog chrome** (header/body/action-row spacing, border weights) rather than introducing a new dialog size — 17b/3c/3d use 440px vs. the 420px used by the Units/Institutions dialogs only because two-up field rows need the extra width; don't vary width further.

## Files
- `Accounts.dc.html` — the prototype (section 3 = 17a–17d)
- `README.md` — this document
