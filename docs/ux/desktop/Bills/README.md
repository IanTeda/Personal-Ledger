# Handoff: Bills — Bill Plan, Bill Schedule, Pay / Skip, History

## Overview
This package covers **Personal Ledger's Bills** surface, built to `docs/bills.md` (concept branch) and ADR-0019. Bills split into two concepts: a **Bill Plan** (the recurring definition — "Telstra Internet, ~$89, monthly") and its **Bill Schedule** (one persisted, dated row per due date, generated ahead of time, never deleted). The view is a three-tab surface — Schedule / Planner / History — plus three modals. Six variants (8a–8f).

## About the Design Files
`Ledger Desktop Shell.dc.html` is a **high-fidelity HTML prototype** — a design reference, not production code. Recreate these designs in the target codebase (Rust + GPUI) using established patterns, informed by the mockup's layout, typography, color and behavior.

## Fidelity
**High-fidelity.** Final colors, typography, exact spacing, and interaction states. Recreate faithfully; translate HTML structure into GPUI idioms.

## Frame
All variants are drawn at **1280 × 800**, the reference desktop window. Vertical stack: header (48px) / body (flex:1) / status bar (28px). Same shell as the other handoff packages — Bills is the active nav item, badge shows the Needs-Attention count (Overdue + Due-within-Attention-Lead), not a plain row count.

---

## Concepts (read this before the screens)

- **Bill Plan** — the recurring definition. Fields: Name, Category (expense-only), Unit (locked at creation), Account (mandatory), Payee (optional), Planned Amount, Fixed | Estimated, Recurrence (Weekly/Fortnightly/Monthly/Quarterly/Annually/One-shot), Ends On (optional), Attention Lead (optional day count), Active (soft-delete).
- **Bill Schedule** — one row per due date, generated ahead of time for the plan period. Status is **derived from today's date, never stored**: `Upcoming` (due date in a future month) → `Due` (due date this month, 1st through the due day inclusive) → `Overdue` (due day passed, unpaid). `Paid` and `Skipped` are terminal. No Schedule row is ever deleted.
- **Needs Attention (⚑)** — every `Overdue` row, plus any `Due` row inside its Plan's own Attention Lead (blank Attention Lead = Overdue-only). This is the same flag that feeds the Dashboard's cross-app Needs Attention list.
- **Marking paid** — never a bare toggle. Either (a) *pay it directly*, which creates a new Transaction seeded from the Plan and links it, or (b) *link an existing Transaction* (e.g. one that came in through CSV import), pointing it at the Schedule row instead of creating a duplicate. Either way the Transaction's own date/amount is what counts afterward — the Plan's amount was only ever an estimate.
- **Skip** ≠ Paid $0 — a Skipped row is excluded entirely from that Bill's history average, not counted as a zero payment.

---

## Screens

### 8a — Bill Schedule (default tab)
**Purpose**: The dated, actionable rows for the current period ("September 2026"), sorted by due date.

**Layout**:
- Header (48px) + primary rail (206px, Bills active, badge **3** = Needs Attention count) — shared shell chrome
- Main pane (`padding:22px 28px`):
  - Header row: "Bills" (28px/800) + meta "9 schedule entries this period · **2** overdue · **1** due within its attention lead" (11.5px `#9b9797`, counts in `#ae1800` 800) · `+ Add bill plan` primary button (36px)
  - **Tab row**: `Schedule` (active, dark fill) / `Planner` / `History`, flex spacer, then a period nav `‹ September 2026 ›` (only shown on this tab)
  - 2px full-width rule
  - Bordered table: BILL (flex:1) / ACCOUNT (100px) / PLANNED (100px, right) / DUE (80px) / STATUS (140px) / ACTIONS (200px, right)
- Status bar: `NORMAL` chip, legend `j/k row · p pay · s skip · tab switch view`, right "September 2026 · 9 entries"

**Row anatomy**:
- **Paid** row: muted link-glyph icon + "PAID" label (`#605d5d`/`#d7d3d3`), ACTIONS cell is a single `view transaction →` link (no pay/skip)
- **Skipped** row: entire row muted `#9b9797`, STATUS reads "SKIPPED", ACTIONS reads "excluded from history" (no buttons)
- **Overdue** row: STATUS is a filled accent pill `⚑ OVERDUE` (`background:#ec3013` on the focused/dark row, `rgba(236,48,19,.12)` fill / `#ae1800` text on normal rows); ACTIONS shows `pay` (filled) + `skip` (outlined) buttons
- **Due** row: STATUS is a neutral pill `DUE`, with the ⚑ prefix only when inside the Plan's Attention Lead; same pay/skip actions
- **Upcoming** row: fully muted, STATUS is plain lowercase "upcoming" text (no pill), ACTIONS reads "not yet actionable" — nothing to do yet
- The focused/keyboard-selected row (topmost Overdue, "Gym") gets the dark treatment: `background:#201e1d;color:#f3f2f2`
- Amounts for **Estimated** plans show a `~` prefix and a small "· estimated" suffix on the name; Fixed plans show a plain number

