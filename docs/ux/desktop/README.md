# Personal Ledger — Claude Code handoff (complete set)

High-fidelity design references for the Personal Ledger desktop app, to be rebuilt in **Rust + GPUI**. Each package holds one destination: a design file (`*.dc.html`, open it in a browser) and a `README.md` spec. Every design file is taken from the current master, `Ledger Desktop Shell.dc.html`, so the frame numbers here match the master.

Generated 3 October 2026; per-view frames re-synced to the new master on 9 October 2026 (header back / forward removed, 1b is the Dashboard, Settings gains Backup, History, Logs and dialogs 16r–16t). This set replaces all earlier `design_handoff_*` folders.

## How to use
1. Read this file for the shell, the rail and the tokens that every destination shares.
2. Build `01-shell` first, then the destinations in any order. Each README covers its frames, interactions, keyboard, state and GPUI notes.
3. Open a package's `.dc.html` from inside its folder. It loads `../support.js`, `../styles.css` and `../_ds_bundle.js`. Links to frames in other packages open the sibling folder.
4. These are design references, not production code. Copy the colours, spacing, type and behaviour; don't port the HTML.

## Packages
| Folder | Destination | Frames | Rail / key |
| --- | --- | --- | --- |
| `../Logo` | App icon & wordmark (shared by Desktop, TUI and Sync Server) | light / dark assets | — |
| `01-shell` | Shell & navigation | 1a–1f | header, rail, status bar, palette |
| `02-dashboard` | Dashboard: Today, Net worth, This month | 2a–2d | LEDGER · `g d` |
| `03-transactions` | Transactions: cross-account ledger | 3a–3c | LEDGER · `g l` |
| `04-documents` | Documents library (pick 4c/4d **or** 4e) | 4c–4e | LEDGER · `g f` |
| `05-events` | Events & balance checks | 5a–5e | LEDGER · `g e` |
| `06-notifications` | Notifications inbox & rules | 6a–6b | LEDGER · `g m` |
| `07-cash` | Cash | 7a–7c | NET WORTH · `g c` |
| `08-inventory` | Inventory (home contents) | 8a–8b | NET WORTH · `g o` |
| `09-loans` | Loans | 9a–9c | NET WORTH · `g n` |
| `10-credit-cards` | Credit cards | 10a–10b | NET WORTH · `g k` |
| `11-investments` | Investments | 11a–11b | NET WORTH · `g i` |
| `12-bills` | Bills: Schedule, Planner, Pay, Skip | 12a–12e | PLAN · `g w` |
| `13-budgets` | Budgets v1: Progress, Plan, History | 13a–13g | PLAN · `g b` |
| `14-budgets-v2` | Budgets v2: multiple budgets, four methods | 14a–14f | PLAN · `g b` |
| `15-reports` | Reports | 15a–15i | PLAN · `g r` |
| `16-settings` | Settings, one page per section | 16a–16u, dialogs 16c–16e, 16r–16t | pinned · `g s` |
| `17-accounts` | Account management & forms | 17a–17d | Settings › Accounts |
| `18-categories` | Category dialogs | 18a–18d | Settings › Categories |
| `19-tags` | Tag dialogs & merge | 19a–19e | Settings › Tags |
| `20-payees` | Payee management & import matching | 20a–20e | Settings › Payees |

**Which to build when two overlap**
- **Budgets:** 14 (v2) extends 13. Build v2's model; use 13 for any view v2 doesn't redraw.
- **Accounts, Categories, Tags, Payees** aren't in the primary rail any more. They're Settings pages (16o, 16i, 16j, Payees). Packages 17–20 are kept for their add / edit / delete / merge dialogs and forms, which Settings reuses. Where a list layout in 17–20 differs from Settings, Settings is correct.
- **Documents:** 4c/4d and 4e are alternatives. Pick one.

## Shell (all destinations)
Every frame is **1280 × 800**: header 48px, body `flex:1`, status bar 28px. Full spec in `01-shell/README.md`.
- **Header:** `#eae9e9`, 2px bottom rule. It holds the panel toggle, the wordmark "Personal Ledger" (800 13.5px), the breadcrumb `ledger › <destination>` (12px #9b9797), the `:` run-a-command affordance, sync status, and three 26px window controls.
- **Primary rail, 206px**, `#eae9e9`, 2px right rule:
  - LEDGER: Dashboard `g d` · Transactions `g l` · Documents `g f` · Events `g e` · Notifications `g m` (red count badge)
  - NET WORTH: Cash `g c` · Inventory `g o` · Loans `g n` · Credit cards `g k` · Investments `g i`
  - PLAN: Bills `g w` · Budgets `g b` · Reports `g r`
  - Settings `g s`, pinned at the bottom below a 2px rule
  - The active item is inverted (`#201e1d` / `#f3f2f2`, label 800).
- **Status bar:** `#eae9e9`, 2px top rule, 11.5px #605d5d. A mode chip (NORMAL / DIALOG; COMMAND in `#ec3013`), the key hints for the view, and a right-aligned context note.
- `b` toggles the rail. Settings › Display can start with it hidden and can hide the `g` hints.

## Design tokens (Modernist)
- **Colour:**
  - Ground `#f3f2f2` · chrome / panel `#eae9e9` · well `#d7d3d3`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6`.
  - Accent `#ec3013` (badges, primary emphasis) · text-safe accent `#ae1800` (warnings, due dates, negative over-budget) · positive `#2ecc71`.
  - Rules: strong 2px `rgba(32,30,29,.38)` · row 1px `#d7d3d3` or `rgba(32,30,29,.15)` · border `rgba(32,30,29,.30)`. Dialog dimmer `rgba(32,30,29,.30)`.
- **Type:** Archivo only, weights 400 and 800 (monospace for logs and paths). Caps labels are 800 10px with `.11em` tracking. Tabular numerals on every figure.
- **Radius:** 0 everywhere. Labels stay flush left, even inside wide buttons.
- **Shadows:** dialog `0 16px 48px rgba(32,30,29,.40)`; palette / menus `var(--shadow-md)`.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Disabled controls render at 45% opacity.
- **Icons:** Lucide, 14px with a 1.5 stroke in the rail; 10–12px inline.
- **2px rules need `flex:none`**, or they collapse.
- The full ramps are in `styles.css`.

## Global conventions
- **Keyboard first.** `g <key>` jumps between destinations, `j/k` moves rows, `/` searches, `:` opens the command palette, and `esc` closes. Every destination's status bar lists its keys.
- **Dialogs** are 420px, `2px solid #201e1d`. Focus is trapped while open. `esc` cancels; `enter` confirms when enabled. Destructive deletes need the name typed back.
- **Fields save on change.** Creating and deleting go through dialogs.
- **Derived figures are queries.** Counts, totals and balances are never stored.
- **Preferences vs configuration:** ledger data syncs; device configuration stays local. Settings pages say which on every page.

## Known gaps
- **Payees** in Settings is indexed but not drawn. Build it from 16j's pattern and 20a.
- **Documents Inbox** (formerly 4b) and the **Bills History tab** (12f) are specified but not drawn.
- **Packages 03, 12–15 and 17–20** were written against older frame numbers. They've been renumbered to match the master, and frames cross-referenced in their prose may predate later shell changes (Events, Notifications, Settings pages). Where they conflict, follow the design file and this README.
