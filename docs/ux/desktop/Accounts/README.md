# Handoff: Accounts Surface

## Overview
This package contains the design for **Personal Ledger's Accounts management surface** — the full-width landing page for the Accounts section, and the three dialogs that create, edit, and retire accounts. Four variants (3a–3d) cover the resting state and each dialog.

Accounts here is a **management page**, not a scoped ledger — it is the destination reached from the primary rail's Accounts item when no single account is selected. Selecting a row opens the per-account transaction ledger (documented separately, see the Shell & Navigation handoff, screen 1b); this page is where accounts are created, renamed, and deleted.

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** showing intended look, layout, and interaction model. It is a **design reference, not production code**. Recreate these designs in the target codebase (Rust + GPUI).

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states.

## Frame
All variants drawn at **1280 × 800**. Shell is header (48px) / body (flex:1) / status bar (28px). In 3b–3d the shell behind the modal is drawn dimmed in the mockup (`opacity:.30`/`.38`) to focus the dialog — in the real app the shell renders at full opacity behind a semi-transparent dimmer layer, per the same pattern used in the command palette and Settings dialogs.

---

## Screens

### 3a — Accounts (resting state)
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

### 3b — Add account
**Purpose**: The dialog behind **+ Add account**. Four required fields, one optional.

**Fields** (dialog width 440px, body `padding:20px`, field `gap:16px`):
- Name — text input, full width, placeholder "e.g. Everyday Account"
- Institution / Type — two selects side by side (`gap:16px`, each `flex:1`). Institution options: ANZ, Westpac, Commonwealth Bank, NAB, Vanguard, Crypto.com. Type options: savings, credit card, offset, loan, investment, cryptocurrency
- Unit / Opening balance — two fields side by side. Unit select: aud, btc, vas. Opening balance: text input, `tabular-nums`, placeholder "0.00"
- Account number — text input, label suffixed "(optional)" in `#9b9797`, placeholder "•••• •••• 1234"

Actions: **Cancel** (secondary, transparent) / **Add account** (primary, `background:#201e1d`).

### 3c — Edit account
**Purpose**: Same form as 3b, pre-filled for "ANZ Everyday", plus a usage notice.

