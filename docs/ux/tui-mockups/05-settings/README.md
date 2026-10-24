# Handoff: Personal Ledger TUI — Settings (index page, keyboard first)

## Overview

The Settings destination for the `bin-tui` Client. It carries the **same sixteen pages, in the same order, with the same content** as Desktop Settings ([`../../desktop-mockups/16-settings/`](../../desktop-mockups/16-settings/README.md), DESKTOP-063–DESKTOP-083), re-hosted in the TUI's one-view shell and reinterpreted for a keyboard-only terminal.

**Navigation is the Desktop's Medium navigation** ([`../../desktop-mockups/01-shell/README.md`](../../desktop-mockups/01-shell/README.md#medium-settings-navigation-turn-2), DESKTOP-119 and DESKTOP-120, decided in [#692](https://github.com/IanTeda/Personal-Ledger/issues/692)), at every terminal width: `g s` lands on a full-page **Settings index**, `enter` opens a page in its place, and on a page `esc` goes back to the index, `p` opens the page picker and `[` / `]` step through the pages in index order. There is no docked index.

This package replaces the earlier "database-backed settings" draft (a generic key/value `settings` table with defaults in a code registry). That model contradicted [ADR-0014](../../../adr/0014-preferences-table-and-leaner-sync-server-config.md), which chose typed Preference tables over a key/value store, and the Desktop design, so it is retired. Settings now follows the same Preference / Configuration split as the Desktop:

- **Preferences and ledger data** are stored in the ledger, sync as Change Sets, and are marked `●` in the index.
- **Configuration** stays on this device: `personal-ledger.conf`, the OS keychain, or local files. It never syncs and is marked `○`.

Twenty cards are drawn: the index page and the page picker, then one per page plus the General editing states and the Sync Server dialog:

| ID | Local | Page or state | Desktop equivalent |
| --- | --- | --- | --- |
| TUI-047 | — | Settings index page, back from Display | DESKTOP-119 |
| TUI-048 | — | Page picker, a centred popup over General | DESKTOP-120, as a popup |
| TUI-019 | 4a | General at rest | DESKTOP-063 (16a), DESKTOP-126 |
| TUI-020 | 4b | General, editing a field in place | 16a field edit |
| TUI-021 | 4c | Base unit guard (typed confirm) | TUI-only, see below |
| TUI-032 | 4d | Display | DESKTOP-064 (16f) |
| TUI-033 | 4e | Units | DESKTOP-065 (16g) |
| TUI-040 | 4l | Institutions | DESKTOP-066 (16h) |
| TUI-041 | 4m | Accounts | DESKTOP-067 (16o) |
| TUI-042 | 4n | Categories | DESKTOP-068 (16i) |
| TUI-043 | 4o | Tags | DESKTOP-069 (16j) |
| TUI-044 | 4p | Payees | not drawn on Desktop; built to its spec |
| TUI-045 | 4q | Documents | DESKTOP-070 (16p) |
| TUI-046 | 4r | Inventory | DESKTOP-071 (16q) |
| TUI-034 | 4f | AI Connections | DESKTOP-072 (16v) |
| TUI-035 | 4g | Sync Server, Edit dialog with a failed test | DESKTOP-073 (16k) + DESKTOP-082 (16s) |
| TUI-036 | 4h | Backup | DESKTOP-074 (16l) |
| TUI-037 | 4i | History | DESKTOP-076 (16u) |
| TUI-038 | 4j | Logs | DESKTOP-075 (16m) |
| TUI-039 | 4k | About | DESKTOP-077 (16n) |

The index page and picker come first, then the pages in index order. The IDs are not consecutive because TUI-019–021 keep their IDs from the earlier draft and new cards take the next free numbers, in the order they were added (see [`../../mockup-ids.md`](../../mockup-ids.md)).

## About the design files

`Ledger TUI Settings.dc.html` (open it in a browser with `support.js` beside it) is a **design reference drawn in HTML**. It is not code to port. The target is Rust + ratatui + crossterm. Read the geometry in **terminal cells**: every px value is an artifact of drawing.

## Fidelity

**Low-to-mid fidelity.** Authoritative about the pages and their order, what each page shows, scope (synced or this device), pane structure and keyboard. For field lists and sample data the Desktop README is the source. Not authoritative about colours, borders or padding: use the theme roles. All figures are fake.

---

## How the Desktop maps to the TUI

