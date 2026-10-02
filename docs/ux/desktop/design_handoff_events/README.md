# Handoff: Events

## Overview
**Events** are notable moments on the ledger's timeline, and the balances you expect at them. Four kinds:

- **Milestone** — a single day that changes the money: a pay rise, a refinance.
- **Occasion** — a dated span with an optional spend target: a trip, a wedding, a renovation.
- **Incident** — a single day with consequences: a car accident, a burst pipe.
- **Balance check** — on this date, I expect this account to hold this much. Flagged ⚑ in red when the ledger disagrees.

Each event links the transactions and documents that belong to it, by tag (auto-link) or by hand. Linked transactions keep their own category; an event is a second way to total them.

Frames:
- **5a** Events list · **5b** Event detail · **5c** Edit event · **5d** New balance check · **5e** Delete event
- **3c** Events inline in Transactions

## About the design files
`Events.dc.html` is a **high-fidelity HTML design reference**, not production code. Recreate it in the target codebase (**Rust + GPUI**) using its existing patterns. Open it in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` beside it. Links to other destinations point to `../Ledger Desktop Shell.dc.html`.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_navigation/`. Only what Events depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Copy is final except sample data.

## Frame & shell
- Every frame is **1280 × 800**: header 48px, body `flex:1`, status bar 28px.
- **Header breadcrumb:** `ledger › events`, and `ledger › events › japan trip` on the detail.
- **Primary rail (206px):** Events is active (`g e`). It sits in LEDGER between Documents and Notifications:
  - LEDGER: Dashboard `g d` · Transactions `g l` · Documents `g f` · **Events `g e`** · Notifications `g m` (red badge)
  - NET WORTH: Cash `g c` · Inventory `g o` · Loans `g n` · Credit cards `g k` · Investments `g i`
  - PLAN: Bills `g w` · Budgets `g b` · Reports `g r`
  - Settings `g s`, pinned below a 2px rule
- **Rail icon:** Lucide-style flag, 14px, 1.5 stroke: `M3 14.5V1.5` + `M3 2h9.5l-2 3 2 3H3`.

## Layout (5a–5e)
| Column | Width | Spec |
| --- | --- | --- |
| Primary rail | 206px | Shell. |
| Events index rail | 190px | `border-right:2px solid rgba(32,30,29,.38)`, `padding:10px 0`, 12.5px. |
| Main pane | `flex:1; min-width:0` | List (5a) or detail (5b). |

### Events index rail (190px)
Same pattern as the Documents index rail. Rows `display:flex; padding:6px 14px`; label `flex:1`; count 11px #9b9797 tabular. Active row `background:#201e1d; color:#f3f2f2; 800`, count `#bab6b6`.

1. **All events · 24**
2. **Upcoming · 2**
3. `KIND` label (`800 10px/1 Archivo; .11em; #9b9797; padding:14px 14px 6px`): Milestones 5 · Occasions 4 · Incidents 2 · Balance checks 13
4. `YEAR` label: 2027 1 · 2026 17 · 2025 6

Facets are single-select. Picking a Kind or Year replaces the list scope; search narrows it further.

---

## 5a · Events list
**Purpose:** see every event on the timeline, and spot balance checks that disagree.

