# Settings (development)

End-user documentation: [Settings](../settings.md).

## Overview

"Settings" covers two genuinely different things in this codebase, and confusing them is the commonest mistake in this domain:

- **Configuration** — deployment-time values needed before the app, or its database, can start. Read from INI files, environment variables and CLI flags by `lib-config`. Edited outside the running app. Never synced.
- **Preferences** — runtime values a user edits from inside a running Client. Ledger-scoped ones are stored in the Ledger's own database by `lib-database` and sync via Change Sets like other Ledger data; Client-scoped ones (Toasts, Colour Theme, Colour Appearance, date style and On start, ADR-0051) are kept by each Client in its own state file and never sync.

[ADR-0014](../adr/0014-preferences-table-and-leaner-sync-server-config.md) draws the line: Configuration covers only what is needed before the database can run — the database location and the tracing level. Everything else the user might tweak is a Preference. A third category, the **Locale**, is Configuration rather than a Preference by [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md), because it must be resolved before any UI string is rendered.

## Data model

### Configuration

No tables. `lib-config` deserialises INI into `LedgerConfig` (re-exported as `lib_config::Config`), whose four sections are:

| Section | Type | Read by |
| --- | --- | --- |
| `[Personal-Ledger]` | `PersonalLedgerConfig` | All three binaries |
| `[keybindings]` | `KeyBindingConfig` | `bin-tui`, `bin-desktop` (the Sync Server merges defaults but never reads them) |
| `[theme]` (plus `[theme.light]`, `[theme.dark]`) | `ThemeConfig` | `bin-tui`, `bin-desktop` |
| `[Sync-Server]` | `SyncServerConfig` | `bin-sync-server` only |

`PersonalLedgerConfig` fields: `config: Option<PathBuf>`, `data: PathBuf` (the folder `:open` starts in), `file: Option<PathBuf>` (an explicit override of On start, with no default), `log: lib_tracing::Levels`, `log_file_path: Option<PathBuf>`, `locale: Option<String>`, `terminal_colours: bool` (TUI only, ADR-0024). `SyncServerConfig`: `bind_address: String`, `database_uri: String`. `KeyBindingConfig`: a fixed `super_key: String` plus a `#[serde(flatten)]` `BTreeMap<String, String>` of command name to key, so commands stay open-ended.

Section headers are normalised before parsing — lower-cased with `-` replaced by `_` (`normalise_ini` in `ledger.rs`) — so `[Personal-Ledger]`, `[personal-ledger]` and `[personal_ledger]` are one section.

### Preferences

`preferences` (`migrations/client/20260909000000_create_preferences_table.sql`):

| Column | Type | Notes |
| --- | --- | --- |
| `id` | UUID | Primary key, a `RowID` (UUIDv7) |
| `created_on` | TEXT | ISO-8601 UTC |
| `updated_on` | TEXT | Maintained by `trg_preferences_set_updated_on` |

Invariants the schema does not enforce:

- **Singleton by convention, not constraint.** There is a plain `RowID` primary key and no `CHECK (id = ...)`, mirroring `sync_users`. The singleton property rests on `find_only` and `get_or_create_default`.
- **No seed row in the migration.** `get_or_create_default` inserts the row in Rust at first use, because its `id` must be a genuine UUIDv7 and plain SQL cannot generate one.
- **No Preference columns yet.** The base Unit is the `ledger` table's (ADR-0044) and the display Preferences are Client-scoped (ADR-0051); the row waits for the Ledger name, owner and financial year.

### Client-scoped Preferences

`lib_core::ClientPreferences` (`client_preferences.rs`): `toasts_on`, the nullable `colour_theme`, `colour_appearance` and `date_style` (same meanings and defaults as before ADR-0051), and `on_start`. Every field defaults on its own, so a state file from before a field existed still loads. Each Client keeps one in its own JSON state file, beside the Ledger file open when it last quit (`last_ledger`):

