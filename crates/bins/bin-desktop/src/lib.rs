//! Personal Ledger desktop library. Everything the `desktop` binary does lives here so the
//! integration tests under `tests/` can build the Shell's [`shell::router::Root`] through the same
//! [`shell::router::build`] the app does; `main.rs` is only config, tracing and a call to [`run`].
//! Boots into the Shell, the persistent-rail, command-palette-driven navigation frame
//! `docs/ux/desktop-mockups/README.md` specifies, in place of the feasibility cycle's flat `TabBar`
//! screen-cycling (ADR-0016).

mod assets;
mod bill_payments;
mod bills;
mod budgets;
mod dialog;
mod documents;
mod error;
mod form;
mod help;
mod import;
mod input;
mod inventory;
mod ledger_store;
pub mod locale;
pub mod navigation;
#[doc(hidden)]
pub mod notices;
mod period;
pub mod persistence;
mod settings;
pub mod shell;
pub mod theme;
mod transactions;
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
use lib_database::{Storage, Stores, open_storage};
use persistence::{PersistedState, WindowGeometry};
pub use shell::view::window_size::WindowSize;

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

/// The Desktop's start-up before any window: opens the Ledger file the configuration names,
/// creating and seeding it when new, with the stub dated from today, then generates the Bill
/// Schedule through the horizon and commits the new entries before the Shell is built
/// (ADR-0019). The app never runs on a file it can't save to, so a failure is the caller's to
/// report and stop on. `main` and the smoke test both start here.
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the Ledger file.
pub fn start(config: &lib_config::Config) -> lib_database::Result<Storage> {
    start_on(config, chrono::Local::now().date_naive())
}

/// [`start`] as of `today`, so a smoke test seeds and generates on the date its Shell is built for.
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the Ledger file.
pub fn start_on(
    config: &lib_config::Config,
    today: chrono::NaiveDate,
) -> lib_database::Result<Storage> {
    let path = config.personal_ledger_config().file();
    let mut storage = open_storage(path, today).inspect_err(|error| {
        tracing::error!(%error, path = %path.display(), "Failed to open the Ledger");
    })?;
    bills::ports::generate(&mut storage.bills, today);
    lib_database::Commit::commit(&storage.pending);
    Ok(storage)
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
/// `logs` for the Tracing page, and `storage` is the Ledger file [`start`] opened.
pub fn run(config: &lib_config::Config, logs: lib_tracing::LogBuffer, storage: Storage) {
    let writer = storage.writer.clone();
    let stores = Stores::from(storage);
    let configured_level = Some(config.personal_ledger_config().log());
    config.theme_config().warn_invalid();
    let theme_overrides = config.theme_config().overrides.clone();
    let keybindings = config.keybindings_config().clone();

    // Resolved once, before any window opens; the Locale never changes at runtime.
    let (requested_locale, locale_source) = config.personal_ledger_config().resolved_locale();
    let locale = locale::init(requested_locale, locale_source);

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            gpui_component::set_locale(locale::gpui_component_tag(locale));
            theme::colours::init(theme_overrides, cx);

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

            // `noun`/window geometry survive restart (`docs/ux/desktop-mockups/README.md`'s
            // "State machine"); everything else in `NavState` starts fresh every launch, so there's
            // nothing else to seed here.
            let persisted = persistence::load();
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
                        shell::router::build(
                            persisted,
                            shell::router::ShellBindings { keybindings },
                            shell::router::ShellLogs {
                                buffer: logs,
                                configured_level,
                            },
                            stores,
                            shell::router::wall_clock(chrono::Local::now().date_naive()),
                            focus_handle,
                            cx,
                        )
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
                quit(&writer);
                let _ = window_handle.update(cx, |shell, window, cx| {
                    let bounds = window.bounds();
                    let state = PersistedState {
                        noun: shell.nav().noun(),
                        start_sidebar_minimised: shell.start_sidebar_minimised(cx),
                        settings_page: Some(shell.settings_page(cx).id().to_string()),
                        explorer_filters: shell.explorer_filters(),
                        documents_mode: Some(shell.documents_persisted(cx).0.id().to_string()),
                        documents_scope: Some(shell.documents_persisted(cx).1.id()),
                        documents_sort: Some(shell.documents_persisted(cx).2.id().to_string()),
                        window: Some(WindowGeometry {
                            x: f32::from(bounds.origin.x),
                            y: f32::from(bounds.origin.y),
                            width: f32::from(bounds.size.width),
                            height: f32::from(bounds.size.height),
                        }),
                    };
                    let _ = persistence::save(&state);
                });
                async {}
            })
            .detach();

            cx.activate(true);
        });
}
