# Claude Code: TUI Settings view

This is the work plan for rebuilding `bin-tui`'s Settings view to the paged design in this folder. The **specification is `README.md`**. This file covers only the order of work, the files each stage touches, and what "done" means. Each stage points at the README section that governs it, so don't implement from this file alone.

## Before you start

1. Read `README.md` end to end. Three sections carry decisions the drawings don't show: **"How the Desktop maps to the TUI"**, **"Keyboard"** and the **Pages** table. The Pages table records where the TUI deliberately differs from the Desktop mockup (Display follows ADR-0021, Payees is built to the Desktop's written spec, and Institutions uses one account-type enum).
2. Open `Ledger TUI Settings.dc.html` in a browser, with `support.js` beside it. It is a **design reference drawn in HTML**, not code to port. Read all geometry in **terminal cells**.
3. Read the ADRs the pages rest on: [0014](../../../adr/0014-preferences-table-and-leaner-sync-server-config.md) (Preference vs Configuration), [0021](../../../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md) (Locale, date style), [0023](../../../adr/0023-colour-theme-preferences-and-theme-role-overrides.md)/[0024](../../../adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md) (colour), [0027](../../../adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md) (Toasts), [0033](../../../adr/0033-git-backup-pushes-a-text-dump-and-documents-to-a-private-repository.md) (Git Backup) and [0034](../../../adr/0034-ai-connections-are-per-client-and-assistant-access-starts-read-only.md) (AI Connections). The scope glyph on each index row and page comes from these ADRs, so get it right.
4. Read `crates/bins/bin-tui/src/view/settings.rs` and `src/popup/settings/`. They implement the **retired** "database-backed settings" design: a groups pane, the "where values live" box, a fake `settings` override table, the §4b editor and the §4c guard. Most of that goes. **Keep what's real:** the Display group's Toasts, Colour Theme and Appearance rows, the `Action::SetToasts`/`OpenColourThemePopup`/`OpenColourAppearancePopup` paths, `popup::settings::colour`, and the read-only Locale row.

The Desktop already builds this destination (`crates/bins/bin-desktop/src/view/settings/`). Use its section enum, scope notes and page order as the source for **what** each page holds, and this folder for **how** the TUI shows it.

**Stage 1 is a prerequisite for everything else.** Stages 2–5 are independent of each other once it lands.

Each stage is one ticket's worth of work. Run `/tracing` and `/unit-tests` as you go, and run `mise run lint` before every push.

## Stage 1 — Index pane and page host