- **Desktop** — `desktop-state.json`, `persistence::PersistedState`'s `preferences` and `last_ledger`.
- **TUI** — `tui-state.json`, `state::TuiState`.

Both live in `dirs::state_dir()`, else `dirs::data_local_dir()`, under `personal-ledger/`. A missing or corrupt file loads as the defaults.

Invariants:

- **`date_style` nullability is load-bearing.** `None` is not "unset, use a hardcoded default" — it means defer to the Locale. Only an explicit ISO value overrides the Locale for both display and typed input.
- **`colour_theme` is kept raw.** A Client that doesn't know an id draws Modernist, logs a `warn` and never writes the default back (ADR-0023).

### On start

`lib_core::OnStart` is `ReopenLastLedger` (the default) or `NoLedger`. `lib_core::ledger_to_open(on_start, file, remembered)` is the one resolver both Clients ask at start: an explicit `file` wins; otherwise Reopen last Ledger opens the remembered path, or none, and Start with no Ledger open always says none. The Desktop still needs a Ledger before its window, so with none named it falls back to `<data>/My-Personal-Ledger.pldb` (`bin_desktop::ledger_path`) until #911. The TUI asks it through `Shell::open_on_start`, which opens the named file as `:open <path>` would: one that won't open raises `toast-ledger-open-failed` and the TUI runs with no Ledger open. The TUI remembers the Open Ledger's path as it quits, so `:close` then quit forgets it.

## Domain types

- `lib_core::DateStyle` (`date_style.rs`) — short, medium, long or ISO, behind the nullable `date_style` column. It replaced the former `NumberFormat` and `DateFormat` types.
- `lib_colour_theme` — the Colour Theme id set, `ColourAppearance` (`from_key` parses the `colour_appearance` column) and `resolve`, which turns the two Preferences, the `[theme]` overrides and the system light/dark setting into `ResolvedColours` plus contrast failures. See [Colour Themes design](../colour-themes-design.md).
- `lib_config::ThemeConfig` (`theme.rs`) — the `[theme]` role-to-`HexColor` map per Colour Variant; unparseable values and unknown keys are dropped with a `warn`.
- `lib_tracing::Levels` (`levels.rs`) — the `log` Configuration value.
- `lib_config::LocaleSource` — records whether the effective Locale came from `Default`, `System` or `Config`, so the Settings screen can show the Locale read-only with its provenance.
- `lib_config::DEFAULT_LOCALE` — `en-US`.

The number-separator Preference was deleted outright by ADR-0021: `en-US`, `en-GB` and `en-AU` all render `1,234.56`, so there was nothing for it to decide.

## Persistence

