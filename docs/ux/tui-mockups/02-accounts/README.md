# Handoff: Personal Ledger TUI — accounts

## Overview
The accounts view for `personal-ledger` — a keyboard-only, local-first personal finance ledger (single user, SQLite, no auth, no cloud). It re-hosts inside the shell specified in `design_handoff_ledger_shell` and follows the same conventions (single full-bleed view region, `:` palette, footer hint bar, centred form overlays over a dimmed view).

**The model this design encodes** comes straight from the repo (`crates/libs/lib-database/src/accounts/`, `crates/libs/lib-core/src/account_type.rs`, `migrations/client/20260905000000_create_accounts_table.sql`):

- An **Account is where money sits** (Category is *what it was for*). Five fixed types: cash, bank, credit card, investment, loan.
- **Exactly one Unit, fixed at creation** (`unit_id` FK, FR.10). The ledger does not convert between Units in V1, so no total spanning Units is ever printed.
- **Balance is computed on read**, never stored: `starting_balance + Σ transactions` (`Accounts::balance`), summed in Rust `BigDecimal` because `Money` is arbitrary-precision TEXT. `balance_as_of(date)` is the same computation pinned to a date, used by the Balance-check variance report (FR.38).
- `is_active` is a **soft-delete flag**, not a delete.
- Delete today is a bare `DELETE` (`accounts/delete.rs`). FR.14 asks for "transfer all the transactions under an account to another account" on delete, deferred to issue #70. **`7d` designs that transfer as the primary path** — implement it when `transactions` lands, and until then the delete overlay should refuse on a non-empty account rather than orphaning rows.

Four states:

| id | state |
| --- | --- |
| `7a` | **Accounts screen** — accounts grouped by type, per-unit subtotals, the selected account's summary, its balance line and its ledger |
| `7b` | **New** — type as a five-way pick, unit chosen once and fixed forever |
| `7c` | **Edit** — name / type / active only; unit and starting balance shown read-only with the reason |
| `7d` | **Delete** — transactions moved to another same-unit account first, name typed to confirm |

