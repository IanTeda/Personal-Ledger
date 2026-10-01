# Handoff: Desktop Shell, Navigation & Settings

## Overview
Personal Ledger's **application shell** (window chrome, primary rail, contextual rail, status bar, the `:` command palette, the `:open` file explorer, Help & About) and its **Settings** destination: one page per section, with the modal dialogs for unit and institution management.

This package replaces `design_handoff_shell_navigation/` and `design_handoff_settings/`. Where they disagree, this README wins. The changes since those packages:
- **Nav regrouped.** Accounts and Payees left the primary rail and moved into Settings. Documents moved into LEDGER. NET WORTH gained **Cash** (`g c`) and now reads Cash, Inventory, Loans, Credit cards, Investments.
- **Settings is paged.** Settings is no longer one long scroll. Each index entry opens its own pane.
- **New Settings pages.** Accounts, Categories, Tags and Payees are now Settings pages. They are reference data: set up once and rarely touched.
- **File explorer filters.** It gained two checkboxes: *show only .pldb files* and *hide hidden files*.

## About the design files
`Shell and Settings.dc.html` is a **high-fidelity HTML design reference**. It shows the intended look, layout and behaviour. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's own patterns. Do not port the markup.

To view it, open the file in a browser from this folder. It loads `support.js`, `styles.css` and `_ds_bundle.js` alongside it. Fonts come from Google Fonts (Archivo). Frames are grouped into sections: section 1 is the shell (1a–1f), section 2 is Settings.

## Fidelity
**High fidelity.** Colours, typography, spacing, rules and states are final. Recreate them faithfully.

## Frame
Every frame is **1280 × 800**: header 48px, body `flex:1`, status bar 28px. Inside the mockup, overlay frames dim the shell behind with opacity. In the app, render the shell at full opacity under a dimmer (`rgba(32,30,29,.30)`).

---

## 1 · Shell

### Primary rail (206px): canonical order
```
LEDGER      Dashboard g d · Transactions g l · Documents g f
NET WORTH   Cash g c · Inventory g o · Loans g n · Credit cards g k · Investments g i
PLAN        Bills g w · Budgets g b · Reports g r
──(2px rule, pinned bottom)──
            Settings g s
```
- **Removed bindings:** `g a` (Accounts), `g p` (Payees) and `g t` (Tags) no longer exist. Those entities are reached through Settings or the command palette.
- **`g c` reassigned.** It now jumps to **Cash**. It used to open Categories, which is now a Settings page.
- **Source of truth:** the order above is canonical and matches frames 1a and 1b. The Settings frames in section 2 still draw Inventory before Cash; follow 1a/1b.
- **Cash page:** a separate design (turn 18 in the main file) that isn't part of this package yet.
- **Order rules:** Reports is always the last PLAN item. Settings is always pinned below the spacer.
- **Shared nav:** the same rail appears in every Settings frame (2a, 2f–2o), with Settings active.

