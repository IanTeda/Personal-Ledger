//! Personal Ledger desktop library. Everything the `desktop` binary does lives here so the
//! integration tests under `tests/` can build the Shell's [`shell::router::Root`] through the same
//! [`shell::router::build`] the app does; `main.rs` is only config, tracing and a call to [`run`].
//! Boots into the Shell, the persistent-rail, command-palette-driven navigation frame
//! `docs/ux/desktop-mockups/README.md` specifies, in place of the feasibility cycle's flat `TabBar`
//! screen-cycling (ADR-0016).

mod account_deletion;
mod accounts;
mod assets;
mod bill_payments;
mod bills;
mod budgets;
mod categories;
mod dialog;
mod document_types;
mod documents;
mod error;
mod form;
mod help;
mod input;
mod institutions;
mod inventory;
mod ledger_store;
pub mod locale;
pub mod navigation;
#[doc(hidden)]
pub mod notices;
mod payees;
mod period;
pub mod persistence;
mod settings;
pub mod shell;
mod start_view;
mod tags;
pub mod theme;
mod transaction_import;
mod transactions;
mod unit_deletion;
mod units;
mod view;

/// This bin's own Messages, generated at build time from `i18n/<locale>/*.ftl`.
mod msg {
    include!(concat!(env!("OUT_DIR"), "/msg.rs"));
}

use std::borrow::Cow;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, prelude::*, px,
    size,
};

pub use error::Error;
use lib_database::{Storage, open_storage};
use persistence::{PersistedState, WindowGeometry};
pub use shell::view::window_size::{WindowSize, WindowSizeExt};

/// Crate Result type alias used across the Desktop binary.
///
/// Use `crate::Result<T>` for functions that return `T` or a `crate::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// Archivo (400/600/800), bundled at compile time rather than fetched from Google Fonts at
/// runtime the way the handoff's own mockup does -- the client is local-first and must render
/// offline. See `crates/bins/bin-desktop/assets/fonts/OFL.txt` for the license.
const ARCHIVO_REGULAR: &[u8] = include_bytes!("../assets/fonts/Archivo-Regular.ttf");
const ARCHIVO_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Archivo-SemiBold.ttf");
const ARCHIVO_EXTRA_BOLD: &[u8] = include_bytes!("../assets/fonts/Archivo-ExtraBold.ttf");

/// The Ledger file the Desktop opens at start: the one On start names, given the explicit `file`
/// and the Ledger open at the last quit (ADR-0051), or `None` to start with no Ledger open. There
/// is no fallback file: `data` is only the folder `:open` and `:new` start in (#977).
pub fn ledger_path(
    config: &lib_config::Config,
    persisted: &PersistedState,
) -> Option<std::path::PathBuf> {
    let config = config.personal_ledger_config();
    lib_core::ledger_to_open(
        config.on_start(),
        config.file(),
        persisted.last_ledger.as_deref(),
    )
}

/// What start-up made of the Ledger file On start names: open, none named, or a named file that
/// is missing or won't open.
pub enum Started {
    /// The Ledger file On start names, open on `storage`.
    Opened {
        path: std::path::PathBuf,
        storage: Box<Storage>,
    },
    /// On start names no Ledger file, so the Desktop starts with no Ledger open.
    NoLedger,
    /// The named file is missing or won't open, so the Desktop starts with no Ledger open and says
    /// why.
    Failed(OpenFailure),
}

/// A Ledger file start-up was told to open and couldn't: its path and why, as the Toast says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFailure {
    pub path: std::path::PathBuf,
    pub reason: String,
}

impl Started {
    /// The open Ledger's file, which the state file remembers at quit; `None` with none open.
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Opened { path, .. } => Some(path),
            Self::NoLedger | Self::Failed(_) => None,
        }
    }

    /// The open Ledger's storage, for a smoke test that started on a named Ledger; `None` with
    /// none open.
    #[doc(hidden)]
    pub fn storage(self) -> Option<Storage> {
        match self {
            Self::Opened { storage, .. } => Some(*storage),
            Self::NoLedger | Self::Failed(_) => None,
        }
    }
}

