repo: IanTeda/Personal-Ledger
branch: main
path: docs/

## Last sync

date: 2026-09-26T05:29:47Z

### Updated in this project

- Added Turn 10 (Reports) to `Ledger Desktop Shell.dc.html`, built from `crates/bins/bin-tui/src/screen/reports.rs` and PRD FR.34–38: one screen, five report kinds as tabs; Category/Payee share one Unit-or-Account scope + date range.
- 10a Category Total, 10b Payee Total, 10c Account Balance (per-Unit totals, no cross-Unit sum), 10d Budget vs Actual, 10e Balance Check Variance (full audit trail, sorted by |variance|).
- Pop-ups: 10f date range picker (AU financial-year presets), 10g category drill-down, 10h explain-a-variance, 10i export.

## Sync history


date: 2026-09-20T03:16:34Z

- Rebuilt Turn 8 (Bills) in `Ledger Desktop Shell.dc.html` against `docs/bills.md` (concept branch): split Bill Plan (recurring definition) from Bill Schedule (dated, persisted rows), added a Schedule/Planner/History tab switcher.
- 8a Schedule: derived status (Upcoming/Due/Overdue/Paid/Skipped), ⚑ Needs Attention flag (Overdue + Due-within-Attention-Lead), pay/skip row actions.
- 8b Planner: Bill Plan fields (Category, Unit, Account, Payee, Planned Amount, Fixed/Estimated, Recurrence, Ends On, Attention Lead, Active) incl. a deactivated example.
- 8d: two-path "Pay" modal (pay directly vs link an existing transaction, per ADR-0019's real-Transaction-only rule). 8e: Skip (excluded from history average, unlike a $0 payment).
- 8f: Bill History filtered list with last-paid/average/same-period-last-year stats.


date: 2026-09-14T18:42:38Z

- Updated 1a mockups across all three copies (root, docs/ux/desktop/, design_handoff_shell_navigation/) to reflect spec changes:
  - Removed Reconcile and Units from the primary rail (both closed nouns).
  - Changed Accounts badge from 14 to 7 to match the spec example.
  - Rewrote "NEEDS ATTENTION" text from "14 unreconciled on ANZ Everyday" to "14 unreconciled across accounts" (aggregated, not account-specific).
  - Added `flex:1` spacer in NEEDS ATTENTION to align it with the Settings divider line (acceptance criterion 6).

date: 2026-09-13T05:27:54Z

- Read the PRD, `CONTEXT.md` glossary, ADR-0007 (gpui) and ADR-0013 (Shell/View nav) as the brief for the desktop mockups.
- Built `Ledger Desktop Shell.dc.html` — four navigation/IA options for the gpui desktop shell at 1280×800, GNOME chrome, light theme.
- Carried the TUI handoff's vocabulary across: mode line, `g`-jumps, `:` palette, status glyphs (○ ◐ ● ⚑), budget track + period marker.

## Screen map

| Screen / option | Built from |
| --- | --- |
| 1a Dashboard, two rails expanded | docs/ux/mockups/README.md (dashboard priority order), docs/product-requirements.md FR.34–38 |
| 1b Account ledger, list–detail rail | CONTEXT.md (Account, Transaction Status, Flagged), FR.16–20 |
| 1c Settings, collapsed icon rail | CONTEXT.md (Preference vs Configuration), CC-DESKTOP-001 |
| 1d Command palette over the shell | docs/ux/mockups/README.md §3a, action registry + command grammar |
| docs/ux/desktop/README.md (handoff) | `Ledger Desktop Shell.dc.html` option 1a, ADR-0007, ADR-0013, docs/ux/mockups/README.md |
| docs/ux/desktop/ (folder shape) | docs/ux/mockups/ — README + mockup + support.js, self-contained |
| 10a–10i Reports | crates/bins/bin-tui/src/screen/reports.rs, docs/product-requirements.md FR.34–38, docs/ux/mockups/README.md (`:report` grammar) |
| 8a–8f Bills (Schedule/Planner/History) | docs/bills.md, ADR-0019 (Bill Schedule as persisted rows linked to a Transaction) |
