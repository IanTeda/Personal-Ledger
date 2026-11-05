# Settings (development)

End-user documentation: [Settings](../settings.md).

## Overview

"Settings" covers two genuinely different things in this codebase, and confusing them is the commonest mistake in this domain:

- **Configuration** — deployment-time values needed before the app, or its database, can start. Read from INI files, environment variables and CLI flags by `lib-config`. Edited outside the running app. Never synced.
- **Preferences** — runtime values a user edits from inside a running Client. Ledger-scoped ones are stored in the Ledger's own database by `lib-database` and sync via Change Sets like other Ledger data; a device's settings (Toasts, Colour Theme, Colour Appearance, date style and On start) are Configuration in the `[Personal-Ledger]` section since ADR-0053, which Settings writes back through `lib_config::ConfigWriter`, and never sync. The TUI reads and writes them there (#981); the Desktop still keeps them in `desktop-state.json` until #982, so until then the two Clients don't share them.

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

`PersonalLedgerConfig` fields: `config: Option<PathBuf>`, `data: PathBuf` (the folder `:open` starts in), `file: Option<PathBuf>` (an explicit override of On start, with no default), `log: lib_tracing::Levels`, `log_file_path: Option<PathBuf>`, `locale: Option<String>`, `terminal_colours: bool` and `hint_bar: bool` (TUI only, ADR-0024), and a device's settings (ADR-0053): `toasts: bool`, `colour_theme: Option<String>`, `colour_appearance: ColourAppearance`, `date_style: Option<DateStyle>`, `status_glyphs: StatusGlyphs` (both Clients), `row_density: RowDensity` (Desktop only), `on_start: OnStart` and `start_view: StartView`. `StatusGlyphs` and `RowDensity` are `lib_core`'s, re-exported by `lib_config` and `lib_viper::display`, so the file is read into the type the Presenters draw with; they spell themselves (`StatusGlyphs::key`: `unicode`/`ascii`, the TUI's spellings since #904; `RowDensity::key`: `compact`/`regular`/`roomy`) and read case-insensitively (#1045). `colour_appearance`, `date_style`, `on_start` and `start_view` are read and written by their `:set` spellings (`light`, `default`/`iso`, `reopen`/`none`, `last`/`units`) through `serde(with)` modules, not their lib-core derives, so the file, the environment and `:set` take one value; `lib_config::date_style_key`, `on_start_key` and `StartView::key` spell them, and `ConfigKey::ALL` names every key Settings writes. `SyncServerConfig`: `bind_address: String`, `database_uri: String`. `KeyBindingConfig`: a fixed `super_key: String` plus a `#[serde(flatten)]` `BTreeMap<String, String>` of command name to key, so commands stay open-ended.

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

### A device's settings

ADR-0053 makes them Configuration: `PersonalLedgerConfig`'s six keys, `toasts`, `colour_theme`, `colour_appearance`, `date_style`, `on_start` and `start_view`, which both Clients read at start and write from Settings through `ConfigWriter` (#981, #982, #983, #984). Each Client's state file keeps only what it remembers itself: `tui-state.json` (`state::TuiState`) the `last_ledger` and the `last_view` Start view's `last` reads, and `desktop-state.json` (`persistence::PersistedState`) the `last_ledger`, the window geometry, the last noun (which Start view's `last` reads) and Settings page, the `:open` browser's filters and the Documents mode, scope and sort. A state file from before still loads, its `preferences` ignored and not migrated.

Both state files live in `dirs::state_dir()`, else `dirs::data_local_dir()`, under `personal-ledger/`. A missing or corrupt file loads as the defaults.

Invariants:

- **`date_style` nullability is load-bearing.** `None` is not "unset, use a hardcoded default" — it means defer to the Locale. Only an explicit ISO value overrides the Locale for both display and typed input.
- **`colour_theme` is kept raw.** A Client that doesn't know an id draws Modernist, logs a `warn` and never writes the default back (ADR-0023).

### On start

`lib_core::OnStart` is `ReopenLastLedger` (the default) or `NoLedger`. `lib_core::ledger_to_open(on_start, file, remembered)` is the one resolver both Clients ask at start: an explicit `file` wins; otherwise Reopen last Ledger opens the remembered path, or none, and Start with no Ledger open always says none. The Desktop asks it through `bin_desktop::ledger_path` in `start`, which returns a `Started`: `Opened` on the named file, `NoLedger` with none named, or `Failed` for a named file that is missing or won't open. Start-up checks the file before `open_storage`, since that would create a missing file and seed an empty one: `unopenable` fails a path that is missing, not a file or empty, and that reason is the Toast's, so start-up never creates or writes a Ledger file (#977). `run` builds the Shell on `Opened`'s stores, or with no Open Ledger, raising `toast-ledger-open-failed` through `Root::ledger_open_failed` on `Failed`, and `last_ledger_at_quit` keeps the remembered Ledger after a failure. Start-up and the Desktop's `:open` open a file through the one `bin_desktop::open_ledger_file`, which runs those checks, then `open_storage`, generates the Bill Schedule and commits it, and returns an `OpenFailure` whose reason is a Message, never the error's own text. `:open`'s file explorer starts in the `data` folder (else home); on Open, `Root::open_ledger_file` flushes the Open Ledger's writes, opens the picked file and only then swaps it in as a `shell::router::LedgerFile` (the stores, the path and the writer), landing on the Dashboard with `toast-ledger-opened`; a failure keeps the Open Ledger, or none, and raises `toast-ledger-open-failed` (#978). An `OpenLedger` flushes its writer as it drops, and the app flushes the Open Ledger and remembers its path (`Root::ledger_path`) at quit; the status line shows that path with the home folder as `~`. The TUI asks it through `Shell::open_on_start`, which opens the named file as `:open <path>` would: one that won't open raises `toast-ledger-open-failed` and the TUI runs with no Ledger open. The TUI remembers the Open Ledger's path as it quits, so `:close` then quit forgets it.

### Start view

`lib_core::StartView` is `Dashboard` (the default), `Last`, `Settings` or `Noun(String)`, the `start_view` key both Clients share (ADR-0053). Any word but the three is a noun, kept lower case and raw as `colour_theme` is, so a noun only one Client has, or a newer release's, never stops the other starting; a Client that doesn't know it lands on the Dashboard. `StartView::resolve(remembered)` turns `Last` into the View remembered at the last quit, or the Dashboard with none. The TUI's nouns and landing rule are `bin_tui::start_view`: `units`, `accounts`, `categories`, `payees` and `tags`, the Views a `g` jump opens, each needing a Ledger, so with none open `landing` returns the Dashboard; `settings` opens the index either way. `main` calls `Shell::land_on_start` after `open_on_start`, and `Shell::quit` keeps the View showing as `quit_view` (`start_view::noun`, `None` for Help and the placeholder Views), which `main` saves as `tui-state.json`'s `last_view`. Settings › General's row is a Choice of `start_view::CHOICES`, and `:set start-view` takes only those, so the TUI never writes a noun it can't land on. A Choice too wide for its row scrolls to keep the chosen segment in view (`FieldEditor::choice_scroll`). The Desktop's are `bin_desktop::start_view` (#983): every `Noun`, spelled lower case with `credit-cards` hyphenated, of which all but the Dashboard and Settings need a Ledger (`navigation::nav::needs_ledger`), so `landing(start_view, remembered, ledger_open)` returns the Dashboard for them with none open. `shell::router::build` takes the Start view and starts `NavState` on `landing`'s noun, in place of the unconditional restore of the state file's `noun`; Settings opens on the restored page, which the frame's Presenter (`lib_settings`, `SettingsFrameEvent::Restore`) swaps for General with no Ledger open. At quit `start_view::noun_at_quit` keeps the state file's `noun` while no Ledger is open, unless Settings is showing. Settings › General's Start view is a radio group of `start_view::CHOICES` (the Dashboard, `last`, Settings and the built Views: Transactions, Documents, Bills and Budgets), saved through `lib_settings::DeviceConfiguration::set_start_view`; a value from the file that isn't a choice checks none and is named under them (`general-start-view-unlisted`).

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

**A device's settings** — the Desktop applies the device's keys in `lib.rs`'s `run`: the colours through `theme::colours::set_device_colours` before the window opens, and the date style, Row density, Status glyphs, Toasts, On start and Start view through `Root::set_configuration`, which hands Settings › General's and Display's Presenters (`lib_settings::GeneralPresenter`, `DisplayPresenter`, #1039) the settings start-up read and a `settings::device::DeviceConfigFile`: the bin's `lib_settings::DeviceConfiguration`, a write port with a typed method per setting over the `ConfigWriter` and the shadows start-up found. Each method answers the saved value and file, the source that shadows the key, or a refusal (no file to write, or a failed write, logged at `error`); the Presenter puts the answer on the status line and shows, or hands on, only a saved value, and draws a control's `Set by` note from `DeviceConfiguration::shadow`. The pages' Router (`settings::router::DeviceRouter`) emits saved Toasts as `ViewEvent::ToastsChanged` and the Display output (date style, Row density, Status glyphs, Start Sidebar minimised) as `ViewEvent::DisplayChanged`, which `Root` fans out to the Transactions, Bills, Dashboard, Documents and Budgets Views, and applies a saved Colour Theme or Appearance (`theme::colours::apply`, deferred so every window refreshes after the input). Start Sidebar minimised is remembered in `desktop-state.json`, the one Display control that isn't Configuration. `:toasts`, `:theme` and `:appearance` go through the same Presenters (`SettingsView::set_toasts`, `choose_colour`). The contract every `DeviceConfiguration` keeps is `lib_settings::contract`, run on the in-memory fake (`lib_settings::fake::InMemoryDeviceConfiguration`) and on `DeviceConfigFile` over a temporary conf file. `start` reads On start from the configuration for `ledger_path`. The UI tests' `Harness` gives each Shell its own conf file (`Harness::conf`, `Harness::with_configuration`). `run` also hands General what it shows of this device (`lib_settings::ThisDevice`: the `data` folder, the Ledger file open and the config file) through `Root::set_this_device`. The TUI's `Shell::with_configuration` applies the five keys from `PersonalLedgerConfig` (Toasts onto its `Toasts`, the colour keys onto its `Colours`, the date style and On start into Settings' `Configuration`), and every change goes through `Shell::save_configuration`, which refuses a key with no file to write or one a higher source shadows, writes it, and only then lets the change apply: the `set` arms, `set_toasts` (the Toasts row and `:toasts`) and `keep_colour_preference` (`Enter` in a colour popup, which reverts the preview when the save is refused). `main` loads `tui-state.json` for `Shell::open_on_start` and saves `Shell::quit_ledger` after `run`.

## UI

Neither Client's Settings screen is wired to `lib_database::Preferences`.

- **Desktop** — `crates/bins/bin-desktop/src/settings/`, one folder per page. The frame (the page on show, the Index/Page focus, the Medium drill-down and the page picker) is `lib_settings::SettingsFramePresenter`, held by `SettingsView` (`state.rs`), drawn from its view model with `PageClicks`, its keys offered at the key route's module step after the open page's own, and its `SettingsRouter` (`router.rs`) raising `PageChanged`, which `Root` feeds to `NavInput::SettingsPage` (#1038); the bin keeps a page's icon and `for_reveal` (`mod.rs`). `lib_settings::SettingsPage` lists the pages in index order and `SettingsPage::scope_group` puts each in `ScopeGroup::ThisDevice` (`general`, `display`, `tracing`, `about`) or `ScopeGroup::ThisLedger` (`ledger` and the rest); the docked index, the Medium index page and the picker draw a heading per group, and `[`/`]` step through `SettingsPage::ALL`, so they run from one group into the next (#976). The Settings View is `Root`'s own, outside the Open Ledger: `SettingsPages` always holds the This device pages and holds the This Ledger ones (`LedgerPages`) only once `SettingsView::open_ledger` builds them on the Open Ledger, and the frame's Presenter refuses a This Ledger page with no Ledger open, so with no Ledger open the selection never lands on a This Ledger page (#977). `general/` holds On start, Toasts and the read-only Locale and `data` folder (DESKTOP-178); `ledger/` runs on `lib-settings` (#1041): `lib_settings::LedgerPresenter` keeps the identity fields and summary as the mockup's placeholders, with no real `Preferences` wiring, beside a real Base unit select that reads the active Units through the `LedgerUnits` port, answered by `lib-database`'s `StoredSettingsReads` over the Units store, and sets the base Unit through `LedgerPreferences`, `lib-database`'s `LedgerPreferencesAdapter`, which runs `lib-units`' `SetBaseUnit` on that store; the page's Router brings the Ledger's read copy in line and announces `Domain::Units`, and the page reads no Units from `Ledger`. Units and Institutions have working dialogs; at X-Large the Units page also keeps a selected row (`j`/`k`, `e`, `d`, `n`, `t`) and docks a 400px detail pane (`units/page/pane.rs`'s `detail_pane`, drawn by the Settings View at its right edge), where the row ACTIONS columns are dropped and Success outcomes land in the pane's ACTIVITY instead of raising Toasts (DESKTOP-125). The Documents page does too: the Documents page (`settings/documents/`, Document Types per ADR-0031, with its Add, Edit and Remove dialogs in `dialogs/`; see [Documents development](documents.md)). Inventory runs on `lib-inventory`'s Presenters (#867): `inventory/` holds the `InventoryPage` Entity (`view/page.rs`, `view/page/render.rs`), its Dialogs as Entities in the Dialog host (`view/dialog/`), the Router (`router.rs`) and the ports (`ports.rs`). The pure model is `lib_inventory`, re-exported as `src/inventory/mod.rs` (Locations, Areas, Items, the cover and every removal rule, `gpui`-free and unit-tested), which Documents reads for Inventory Item Document Links. Tests are `settings_inventory_keyboard.rs` and `settings_inventory_mouse.rs`. Tracing is live rather than stubbed and runs on `lib-settings` (#1040): `lib_settings::TracingPresenter` mirrors the capture behind the level filter (a session-only view that opens on the configured `log` level) through the `TracingLog` read port, and tells its `TracingRouter` how the list changed; `settings/tracing/` holds the `TracingPage` Entity (`mod.rs`, which holds the Presenter and the list's `ListState`, and takes keys through `lib_settings::keymap::tracing_keymap`), `render.rs` (the level row and a virtualised `gpui::list` log box, drawn from the view model with `PageClicks`), the Clear logs Dialog (`dialogs/clear_logs.rs`, an Entity in the Dialog host over `lib_settings::ClearLogsPresenter`) and `log.rs`, `CaptureLog`, which answers `TracingLog` over `lib_tracing::LogBuffer`. The capture feed (`shell/router/log_feed.rs`) signals the page with `TracingEvent::Appended` on a `Notify`-driven loop. `build_shell` takes the capture as `ShellLogs`, so the tests in `settings_tracing_keyboard.rs` and `settings_tracing_mouse.rs` push entries straight into a buffer and call `Harness::settle_logs`. See [Tracing](../tracing.md). Expanded state is session-only and nothing persists yet. `lib_settings::SettingsPage` carries each page's label, description and scope note. The Settings index rail is `chrome/rail/settings_index.rs`. Spec: `docs/ux/desktop-mockups/16-settings/README.md`.
- **TUI** — `crates/bins/bin-tui/src/view/settings.rs` is at wireframe stage: the two-pane layout from `docs/ux/tui-mockups/05-settings/README.md` §4a, built before the database-backed registry behind it. It has no real row-navigation state — `selected` is hardcoded to `base unit` rather than driven by `j`/`k` — so the two popups in `popup/settings` are reached by fixed keys (`e` for the in-place editor, `enter` for the base-unit guard), each always showing its own worked example, rather than acting on a selected row. Its left column is widened to `units::view::units`'s `LEFT_COLUMN_WIDTH` so the two views line up when switching.

**Colour Theme and Colour Appearance** are the one working colour pair in both Settings screens, held in memory while the Client runs and kept between runs in `personal-ledger.conf`:

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
- [ADR-0053](../adr/0053-a-devices-settings-are-configuration-in-the-conf-file.md) — a device's settings are Configuration in the conf file, not the Clients' state files.
- [ADR-0010](../adr/0010-oauth2-pkce-native-app-auth.md) — why the Sync Server has one listener, and so one `bind_address`.
- [ADR-0022](../adr/0022-seven-stored-colour-roles-with-calculated-shades.md), [ADR-0023](../adr/0023-colour-theme-preferences-and-theme-role-overrides.md), [ADR-0024](../adr/0024-tui-draws-colour-themes-in-rgb-with-opt-in-terminal-colours.md), [ADR-0025](../adr/0025-lib-colour-theme-resolves-shared-colours-from-build-time-ini-files.md) — Colour Roles, the Colour Theme Preferences and `[theme]`, the TUI's RGB default, and `lib-colour-theme`.
- `GLOSSARY.md` — Colour Theme, Colour Role, Colour Variant, Colour Appearance, Preference, Configuration, Client, Ledger.
- Design: [Localisation design](../localisation-design.md) for how the Locale is resolved.

## Open questions and known gaps

- **Neither Settings screen reads or writes `Preferences`.** The row has no columns yet, so `get_or_create_default` has no production call site.
- **The TUI Settings registry does not exist.** The wireframe is explicitly built ahead of "the database-backed registry behind it", and the view has no selection state.
- **Keybindings are parsed but unused.** `KeyBindingConfig::key_for` is called only from `lib-config`'s own tests; neither bin reads its shortcuts from configuration, and the `g`-leader grammar in [navigation-design.md](../navigation-design.md) is not wired to it.
- **No Change Set emission.** The preferences table is sync-ready in shape — a normal `RowID` and an `updated_on` trigger — but nothing emits Change Sets on write, so Ledger-scoped Preferences do not actually sync yet.
- **TUI System appearance is read once.** Live mode 2031 notifications aren't followed: crossterm 0.29's parser would swallow input after the unknown `CSI ?` sequence.