/// The Desktop's start-up before any window: opens the Ledger file [`ledger_path`] names, with the
/// stub dated from today, then generates the Bill Schedule through the horizon and commits the new
/// entries before the Shell is built (ADR-0019). Start-up never creates a Ledger file: with none
/// named it opens nothing, and a named file that is missing or won't open leaves no Ledger open,
/// which the Shell then says (#977). Only `:new` creates one. `main` starts here. The Locale is set
/// first, once and before any window opens, since the Toast saying why a file didn't open is
/// written in it.
pub fn start(config: &lib_config::Config) -> Started {
    let (requested_locale, locale_source) = config.personal_ledger_config().resolved_locale();
    locale::init(requested_locale, locale_source);
    start_on(config, chrono::Local::now().date_naive())
}

/// [`start`] as of `today`, on the Locale already set, so a smoke test generates on the date its
/// Shell is built for.
pub fn start_on(config: &lib_config::Config, today: chrono::NaiveDate) -> Started {
    start_with(config, &persistence::load(), today)
}

/// [`start_on`] given the state file's `persisted` state, so a test needn't read the real one.
fn start_with(
    config: &lib_config::Config,
    persisted: &PersistedState,
    today: chrono::NaiveDate,
) -> Started {
    let Some(path) = ledger_path(config, persisted) else {
        return Started::NoLedger;
    };
    match open_ledger_file(&path, today) {
        Ok(storage) => Started::Opened {
            path,
            storage: Box::new(storage),
        },
        Err(failure) => Started::Failed(failure),
    }
}

/// Opens the Ledger file at `path` as start-up and `:open` do, with the stub dated from `today`,
/// then generates the Bill Schedule through the horizon and commits the new entries (ADR-0019).
/// Never creates a Ledger file: a missing or empty file, a folder, or one that isn't a Ledger is
/// an [`OpenFailure`], written for the Toast that says so.
pub fn open_ledger_file(
    path: &std::path::Path,
    today: chrono::NaiveDate,
) -> std::result::Result<Storage, OpenFailure> {
    let failed = |reason: String| OpenFailure {
        path: path.to_path_buf(),
        reason,
    };
    if let Some(reason) = unopenable(path) {
        tracing::warn!(path = %path.display(), %reason, "Couldn't open the Ledger file");
        return Err(failed(reason));
    }
    match open_storage(path, today, &lib_locale::msg::institution_personal()) {
        Ok(mut storage) => {
            bills::ports::generate(&mut storage.bills, today);
            lib_database::Commit::commit(&storage.pending);
            tracing::info!(path = %path.display(), "Opened the Ledger file");
            Ok(storage)
        }
        Err(error) => {
            // Recovered: the Desktop keeps whatever Ledger was open, or none, and a Toast says why.
            // The error's own text is for the log, never spliced into a Message.
            tracing::warn!(%error, path = %path.display(), "Failed to open the Ledger");
            Err(failed(match error {
                lib_database::Error::LedgerOutOfDate(_) => msg::desktop_ledger_open_out_of_date(),
                _ => msg::desktop_ledger_open_unreadable(),
            }))
        }
    }
}

/// Creates a Ledger file at `path` and opens it as [`open_ledger_file`] does, for `:new`. Until the
/// new-Ledger workflow (#912) decides what a new Ledger starts with, it is the stub seed
/// `open_storage` writes into a new file. Never touches a file already at `path`: claiming the
/// name is the first write, and a file there by then is a failure. A Ledger that won't open once
/// created is removed again, so a failed `:new` leaves nothing behind.
pub fn create_ledger_file(
    path: &std::path::Path,
    today: chrono::NaiveDate,
) -> std::result::Result<Storage, OpenFailure> {
    let failed = |reason: String| OpenFailure {
        path: path.to_path_buf(),
        reason,
    };
    // `create_new` claims the name atomically, so a file made after the explorer checked the name
    // is refused rather than overwritten.
    if let Err(error) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        tracing::warn!(%error, path = %path.display(), "Couldn't create the Ledger file");
        return Err(failed(
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                msg::desktop_ledger_create_exists()
            } else {
                msg::desktop_ledger_create_unwritable()
            },
        ));
    }
    match open_storage(path, today, &lib_locale::msg::institution_personal()) {
        Ok(mut storage) => {
            bills::ports::generate(&mut storage.bills, today);
            lib_database::Commit::commit(&storage.pending);
            tracing::info!(path = %path.display(), "Created the Ledger file");
            Ok(storage)
        }
        Err(error) => {
            // Recovered: the Desktop keeps whatever Ledger was open, or none, and a Toast says why.
            tracing::warn!(%error, path = %path.display(), "Failed to create the Ledger");
            remove_ledger_file(path);
            Err(failed(msg::desktop_ledger_create_unwritable()))
        }
    }
}

