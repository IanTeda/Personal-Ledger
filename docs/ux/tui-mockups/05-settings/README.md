# Handoff: Personal Ledger TUI — Settings (paged, keyboard first)

## Overview

The Settings destination for the `bin-tui` Client. It carries the **same sixteen pages, in the same order, with the same content** as Desktop Settings ([`../../desktop-mockups/16-settings/`](../../desktop-mockups/16-settings/README.md), DUI-063–DUI-083), re-hosted in the TUI's one-view shell and reinterpreted for a keyboard-only terminal.

This package replaces the earlier "database-backed settings" draft (a generic key/value `settings` table with defaults in a code registry). That model contradicted [ADR-0014](../../../adr/0014-preferences-table-and-leaner-sync-server-config.md), which chose typed Preference tables over a key/value store, and the Desktop design, so it is retired. Settings now follows the same Preference / Configuration split as the Desktop:

- **Preferences and ledger data** are stored in the ledger, sync as Change Sets, and are marked `●` in the index.
- **Configuration** stays on this device: `personal-ledger.conf`, the OS keychain, or local files. It never syncs and is marked `○`.

Eighteen cards are drawn, one per page plus the General editing states and the Sync Server dialog:

| ID | Local | Page or state | Desktop equivalent |
| --- | --- | --- | --- |
| TUI-019 | 4a | General, with the index focused | DUI-063 (16a) |
| TUI-020 | 4b | General, editing a field in place | 16a field edit |
| TUI-021 | 4c | Base unit guard (typed confirm) | TUI-only, see below |
| TUI-032 | 4d | Display | DUI-064 (16f) |
| TUI-033 | 4e | Units | DUI-065 (16g) |
| TUI-040 | 4l | Institutions | DUI-066 (16h) |
| TUI-041 | 4m | Accounts | DUI-067 (16o) |
| TUI-042 | 4n | Categories | DUI-068 (16i) |
| TUI-043 | 4o | Tags | DUI-069 (16j) |
| TUI-044 | 4p | Payees | not drawn on Desktop; built to its spec |
| TUI-045 | 4q | Documents | DUI-070 (16p) |
| TUI-046 | 4r | Inventory | DUI-071 (16q) |
| TUI-034 | 4f | AI Connections | DUI-072 (16v) |
| TUI-035 | 4g | Sync Server, Edit dialog with a failed test | DUI-073 (16k) + DUI-082 (16s) |
| TUI-036 | 4h | Backup | DUI-074 (16l) |
| TUI-037 | 4i | History | DUI-076 (16u) |
| TUI-038 | 4j | Logs | DUI-075 (16m) |
| TUI-039 | 4k | About | DUI-077 (16n) |

Cards appear in index order. The IDs are not consecutive because TUI-019–021 keep their IDs from the earlier draft and new cards take the next free numbers, in the order they were added (see [`../../mockup-ids.md`](../../mockup-ids.md)).

## About the design files

`Ledger TUI Settings.dc.html` (open it in a browser with `support.js` beside it) is a **design reference drawn in HTML**. It is not code to port. The target is Rust + ratatui + crossterm. Read the geometry in **terminal cells**: every px value is an artifact of drawing.

## Fidelity

**Low-to-mid fidelity.** Authoritative about the pages and their order, what each page shows, scope (synced or this device), pane structure and keyboard. For field lists and sample data the Desktop README is the source. Not authoritative about colours, borders or padding: use the theme roles. All figures are fake.

---

## How the Desktop maps to the TUI

| Desktop | TUI interpretation |
| --- | --- |
| Settings is a destination with the primary rail kept open | Settings is one View in the shell. There is no primary rail. `g s` or `:settings [page]` opens it. |
| Second rail (214px) is the index; clicking swaps the pane | Left pane (20 cols) is the index. **Moving the selection with `j/k` swaps the page live**, so you can scan pages without opening them. |
| Scope note on each page header only | The scope note stays on every page heading. The index also gets a one-glyph scope column (`●` syncs · `○` this device), with a legend pinned to the bottom of the index. |
| Rail footer "preferences sync · configuration local" | The index legend described above. |
| Click a field; fields save on change | Focus the page (`l`/`enter`), move to a field (`j/k`), and `enter` opens it **in place**. `enter` saves, `esc` keeps the old value. Saving on commit is the TUI equivalent of "saved automatically". |
| Segmented controls, radios, checkboxes | Segmented row with `←→` · checkbox `[x]` with `space` · text fields become a one-line input. |
| `+ Add …` buttons on list pages | The key shown in the block heading (`n add unit`). `n` always adds to the focused block. |
| Row actions (edit · delete · test …) | Keys in the hint bar acting on the selected row (`e`, `d`, `t` …). |
| Modal dialogs, 420–540px | Centred floating overlay (`Clear` + bordered `Block`, ~76% width, top third) over the dimmed page, the same treatment as the command palette. |
| Status bar NORMAL chip + hints | Hint bar per focus. The status line shows the mode (`INSERT`, `CONFIRM`) when it isn't NORMAL, and `✓ Synced HH:MM` on the right. |
| Header breadcrumb `settings › <page>` | Status line `Personal Ledger │ settings › <page>`. |