| Desktop | TUI interpretation |
| --- | --- |
| Settings is a destination with the primary rail kept open | Settings is one View in the shell. There is no primary rail. `g s` or `:settings` opens the index; `:settings <page>` opens a page directly. |
| Medium: no docked index; `g s` opens a full-page index (DESKTOP-119) | The same, at every width. The index page lists the 16 pages, each with its scope glyph and a one-line description, and `enter` opens the selected page in its place (TUI-047). |
| Medium: the breadcrumb heading **Settings › <Page> ▾**; **Settings** goes back to the index | The page heading is the breadcrumb `Settings › <Page>`, without the caret, since the picker isn't anchored to it. `esc` is the way back to the index, with the page you left still selected. |
| Medium: the caret (or `p`) drops the page picker under the page name (DESKTOP-120) | `p` opens the picker as a **centred popup** over the dimmed page, the command popup's treatment rather than a drop-down: the 16 pages in index order, all of them in view at once, so there is no filter, `enter` to open, `esc` to close (TUI-048). A terminal has no pointer to anchor a drop-down to, and a centred popup is where the TUI's other popups already open. |
| Medium: `[` / `]` step to the previous and next page | The same keys. `[` does nothing on General and `]` nothing on About. |
| Scope note on each page header only | The scope note stays on every page heading. The index page also gives each row a one-glyph scope column (`●` syncs · `○` this device), with a legend pinned to its bottom. |
| Rail footer "preferences sync · configuration local" | The index page legend described above. |
| Click a field; fields save on change | Move to a field (`j/k`), and `enter` opens it **in place**. `enter` saves, `esc` keeps the old value. Saving on commit is the TUI equivalent of "saved automatically". |
| Segmented controls, radios, checkboxes | Segmented row with `←→` · checkbox `[x]` with `space` · text fields become a one-line input. |
| `+ Add …` buttons on list pages | The key shown in the block heading (`n add unit`). `n` always adds to the focused block. |
| Row actions (edit · delete · test …) | Keys in the hint bar acting on the selected row (`e`, `d`, `t` …). |
| Modal dialogs, 420–540px | Centred floating overlay (`Clear` + bordered `Block`, ~76% width, top third) over the dimmed page, the same treatment as the command palette. |
| Status bar NORMAL chip + hints | Hint bar per page. The status line shows the mode (`INSERT`, `CONFIRM`) when it isn't NORMAL, and `✓ Synced HH:MM` on the right. |
| Medium status line `j/k section · [/] pages · p picker · esc settings` | The page's own keys, then `[/] pages · p picker · esc settings` at the end of every page's hint bar. |
| Header breadcrumb | The status line names the View as it does for every View: `Personal Ledger │ Settings` on the index page, `Personal Ledger │ Settings › <Page>` on a page, the page name in sentence case. |

**List pages and the full views.** Units, Accounts, Categories, Tags and Payees already have their own full TUI views (`../02-accounts`, `../03-categories`, `../04-payees`, `../06-tags`, `../07-units`). Their Settings pages are the compact management lists Desktop shows, and they use **the same forms and keys** as those views (`n`/`e`/`d`, typed delete confirm). `o` opens the full view, and `esc` from it returns to the Settings page. Their existing `g` jumps stay as they are.

**Considered:** keeping the earlier two-pane layout (a 20-col docked index beside the page, `j/k` swapping the page live, focus moving between index and page with `l` / `h`), and docking it only on wide terminals, like the Desktop's Large. One layout at every width was chosen so the TUI matches the Desktop's Medium, which the 96-col minimum is closest to. The index page has room to say what each page holds, every page gets the whole View, focus never has to move between panes, and the picker and `[` / `]` keep any page one step from any other. What it gives up is scanning pages without opening them; `[` / `]` is the replacement.

## Terminal geometry

Drawn at the shell's **96 × 30 cell** minimum.

```
row 0        status line   Personal Ledger │ Settings [› <Page>]      [MODE ·] ✓ Synced HH:MM
rows 1..n-2  view          the index page, or one page, Min(0)
row n-1      hint bar      the index page's keys, or the page's keys then [/] pages · p picker · esc settings
```

- **Index page (TUI-047):** heading row (`Settings` bold on the left, `16 pages` dim on the right) over a strong rule, then 16 rows, then `Min(0)`, then the 1-row legend `● syncs with the ledger · ○ this device only` pinned to the bottom over a rule. A row is the page name (16 cols), its scope glyph (2 cols), the one-line description, dim, and a dim `›` at the right edge. The selected row is reversed. The descriptions are Messages; they clip with `…` rather than wrap.
- **Page:** heading row (the breadcrumb `Settings › <Page>` on the left, `Settings ›` dim; the scope note dim on the right) over a strong rule, then the page body across the whole View. Bodies are `Length` blocks over a `Min(0)`. Pages that fill the height (Logs, History) give the `Min(0)` to their list.
- **Page picker (TUI-048):** a popup centred in the View, 40 cols wide and 20 rows tall (`Clear` + bordered `Block`): a title row (`Go to Settings Page` bold, `16 pages` dim) over a strong rule, then all 16 pages in index order, each with its scope glyph and a `•` on the page you are on. The page beneath dims.
- **Degrading below 96 columns:** drop the page's notes column, then the index page's descriptions, leaving names and glyphs. The picker keeps its size and stays centred.

## Keyboard

Settings has two places: the **index page** and a **page**. Opening a page from the index, `[` / `]` and the picker all swap what Settings shows in place. None of them push onto the Shell's view stack, so `esc` on a page always comes back to the index, and `esc` on the index leaves Settings.