/// Removes a Ledger file a failed `:new` made, with the journal files SQLite keeps beside it.
fn remove_ledger_file(path: &std::path::Path) {
    let mut files = vec![path.to_path_buf()];
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut side = path.as_os_str().to_owned();
        side.push(suffix);
        files.push(side.into());
    }
    for file in files {
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(%error, path = %file.display(), "Couldn't remove a failed Ledger file");
            }
        }
    }
}

/// Why `path` can't be opened without creating a Ledger in it, or `None` if `open_storage` may
/// open it. `open_storage` creates a missing file, and SQLite takes an empty one for a new database
/// that it then migrates and seeds, so start-up refuses both rather than create a Ledger (#977).
fn unopenable(path: &std::path::Path) -> Option<String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(msg::desktop_ledger_open_missing());
        }
        Err(error) => return Some(error.to_string()),
    };
    if !metadata.is_file() {
        Some(msg::desktop_ledger_open_not_a_file())
    } else if metadata.len() == 0 {
        Some(msg::desktop_ledger_open_empty())
    } else {
        None
    }
}

/// The Settings page the state file remembers at quit: the one on show, unless no Ledger is open
/// and the remembered one is a This Ledger page, which start-up swapped for General and a later
/// start with a Ledger open should still restore.
fn settings_page_at_quit(
    ledger_open: bool,
    showing: settings::SettingsPage,
    remembered: Option<&str>,
) -> String {
    match remembered {
        Some(id) if !ledger_open && settings::SettingsPage::from_id(id).needs_ledger() => {
            id.to_string()
        }
        _ => showing.id().to_string(),
    }
}

/// The Ledger file the state file remembers at quit: the one open, else the one remembered before,
/// so a Ledger that failed to open is tried again at the next start.
pub fn last_ledger_at_quit(
    open: Option<&std::path::Path>,
    remembered: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    open.or(remembered).map(std::path::Path::to_path_buf)
}

/// The Desktop's quit: commits the writes still queued, since the writer thread dies with the
/// process. `run` calls it as the app quits and the smoke test calls it in place of a window
/// closing.
pub fn quit(writer: &lib_database::Writer) {
    if let Err(error) = writer.flush() {
        tracing::error!(%error, "Failed to save the last Ledger writes");
    }
}