**List pages and the full views.** Units, Accounts, Categories, Tags and Payees already have their own full TUI views (`../02-accounts`, `../03-categories`, `../04-payees`, `../06-tags`, `../07-units`). Their Settings pages are the compact management lists Desktop shows, and they use **the same forms and keys** as those views (`n`/`e`/`d`, typed delete confirm). `o` opens the full view, and `esc` from it returns to the Settings page. Their existing `g` jumps stay as they are.

## Terminal geometry

Drawn at the shell's **96 × 30 cell** minimum.

```
row 0        status line   Personal Ledger │ settings › <page>        [MODE ·] ✓ Synced HH:MM
rows 1..n-2  two panes     index Length(20) │ page Min(0)
row n-1      hint bar      contextual to the focused pane and page
```

- **Index pane:** `settings 16` heading, 16 rows (label + scope glyph), then `Min(0)`, then the 2-row legend pinned to the bottom. The selected row is reversed when the index has focus. When the page has focus, the row is bold with a 1-col accent bar on the left, so you can still see where you are.
- **Page pane:** heading row (`settings – <page>` bold on the left, the scope note dim on the right) over a strong rule, then the page body. Bodies are `Length` blocks over a `Min(0)`. Pages that fill the height (Logs, History) give the `Min(0)` to their list.
- **Degrading below 96 columns:** drop the page's notes column, then collapse the index to its scope glyph and the first 3 letters (`gen ●`), then hide the index entirely and show it as a breadcrumb that `h` reopens.

## Keyboard

Focus has two places: the **index** and the **page**.

| Key | Index focused | Page focused |
| --- | --- | --- |
| `j` / `k` | previous / next page (swaps live) | previous / next field or row |
| `g` / `G` | first / last page | first / last field or row |
| `l` / `enter` | focus the page | open the field to edit (form pages); page-specific on list pages |
| `h` | — | back to the index |
| `esc` | pop the view stack (leave Settings) | back to the index; inside an editor, keep the old value |
| `tab` | — | next block on pages with two (Units, AI Connections, Backup) |
| `u` / `ctrl r` | undo / redo the last change, anywhere in the app (History) | same |

Inside an open field (mode `INSERT`): `←→` choose · `space` toggle · `enter` save · `esc` keep the old value · `tab` save and move to the next field.

Page keys (shown in the hint bar):

| Page | Keys |
| --- | --- |
| Units, Institutions, Accounts, Categories, Tags, Payees, Documents, Inventory | `n` add · `e` edit · `d` delete (typed confirm) · `o` open the full view where one exists. Plus Desktop's per-page keys: Tags `x` remove, `m` merge · Documents and Inventory rooms `J`/`K` reorder · Categories and Inventory `→`/`←` expand / collapse · Inventory `r` new room · Accounts `enter` open the account's ledger · Units `t` test a price source |
| AI Connections | `n` add (provider or assistant, by focused block) · `e` edit · `t` test · `D` make default · `x` remove / revoke |
| Sync Server | `s` sync now · `e` edit (dialog) |
| Backup | `b` back up now · `X` export ledger (CSV) · `e` edit Git backup (dialog) · `P` push now |
| History | `enter` restore to here (or redo to here, on a greyed row) · `f` area filter |
| Logs | `←→` level · `j/k` scroll · `g/G` newest / oldest · `C` clear · `y` copy line |
| About | `j/k` link · `enter` open in the browser · `y` copy link |

Dialogs: `tab` next field · `ctrl t` test (where there is a Test) · `enter` save, only when enabled · `esc` cancel. The hint for a disabled `enter` is drawn dim.

### Command grammar

Every action is reachable by name ([`../navigation.md`](../navigation.md)):

```
settings   [page]              opens Settings on that page (last visited if omitted)
set        <field> [value]     edits a Preference or Configuration field; without a value, opens it in place
sync       now | edit
backup     now | export | git
undo | redo
```

`<page>` completes on the 16 index labels. `<field>` completes on the field names of the form pages (`fy-start`, `base`, `date-style`, `glyphs` …). TUI-020 shows the command line echoing the equivalent `:set`, which is how users learn the typed form.