### Frames
| Frame | What it shows |
| --- | --- |
| **1a** | Cold start, no ledger open. Full grouped rail and no context rail. The main pane shows the centred empty state: "No ledger open" (800 26px/1.1, −.01em), then "Run `:open` to load a ledger file, or `:new` to start one." (13px #605d5d, max 380px). Status bar shows the `NORMAL` mode chip. |
| **1b** | The active dashboard: net worth, cash / investments / liabilities, and today's actions. The second rail is an "accounts at a glance" quick-jump panel. Dashboard has no records of its own, but account jumps earn their place here. |
| **1c** | Primary rail collapsed to icons (~48px; `b` toggles it; hover reveals label + binding). The second rail becomes the screen's table of contents. Its settings index is superseded by section 2 — use the index below. |
| **1d** | `:` command palette (820px wide, `top:96px`, centred horizontally) over the dimmed shell. It contains the input row with a red 8×17 block cursor, results with bindings, an argument-preview row and a key hint footer. |
| **1e** | `:open` file explorer (640px, `top:80px`) over the 1a shell. Details below. |
| **1f** | Help & About (`?`): app identity, links and a shortcut cheat-sheet. Uses the same overlay treatment as 1d and 1e. Build its cheat-sheet from the command registry so it lists the bindings above, including `g c` Cash. The drawn frame predates Cash. |

### 1e: file explorer
- **Dialog chrome:** `border:2px solid #201e1d; box-shadow:0 16px 48px rgba(32,30,29,.40)`.
- **Header:** "Open ledger file" (800 16px), `padding:16px 20px`, 2px bottom rule.
- **Path bar:** folder icon + `~ / documents / ledgers`. The current segment is 800 #201e1d. The item count sits on the right.
- **Columns:** NAME / SIZE (90px) / MODIFIED (120px).
- **Rows**, `padding:9px 20px`:
  - folders `archive` and `backups` (#9b9797)
  - `expenses-2024.csv` (#605d5d)
  - decoy `old-ledger.pldb.bak`
  - **selected** `teda.pldb` (`background:#201e1d; color:#f3f2f2`)
  - `personal-2023.pldb`
- **Footer**, `padding:16px 20px; border-top:1px solid #d7d3d3`: on the left, two stacked checkboxes (`gap:8px`, 12px). On the right, Cancel (outline) + Open (dark).
  - ☐ **show only `.pldb` files**: default **off**. When on, hide every entry that isn't a folder or a real `.pldb` file. Folders stay visible so you can still navigate.
  - ☑ **hide hidden files — dot files and folders**: default **on**. Hides entries whose name begins with `.`. The em-dash clause is #605d5d.
  - Checkboxes are square, 14×14, ink-filled when checked, with no radius.
- **Behaviour:**
  - Only `.pldb` rows can be the open target. Folders navigate in; other files are inert.
  - Open is disabled until a `.pldb` row is selected. Double-clicking a `.pldb` row opens it.
  - `esc` and Cancel close the explorer. Path segments are clickable.
  - Eligibility is decided by the **real extension**, not a substring. The `.pldb.bak` decoy is there to catch `.includes(".pldb")`.
  - Both checkbox states persist as client configuration and re-filter the list live. The item count reflects the filtered list.

---

## 2 · Settings

### Model
- **Settings is a destination, not a mode.** The primary rail stays expanded with Settings active.
- **The second rail (214px) is the index.** Clicking an entry **swaps the pane**. Each page stands alone — there is no long scroll.
- **Configuration vs Preferences stays visible.** Each page header carries a scope note on the right.

### Index (canonical order)
`General · Display · Units · Institutions · Accounts · Categories · Tags · Payees · Sync server · Data & backup · Tracing (Logs) · About`

- **Rail chrome:** the search box `/ search` has wrapper padding `22px 12px 25.4px` plus a 2px rule. That padding aligns its rule with the page-heading rule.
- **Index rows:** the `SETTINGS` label sits above the entries. Entries are `padding:8px 14px`; the active one is `background:#201e1d; color:#f3f2f2; 800`.
- **Rail footer:** "preferences sync · configuration local".

### Page anatomy (all pages)
- **Body:** `flex:1; min-width:0; padding:22px 28px; overflow:auto`.
- **Page heading:** `h2` "Settings" (28px/800) with a scope note right-aligned on the baseline (11.5px #9b9797), then a 2px rule (`margin-bottom:24px`).
- **Section heading:** `h4` with the page name (20px) and its own right-aligned meta, then a 2px rule (`margin:14px 0 18px`).
- **List pages:** a kicker row comes next — a label on the left (`800 10px/1, .11em, #9b9797`) and a primary `+ Add …` button (`.btn.btn-primary`, 32px) on the right.
- **Breadcrumb:** the header reads `settings › <page>`.

### Pages
| Frame | Page | Scope note / meta | Content |
| --- | --- | --- | --- |
| **2a** | General | preferences · synced / ledger identity | Ledger name, Owner, Financial year starts, Base unit (320px field column) beside a THIS LEDGER summary (accounts 7 · transactions 680 · units 3 · institutions 7). |
| **2f** | Display | configuration · local / client-scoped · never synced | Date format, separators and row density (segmented), plus status glyphs (radio), beside a live PREVIEW of three ledger rows. |
| **2g** | Units | ledger data · synced / 3 units · synced | UNITS table: CODE 100 · NAME 180 · FLAGS flex · SOURCE 130 · TYPE 80 · ACTIONS 120. `aud` carries the `base` (`.tag-accent`) and `default` (`.tag-outline`) flags. Then **+ Add unit**. PRICE SOURCES table: NAME 130 · SOURCE flex · LAST UPDATED 130 · ACTIONS 170 (test · edit · delete), then **+ Add price source**. |
| **2h** | Institutions | ledger data · synced / 7 institutions | INSTITUTION flex · ACCOUNT TYPE 140 · ACTIONS 120, then **+ Add institution** (opens 2e). |
| **2o** | Accounts | ledger data · synced / 7 accounts · net worth 83,995.87 aud | Kicker BY TYPE + **+ Add account**. Grouped tables (BANK, CREDIT CARD, LOAN, INVESTMENT), each with NAME flex · INSTITUTION 130 · UNIT 90 · BALANCE 120 (right, tabular) · ACTIONS 150 (edit · delete). Rows are `padding:8px 16px`. Add and edit reuse the account form from section 3 (3b/3c). Status bar: `j/k row · enter open ledger · e edit · d delete · n new`. |
| **2i** | Categories | ledger data · synced / 12 categories · 3 levels deep | Kicker EXPENSE · 10 + **+ Add category**. A nested tree: ▾ disclosure, parent rows in 800 with an "n subcategories" note, row actions `+ sub · edit · delete`. No budget or spend columns. Keys: `j/k`, `→/←` expand/collapse, `e`, `d`, `n`. |
| **2j** | Tags | ledger data · synced / 18 tags · **2** likely duplicates (#ae1800) | Kicker A–Z + **+ Add tag**. Each row has a colour swatch (10×10) and the tag name in 800, with edit · remove. Likely-duplicate rows carry a warning and a direct **merge**; the merge flow is unchanged from 7e. No transaction, total or last-used columns. Keys: `j/k`, `e`, `x`, `m`, `n`. |
| — | Payees | ledger data · synced | **Indexed but not yet drawn.** Build it to the same pattern as 2j: a kicker A–Z + **+ Add payee**, then name · default category · match rules · actions (edit · delete), reusing the Payees management view from turn 6 (6a) with its count and spend columns dropped. |
| **2k** | Sync server | this device | Server URL, Status (green dot `#2ecc71` + "connected"), Last sync, **Sync now**. |
| **2l** | Data & backup | this device | Store location, Last backup, **Backup now**, **Export ledger (CSV)**. |
| **2m** | Tracing (Logs) | this device | Level radios (error / warn / info / debug), a monospace log viewport (120px, scrolls), **Clear logs**. |
| **2n** | About | this device | Version 2.1.4, released 08 sep 2026, build stack. |

### Dialogs
All dialogs are 420px, `border:2px solid #201e1d`, with the shadow `0 16px 48px rgba(32,30,29,.40)`. The shell behind is dimmed.

| Frame | Dialog | Notes |
| --- | --- | --- |
| — | Add unit | Not drawn separately. It is 2c's form, empty, with **Add** as the confirm. Fields: Code, Name, Type (currency / cryptocurrency / custom). |
| **2c** | Edit unit | Pre-filled `aud`. Usage notice (`#eae9e9`, 2px left rule `#ec3013`, 11.5px): "Used by 4 accounts · 604 transactions. Renaming is safe; changing the code rewrites references." Cancel / Save. |
| **2d** | Delete unit | Red treatment: border and header rule `#ec3013`, title `#ae1800`. Names every reference. Has a "Type btc to confirm" field; **Delete unit** stays disabled until the text matches exactly. |
| **2e** | Add institution | Name, Account types as **multi-select chips** (savings / credit card / offset / loan / investment), Default unit. Cancel / Add institution. |

- Opening a dialog moves focus to its first field and traps focus while open. Closing returns focus to the trigger.
- `esc` cancels. `enter` submits when the confirm button is enabled.
- Plain fields **save on change** — there is no page Save button. Creating and deleting always go through a dialog.

---

## Components
| Part | Spec |
| --- | --- |
| Header | 48px, `#eae9e9`, `border-bottom:2px solid rgba(32,30,29,.38)`, `padding:0 10px 0 12px`, gap 12. Wordmark 800 13.5px; breadcrumb 12px #9b9797; `:` command affordance `padding:4px 9px; border:1px solid rgba(32,30,29,.30)`; three 26px window controls `rgba(32,30,29,.08)`. |
| Rail group label | `800 10px/1 Archivo; letter-spacing:.11em; #9b9797; padding:0 14px 8px` (16px top padding for later groups). |
| Nav item | `flex; gap:9px; padding:7px 14px`; 14×14 Lucide icon (1.5 stroke); label `flex:1`; binding 11px #9b9797. Active: `#201e1d` ground, `#f3f2f2` text and icon, label 800, binding `#bab6b6`. |
| Status bar | 28px, `#eae9e9`, 2px top rule, 11.5px #605d5d. Mode chip `#201e1d/#f3f2f2 800 10px .1em padding:2px 7px`; COMMAND mode uses `#ec3013`. |
| Table header | `padding:12px 16px` (6–10px on dense lists), `#eae9e9`, `800 10px .11em #605d5d`, 1px bottom rule. |
| Table row | `padding:12px 16px` (8px on Accounts); `border-bottom:1px solid #d7d3d3`, omitted on the last row; primary cell 800. |
| Row action | `padding:4px 10px; 11px; border:1px solid rgba(32,30,29,.30); transparent`. |
| Section rule | `height:2px; flex:none; rgba(32,30,29,.38)` — `flex:none` is mandatory. |
| Info panel | `#eae9e9`, `border-left:2px solid #ec3013`, 11.5px #605d5d. |

## Design tokens
- **Colour:**
  - Ground `#f3f2f2`. Chrome / panel `#eae9e9`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6`.
  - Accent `#ec3013`; text-safe accent `#ae1800`. Positive `#2ecc71`.
  - Rules: strong `rgba(32,30,29,.38)`, medium `#d7d3d3`. Border `rgba(32,30,29,.30)`. Dimmer `rgba(32,30,29,.30)`.
  - The full ramps are in `styles.css` (Modernist).
- **Type:**
  - Archivo only (monospace for logs and paths), weights 400 and 800.
  - Sizes 10 / 11 / 11.5 / 12 / 12.5 / 13 / 13.5 / 16 / 20 / 26 / 28px.
  - Caps labels `.11em`. Tabular numerals on every figure.
- **Spacing:** 6, 7, 8, 9, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 40, 48px.
- **Radius:** **0 everywhere.**
- **Shadows:**
  - Dialog `0 16px 48px rgba(32,30,29,.40)`.
  - Palette `0 12px 32px rgba(45,43,43,.30)`.
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`. Disabled controls render at 45% opacity.

## State
```
shell
  ledgerOpen: bool
  activeSection: Dashboard | Transactions | Documents | Cash | Inventory | Loans | CreditCards
               | Investments | Bills | Budgets | Reports | Settings
  railCollapsed: bool
  mode: Normal | Insert | Command
  overlay: None | Palette | FileExplorer | Help
fileExplorer
  currentPath, entries[], selected: Option<PathBuf /* .pldb only */>
  onlyPldb: bool = false          // client config, persisted
  hideHidden: bool = true         // client config, persisted
settings
  page: General | Display | Units | Institutions | Accounts | Categories | Tags | Payees
      | SyncServer | DataBackup | Tracing | About
  searchQuery: String             // narrows the index only
  dialog: None | AddUnit | EditUnit(id) | DeleteUnit(id) | AddInstitution
        | AddAccount | EditAccount(id) | AddCategory(parent?) | AddTag | MergeTags(a,b) | AddPayee
  confirmInput: String
```

## Keyboard
| Key | Action |
| --- | --- |
| `g` + `d l f c o n k i w b r s` | Jump to a section. |
| `j` / `k` | Move to the next / previous row or field. |
| `b` | Collapse / expand the rail. |
| `:` | Command palette. |
| `/` | Search. |
| `?` | Help. |
| `esc` | Close an overlay or clear the search. |
| `e` / `d` / `n` | Edit / delete / new on list pages. |
| `m` | Merge (Tags). |

Settings pages are reachable from the palette (`:settings accounts`, etc.). One command registry drives the palette, the bindings, the status-bar legend and the 1f cheat-sheet.

## Implementation notes (GPUI)
1. **Rails are chrome.** They render identically whether or not a ledger is open.
2. **Settings pages swap; they don't scroll to anchors.** Remember the last-visited page per session.
3. **The 2px rules need `flex:none`.** Without it they collapse to hairlines.
4. **Delete confirms are typed.** Never substitute an "are you sure?" prompt.
5. **File eligibility comes from the real extension.** Folders stay visible under *only .pldb*.
6. **Zero radius, flush-left labels.** Labels stay flush left even inside wide buttons. Icons are Lucide at 14px with a 1.5px stroke.

## Files
- `Shell and Settings.dc.html` — design reference. Section 1 = 1a–1f; section 2 = 2a, 2f, 2g, 2h, 2o, 2i, 2j, 2k–2n, then dialogs 2c–2e.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components the reference loads.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.

## Acceptance pass

The closing walk of the [Desktop Shell Navigation and Paged Settings](https://github.com/IanTeda/Personal-Ledger/issues/413) map (#423), against frames 1a–1f, 2a, 2f–2o and dialogs 2c–2e, now that the regrouped rail (#420), the paged Settings surface (#415), the Accounts, Categories, Tags and Payees pages (#416–#419), the explorer filters (#421) and the per-focus legends (#424) have landed in `crates/bins/bin-desktop`. All data is stubbed and in-memory.

### Verified by code review and tests

This pass was **not** verified live: the sandbox it ran in has no GPU, so no frame was screenshotted. What it did check, against the code:

- **Rail order and bindings** (`rail/primary.rs`, `nav.rs`): LEDGER, NET WORTH and PLAN list the canonical rows with the canonical `g` keys, Settings `g s` is pinned below the rule, and `g p`/`g t` no longer exist. `g c` is Cash.
- **Settings index** (`settings.rs`): twelve entries in the canonical order, one page mounted at a time, scope notes per page, `:settings <page>` plus the `:accounts`/`:categories`/`:payees`/`:tags` aliases (`command.rs`), and a legacy persisted `Accounts` noun mapped to its Settings page (`persistence.rs`).
- **Explorer filters** (`explorer.rs`): *only .pldb* defaults off and *hide hidden* defaults on, both persist, and both re-filter the list.
- `cargo test -p bin_desktop` (671 tests) and `mise run lint` pass.

### Deliberate departures from the handoff

- **Notifications** is a fourth LEDGER row after Documents, bound `g a`. The handoff retires `g a`, but the map settled that a durable-notifications noun lands as a placeholder here (surface and model are a later map).
- **Display** keeps Colour Theme, Toasts and "start with sidebar minimised", which 2f doesn't draw.
- **Rail badges and context rails** for the moved nouns are gone; each Settings page's section-heading meta carries the count. The Dashboard's 1b "accounts at a glance" rail is kept, its jump target now Settings › Accounts.
- **Payees** has no drawn frame; it follows the 2j pattern with name, default category, match rules and actions.
- Letter-spacing, tabular figures and the `outline: 2px` focus ring are not reproduced (`gpui` 0.2 has no hook; the shell has one keyboard focus), as on the other desktop surfaces.

### Still to do

- A live walk of every frame (1a–1f, 2a, 2f–2o, 2c–2e) on a machine with a GPU, with the temporary keystroke injector and `grim`. Pages #415–#419 and #424 were each built and linted but not seen on screen.
- The Dashboard 1b rail's contents, now that Accounts has no noun of its own, are still undecided.