**Configuration** — `crates/libs/lib-config/src/`: `ledger.rs` (the loader and section normalisation), `personal_ledger.rs`, `sync_server.rs`, `keybindings.rs`, `theme.rs`, `cli.rs` (the shared `--config`/`--data`/`--file`/`--log`/`--locale` argument group flattened into each binary's parser), `error.rs`.

Two entry points, per ADR-0014:

- `Config::parse` — Clients. Precedence lowest to highest: built-in defaults, system config, user config, executable-directory config, the working directory's `config/personal-ledger.conf`, an explicit path, then environment variables.
- `Config::parse_for_sync_server` — the Sync Server. Defaults, explicit path, environment only. The system, user, executable-directory and working-directory tiers mean nothing inside a container.

Environment variables use the `PERSONAL_LEDGER_` prefix with double-underscore nesting, e.g. `PERSONAL_LEDGER_PERSONAL_LEDGER__LOCALE`. This requires `prefix_separator("_")` and `separator("__")` to be set separately on the `config` crate's `Environment` source, because it otherwise reuses one separator for both and nested keys never resolve.

**Preferences** — `crates/libs/lib-database/src/preferences/`: `model.rs`, `builder.rs`, `find.rs` (`Preferences::find_only`), `update.rs` (`Preferences::get_or_create_default`). There is no `delete.rs`: the row is never deleted, and no update until it has a column to update.

**Client-scoped Preferences** — the Desktop loads them in `lib.rs`'s `run` (the colours through `theme::colours::set_client_preferences`, the date style and Toasts through `Root::set_client_preferences` in `build`) and saves `Root::client_preferences` at quit. The TUI's `main` loads `tui-state.json`, applies it with `Shell::with_client_preferences`, and saves `Shell::client_preferences` and `Shell::quit_ledger` after `run`.

## UI

Neither Client's Settings screen is wired to `lib_database::Preferences`.

- **Desktop** — `crates/bins/bin-desktop/src/settings/` with eight sections: `general`, `display`, `units`, `institutions`, `sync_server`, `tracing`, `data_backup`, `about`, plus five dialogs (`add_unit_dialog`, `edit_unit_dialog`, `delete_unit_dialog`, `add_institution_dialog`). `general.rs` states outright that there is no real `Preferences` wiring in it — the values are stubs matching the mockup. Units and Institutions have working dialogs; at X-Large the Units page also keeps a selected row (`j`/`k`, `e`, `d`, `n`, `t`) and docks a 400px detail pane (`units/page/pane.rs`'s `detail_pane`, drawn by the Settings View at its right edge), where the row ACTIONS columns are dropped and Success outcomes land in the pane's ACTIVITY instead of raising Toasts (DESKTOP-125). The Documents page does too: the Documents page (`settings/documents/`, Document Types per ADR-0031, with its Add, Edit and Remove dialogs in `dialogs/`; see [Documents development](documents.md)). Inventory runs on `lib-inventory`'s Presenters (#867): `inventory/` holds the `InventoryPage` Entity (`view/page.rs`, `view/page/render.rs`), its Dialogs as Entities in the Dialog host (`view/dialog/`), the Router (`router.rs`) and the ports (`ports.rs`). The pure model is `lib_inventory`, re-exported as `src/inventory/mod.rs` (Locations, Areas, Items, the cover and every removal rule, `gpui`-free and unit-tested), which Documents reads for Inventory Item Document Links. Tests are `settings_inventory_keyboard.rs` and `settings_inventory_mouse.rs`. Tracing is live rather than stubbed: `settings/tracing/` holds the `TracingPage` Entity (`mod.rs`, which owns the log mirror and the list's `ListState`), `render.rs` (the level row and a virtualised `gpui::list` log box), the `ClearLogs` confirm (`apply.rs`, `dialogs/clear_logs.rs`) and `log.rs`, its pure model (a mirror of `lib_tracing::LogBuffer`, the level filter and the entry format) that `Shell` pulls into on a `Notify`-driven loop and splices into the list. `build_shell` takes the capture as `ShellLogs`, so the tests in `settings_tracing_keyboard.rs` and `settings_tracing_mouse.rs` push entries straight into a buffer and call `Harness::settle_logs`. See [Tracing](../tracing.md). Expanded state is session-only and nothing persists yet. `SettingsPage` (in `crate::settings`) carries each section's placeholder issue number and its scope note. The Settings index rail is `chrome/rail/settings_index.rs`. Spec: `docs/ux/desktop-mockups/16-settings/README.md`.
- **TUI** — `crates/bins/bin-tui/src/view/settings.rs` is at wireframe stage: the two-pane layout from `docs/ux/tui-mockups/05-settings/README.md` §4a, built before the database-backed registry behind it. It has no real row-navigation state — `selected` is hardcoded to `base unit` rather than driven by `j`/`k` — so the two popups in `popup/settings` are reached by fixed keys (`e` for the in-place editor, `enter` for the base-unit guard), each always showing its own worked example, rather than acting on a selected row. Its left column is widened to `units::view::units`'s `LEFT_COLUMN_WIDTH` so the two views line up when switching.

**Colour Theme and Colour Appearance** are the one working Preference pair in both Settings screens, held in memory while the Client runs and kept in its state file between runs:

- **Desktop** — `settings/colour_theme/mod.rs`, a full-width group under Display: a Light/Dark/System segmented control and a grid of preview cards, each drawn through `resolve` with this Client's `[theme]` overrides. `src/theme/colours.rs` holds the `Colours` Global and the two in-memory Preferences; a change re-resolves, replaces the Global, writes `gpui_component::Theme` and redraws. System follows the main window's appearance observer live. `navigation/command.rs` registers `theme <id>` and `appearance <light|dark|system>` palette commands.
- **TUI** — rows in the Display group of `view/settings.rs`; `popup/settings/colour.rs` is the §4b list popup with seven `█` swatches per Colour Theme, live preview on `j`/`k`, `Enter` keeps and `Esc` restores the exact prior value (null included). `src/colours.rs` builds ratatui `Style`s for the colour depth and `terminal_colours`; `src/detect.rs` probes mode 2031, OSC 11 and `COLORFGBG` once at start.

Both show the `[theme]` override note and any contrast failures `resolve` returns.

Both Clients show the effective Locale read-only, sourced from `lib-config`, with no control to change it — the Locale changes only by editing configuration and restarting.

## Traceability

Omitted: [settings.md](../settings.md) is a configuration reference and carries no requirement checklist, so there is nothing ticked to trace. Add the table here when `SET-` requirements are written.

## Decisions

- [ADR-0014](../adr/0014-preferences-table-and-leaner-sync-server-config.md) — the Configuration/Preference split, the preferences table, and the Sync Server's reduced precedence chain.
- [ADR-0021](../adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md) — the Locale owns formatting; `NumberFormat`/`DateFormat` Preferences are replaced by the nullable `DateStyle`, and the number-separator Preference is deleted.
- [ADR-0051](../adr/0051-colour-theme-appearance-and-date-style-are-client-scoped.md) — Colour Theme, Colour Appearance and date style are Client-scoped, kept beside Toasts and On start.
- [ADR-0010](../adr/0010-oauth2-pkce-native-app-auth.md) — why the Sync Server has one listener, and so one `bind_address`.
- [ADR-0022](../adr/0022-seven-stored-colour-roles-with-calculated-shades.md), [ADR-0023](../adr/0023-colour-theme-preferences-and-theme-role-overrides.md), [ADR-0024](../adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md), [ADR-0025](../adr/0025-lib-colour-theme-resolves-shared-colours-from-build-time-ini-files.md) — Colour Roles, the Colour Theme Preferences and `[theme]`, the TUI's RGB default, and `lib-colour-theme`.
- `GLOSSARY.md` — Colour Theme, Colour Role, Colour Variant, Colour Appearance, Preference, Configuration, Client, Ledger.
- Design: [Localisation design](../localisation-design.md) for how the Locale is resolved.

## Open questions and known gaps

- **Neither Settings screen reads or writes `Preferences`.** The row has no columns yet, so `get_or_create_default` has no production call site.
- **On start has no control on the Desktop yet.** The TUI's Settings › General sets it (`:set on-start reopen|none`); the Desktop reads and writes it back unchanged until #911.
- **The TUI Settings registry does not exist.** The wireframe is explicitly built ahead of "the database-backed registry behind it", and the view has no selection state.
- **Keybindings are parsed but unused.** `KeyBindingConfig::key_for` is called only from `lib-config`'s own tests; neither bin reads its shortcuts from configuration, and the `g`-leader grammar in [navigation-design.md](../navigation-design.md) is not wired to it.
- **No Change Set emission.** The preferences table is sync-ready in shape — a normal `RowID` and an `updated_on` trigger — but nothing emits Change Sets on write, so Ledger-scoped Preferences do not actually sync yet.
- **TUI System appearance is read once.** Live mode 2031 notifications aren't followed: crossterm 0.29's parser would swallow input after the unknown `CSI ?` sequence.