---

## Pages

The content of each page is the Desktop's (see its Pages table). Only TUI-specific changes are listed here.

| Page | Scope | TUI notes |
| --- | --- | --- |
| **General** (TUI-019/020) | ● preferences · synced | Fields: ledger name, owner, financial year starts, base unit. Desktop's side-by-side THIS LEDGER summary sits below the fields as a 4-cell box. The editor (TUI-020) shows what the change does: for financial year, the resulting year range, the old value, and what it affects. Changing **base unit** goes through the guard (TUI-021). |
| **Display** (TUI-032) | ○ configuration · local, but appearance and colour theme ● sync | Every row carries its own scope glyph, because this page mixes scopes. **Appearance** (light · dark · system) and **Colour Theme** are Ledger-scoped Preferences ([ADR-0023](../../../adr/0023-colour-theme-preferences-and-theme-role-overrides.md)) and are already built: `enter` on Colour Theme opens the existing theme list popup. **Toasts** is the Client-scoped Preference from [ADR-0027](../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md), also already built. Desktop's Date format and Separator controls are replaced to follow [ADR-0021](../../../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md). **Date style** (locale default · short · medium · long · iso) replaces date format. **Locale** is shown read-only. There is no separator control. Status glyphs (unicode / ascii) carry over. TUI-only changes: **Row density** and **Hide sidebar** don't apply. **Terminal colours** (theme RGB / terminal palette, [ADR-0024](../../../adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md)) is added. Desktop's **Show keyboard navigation hints** becomes **hint bar**. The preview box redraws as you choose. |
| **Units** (TUI-033) | ● ledger data | UNITS and PRICE SOURCES blocks, switched with `tab`. The base and default flags are drawn as outlined chips. A note for the selected unit states its usage, as Desktop's Edit unit (16c) does. Add, edit and delete reuse the TUI unit forms (TUI-028–031). |
| **Institutions** (TUI-040) | ● ledger data | INSTITUTION · ACCOUNT TYPES · UNIT · ACCOUNTS, A–Z, then a `selected` box naming the institution's accounts and the full account-type set (chosen types bright, the rest dim). Account types use **one enum** shared with the account form (savings · offset · credit card · loan · investment), fixing Desktop known inconsistency 3. Institution names are the stored names the Accounts page shows, fixing inconsistency 2. |
| **Accounts** (TUI-041) | ● ledger data | Grouped tables by type (BANK, CREDIT CARD, LOAN, INVESTMENT), each with a dim `type · count` label. NAME · INSTITUTION · UNIT · BALANCE, negatives in the accent. Net worth sits in the heading note. `enter` opens the account's ledger, as on Desktop. Add and edit reuse the TUI account form (TUI-007/008). |
| **Categories** (TUI-042) | ● ledger data | A tree per kind (EXPENSE, INCOME), with `▾`/`▸` on parents, `├`/`└` connectors and an `n subcategories` note. `→`/`←` expand and collapse. `n` adds a top-level category and `s` adds a subcategory under the selected row, replacing Desktop's `+ sub` row action. No spend or budget columns. Counts are derived (Desktop said "3 levels deep" with only 2 drawn). |
| **Tags** (TUI-043) | ● ledger data | A–Z with a 1-cell colour swatch and the name in bold. A likely duplicate carries `⚑ looks like a duplicate of "…"` in words plus an inline `m merge`. The box below says what the merge does before you press it. `x` removes, as on Desktop. The merge form is the Tags view's (TUI-025). |
| **Payees** (TUI-044) | ● ledger data | Desktop indexes Payees but never drew the page. This one follows its spec: NAME · CATEGORY (default) · MATCH RULES, A–Z, without count or spend columns. The `selected` box shows all match rules, the rename aliases ([ADR-0012](../../../adr/0012-payee-entity-with-rename-aliases.md)) and the default category. Draw the Desktop page from this when it's designed. |
| **Documents** (TUI-045) | ● ledger data | Document types in Documents type-filter order ([ADR-0031](../../../adr/0031-document-types-are-user-managed-ledger-data.md)): TYPE · TRACKS · REMIND · TAX YEAR · FILES. `J`/`K` reorder. The three Desktop notes become four dim lines under the table. |
| **Inventory** (TUI-046) | ● ledger data | A properties tree. Property rows show policy, sum insured, item limit and items. Room rows are indented under them with derived value and items; room value shares the POLICY · VALUE column, because rooms have no policy. The addresses Desktop shows under each property are left out at 96 columns, to keep each property on one row; show them in the edit form. `n` new property, `r` new room under the selected property, `J`/`K` reorder rooms, `x` remove (asks where to move items). |
| **AI Connections** (TUI-034) | ○ this device | Model providers and Assistant access (MCP server). Status is a glyph plus a word (`● connected`, `■ key rejected`), never colour alone. Per Client and never synced, and Assistants are read only: Read & write is drawn dim as "not yet offered" ([ADR-0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md)). |
| **Sync Server** (TUI-035) | ○ this device · synced | Server URL, status, last sync, `s` sync now. `e` opens the Edit dialog (Desktop 16r/16s). Save stays disabled until a test passes, and a failed test names the cause in plain words. |
| **Backup** (TUI-036) | ○ this device · local files | LOCAL and GIT BACKUP blocks. Git Backup commits a text dump of the Ledger plus its Documents through LFS, only to a private repository ([ADR-0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md)), so the block adds a **contents** row. `e` opens Set up Git backup (Desktop 16t) as an overlay like TUI-035. Its Test also checks visibility and LFS support. The commit-message prefix is a fixed dim chip. |
| **History** (TUI-037) | ● all devices | Same undo stack as Desktop 16u. Undone rows are dim and struck through above a reversed `▸ CURRENT STATE` marker band. The selected applied row reads `enter restore to here`. The area filter is a segmented row cycled with `f`. |
| **Logs** (TUI-038) | ○ this device · diagnostic | The level filter and `C clear logs` share one row. The log box takes the `Min(0)` and always draws the 1-col scrollbar. Lines are `[hh:mm:ss] <subsystem>: <message>`, newest first. |
| **About** (TUI-039) | — | "Built with Rust + ratatui + SQLite". Links are rows (`j/k`, `enter` opens in the browser, `y` copies). The copyright is pinned to the bottom. |