**Files:** `src/view/settings.rs` (split it into `src/view/settings/{mod,index}.rs` plus one file per page, following the Desktop's `view/settings/` layout), `src/view/mod.rs`, `src/shell.rs`, `i18n/en-US/settings.ftl`
**Spec:** README § *How the Desktop maps to the TUI*, § *Terminal geometry*, § *Keyboard* (the index/page focus table)

The work in this stage:
- Replace the two-pane groups layout with `Layout::horizontal([Length(20), Min(0)])`.
- The index lists the 16 pages in the Desktop's order, each with a `●`/`○` scope glyph, and a 2-row legend pinned to the bottom under a `Min(0)` gap.
- The page pane draws a heading row (`settings – <page>` on the left, the scope note on the right) over a strong rule, then hands the rest of its area to the page.
- `SettingsView` gains `focus: Index | Page` and `page: SettingsPage`, plus per-page state.

Done when:
- `j/k` and `g/G` in the index **swap the page live** without moving focus.
- `l`/`enter` focuses the page, and `h`/`esc` returns to the index.
- `esc` in the index leaves the `Shell` view stack, as today.
- The index row is reversed while the index has focus. While the page has focus, it is bold with a 1-col accent bar.
- The last-visited page is remembered for the session, so `g s` reopens it.
- `:settings <page>` opens a page directly, and `<page>` completes on the 16 labels in the command popup registry.
- Pages without content yet render their heading and scope note over an empty body, not a panic.
- The old `GROUPS`, the "where values live" box, the reset block, the fake settings table and their layout tests are deleted along with the `H log` and `r`/`R` key paths. Their replacement tests cover the index: every page listed, live swap, the focus round trip, and layout at 96×30 and on a taller terminal.
- All new text is Messages in `settings.ftl`, read through `msg::`. Page labels and scope notes are Messages. Command ids are not.

Don't build a shared "settings registry" type. The old design needed one for the key/value store, and this design has none.

## Stage 2 — Form pages: General and Display, in-place editing, base unit guard

**Files:** `src/view/settings/{general,display}.rs`, `src/popup/settings/{edit,guard}.rs`, `src/shell.rs`
**Spec:** README § *Pages* (General, Display), § *TUI-021 — Base unit guard*; cards TUI-019, TUI-020, TUI-021, TUI-032

The work in this stage:
- A form page is a list of field rows: label (22 cols), then value, then a dim note.
- `enter` opens a field **in place**: the row becomes a bordered, accent-framed box with the field's own sub-rows, and the rest of the page dims. The status line shows `INSERT`.
- `←→` chooses a segment, `space` toggles a checkbox, `enter` saves and `esc` keeps the old value.
- Rework `popup::settings::edit` from a centred overlay into the inline box. The README's "editing in place" is now the spec, not a simplification to work around.

Done when:
- **Display keeps every real behaviour.** Toasts, Colour Theme (`enter` opens the existing list popup) and Appearance still work, and the existing tests (`tab_focuses_display_where_enter_opens_each_colour_popup`, `under_terminal_colours_the_rows_name_the_key` and the rest) are migrated rather than deleted.
- **Date style** reads and writes the existing nullable Preference (`lib_core::DateStyle`), and **Locale** stays read-only with its source shown.
- Status glyphs, terminal colours and hint bar are Configuration written back to `personal-ledger.conf`. If `lib-config` has no write path yet, render the rows read-only, mark them `config file` in the note, and file an issue. Don't invent a writer in this stage.
- Each Display row carries its own scope glyph.
- **General** renders ledger name, owner, financial year starts and base unit, plus the THIS LEDGER box with counts derived from the existing stores.
  - Ledger name, owner and financial-year start have **no Preference columns yet**. Keep them as wireframe values and say so in the module doc.
  - Base unit is the existing default-Unit Preference.
- `:set <field> [value]` reaches the same code path as the key, and the command line echoes it while a field is open (TUI-020).
- **The base unit guard** (`popup::settings::guard`) opens when base unit is committed, not on a fixed `enter`.
  - Confirm by typing the unit code. `enter` stays dim until the text matches exactly.
  - Its facts may stay worked examples until the conversion queries exist. Put a `// wireframe:` comment on each one, never a fabricated "real" number.

## Stage 3 — Ledger-data list pages

**Files:** `src/view/settings/{units,institutions,accounts,categories,tags,payees,documents,inventory}.rs`, `src/view/mod.rs` (store accessors), `src/shell.rs`
**Spec:** README § *Pages*; cards TUI-033 and TUI-040–046

Build one list-page pattern and reuse it for every page:
- a block heading with its `n add …` key;
- a column header;
- rows whose selection is reversed;
- an optional `selected` detail box;
- dim notes pinned under a `Min(0)` gap.

Actions are keys, never row buttons.

Done when:
- **Units, Accounts, Categories, Tags and Payees** read the existing stores (`account_store`, `category_store`, `tag_store`, `payee_store` and the unit store), and `n`/`e`/`d` open **the existing forms** from `popup::{unit,account,category,tag,payee}`. Don't build parallel forms.
- `o` opens the full domain view and pushes Settings onto the view stack, so `esc` returns to the same page and row.
- Accounts `enter` opens the account's ledger, or shows the "not yet built" flash until that view exists.
- Categories and Inventory are trees: `→`/`←` expand and collapse, and `s` adds a subcategory under the selected category.
- Tags shows the duplicate flag in words with `m` merge.
- Documents and Inventory reorder with `J`/`K`.
- **Institutions, Documents and Inventory** have no TUI store yet. Render the README's worked data from a `fixture`-style module, clearly marked. Wire them when `lib-database` gains the entities (Documents: ADR-0031). Reuse any shared model crate that has landed by then (the Desktop's `inventory.rs`) rather than copying it.
- Counts in headings and scope notes are **derived** from the rows each draw, never stored.
- **Institutions uses one account-type enum.** If the codebase has two, raise it instead of picking one.