/// Opens the main window and runs the app until it quits. Tracing is already initialised, feeding
/// `logs` for the Tracing page, and `started` is what [`start`] made of the Ledger file: the Shell
/// opens it as its Open Ledger, or starts with none, raising the failure as a Toast.
pub fn run(config: &lib_config::Config, logs: lib_tracing::LogBuffer, started: Started) {
    let opened = started.path().map(std::path::Path::to_path_buf);
    let (ledger, failure) = match started {
        Started::Opened { path, storage } => (
            Some(shell::router::LedgerFile::opened(path, *storage)),
            None,
        ),
        Started::NoLedger => (None, None),
        Started::Failed(failure) => (None, Some(failure)),
    };
    let configured_level = Some(config.personal_ledger_config().log());
    config.theme_config().warn_invalid();
    let theme_overrides = config.theme_config().overrides.clone();
    let keybindings = config.keybindings_config().clone();
    // `noun`/window geometry survive restart (`docs/ux/desktop-mockups/README.md`'s "State
    // machine"), the noun for Start view's `last`; everything else in `NavState` starts fresh every
    // launch, so there's nothing else to seed here. A device's settings are Configuration (ADR-0053), not state.
    let persisted = persistence::load();
    let device_settings = config.personal_ledger_config().clone();
    let device_writer = config.writer().clone();
    // The file `start` opened, else the one remembered before, which the General page shows. The
    // state file keeps the one open at quit, which `:open` may have changed, for On start.
    let last_ledger = last_ledger_at_quit(opened.as_deref(), persisted.last_ledger.as_deref());
    let remembered_ledger = persisted.last_ledger.clone();
    let this_device = settings::general::ThisDevice {
        ledger_folder: config.personal_ledger_config().data().to_path_buf(),
        last_ledger,
        configuration: config.writer().target().map(std::path::Path::to_path_buf),
    };
    // What the state file holds that only a Ledger open at quit replaces.
    let remembered_settings_page = persisted.settings_page.clone();
    let remembered = (
        persisted.noun,
        persisted.documents_mode.clone(),
        persisted.documents_scope.clone(),
        persisted.documents_sort.clone(),
    );

    // Set once by `start`; the Locale never changes at runtime.
    let locale = locale::info().effective;

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            gpui_component::set_locale(locale::gpui_component_tag(locale));
            theme::colours::init(theme_overrides, cx);
            theme::colours::set_device_colours(&device_settings, cx);

            // Registered once, before any window opens, so every `Font { family: "Archivo".into(),
            // .. }` request resolves against the bundled weights rather than a fallback.
            // A failure here only costs the bundled face -- text still renders in gpui's
            // fallback font -- so it's reported rather than stopping the app.
            if let Err(error) = cx.text_system().add_fonts(vec![
                Cow::Borrowed(ARCHIVO_REGULAR),
                Cow::Borrowed(ARCHIVO_SEMIBOLD),
                Cow::Borrowed(ARCHIVO_EXTRA_BOLD),
            ]) {
                tracing::error!(%error, "Failed to register the bundled Archivo fonts");
            }

            let window_geometry = persisted.window;

            // Restore the last saved window geometry; otherwise 1280x800 centered, the handoff's
            // own window size (`docs/ux/desktop-mockups/01-shell/README.md`, option 1a).
            let bounds = match window_geometry {
                Some(WindowGeometry {
                    x,
                    y,
                    width,
                    height,
                }) => Bounds {
                    origin: point(px(x), px(y)),
                    size: size(px(width), px(height)),
                },
                None => Bounds::centered(None, size(px(1280.0), px(800.0)), cx),
            };

            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Medium is the floor (ADR-0042): no layout is designed narrower.
                    window_min_size: Some(shell::view::window_size::MIN_WINDOW),
                    // Wayland app_id / X11 WM_CLASS -- without this the window reports an
                    // empty class, which leaves window-manager rules (workspace assignment,
                    // etc.) with nothing to match on. Mirrors the packager identifier in
                    // this crate's Cargo.toml (`[package.metadata.packager]`).
                    app_id: Some("au.id.teda.personal-ledger.desktop".into()),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Personal Ledger".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    let focus_handle = cx.focus_handle();
                    // Grabs keyboard focus for the shell's own key handling (issue #149)
                    // as soon as the window opens -- nothing else in the window competes
                    // for it yet.
                    window.focus(&focus_handle);
                    // System follows the OS light/dark setting live. Held for the window's
                    // life, which is the app's, hence `detach()`.
                    window
                        .observe_window_appearance(|window, cx| {
                            theme::colours::set_system(window.appearance(), cx);
                        })
                        .detach();
                    cx.new(|cx| {
                        let mut shell = shell::router::build(
                            persisted,
                            device_settings.start_view(),
                            shell::router::ShellBindings { keybindings },
                            shell::router::ShellLogs {
                                buffer: logs,
                                configured_level,
                            },
                            ledger,
                            shell::router::wall_clock(chrono::Local::now().date_naive()),
                            focus_handle,
                            cx,
                        );
                        shell.set_configuration(&device_settings, device_writer, cx);
                        shell.set_this_device(this_device, cx);
                        if let Some(failure) = &failure {
                            shell.ledger_open_failed(failure);
                        }
                        shell
                    })
                },
            );
            // Without a window there is nothing to run, so report and quit rather than panic.
            let window_handle = match opened {
                Ok(handle) => handle,
                Err(error) => {
                    tracing::error!(%error, "Failed to open the main window");
                    cx.quit();
                    return;
                }
            };

            // No action mutates `NavState` yet (the keybinding/rail-toggle tickets do), so this
            // currently ever only re-saves whatever `persistence::load` produced -- registered now
            // anyway so the mechanism is real and exercised, rather than added later as an
            // afterthought. `on_app_quit`'s `Subscription` must outlive this closure to stay
            // registered, hence `detach()` (mirroring `cx.spawn(..).detach()` elsewhere in this
            // crate) rather than binding and dropping it.
            cx.on_app_quit(move |cx| {
                let _ = window_handle.update(cx, |shell, window, cx| {
                    // The writer thread dies with the process, so the Open Ledger's writes land first.
                    shell.flush_ledger();
                    let bounds = window.bounds();
                    // With no Ledger open the Shell has no Documents page, so the Documents state
                    // the state file remembers stays, and so does its noun unless Settings is
                    // showing (`start_view::noun_at_quit`), for Start view's `last`.
                    let (noun, documents_mode, documents_scope, documents_sort) =
                        match shell.documents_persisted(cx) {
                            Some((mode, scope, sort)) => (
                                shell.nav().noun(),
                                Some(mode.id().to_string()),
                                Some(scope.id()),
                                Some(sort.id().to_string()),
                            ),
                            None => {
                                let (noun, mode, scope, sort) = remembered.clone();
                                (
                                    start_view::noun_at_quit(
                                        shell.ledger_open(),
                                        shell.nav().noun(),
                                        noun,
                                    ),
                                    mode,
                                    scope,
                                    sort,
                                )
                            }
                        };
                    let state = PersistedState {
                        noun,
                        start_sidebar_minimised: shell.start_sidebar_minimised(cx),
                        settings_page: Some(settings_page_at_quit(
                            shell.ledger_open(),
                            shell.settings_page(cx),
                            remembered_settings_page.as_deref(),
                        )),
                        explorer_filters: shell.explorer_filters(),
                        documents_mode,
                        documents_scope,
                        documents_sort,
                        window: Some(WindowGeometry {
                            x: f32::from(bounds.origin.x),
                            y: f32::from(bounds.origin.y),
                            width: f32::from(bounds.size.width),
                            height: f32::from(bounds.size.height),
                        }),
                        last_ledger: if shell.last_ledger_forgotten() {
                            None
                        } else {
                            last_ledger_at_quit(shell.ledger_path(), remembered_ledger.as_deref())
                        },
                    };
                    let _ = persistence::save(&state);
                });
                async {}
            })
            .detach();

            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn config(file: Option<&str>) -> lib_config::Config {
        let mut config = lib_config::Config::default();
        lib_config::ConfigArgs {
            data: Some(PathBuf::from("/data")),
            file: file.map(PathBuf::from),
            ..Default::default()
        }
        .apply_overrides(&mut config);
        config
    }

    fn remembering(last_ledger: Option<&str>) -> PersistedState {
        PersistedState {
            last_ledger: last_ledger.map(PathBuf::from),
            ..Default::default()
        }
    }

    /// [`config`] with On start set as the conf file would set it.
    fn starting(file: Option<&str>, on_start: lib_core::OnStart) -> lib_config::Config {
        let mut config = config(file);
        config.personal_ledger.on_start = on_start;
        config
    }

    #[test]
    fn the_desktop_opens_the_ledger_on_start_names() {
        let reopen = lib_core::OnStart::ReopenLastLedger;
        assert_eq!(
            ledger_path(
                &starting(Some("/x/explicit.pldb"), reopen),
                &remembering(Some("/x/last.pldb"))
            )
            .as_deref(),
            Some(Path::new("/x/explicit.pldb"))
        );
        assert_eq!(
            ledger_path(&starting(None, reopen), &remembering(Some("/x/last.pldb"))).as_deref(),
            Some(Path::new("/x/last.pldb"))
        );
    }

    #[test]
    fn with_no_ledger_named_the_desktop_opens_none() {
        for (config, state) in [
            (
                starting(None, lib_core::OnStart::ReopenLastLedger),
                remembering(None),
            ),
            (
                starting(None, lib_core::OnStart::NoLedger),
                remembering(Some("/x/last.pldb")),
            ),
        ] {
            assert_eq!(ledger_path(&config, &state), None);
        }
    }

    /// A configuration whose `data` folder is `data` and whose `file`, if any, is `file`.
    fn config_in(data: &Path, file: Option<&Path>) -> lib_config::Config {
        let mut config = lib_config::Config::default();
        lib_config::ConfigArgs {
            data: Some(data.to_path_buf()),
            file: file.map(Path::to_path_buf),
            ..Default::default()
        }
        .apply_overrides(&mut config);
        config
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 3).expect("a valid date")
    }

    fn entries(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .expect("a readable directory")
            .count()
    }

    #[test]
    fn starting_with_no_ledger_named_creates_no_file() {
        locale::init_for_tests();
        let data = tempfile::tempdir().expect("a temporary directory");
        let started = start_with(
            &config_in(data.path(), None),
            &PersistedState::default(),
            today(),
        );
        assert!(matches!(started, Started::NoLedger));
        assert_eq!(entries(data.path()), 0, "no fallback Ledger is created");
    }

    #[test]
    fn a_missing_named_ledger_fails_to_open_and_creates_nothing() {
        locale::init_for_tests();
        let data = tempfile::tempdir().expect("a temporary directory");
        let missing = data.path().join("missing.pldb");
        for (config, persisted) in [
            (
                config_in(data.path(), Some(&missing)),
                PersistedState::default(),
            ),
            (
                config_in(data.path(), None),
                PersistedState {
                    last_ledger: Some(missing.clone()),
                    ..Default::default()
                },
            ),
        ] {
            let Started::Failed(failure) = start_with(&config, &persisted, today()) else {
                panic!("a missing Ledger file fails to open");
            };
            assert_eq!(failure.path, missing);
            assert_eq!(failure.reason, "no such file");
            assert_eq!(entries(data.path()), 0, "start-up creates no Ledger file");
        }
    }

    #[test]
    fn a_named_file_that_is_not_a_ledger_fails_to_open() {
        locale::init_for_tests();
        let data = tempfile::tempdir().expect("a temporary directory");
        let file = data.path().join("notes.pldb");
        std::fs::write(&file, "not a ledger").expect("a writable file");
        let started = start_with(
            &config_in(data.path(), Some(&file)),
            &PersistedState::default(),
            today(),
        );
        assert!(matches!(started, Started::Failed(failure) if failure.path == file));
    }

    #[test]
    fn a_named_ledger_that_exists_opens() {
        locale::init_for_tests();
        let data = tempfile::tempdir().expect("a temporary directory");
        let file = data.path().join("ledger.pldb");
        let created = open_storage(&file, today(), "Personal").expect("a new Ledger");
        quit(&created.writer);
        drop(created);
        let started = start_with(
            &config_in(data.path(), Some(&file)),
            &PersistedState::default(),
            today(),
        );
        assert_eq!(started.path(), Some(file.as_path()));
        assert!(started.storage().is_some());
    }

    #[test]
    fn an_empty_named_file_or_a_folder_fails_to_open_and_is_left_alone() {
        locale::init_for_tests();
        let data = tempfile::tempdir().expect("a temporary directory");
        let empty = data.path().join("empty.pldb");
        std::fs::write(&empty, "").expect("a writable file");
        let folder = data.path().join("folder.pldb");
        std::fs::create_dir(&folder).expect("a new folder");
        for (path, reason) in [(&empty, "the file is empty"), (&folder, "not a file")] {
            let Started::Failed(failure) = start_with(
                &config_in(data.path(), Some(path)),
                &PersistedState::default(),
                today(),
            ) else {
                panic!("{} fails to open", path.display());
            };
            assert_eq!(failure.reason, reason);
        }
        assert_eq!(
            std::fs::metadata(&empty).expect("still there").len(),
            0,
            "no Ledger is written into an empty file"
        );
    }

    #[test]
    fn with_no_state_file_and_no_conf_file_the_desktop_starts_with_no_ledger_open() {
        locale::init_for_tests();
        let home = tempfile::tempdir().expect("a temporary directory");
        let persisted = persistence::load_from(&home.path().join("desktop-state.json"));
        assert_eq!(persisted, PersistedState::default());
        let config = config_in(home.path(), None);
        assert_eq!(config.personal_ledger_config().file(), None);
        assert!(matches!(
            start_with(&config, &persisted, today()),
            Started::NoLedger
        ));
        assert_eq!(entries(home.path()), 0, "neither file is created");
    }

    #[test]
    fn with_no_ledger_open_a_remembered_this_ledger_settings_page_is_kept_at_quit() {
        use settings::SettingsPage;
        assert_eq!(
            settings_page_at_quit(false, SettingsPage::General, Some("units")),
            "units"
        );
        assert_eq!(
            settings_page_at_quit(false, SettingsPage::Display, Some("general")),
            "display"
        );
        assert_eq!(
            settings_page_at_quit(true, SettingsPage::General, Some("units")),
            "general"
        );
    }

    #[test]
    fn a_ledger_that_failed_to_open_is_still_remembered_at_quit() {
        let remembered = Path::new("/x/last.pldb");
        assert_eq!(
            last_ledger_at_quit(None, Some(remembered)).as_deref(),
            Some(remembered)
        );
        assert_eq!(
            last_ledger_at_quit(Some(Path::new("/x/open.pldb")), Some(remembered)).as_deref(),
            Some(Path::new("/x/open.pldb"))
        );
        assert_eq!(last_ledger_at_quit(None, None), None);
    }
}