## TUI-021 — Base unit guard

Desktop's General page changes base unit with a plain select. The TUI keeps the earlier draft's guard, because every reported total is converted again when it changes. The commit is cheap (one Preference write, synced as one Change Set, undoable with `u`), but the result can surprise you. The guard follows the Desktop's typed-delete convention (16d):

- Header: `base unit  aud → btc`, with the equivalent `:set base btc` on the right.
- Prose: transactions keep their own units, and reported totals are converted again.
- Facts, each a query: transactions (unchanged), accounts (how many are already in the new unit), how many months are converted again, and **missing prices** as a count *and* a range, in the accent. Then `syncs as one Change Set · u undoes it`.
- The fix is offered as a choice, not an error: `p` opens Settings › Units › price sources.
- **Type the unit code to confirm.** `enter` is dim until the text matches exactly.

## State

```
settings_view
  focus: Index | Page
  page: General | Display | Units | Institutions | Accounts | Categories | Tags | Payees | Documents
      | Inventory | AiConnections | SyncServer | Backup | History | Logs | About   // remembered per session
  page_state: per page — selected field/row, focused block, expanded tree nodes
  editor: None | Field { field, candidate, was }            // INSERT
  overlay: None | BaseUnitGuard { to, typed } | EditSyncServer | SetupGitBackup
         | AddProvider | EditProvider(id) | AddAssistant | the domain forms (unit, account, …)
```

The data is Desktop's (`preferences` / ledger data / `configuration`, see its State block). Don't cache counts: derive them on each draw from the same queries the domain views use.

## Style

Use theme roles, not the wireframe hex values:

| Role | Used here for |
| --- | --- |
| accent | the focused-page index bar, the open field's border, missing prices, failed tests, link text |
| dim | labels, scope notes, the index legend, undone History rows, disabled hints |
| selection (reversed) | the selected index row (index focused) or field/row (page focused), the chosen segment, the History marker |
| header bar | the status line |

Never rely on colour alone. The scope glyphs `●`/`○` are distinct shapes, failures use `■` plus words, and disabled keys are named as disabled in the dialog's own note.

## Known gaps

1. **The Desktop mockup itself predates ADR-0021** on Display (it still shows date format and separators). The TUI follows the ADR. Raise the same correction on the Desktop package.
2. **AI Connections and Git Backup** are decided in [ADR-0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md) and [ADR-0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md). The Desktop mockup still shows Read & write as selectable and says Git backup "commits the store"; build to the ADRs.
3. **`g s`** already works in `shell.rs`, but [`../navigation.md`](../navigation.md)'s jump table doesn't list it. Add it there.

## Files

- `Ledger TUI Settings.dc.html` — TUI-019–021 and TUI-032–046, in index order. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- `CLAUDE_CODE.md` — the staged work plan for building this design in `bin-tui`.