| Key | Index page | A page |
| --- | --- | --- |
| `j` / `k` | next / previous page | next / previous field or row |
| `g` / `G` | first / last page | first / last field or row |
| `enter` / `l` | open the selected page | open the field to edit (form pages); page-specific on list pages |
| `[` / `]` | — | previous / next page in index order |
| `p` | — | open the page picker |
| `esc` | pop the view stack (leave Settings) | back to the index page, with this page selected; inside an editor, picker or dialog, close that first |
| `tab` | — | next block on pages with two (Units, AI Connections, Backup) |
| `u` / `ctrl r` | undo / redo the last change, anywhere in the app (History) | same |

In the page picker: `j` / `k` (or `↑` / `↓`) move · `g` / `G` first / last · `enter` opens the selected page · `esc` closes the picker and stays on the page. It opens with the current page selected. There is no filter: all 16 pages are in view, and the mode stays `NORMAL`.

Inside an open field (mode `INSERT`): `←→` choose · `space` toggle · `enter` save · `esc` keep the old value · `tab` save and move to the next field.

Page keys (shown in the hint bar):

| Page | Keys |
| --- | --- |
| Units, Institutions, Accounts, Categories, Tags, Payees, Documents, Inventory | `n` add · `e` edit · `d` delete (typed confirm) · `o` open the full view where one exists. Plus Desktop's per-page keys: Tags `x` remove, `m` merge · Documents and Inventory areas `J`/`K` reorder · Categories and Inventory `→`/`←` expand / collapse · Inventory `a` new area · Accounts `enter` open the account's ledger · Units `t` test a price source |
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
settings   [page]              opens that page directly; without a page, the index page
set        <field> [value]     edits a Preference or Configuration field; without a value, opens it in place
sync       now | edit
backup     now | export | git
undo | redo
```

`<page>` completes on the 16 index labels. A page opened by name sits on the index as if it had been opened from there, so `esc` goes back to the index page with it selected. `<field>` completes on the field names of the form pages (`fy-start`, `base`, `date-style`, `glyphs` …). TUI-020 shows the command line echoing the equivalent `:set`, which is how users learn the typed form.

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
| **Inventory** (TUI-046) | ● ledger data | A locations tree, NAME only, as Desktop's 16q. Location rows show the name and a muted "n areas · n items" note; Area rows are indented under them, name only. Address and cover are kept on the register in Inventory, not here. `n` new location (name and Unit only), `a` new area under the selected location, `J`/`K` reorder areas, `x` remove (asks where to move items). |
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
  showing: Index | Page                                     // g s and :settings show Index
  selected: General | Display | Units | Institutions | Accounts | Categories | Tags | Payees | Documents
      | Inventory | AiConnections | SyncServer | Backup | History | Logs | About
      // the index selection and the open page are one value: esc from a page lands on it,
      // and [ / ] and the picker move it. Remembered per session.
  page_state: per page — selected field/row, focused block, expanded tree nodes
  editor: None | Field { field, candidate, was }            // INSERT
  picker: None | Picker { selected }
  overlay: None | BaseUnitGuard { to, typed } | EditSyncServer | SetupGitBackup
         | AddProvider | EditProvider(id) | AddAssistant | the domain forms (unit, account, …)
```

The page order is one list, shared by the index page, the picker and `[` / `]`, so the three never drift. Each page's description and label are Messages beside it.

The data is Desktop's (`preferences` / ledger data / `configuration`, see its State block). Don't cache counts: derive them on each draw from the same queries the domain views use.

## Style

Use theme roles, not the wireframe hex values:

| Role | Used here for |
| --- | --- |
| accent | the open field's border, missing prices, failed tests, link text |
| dim | labels, scope notes, the index descriptions and legend, `Settings ›` in the breadcrumb, the page under the picker, undone History rows, disabled hints |
| selection (reversed) | the selected index or picker row, the selected field or row on a page, the chosen segment, the History marker |
| header bar | the status line |

Never rely on colour alone. The scope glyphs `●`/`○` are distinct shapes, failures use `■` plus words, and disabled keys are named as disabled in the dialog's own note.

## Known gaps

1. **The Desktop mockup itself predates ADR-0021** on Display (it still shows date format and separators). The TUI follows the ADR. Raise the same correction on the Desktop package.
2. **AI Connections and Git Backup** are decided in [ADR-0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md) and [ADR-0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md). The Desktop mockup still shows Read & write as selectable and says Git backup "commits the store"; build to the ADRs.
3. **`bin-tui` still draws the retired two-pane groups layout** (`src/view/settings.rs`). Stage 1 of `CLAUDE_CODE.md` replaces it with the index page and page host.

## Files

- `Ledger TUI Settings.dc.html` — TUI-047 and TUI-048, then TUI-019–021 and TUI-032–046 in index order. Open in a browser with `support.js` beside it.
- `support.js` — runtime for the HTML file, not part of the deliverable.
- `CLAUDE_CODE.md` — the staged work plan for building this design in `bin-tui`.