### Header
`padding:16px 28px 12px`, 2px bottom rule.
- **Left:** `h2` "Events" (26px/800, `margin:0 0 4px`), subline "24 events · 13 balance checks, 1 off" (11.5px #9b9797).
- **Right:** **new event** `n` (`.btn.btn-primary`, 32px; binding at `opacity:.75`).
- **Filter row** (`gap:8px`, `margin-top:14px`): chips `kind: all ▾` · `account: any ▾` · `all years ▾` (`.tag.tag-outline`, `padding:2px 8px`), spacer, then `.input` 28px × 200px, 12px, placeholder "/ search events".

### Table
- **Column header:** `padding:8px 28px; 800 10px/1 Archivo; .11em; #9b9797; 1px bottom rule #d7d3d3`.

  | Column | Width | Notes |
  | --- | --- | --- |
  | DATE | 112px | `14 mar 2027`; spans as `14–28 jun`. Year omitted inside its year group. |
  | KIND | 112px | #605d5d. |
  | EVENT | flex | Name 600, then `· detail` in #605d5d; one line, ellipsis. |
  | LINKED | 128px | `38 txns · 6 docs`, #9b9797; `—` when none. |
  | AMOUNT | 150px, right | See below. |
  | STATUS | 130px, `padding-left:20px` | See below. |

- **Group rows:** `padding:12px 28px 6px; 1px bottom rule #d7d3d3`, caps label "UPCOMING · 2", "2026 · 17", "2025 · 6". Upcoming comes first (soonest first); then years, newest first. Older years collapse to "6 events, collapsed ›".
- **Rows:** `padding:9px 28px; 1px bottom rule #eae9e9`, tabular.
- **Amount by kind:**
  - Occasion with target: `spent / target` ("6,840.20 / 7,500").
  - Balance check: the expected balance.
  - Incident: net linked spend ("−850.00").
  - Milestone: the change it records, if any ("+412.00 / fn"); otherwise `—`.
- **Status by kind:**
  - Occasion: "91% of target" (#605d5d).
  - Balance check: "✓ matches" (#605d5d) · **"⚑ off by −18.00"** (800 `#ae1800`) · "due at statement" for a future check.
  - Incident: a free status, e.g. **"claim open"** (800 ink).
  - Milestone: `—`.
- **Footer:** `padding:8px 28px; 1px top rule`, 11.5px #605d5d: "9 of 24 events shown" left; "balance checks: 12 match · 1 off" right.

Sample rows: Our wedding (14 mar 2027, upcoming) · ANZ Everyday check (31 oct 2026, upcoming) · Car accident (20 sep) · ANZ Everyday check, matches (13 sep) · Amex Platinum check, off by −18.00 (31 aug) · Pay rise (01 aug) · ANZ Offset check (30 jun) · Japan trip (14–28 jun) · Refinanced home loan (12 mar).

### Status bar
`NORMAL` · `j/k row · enter open · n new · e edit · d delete · / search` · right: "⚑ off = ledger disagrees with expected balance".

---

## 5b · Event detail (`enter`)
**Purpose:** what an occasion cost, and what belongs to it.

Main pane `padding:16px 28px`.
- **Back link:** "‹ events" (11.5px #605d5d, no underline, `margin-bottom:8px`). `esc` does the same.
- **Title row:** `h2` "Japan trip" (26px/800) + `.tag.tag-neutral` "occasion". Subline (11.5px #605d5d): "14 jun – 28 jun 2026 · 15 days · auto-links transactions tagged **japan-2026**, 3 more linked by hand".
- **Actions (right):** edit `e` (`.btn-secondary`, 32px), link `l` and delete `d` (`.btn-ghost`, 32px).
- **Figures strip:** 4 equal columns, `border-top` and `border-bottom` 2px strong rule; cells `padding:12px 14px`, 1px `#d7d3d3` dividers. Caps label, value `800 22px/1` tabular, note 11px #9b9797.
  - SPENT 6,840.20 "across 4 accounts" · TARGET 7,500.00 "set 02 mar" · LEFT 659.80 "9% unspent" · TRANSACTIONS 38 "35 by tag · 3 by hand"
  - **Progress bar** under the strip (`margin:14px 0 18px`, gap 12): 8px track `#d7d3d3`, fill `#201e1d` at spent / target, then "91% of target" (11.5px #605d5d). Over target, the fill turns `#ec3013` and LEFT `#ae1800`.
- **Two columns below:**
  - **LINKED TRANSACTIONS · 38** with "open in Transactions" on the right. Rows: date · account · payee · category · amount · `tag` / `hand` marker (11px #9b9797). "31 more ›".
  - **BY CATEGORY:** accommodation 2,410.00 · flights 2,180.00 · food 1,120.40 · transport 620.00 · shopping 509.80.
  - **DOCUMENTS · 6:** file name + date rows, "2 more ›". Each opens in Documents.
  - **NOTES:** free text, 12px #605d5d.
- **Status bar:** `NORMAL` · `esc back · e edit · l link transactions · u unlink · d delete` · right: "linked transactions keep their own category".

Hand-linked transactions exist because tags don't catch everything: the flights and JR Pass were booked months before the trip.

---

## 5c · Edit event (`e`)
Dialog over the dimmed 5a list.
- **Dimmer:** full window, `rgba(32,30,29,.30)`, z-index 10. Dialog top-anchored at 64px, centred.
- **Dialog:** 540px, `background:#f3f2f2; border:2px solid #201e1d; box-shadow:0 16px 48px rgba(32,30,29,.40)`.
- **Header:** `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30)`, "Edit event" 800 16px.
- **Body:** `padding:20px; gap:16px`. Labels 800 12px, `margin-bottom:6px`; fields `padding:8px 10px; border:1px solid rgba(32,30,29,.30); 13px`.
  - **Kind:** 4-option segmented control (Milestone · Occasion · Incident · Balance check); active `#201e1d` / `#f3f2f2` 800.
  - **Name**
  - **Starts** / **Ends (optional)** — two columns, gap 12. Ends only for Occasion.
  - **Spend target (optional)** with the unit suffix `aud` — Occasion only.
  - **Link transactions:** checkbox rows. "✓ Auto-link everything tagged **japan-2026** · 35 now"; "Auto-link everything on Wise travel between the dates"; note "3 linked by hand — manage them on the event".
  - **Notes:** multi-line.
- **Footer:** `padding:16px 20px; 1px top rule #d7d3d3`. "Delete event…" (text, `#ae1800`) left; **Cancel** (secondary) and **Save** (primary ink) right.

**Rules:** the kind decides the fields. Kind cannot change to or from Balance check (the record shape differs); those options are disabled when editing an existing event.

## 5d · New event (`n`), Balance check
Same dialog shell as 5c, titled "New event", with Balance check selected.
- **Account** (select) · **At end of** (date) · **Expected balance** + unit · **Source** (Statement / Bank app / Counted) · **Document (optional)** — a chip with the file name and ✕.
- **Live comparison note:** `padding:10px 12px; background:#eae9e9; border-left:2px solid #ec3013; 11.5px/1.5 #605d5d`. "Ledger balance at end of 30 sep: **9,172.15**. Expected **9,130.25** — off by **−41.90** (`#ae1800`). Saving records the check as ⚑ off and raises a notification; no transaction is changed." When they match, the note reads "✓ matches" with no red border.
- **Footer:** Cancel · **Save check**.

**Rules:** saving never posts an adjustment. A check is the same record as `u` (update balance) in Cash (7a) and the rows of the Balance Check Variance report (15e); explaining a mismatch happens there (15h).

## 5e · Delete event (`d`)
- Dialog 440px, top-anchored at 170px, same shell.
- Body: "**Japan trip** and its spend target will be removed." Then (12.5px #605d5d): "The 38 linked transactions and 6 documents stay in the ledger — only their link to this event goes."
- Checkbox (unchecked by default): "Also remove the tag **japan-2026** from 35 transactions".
- Footer: Cancel · **Delete event** (primary, accent).
- No typed confirmation — nothing is destroyed but the event itself (same weight as tag removal).

---

## 3c · Events inline in Transactions
The ledger scoped to one account (ANZ Everyday, September 2026). Events appear between transactions on their date.
- **Header subline:** "ANZ Everyday · 41 transactions and 2 events in september".
- **Filter chips:** the standard set plus **`events: all kinds ▾`**.
- **Event marker row:** `padding:7px 28px; background:#eae9e9; border-bottom:1px solid #d7d3d3`, tabular.
  - 20px flag icon column (12px, 1.6 stroke) · date 70px · caps kind label (`800 10px .11em #9b9797`) · name 800 · detail 12px #605d5d.
  - Right (226px, 12px #605d5d): "1 linked here · 3 in all · open ›".
- **Balance check row:** same treatment. Name reads "Expected 4,182.55", detail "from statement". Right column "**✓ matches**" (800 ink), or "⚑ off by …" (800 `#ae1800`). The BALANCE column shows the ledger's running balance at that point (800).
- **Linked transactions** show the event in TAGS ("car accident").
- **Footer:** "8 of 41 transactions · 2 events"; BALANCE 9,172.15.
- **Status bar:** `NORMAL` · `j/k row · enter open · e edit · E hide events · / filter`.
- Event rows are selectable with `j`/`k`; `enter` opens 5b.

---

## Interactions & behaviour
- **Auto-link:** an event with an auto-link tag (or an account + date window) picks up matching transactions as they are tagged or imported. Hand links are stored separately and survive tag changes.
- **Unlink (`u`)** on a linked transaction removes a hand link; for a tag-linked one it offers to remove the tag.
- **Balance check evaluation:** compare expected with the ledger balance at end of the date, in the account's own unit. Re-evaluate whenever a transaction on or before that date changes. A mismatch raises a needs-action notification (Notifications 6a); it clears when the check matches again.
- **Upcoming:** events dated after today. A future balance check shows "due at statement" until its date passes.
- **Delete** keeps transactions and documents; optionally removes the auto-link tag.
- **Empty states:** no events — "No events yet" + **new event**. Facet with 0 — a one-line note.

## Keyboard
| Key | Action |
| --- | --- |
| `g e` | Open Events. |
| `j` / `k` | Next / previous row. |
| `enter` | Open event detail. |
| `esc` | Back to list / close dialog. |
| `n` | New event. |
| `e` | Edit event. |
| `d` | Delete event. |
| `l` | Link transactions (picker). |
| `u` | Unlink selected transaction (detail). |
| `/` | Search. |
| `E` | Hide / show event rows (Transactions 3c). |

Palette: `:events`, `:event new`, `:check <account> <amount>`.

## State
```
events
  scope: All | Upcoming | Kind(EventKind) | Year(i32)
  query: String
  selectedId: Option<EventId>
  dialog: None | Edit(EventId) | New(EventKind) | Delete(EventId)

Event { id, kind: EventKind, name, detail?, starts: Date, ends?: Date,
        target?: Money, status?: String, notes?: String,
        autoLink?: { tag?: TagId, account?: AccountId },
        handLinks: Vec<TransactionId>, documents: Vec<DocumentId> }
EventKind = Milestone | Occasion | Incident | BalanceCheck
BalanceCheck { account: AccountId, at: Date, expected: Money,
               source: Statement | BankApp | Counted, document?: DocumentId }
```
- Spent, linked counts, match / off status and "Upcoming" are derived; don't store them.
- `ends` and `target` exist only for Occasion; BalanceCheck carries its own shape (hence no kind change to/from it).
- Balance checks are shared with Cash (`u`) and Reports (15e, 15h).

## Design tokens
- **Colour:** ground `#f3f2f2` · chrome / marker rows `#eae9e9` · ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · accent `#ec3013` · text-safe accent `#ae1800` (⚑ off, over target) · rules: strong `rgba(32,30,29,.38)` 2px, medium `#d7d3d3`, row `#eae9e9`, border `rgba(32,30,29,.30)` · dimmer `rgba(32,30,29,.30)`.
- **Type:** Archivo 400/600/800. Sizes 10 / 11 / 11.5 / 12 / 12.5 / 13 / 16 / 22 / 26px. Caps labels `.11em`. Tabular numerals on every date and amount.
- **Radius:** 0 everywhere. **Shadow:** dialogs only, `0 16px 48px rgba(32,30,29,.40)`.
- **Components:** `.btn` (primary / secondary / ghost), `.tag` (outline / neutral), `.input` from Modernist.

## Files
- `Events.dc.html` — design reference: 5a–5e and 3c.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — viewer runtime.
- `README.md` — this document.

Combined Documents + Events explorations (Records, Timeline, Events as folders) are experiments, not part of this handoff — see `Personal Ledger - Desktop Experiments.dc.html`.