## About the design files
`Ledger TUI Accounts.dc.html` (open in a browser; `support.js` must sit beside it) is a **design reference drawn in HTML**. It is not code to port. HTML was only a fast way to draw a character-grid interface — the target is **Rust + [ratatui](https://ratatui.rs) + crossterm**, in the repo's existing `bin-tui` screen/action architecture (`screen/accounts_list.rs`, `screen/account_detail.rs`, `Action::OpenAccounts` / `OpenAccountDetail` / `AccountsLoaded` / `AccountSaved` / `AccountDeleted`, `AccountBalancesLoaded`).

Every px value in the HTML is an artifact of drawing. Read the geometry in **terminal cells** only.

## Fidelity
**Low-to-mid fidelity.** Authoritative about the account model (five types, one fixed unit, computed balance, active vs delete, transfer-on-delete), pane structure, keybindings, the command grammar, and what each element implies as a ratatui widget. Not authoritative about exact colors, borders or padding — apply the repo's own theme module. All names, amounts and dates in the mock are fake.

---

## Changes from what’s implemented today

`crates/bins/bin-tui/src/screen/accounts_list.rs` and `account_detail.rs` already exist and are tested. This design **changes four things about them** — each is a deliberate departure, and each breaks or moves an existing test. Nothing here is an accident; if a change looks unnecessary, keep the current behaviour rather than splitting the difference.

| today | this design | why | test affected |
| --- | --- | --- | --- |
| `enter` → `OpenAccountDetail(Some(..))` (opens the edit form) | `enter` opens the **account ledger**; edit moves to `e` | Once transactions exist, opening the ledger is the overwhelmingly common intent, and `e` for edit matches units and categories | `enter_opens_an_edit_form_for_the_selected_account` — rewrite as `e_opens_an_edit_form…`, add an `enter` → open-ledger case |
| `d` arms `pending_delete`, a footer line takes a single `y` | `d` opens the `7d` overlay: transfer target, preview, name typed to confirm | A single `y` is fine for a row with nothing attached; it is not fine for one holding 1 284 transactions and 8 balance checks | `d_arms_delete_confirmation_and_a_non_y_key_cancels_it` — keep the arming concept, replace the confirm |
| Flat `Table` of every account, sorted by name, columns Name / Type / Starting Balance / Active | Grouped by `AccountType`, per-group subtotals, `UNIT` and computed `BALANCE` columns | Grouping is what makes the per-unit totalling rule legible; a flat name sort hides it | `renders_loaded_without_panicking` still passes; add group-header and mixed-unit cases |
| Column header reads `Starting Balance` — deliberately, because computed Balance isn’t honest until transactions exist | Column reads `BALANCE` and shows `starting_balance + Σ txns` | This is the **target state, not today’s**. Until [#70](https://github.com/IanTeda/Personal-Ledger/issues/70) lands the two are numerically identical — keep the honest `Starting Balance` header and switch it with #70, not before | none |

Two things this design explicitly does **not** change: `unit` and `starting_balance` stay fixed after creation (FR.13, `EDIT_FIELDS` vs `CREATE_FIELDS` — an earlier draft of `7c` made the starting balance editable and that was wrong), and account names stay non-unique.

`7d`’s transfer is FR.14’s deferred requirement ([#70](https://github.com/IanTeda/Personal-Ledger/issues/70)). Until `transactions` exists there is nothing to transfer: **refuse delete on a non-empty account** and ship the overlay’s confirm half only.

---

## Terminal geometry

Drawn at **96 × 30 cells** (one cell ≈ 6.6 × 16.5 px in the HTML), inside the shell's view region.

```
row 0        status line     "ledger · accounts / everyday spending"
                             right: "9 accounts · 7 active · base AUD"
rows 1..n-2  two panes       left 41 cols fixed · right Min(0)
row n-1      keybind hints   j/k · enter ledger · n new · e edit · d delete
                             · a off · b check · tab focus  (one row, always)
```

`Layout::horizontal([Constraint::Length(41), Constraint::Min(0)])`. Each pane is a stack of `Constraint::Length` blocks over a `Min(0)` list — no fixed heights on anything holding a list.

Degrade below 96 columns: drop the ledger's `BALANCE` column → drop the `UNIT` column (it moves into the summary only) → drop the balance graph → collapse the left pane to the list alone, with the summary reachable on `enter`.

---

## 7a — Accounts screen

### Left pane (41 cols) — the list, then the summary

Three columns: name (Min(0)), `UNIT` (4), `BALANCE` (10, right, tabular).

```
CASH                  2        320.40
├ Wallet            AUD        320.40
└ Travel cash       USD      1 500.00
BANK                  2     28 640.15
├ Everyday Spending AUD      4 210.65     ← selected: full-width reversed block
└ Mortgage Offset   AUD     24 429.50
CREDIT CARD           1     -1 284.30
└ Amex Platinum     AUD     -1 284.30
INVESTMENT            2    mixed units
├ Vanguard VDHG    VDHG      412.4800
└ Cold wallet       BTC    0.18400000
LOAN                  1   -612 400.00
└ Home Loan         AUD   -612 400.00
```

- **Grouped by the five `AccountType` variants, in the enum's own order** (cash, bank, credit card, investment, loan). Type headers are bold; empty types are omitted entirely.
- **The type header's right column is a subtotal only when every account in the group shares the base unit.** Mixed groups print `mixed units` — never a summed number. This is the whole reason the layout groups rather than sorting: a column of numbers in four units invites addition that the domain forbids.
- The `UNIT` column on a type header is repurposed as the **group count**.
- **Amounts render at their unit's own precision** (`412.4800` for a fund, `0.18400000` for BTC, 2 dp for currency) — take the precision from `units`, don't hardcode 2.
- Negative balances (credit cards, loans) are **accent plus a minus sign**, never colour alone.
- **Inactive accounts are hidden** by default, shown dim with a trailing `· inactive` under `za`. The status line's "9 accounts · 7 active" is how the user knows two are hidden.

**Summary box** — directly under the list, the selected account's record. These are exactly the fields `7c` exposes, plus the computed ones:

```
bank · AUD · start 1 000.00
──────────────────────────────
balance now · computed   4 210.65
transactions             1 284 · 12 open
last check               31 aug · var 0.00
active                   [×] · offered when posting
```

`balance now` is labelled **computed** because it is: nothing in the row it came from stores it. `last check` is the most recent Balance Check with its variance against `balance_as_of(check.date)` — zero variance is worth printing, since "checked and matched" is the state the user wants to see. The unit and the starting balance are not repeated as their own rows — the box's first line carries the three facts that never change (type, unit, starting balance), and the rows below carry the four that do. The box is capped at five lines; anything more and it pushes the account list off the pane.

### Right pane — the shape, then the evidence

**Balance line** — month-end balance, 24 points, `oct 24 – sep 26`, in the account's own unit. `Chart` + `Dataset` with `GraphType::Line`, `Marker::Braille`; a `Sparkline` row if braille is unavailable. Labels beneath: first point, low with its month, last point. Month-end balances come from repeated `balance_as_of` — compute the series in one pass over the account's transactions rather than N queries.

**Ledger list** — `10 of 1 284 · newest first`, five columns: status glyph (1), `DATE` (6), `PAYEE` (Min(0)), `AMOUNT` (9, right, tabular, signed), `BALANCE` (10, right, running). Selected row is a full-width reversed block; the 1-col scrollbar sits on the right edge when the list overflows.

- **Status is a glyph, not a colour**: `○` open, `✓` reconciled — legend in the footer row alongside the open count.
- The running `BALANCE` column runs **newest-first downward**, i.e. each row shows the balance *after* that transaction. It is only meaningful when the list is sorted by date descending with no filter applied — **blank the column whenever a filter or a non-date sort is active** rather than printing a running total of an arbitrary subset.

**Footer rows** — `net AUD 84 210.15 · USD, VDHG, BTC held separately`, then the command hints `:acct new <name> <type> <unit>` and `:acct delete <name>`. The net line states the base unit explicitly and names the units it excluded; it never silently omits them.

---

## 7b — New (`:acct new <name> <type> <unit>`)

`n` from the list. Centred floating overlay (`Clear` + bordered `Block`, ~88% width) over the dimmed list; the view behind keeps its greyed `esc close new form` hint row.

| field | rule |
| --- | --- |
| `name` | required, free text, **not unique** — ids are (`RowID`, UUIDv7). Don't add a uniqueness check the schema doesn't have |
| `type` | five-way inline pick, focused by default, `h`/`l` or first letter; `cash` is the enum's `Default` |
| `unit` | required, completes on `units.code`, **fixed at creation** |
| `starting bal` | `Money` at the chosen unit's precision, defaults `0.00`, may be negative — **the only chance to set it** (FR.13) |
| `active` | `[×]` by default — "offer it when posting" |

The note block states the two things a user can only learn by being told: **unit cannot change afterwards** (to hold a second unit, make a second account), and **balance is never stored** — it is the starting balance plus every transaction, on read.

Keys: `tab` next field · `^s` create · `^a` create and start another · `esc` cancel.

## 7c — Edit (`:acct edit <name>`)

`e` on a row. **Editable: `name`, `type`, `is_active` — and nothing else.** FR.13 fixes both `unit` and `starting_balance` at creation, which `account_detail.rs` already implements as a narrower `EDIT_FIELDS` cycle than `CREATE_FIELDS`. Keep that split.

The form is three sections, and the two read-only ones are **shown, not hidden** — a user who came here to fix an opening balance needs to see it and be told why they can't:

```
name          Everyday Spending▌
type          cash [bank] credit card loan
active        [×] · clear to deactivate

fixed at creation                        FR.13
unit          AUD
starting bal  1 000.00

computed
balance now   4 210.65 · not stored
transactions  1 284 · enter ledger
created       14 oct 2024 · upd 08 sep 26

name and type are labels — nothing derives
from them, so renaming is always safe.
correct a wrong opening balance with an        ← accent
adjusting transaction, not by rewriting it     ← accent
```

The closing block does the real work: it answers "then how do I fix it?" in the same breath as the refusal. An **adjusting transaction** is the correct remedy — it is dated, visible in the ledger, and leaves every Balance Check that was true at the time still true. Rewriting the starting balance would silently shift every historical balance and put past checks into variance, which is exactly why FR.13 forbids it.

Keys: `tab` next field · `^s` save · `^a` deactivate · `^d` delete · `esc` cancel.

## 7d — Delete (`:acct delete <name>`)

`d` on a row, or `^d` from the edit form. **The only irreversible operation in this view**, so it does three things before it will run: it says what the account holds, it makes the user choose where the transactions go, and it takes the name as the confirm.

```
holds         1 284 txns · 4 210.65 AUD
              12 unreconciled · 8 balance checks

transactions  ( ) delete them too · refused          ← accent on "refused"
              (•) move to another account
┌ move to     Mortgage Offset▌                        ← focused
└ candidates  AUD only · 4 of 9  · tab

after
into          Mortgage Offset
its balance   24 429.50 → 28 640.15
moves         1 284 txns · status kept
loses             8 balance checks · they assert a
                  balance this account no longer has

confirm       Everyday Spending▌
deletion is permanent and not synced back            ← accent
a deactivate instead — keeps everything readable
```

- **"delete them too" is present but refused.** Showing the refused option is deliberate: it answers the question the user is about to ask instead of leaving them to guess why there's only one radio.
- **Candidates are filtered to the same unit** (`AUD only · 4 of 9`) — Transactions never cross Units, so a cross-unit target is not a warning, it is not offered. Inactive accounts and the account being deleted are excluded too.
- **`after` previews the receiving account's new balance** as `old → new`. This is the check the user actually performs. The target's **name sits in the value column, never as a label** — an account name is unbounded and a fixed label column will clip it, so `after` takes two rows (`into`, then `its balance`) rather than one.
- **Balance Checks on the deleted account are deleted with it**, and the overlay says why: a check asserts a balance for *that* account on a date, and there is no honest way to reattach it to another one. They are the one thing that does not survive.
- **The confirm is the account's name typed out**, matching `3e`'s unit-delete pattern — same muscle memory across the app.
- `a` deactivates instead, from inside the delete overlay, because that is what most users reaching for delete actually want.

Order of operations on submit, in one transaction: `UPDATE transactions SET account_id = :target WHERE account_id = :source` → `DELETE FROM balance_checks WHERE account_id = :source` → `DELETE FROM accounts WHERE id = :source`. With an empty account, the transfer field is skipped entirely and the overlay is just the confirm.

Keys: `tab` next field · `^s` delete · `a` deactivate · `esc` cancel.

---

## Command grammar

Extends the shell's registry (noun-first, tab-completable at every position; `<acct>` completes on name, `<unit>` on unit code):

```
acct                                  opens 7a
acct new    <name> <type> <unit>      starting balance defaults 0
acct edit   <acct>                    opens 7c
acct delete <acct> [into <acct>]      opens 7d prefilled; `into` must share the unit
acct off    <acct>                    is_active = 0
acct on     <acct>                    is_active = 1
acct check  <acct> <amount> [date]    records a Balance Check, prints the variance
```

## Interactions & behaviour
- **Modal**: `NORMAL` in the list, `INSERT` inside a form overlay. Overlays dim the view behind and grey the app command line, exactly like the shell's palette.
- **Motion**: `j/k` row (skipping type headers) · `g/G` top/bottom · `tab` list↔ledger focus · `za` show inactive · `/` filters by name across all types.
- **Operations**: `n` new · `e` edit · `d` delete · `a` deactivate (`off` in the hint bar) · `b` balance check · `enter` opens the account ledger (not designed yet — see below).
- Redraw is event-driven. A save emits `AccountSaved`, a delete `AccountDeleted`, and the list reloads via `AccountsLoaded` + `AccountBalancesLoaded` — the same balance query the dashboard's account snapshot uses, so compute it once.
- Balances are the expensive part of this screen: `AccountBalancesLoaded` carries `Vec<(RowID, Money)>` for every account at once. Don't compute per row during render.
- No mouse support required.

## State
List: accounts ordered by type then name · per-account computed balances · group subtotals with a same-unit flag · selection · show-inactive flag · filter string. Selection detail: the `Accounts` row, its transaction count and first posting, its latest Balance Check and that check's variance, its unreconciled count and sum. Right pane: the month-end balance series · the ledger page (offset, running balance validity flag) · scroll offsets. Forms: field drafts, resolved `unit_id` and target `account_id`, per-field validation, and for delete the same-unit candidate list and the typed confirm string.

## Style
The wireframe uses ink `#201e1d` on ground `#f3f2f2`, accent `#ec3013`, greys `#605d5d` / `#9b9797` / `#d7d3d3`, dark variant on `#161413`. Map to **theme roles, not literal RGB** (same table as the shell handoff):

| role | use here | ANSI |
| --- | --- | --- |
| accent | negative balances, the refused option, the variance warning, the cursor | red |
| dim | column heads, read-only fields, notes, inactive accounts | dark grey / `DIM` |
| selection | current account, current ledger row | reversed |
| header bar | status line, the chosen `type` chip | reversed |

Never rely on colour alone: negatives carry a minus sign, transaction status is a glyph (`○`/`✓`), refusals are stated in words.

## Not yet designed
- **The account ledger** behind `enter` — the full transaction list with filters, jump-to-date and status stepping (sketched early as `6e`).
- **Balance check / reconcile** (`b`) — asserted vs computed, variance, and how a check is superseded (sketched early as `6i`).
- **Transfers between accounts** — the `Transfer → Offset` row in `7a` is a placeholder payee; the two-sided transfer is not modelled yet.
- **Institution** — deliberately not an entity in V1; accounts carry the bank's name in their own name.

## Files
- `Ledger TUI Accounts.dc.html` — turn 7: `7a` screen, `7b` new, `7c` edit, `7d` delete. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- `CLAUDE_CODE.md` — the staged work plan: what to build in what order, which files each stage touches, and the acceptance criteria per stage. This README is the specification; that file is the order of work.
- Companion bundles: `design_handoff_ledger_shell` (the shell, palette and action registry this view sits in), `design_handoff_ledger_units` (units and prices — an account's unit comes from there), `design_handoff_ledger_categories` (the other half of every transaction), `design_handoff_ledger_settings` (`general.base_unit` is what `7a`'s net line is stated in).

## Suggested Claude Code prompt
> Read `docs/ux/accounts/README.md` — start with its "Changes from what's implemented today" table, which lists the keybinding and column changes and the tests each one breaks — and open `docs/ux/accounts/Ledger TUI Accounts.dc.html` in a browser for reference. Build, in this order: (1) the accounts list screen — grouped by `AccountType` in enum order, per-group subtotals only when the group is single-unit (`mixed units` otherwise), amounts at each unit's own precision, inactive hidden behind `za`; (2) the summary box and the month-end balance series, both fed by `AccountBalancesLoaded` and `balance_as_of` rather than per-row queries; (3) the ledger list with its status glyphs and running balance, blanking the balance column whenever a filter or non-date sort is active; (4) the new and edit forms — keep `account_detail.rs`'s existing `CREATE_FIELDS`/`EDIT_FIELDS` split exactly (FR.13: unit and starting balance are create-only), showing both read-only in edit with the "use an adjusting transaction" remedy rather than hiding them; (5) the delete overlay last — same-unit-only transfer target, the receiving account's `old → new` preview, balance checks stated as lost, name typed to confirm, and the whole thing in one transaction. Until `transactions` exists (issue #70), refuse delete on a non-empty account instead of orphaning rows. Follow the existing `bin-tui` screen/action architecture and use theme roles rather than the hex values.