**Sample data shown** (chronological): Streaming Bundle (Paid, 05 sep) · Car Wash Membership (Skipped, 10 sep) · Gym — Fitness First (Overdue, 15 sep, focused row) · Netflix (Overdue, 18 sep) · Telstra Internet (Due + ⚑ attention lead, 22 sep) · Origin Energy (Due, estimated, 24 sep) · Car Insurance — AAMI (Due, 30 sep) · Rent (Upcoming, 01 oct) · Council Rates (Upcoming, estimated, 05 nov).

---

### 8b — Bill Planner tab
**Purpose**: The recurring Bill Plan definitions themselves — created once, edited occasionally.

**Layout**: Same shell; tab row shows `Planner` active, no period nav (not period-scoped). Header meta: "10 bill plans · **1** inactive". Table columns: NAME (flex:1) / CATEGORY (95px) / ACCOUNT (75px) / PLANNED (100px, right) / RECURS (85px) / LEAD (60px) / ACTIVE (60px) / ACTIONS (60px, `edit` only — deactivating happens inside the edit modal, matching the tag/category/account soft-delete pattern already used elsewhere in the app).

**Row treatment**: Focused row (Rent) gets the dark treatment. A row for a Plan whose Attention Lead is set (Telstra Internet) is tinted `#faf3ef` to draw the eye to that column. An inactive Plan (**Basic Fitness (closed)**) renders fully muted `#9b9797` with no LEAD/ACTIVE distinction beyond the "no" — its history stays queryable in the History tab.

**Sample data shown**: Rent, Netflix, Gym — Fitness First, Telstra Internet (3d lead), Origin Energy (estimated), Car Insurance — AAMI (5d lead), Council Rates (estimated), Streaming Bundle, Car Wash Membership, Basic Fitness (closed, inactive).

---

### 8c — Edit bill plan (modal)
**Purpose**: The Plan's full field set. Add uses the identical form with **Unit editable** (it locks after creation) and **no Active toggle** (a Plan is always active on creation).

**Layout**: Standard centered dialog, 480px wide (wider than other app modals — this form has more fields), scrollable body (`max-height:480px`). Title "Edit bill plan — Telstra Internet".

**Fields, in order**: Name (text) → Category *(select, labeled "(expense only)")* + Unit *(disabled text input on Edit, showing the locked currency)* → Account (select) + Payee *(select, labeled "(optional)")* → Planned amount (tabular text input) + Fixed/Estimated (`.seg`, two options) → Recurrence (select: Weekly/Fortnightly/Monthly/Quarterly/Annually/One-shot) + Ends on *(text input, placeholder "Recurs indefinitely")* → Attention lead *(number input, ~140px wide, labeled with the full explanation inline since this field has no other affordance)* → Active checkbox (label "Active — generate future Schedule rows").

Footer: `Cancel` (outlined) + `Save` (solid dark), right-aligned — no destructive/delete action in this modal (see 8b's note: deactivating is just unchecking Active and saving, not a separate confirmation flow, since it's reversible and non-destructive by design).

---

### 8d — Pay a Schedule entry (modal)
**Purpose**: The view's key differentiator — the spec's two settlement paths, presented as a segmented switch inside one modal rather than two separate flows.

