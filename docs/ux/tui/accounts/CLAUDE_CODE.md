# Claude Code: accounts view

Work plan for implementing the accounts design in `IanTeda/Personal-Ledger`. The **specification is `README.md`** in this folder — this file is only the order of work, the files each stage touches, and what "done" means for each. Don't implement from this file alone; each stage points at the README section that governs it.

## Before you start

1. Read `README.md` end to end, starting with **"Changes from what's implemented today"**. Four behaviours in this design deliberately depart from working, tested code, and that table names the test each one breaks. If a change there looks unnecessary, keep the current behaviour — don't split the difference.
2. Open `Ledger TUI Accounts.dc.html` in a browser (`support.js` must sit beside it). It is a **design reference drawn in HTML**, not code to port. The target is Rust + ratatui + crossterm in the existing `bin-tui` screen/action architecture. Read all geometry in **terminal cells**; every px value is an artifact of drawing.
3. Confirm whether `transactions` exists yet ([#70](https://github.com/IanTeda/Personal-Ledger/issues/70)). Several stages change shape depending on the answer, and each one below says how.

Copy this folder to `docs/ux/accounts/` so the paths in the README's suggested prompt resolve.

## Stage 1 — Group the list

**Files** `crates/bins/bin-tui/src/screen/accounts_list.rs`
**Spec** README § *7a → Left pane*

Replace the flat name-sorted `Table` with a grouped one: `AccountType` in the enum's own declared order, bold type headers, empty types omitted, inactive hidden behind `za`. Columns `name / UNIT(4) / BALANCE(10, right, tabular)`.

Done when: a group subtotals **only** if every account in it shares the base unit and prints `mixed units` otherwise; amounts render at each unit's own precision from `units`, not a hardcoded 2 dp; negatives carry a minus sign as well as the accent; the type header's `UNIT` cell shows the group count; `j/k` skips header rows.

Keep the column header reading `Starting Balance` until #70 lands — until then the two numbers are identical and the honest header is the current one. Existing `renders_loaded_without_panicking` should still pass; add group-header and mixed-unit cases.

## Stage 2 — Summary box and balance series

**Files** `accounts_list.rs`, `crates/libs/lib-database/src/accounts/balance.rs`
**Spec** README § *7a → Summary box*, § *Right pane → Balance line*

Five lines, no more — the box competes with the list for the same pane. First line carries the three facts that never change (`bank · AUD · start 1 000.00`); the four rows below carry the ones that do.

Done when: `balance now` is labelled **computed**; balances for the whole list arrive in one `AccountBalancesLoaded` carrying `Vec<(RowID, Money)>` rather than a query per row; the month-end series comes from **one pass** over the account's transactions, not N `balance_as_of` calls; `last check` shows the latest Balance Check with its variance, and prints a zero variance rather than hiding it.

Without #70: the series is a flat line at the starting balance. Ship the widget, skip the sparkline.

## Stage 3 — Ledger list

**Files** new `crates/bins/bin-tui/src/screen/account_ledger.rs`, `action.rs`
**Spec** README § *7a → Ledger list*

Five columns: status glyph(1) `DATE`(6) `PAYEE`(Min 0) `AMOUNT`(9, right, signed) `BALANCE`(10, right, running). `enter` on an account opens this; `tab` moves focus between list and ledger.

Done when: status is a **glyph** (`○` open, `✓` reconciled) with its legend in the footer, never colour alone; the running `BALANCE` column is **blanked whenever a filter or a non-date sort is active** — it is only meaningful newest-first unfiltered; the footer net line names its base unit and names the units it excluded (`net AUD 84 210.15 · USD, VDHG, BTC held separately`).

This stage is blocked on #70. Until then `enter` can open the edit form as it does today — but move edit onto `e` in stage 4 regardless, so the keybinding doesn't have to change twice.

## Stage 4 — New and edit forms

**Files** `crates/bins/bin-tui/src/screen/account_detail.rs`
**Spec** README § *7b*, § *7c*

Centred overlay (`Clear` + bordered `Block`, ~88% width) over the dimmed list, `INSERT` in the status line — the same pattern as the units and categories forms.

Done when: **`CREATE_FIELDS` / `EDIT_FIELDS` keep their existing split exactly** — FR.13 fixes `unit` and `starting_balance` at creation, and edit is name / type / active only; both fixed fields are **shown read-only under a "fixed at creation · FR.13" heading**, not hidden, with the closing note giving the remedy (*correct a wrong opening balance with an adjusting transaction, not by rewriting it*); no uniqueness check on `name` (the schema has none — ids are `RowID`/UUIDv7); `type` is a five-way inline pick defaulting to the enum's `Default`; `^s` saves, `^a` deactivates, `esc` cancels.

Rewrite `enter_opens_an_edit_form_for_the_selected_account` as `e_opens_an_edit_form…`.

## Stage 5 — Delete overlay

**Files** `account_detail.rs` or a new `account_delete.rs`, `crates/libs/lib-database/src/accounts/delete.rs`
**Spec** README § *7d*

The only irreversible operation in this view. It states what the account holds, makes the user choose where the transactions go, and takes the **account name typed out** as the confirm — matching the unit-delete pattern so the muscle memory is shared.

Done when: transfer candidates are filtered to the **same unit** (a cross-unit target is not offered, not warned about), excluding inactive accounts and the account being deleted; *delete them too* is **shown and refused**, because that answers the question the user is about to ask; `after` previews the receiving account as `into` / `its balance  old → new`, with the name in the value column, never as a label; balance checks are stated as **lost**, with the reason (a check asserts a balance for *that* account on a date); `a` deactivates from inside the overlay.

One transaction, in order: `UPDATE transactions SET account_id = :target WHERE account_id = :source` → `DELETE FROM balance_checks WHERE account_id = :source` → `DELETE FROM accounts WHERE id = :source`. On an empty account the transfer field is skipped and the overlay is just the confirm.

**Until #70 exists there is nothing to transfer: refuse delete on a non-empty account rather than orphaning rows**, and ship the confirm half only. Today's `delete.rs` is a bare `DELETE` — don't leave it that way once transactions land. `d_arms_delete_confirmation_and_a_non_y_key_cancels_it` keeps its arming concept but replaces the single-`y` confirm.

## Stage 6 — Command grammar

**Files** the shell's command registry (see `design_handoff_ledger_shell`)
**Spec** README § *Command grammar*

`acct` · `acct new <name> <type> <unit>` · `acct edit <acct>` · `acct delete <acct> [into <acct>]` · `acct off|on <acct>` · `acct check <acct> <amount> [date]`. Noun-first, tab-completable at every position; `into` must share the unit. Each form's typed equivalent should reach the same code path as the keybinding, not a parallel one.

## Throughout

- **Theme roles, not hex.** The README's style table maps the wireframe's colours to roles (accent / dim / selection / header bar). Use the repo's own theme module.
- **Never colour alone** — negatives carry a minus sign, status is a glyph, refusals are stated in words.
- **Event-driven redraw.** A save emits `AccountSaved`, a delete `AccountDeleted`; the list reloads via `AccountsLoaded` + `AccountBalancesLoaded`. Never compute a balance during render.
- **Cells clip, they don't wrap.** Row heights are fixed; a too-long name truncates.
- Degrade below 96 columns in this order: drop the ledger's `BALANCE` column → drop `UNIT` (it stays in the summary) → drop the balance graph → collapse to the list alone with the summary on `enter`.
- No mouse support required.

## Out of scope

The account ledger's filters and jump-to-date, balance check / reconcile (`b`), two-sided transfers, and any notion of Institution (deliberately not an entity in V1 — accounts carry the bank's name in their own name). See README § *Not yet designed*.