**Difference from 3b**: fields carry `value=` instead of `placeholder=`; Account number replaces Opening balance in the second row (balance isn't editable once an account has history); an info panel closes the form:

> Opened Mar 2019 · 312 transactions. Renaming is safe; changing the unit after transactions exist is not recommended.

Panel style: `padding:10px; background:#eae9e9; border-left:2px solid #ec3013; font-size:11.5px; color:#605d5d` — same as the Units edit notice (2c). Actions: Cancel / **Save**.

### 3d — Delete account
**Purpose**: Destructive confirm for "Amex Platinum", following the exact pattern used for unit deletion (2d) — named consequences, typed-back confirmation.

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

### Page (3a)
- Row click / `enter` → navigate to that account's transaction ledger (screen 1b)
- `e` or the row's edit button → open 3c pre-filled for that row
- `d` or the row's delete button → open 3d for that row
- `n` or **+ Add account** → open 3b
- `j`/`k` move the row selection

### Dialog lifecycle
| Trigger | Flow |
| --- | --- |
| **+ Add account** | open 3b → fill Name, Institution, Type, Unit, Opening balance (Account number optional) → Add account → validate all required fields → append to the matching type group → close |
| Row **edit** | open 3c pre-filled → modify Name / Institution / Type / Unit / Account number → Save → update row in place → close |
| Row **delete** | open 3d → type the account name → Delete account enables on exact match → Delete account → remove row + close |

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
8. **Reuse the existing dialog chrome** (header/body/action-row spacing, border weights) rather than introducing a new dialog size — 3b/3c/3d use 440px vs. the 420px used by the Units/Institutions dialogs only because two-up field rows need the extra width; don't vary width further.

## Acceptance pass

The closing walk of the [Desktop Accounts Surface](https://github.com/IanTeda/Personal-Ledger/issues/190) map: every 3a page element and 3b–3d dialog checked against this document, now that all four have landed against `crates/bins/bin-desktop/src/view/accounts/` and `crates/bins/bin-desktop/src/accounts.rs` — the same closing walk `docs/ux/desktop/Settings/README.md`'s "Acceptance pass" made for that bundle. All data is stubbed and in-memory, as the map's Destination says.

- **3a — page.** Met. Header row (title, count and net worth line, **+ Add account**), the 2px rule, then one bordered table per type with the NAME / INSTITUTION / UNIT / BALANCE / ACTIONS header (10px, 800, `#605d5d` on `#eae9e9`), 10px 16px rows with the 1px rule between them and none after the last, names at 800, institution and unit in the secondary ink, balances right-aligned, edit and delete buttons per row, and the status bar (`j/k row · enter open ledger · e edit · d delete · n new`, right-aligned account count). The primary rail's Accounts badge is the live count, not a fixed `7`.
- **Negative balances are a rule, not a per-row choice.** Met — `view::accounts::row` colours any balance `accounts::format_amount` reports negative with `ACCENT_TEXT` (`#ae1800`) and a U+2212 minus, for every account type alike; a zero is never negative even if its text is `-0.00`.
- **Group order is fixed.** Met — `accounts::GROUP_ORDER` (tested, and tested against `AccountType::all()`, whose own order puts Investment before Loan): Cash, Bank, Credit card, Loan, Investment. Empty types are omitted, and within a group accounts keep the order they were added.
- **3b — Add.** Met. Name, Institution / Type, Unit / Opening balance, and Account number (optional, UI-only); 440px, 16px field gaps; required fields validated (Add account stays at 45% opacity until valid); a new account appends to its type's group and becomes the selection.
- **3c — Edit.** Met, with the departure below: Name, Institution, Type and Account number editable; Unit and Opening balance read-only; the usage notice in the info panel.
- **3d — Delete.** Met. Destructive border and header, the balance and transaction count named, the reference panel, and **Delete account** disabled until the typed value matches the name exactly (`DeleteAccountForm::matches`, a case-sensitive `==` with no trimming); `Enter` is equally inert until then.
- **Dialog lifecycle.** Met. `Esc` closes any dialog without saving (an open dropdown list closes first); `Enter` submits when the confirm action is enabled; focus moves into the first field on open and `NavState::exit_mode` restores the zone that had it on close; while a dialog is open every keystroke routes through `Shell::handle_dialog_key`, which is how this shell "traps focus".
- **States.** Rows take a `HOVER_TINT` tint; the page's buttons now do too (**+ Add account** lightens one step inside the palette, the row buttons take the tint) — a gap this pass found and fixed. Disabled is 45% opacity. **Focus** is drawn as the accent border on the focused field, the same treatment every Settings dialog uses; there is no separate `outline: 2px solid #ec3013` ring on buttons, since this shell has one keyboard focus (`Shell`) and no per-button focus — the shell's acceptance criterion 9 forbids an unthemed ring for the same reason.

### Deliberate departures from the mockup

Each was decided on the map, with Ian, and recorded on its ticket:

- **The type list follows the glossary, not the mockup.** Groups and the Type control are Cash, Bank, Credit card, Loan, Investment (Cash and Bank being the Transaction Account Type). The mockup's `savings`, `offset` and `cryptocurrency` are not domain concepts. The mockup's own row data was reused with one added Cash row (the mockup says "7 accounts" but draws six).
- **Institution and Unit options come from Settings**, live, not the mockup's fixed lists. Institution is read-only ("No institution") while Type is Cash, the glossary's system-seeded placeholder.
- **Edit fixes Unit and Opening balance**, shown greyed and labelled `(fixed)` with the reason in the info panel, instead of the mockup's editable Unit with an advisory notice. The glossary fixes a Unit at creation.
- **The net worth line respects the no-cross-Unit rule**: one figure in the base Unit only, then every other Unit named ("· vas, btc held separately"), computed from the rows. The mockup's printed figure matched no sum of its own rows, so two seeded balances were adjusted (ANZ Offset 463,203.10, Wallet 240.00) so the base-Unit rows sum to exactly the printed 83,995.87.
- **No context rail** beside the page, as in the mockup; `Noun::has_context_entities` is now false for Accounts as well as Settings.
- **Delete removes the account's transactions with it**, as designed here. The TUI design's transfer-to-another-account path (FR.14) is not designed for desktop and is out of scope.
- **Palette commands, added after the map closed**: `:accounts new [<account name>]`, `:accounts edit [<account name>]` and `:accounts delete [<account name>]`, each opening the same dialog as `n`/`e`/`d` from any page (it jumps to Accounts first). `new` pre-fills Name with the argument; `edit` and `delete` resolve the typed name case-insensitively (an exact name, else a unique prefix, else a unique substring) and, given none, use the selected row. A name matching nothing or several accounts flashes a status-line message naming the problem rather than guessing. The TUI's `account off`/`on`/`check` have no desktop counterpart (no active flag, no balance checks), and the desktop uses the plural noun (`accounts`) to match its `accounts` page jump.
- **`enter` and a row click open Transactions filtered to that account** (added by the [Desktop Transactions Surface](https://github.com/IanTeda/Personal-Ledger/issues/199) map), not screen 1b's per-account ledger, which stays unbuilt; they flashed "open ledger -- not yet built" when this pass was written.
- **Institution column is 170px, not 130px**, so seeded names such as "Cryptocurrency Exchange" fit; NAME has a 140px floor so a narrow window clips the other cells instead of squeezing it to nothing.

### Verified live vs by code review

The page and all three dialogs were confirmed in a real window, driven by keystrokes injected through `gpui::Window::dispatch_keystroke` (a temporary, env-driven diagnostic, removed before each commit) and screenshotted: the page against the mockup; Add opening, typing, the inline dropdown list with its highlight, Cash making Institution read-only, the first `Esc` closing only the list, and a full create landing in the right group with the header and badge updating; Edit pre-filled with the fixed fields greyed, then saving a rename and Type change that regrouped the row; and Delete with the button disabled on a partial name and a full-name `Enter` removing the row, emptying its group and updating the header and badge. **Mouse clicks and hover** are verified by code review and this crate's test coverage only — this sandbox has no synthetic mouse path — so the click handlers (field, dropdown row, buttons) follow the Settings dialogs' proven wiring but were not clicked. The pure logic (grouping, amount formatting, net worth, the dropdown state machine, both forms) is covered by `cargo test -p bin_desktop`.

### Known gaps

- `gpui` 0.2 has no letter-spacing or `tabular-nums` hook, so the table headers' `.11em` tracking and the balance figures' tabular alignment are not reproduced (right-aligned, with Archivo's own digit metrics).
- The Settings Display section's decimal-separator preference is not applied to balances here (the Transactions table does honour it, via `format.rs`).
- Stub counts (transactions, budgets) on every account are fixed figures; nothing real backs them.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section 3 = 3a–3d)
- `README.md` — this document