**Layout**: Standard centered dialog, 460px. Title "Pay — Telstra Internet, 22 sep". Directly under the title bar, a two-option `.seg`-style switch: **Pay it directly** / **Link existing transaction** (shown selected/dark in this mock, since it's the more novel path).

**"Link existing transaction" panel** (shown): label "UNMATCHED TRANSACTIONS FROM TELSTRA" (11px/800, letter-spacing `.08em`, `#9b9797`) above a small radio list — each option is a full-width row (`padding:10px 12px`, bordered), leading 14px radio dot, transaction summary (`Telstra · 21 sep · −79.99`), trailing muted account label. The matching candidate is pre-selected (accent border + `#faf3ef` fill). A final fallback option, "None of these — pay it directly instead," switches the segmented control to the other panel. Inline notice below: explains linking discards the Plan's estimate in favor of the Transaction's real figures and won't create a duplicate.

**"Pay it directly" panel** (not pictured, build per spec): the same modal chrome, seg switched to that option, showing an editable Amount (prefilled from the Plan's planned figure), a Date field (defaults to today), and read-only chips for the seeded Category/Payee/Account. Confirm button reads "Create transaction & mark paid".

Footer: `Cancel` + confirm button, label matches the active panel (`Link & mark paid` / `Create transaction & mark paid`).

---

### 8e — Skip this cycle (modal)
**Purpose**: Lightweight, non-destructive confirmation for a cycle you deliberately didn't pay. No typed-name confirmation (mirrors the Tags-remove pattern, 7d) — skipping doesn't destroy or orphan anything, it just resolves this one Schedule row.

**Layout**: Standard centered dialog, 400px (smallest size, used for single-paragraph confirmations app-wide). Title "Skip this cycle — Origin Energy". Body: one paragraph explaining the 24 sep entry becomes `Skipped` (not `Paid`), is excluded from the average (not counted as $0), and that October's entry is generated independently. Footer: `Cancel` + `Skip this cycle` (solid dark — this is not framed as destructive, so it does **not** get the accent/red treatment used for true deletes).

---

### 8f — Bill History tab
**Purpose**: Every Schedule row ever generated, filterable, since none are ever deleted. Doubles as the source data behind each Plan's last-paid/average/same-period-last-year figures.

**Layout**: Tab row shows `History` active. No `+ Add` button (this tab is read-only/query). Filter row (`gap:8px`, wraps): two toggled status chips (`✓ Paid`, `✓ Skipped`, both dark/active) + one untoggled (`Overdue`, outlined), a vertical divider, then four selects — Bill (accent-bordered when scoped to one bill, as shown: "Bill: Telstra Internet"), Category, Account, and a date-range preset ("This financial year").

Below the filters, a **stat callout** (`background:#eae9e9;border-left:2px solid #201e1d`, `padding:14px 16px`, three inline stats `gap:24px`): "Last paid **$79.99** · 21 aug" / "Average **$81.40**" / "Same month last year **$76.20**" — only shown/meaningful when filtered to a single Bill.

Table columns: BILL (flex:1) / DUE (90px) / STATUS (100px) / PLANNED (100px, right) / ACTUAL (100px, right) / PAID (90px, the settlement date). A Skipped row shows its planned figure but `—` for ACTUAL and PAID. Focused row (topmost) gets the dark treatment. Footer note clarifies Skipped rows are excluded from the stat callout's math.

Status bar right label: "4 of 42 rows shown" — this table paginates; 42 is the full historical row count across all Plans before filtering.

---

## Components (shared with other handoff packages — see Desktop Shell & Navigation for header/rail/status-bar specs)

| Part | Spec |
| --- | --- |
| Tab switcher | inline row, active tab `padding:8px 16px;background:#201e1d;color:#f3f2f2;font-weight:800`, inactive tabs plain link-styled text `#605d5d`, no underline |
| Period nav (Schedule tab only) | `‹ September 2026 ›`, 12px, label `#201e1d` 800, arrows `#605d5d` clickable |
| Needs-Attention flag | `⚑` prefix character directly inside the STATUS pill text, not a separate icon/column |
| Status pill — Overdue | filled `background:#ec3013;color:#f3f2f2` (on dark focused row: `background:#ec3013` still, or inverted `#f3f2f2`/`#201e1d` for the button — see 8a row 3 for the focused-row variant); normal row: `background:rgba(236,48,19,.12);color:#ae1800` |
| Status pill — Due | `background:#eae9e9;color:#201e1d`, 10.5px/800, `padding:2px 7px` |
| Status text — Upcoming/Skipped/Paid | no pill box, plain lowercase (upcoming) or uppercase (PAID/SKIPPED) text at reduced weight/color |
| Row action buttons | `pay`/`skip`/`edit` micro-buttons, `padding:4px 8px–9px;font-size:11px`; `pay` is the filled/primary-weight one when present |
| Segmented pay-path switch | two `.seg-opt` cells directly under a modal's title bar (not in the body), selected cell `background:#201e1d;color:#f3f2f2` |
| Radio match row | `padding:10px 12px`, bordered, 14px circular radio dot; selected = accent border + `#faf3ef` fill |
| Filter chip (toggled) | dark filled `✓ Label`, same shape as an outlined chip but active |
| Stat callout | `background:#eae9e9;border-left:2px solid #201e1d;padding:14px 16px`, inline stats `gap:24px`, label `#605d5d` + bold figure |
| Modal (standard) | 400/460/480px depending on field count, centered, `border:2px solid #201e1d`, `box-shadow:0 16px 48px rgba(32,30,29,.40)` over `rgba(32,30,29,.30)` dimmer |

## Design Tokens
Same palette as the rest of the shell (see Desktop Shell & Navigation README for the full token table): ground `#f3f2f2`, chrome `#eae9e9`, ink `#201e1d`, ink secondary `#605d5d`, ink tertiary `#9b9797`, accent `#ec3013`, accent text-safe `#ae1800`, rule strong `rgba(32,30,29,.38)`, rule medium `#d7d3d3`, border `rgba(32,30,29,.30)`. Family: Archivo throughout, weights 400/800 only, `font-variant-numeric:tabular-nums` on every money figure. Radius: 0 everywhere.

## Interactions

### Schedule tab (8a)
- `j`/`k` move row focus; `p` opens Pay (8d) for the focused row; `s` opens Skip (8e); `enter` opens the linked Transaction for a Paid row
- Period nav moves the whole Schedule table to the adjacent month — Paid/Skipped/Overdue/Due/Upcoming rows for that period only
- Upcoming rows have no row actions — nothing is actionable until the row enters Due

### Planner tab (8b)
- `edit` opens 8c; unchecking **Active** and saving is how a Plan is soft-deleted — stops future Schedule generation, existing rows untouched
- `+ Add bill plan` opens the same modal as 8c with Unit unlocked and no Active checkbox

### Pay modal (8d)
- Switching the segmented control swaps the panel without closing the modal or losing the Schedule-row context in the title
- Link path: `j`/`k` choose a candidate transaction, `enter` confirms
- Direct path: Amount is editable and pre-filled from the Plan's estimate; Date defaults to today

### Skip modal (8e)
- Single confirm action, no typed-name gate (non-destructive, reversible only by editing history if ever needed — out of scope here)

### History tab (8f)
- Status chips are multi-select toggles; Bill/Category/Account are single-select scopes; changing any filter recomputes both the table and the stat callout
- Selecting a single Bill is what populates the stat callout; leaving it at "All bills" hides the callout (build per spec, not separately pictured)

## State
```
billPlan
  id, name: String
  categoryId: Id                      // expense-type only
  unit: CurrencyCode                  // locked after creation
  accountId: Id                       // mandatory
  payeeId: Option<Id>
  plannedAmount: Decimal
  amountKind: Fixed | Estimated
  recurrence: Weekly | Fortnightly | Monthly | Quarterly | Annually | OneShot
  endsOn: Option<Date>
  attentionLeadDays: Option<int>       // unset = Overdue-only
  active: bool                         // soft-delete

billScheduleEntry
  id: Uuidv7                           // number-prefixed with the plan-period name
  billPlanId: Id
  dueDate: Date
  status: Upcoming | Due | Overdue | Paid | Skipped   // Upcoming/Due/Overdue derived from today vs dueDate; Paid/Skipped stored
  linkedTransactionId: Option<Id>      // set only when Paid
  needsAttention: bool                 // Overdue, or (Due && withinAttentionLead)

billHistoryFilter
  statuses: Set<Status>
  billPlanId: Option<Id>
  categoryId, payeeId, accountId: Option<Id>
  dateRange: { from: Date, to: Date }

payModal
  scheduleEntryId: Id
  mode: PayDirect | LinkExisting
  // PayDirect: draftAmount, draftDate (defaults today)
  // LinkExisting: candidateTransactionId (from unmatched txns matching the Plan's payee/account/amount window)
```

## Implementation notes (GPUI)
1. **HTML is a reference.** Translate to GPUI's element tree; do not port markup.
2. **Status is computed, not stored** for Upcoming/Due/Overdue — recompute from `dueDate` vs. today on every render/tick, per docs/bills.md. Only `Paid`/`Skipped` are persisted terminal states.
3. **No Schedule row is ever deleted** — deactivating or ending a Plan only stops future generation; existing rows (any status) are permanent and drive the History tab.
4. **"Mark paid" is never a bare boolean toggle** — always route through either transaction creation (seeded from the Plan) or transaction linking. This is an explicit ADR-0019 constraint, not just a UI preference.
5. **Known Costs (Bills) in Budget** (out of scope for this package, but this view is its data source): unpaid Due/Overdue Schedule rows matching a Budget's Category+Unit sum into that Budget's Known Costs figure for the period. Don't let a Paid row double-count once its linked Transaction lands in ordinary spend.
6. **Needs Attention** (⚑) is a derived boolean (`Overdue OR (Due AND withinAttentionLead)`), surfaced both here and on the Dashboard — compute it once, don't duplicate the rule.
7. **Tabular numerals** on every money/count figure so columns align.
8. **Zero radius, 2px section rules, 1px row rules** — consistent with the rest of the shell.
9. **Icons**: Lucide, 14×14 at rail size, 1.5px stroke, `fill: none`.

## Files
- `Ledger Desktop Shell.dc.html` — the prototype (section `#t8`, options 8a–8f)
- `README.md` — this document
- `screenshots/` — one PNG per screen (8a–8f)