## Stage 4 — This-device pages

**Files:** `src/view/settings/{ai_connections,sync_server,backup,history,logs,about}.rs`, new `src/popup/settings/{sync_server,git_backup,provider,assistant}.rs`, `src/main.rs`
**Spec:** README § *Pages*; cards TUI-034–039; ADR-0033, ADR-0034

Done when:
- **About:** the version, date and links come from build metadata (`env!`/`option_env!`, the same source the Desktop uses). `enter` opens a link in the browser and `y` copies it.
- **Logs:** `main.rs` passes a `lib_tracing::LogBuffer` to `lib_tracing::init` instead of `None`, as the Desktop does for its Tracing page.
  - The page shows the level filter (`←→`), `C` clear and the buffer, newest first.
  - The scrollbar is always drawn and the log box takes the `Min(0)`.
- **Sync Server:** the page and its Edit overlay follow TUI-035. Save stays disabled until a test passes, and a failed test names its cause in plain words. The overlay may be wireframe until the Client can call the Sync Server. If it is, say so in the module doc.
- **Backup, AI Connections and History:** render to their cards with the ADR rules visible on screen:
  - Git Backup's `contents` row, plus the private-repo and LFS checks in its Test;
  - AI Connections shows Read & write dim as "not yet offered", and its status is a glyph plus a word;
  - History's undone rows are struck through above the `▸ CURRENT STATE` marker.
- None of these three has a backend yet (no Git Backup writer, no `lib-mcp` server, no undo stack). Ship them as wireframes with their keys in place: each key that has no behaviour yet shows the shell's "not yet built" flash rather than doing nothing.
- `u`/`ctrl r` are **not** bound app-wide in this stage. History's undo needs its own design first.
- Every secret field (access token, API key) renders masked and is never logged. Use `secrecy::SecretString` once any of them holds a real value (`/rust-style`).

## Stage 5 — Commands, hints and docs

**Files:** the command popup registry (`src/popup/command/commands/`), `docs/ux/tui-mockups/navigation.md`, `docs/settings.md`
**Spec:** README § *Command grammar*

Done when:
- `settings [page]`, `set <field> [value]`, `sync now|edit` and `backup now|export|git` are registry commands that reach the same code paths as their keys.
- Commands whose backend doesn't exist show "not yet built".
- Each page's hint bar is generated from the same key table its `handle_key` uses, so a key can't be shown without working, or work without being shown.
- `navigation.md`'s jump table lists `g s`, which `shell.rs` already binds.
- `docs/settings.md` describes the TUI Settings pages (run `/end-user-docs` if it needs restructuring).

## Throughout

- **Theme roles, not hex.** See the README's Style table. Use `crate::colours::Colours`.
- **Never rely on colour alone.** Scope is `●`/`○`, failures are `■` plus words, and disabled keys are drawn dim *and* named in the dialog's note.
- **Cells clip, they don't wrap.** Row heights are fixed. Degrade below 96 columns in the README's order: drop notes, then shorten the index, then hide the index behind `h`.
- **Wireframe honesty.** Any figure without a real query behind it lives in a clearly named fixture or carries a `// wireframe:` comment. The module doc says which parts are real.
- **Tests:** unit tests beside each page module for rendering at 96×30, key handling and focus. Use deterministic `fake` data for list pages. There are no doctests.
- No mouse support required.

## Out of scope

- **Out of scope:**
  - the schema for ledger name, owner and financial-year start;
  - Institutions, Documents and Inventory persistence in `lib-database`;
  - the Git Backup dump writer and importer (ADR-0033);
  - the `lib-mcp` server and Model Provider calls (ADR-0034);
  - the app-wide undo stack behind History;
  - writing Configuration back to `personal-ledger.conf`.
- Each of these needs its own ticket. This plan builds the screens they will plug into.
